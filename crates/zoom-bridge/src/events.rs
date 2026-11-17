//! Owned, thread-safe event values produced from adapter callbacks.

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::Sender;
use serde::Serialize;

use crate::ffi;

/// A participant as reported by Zoom Workplace (copied; owns its data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Participant {
    /// Meeting-scoped user id (not stable across meetings/rejoins).
    pub user_id: u32,
    pub name: String,
    pub flags: ParticipantFlags,
    /// Raw `ZMToolSuiteAudioType` (0 none, 1 VoIP, 2 telephony).
    pub audio_type: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ParticipantFlags(pub u32);

impl ParticipantFlags {
    pub fn host(self) -> bool { self.0 & ffi::ZC_USER_HOST != 0 }
    pub fn video_on(self) -> bool { self.0 & ffi::ZC_USER_VIDEO_ON != 0 }
    pub fn myself(self) -> bool { self.0 & ffi::ZC_USER_MYSELF != 0 }
    pub fn spotlighted(self) -> bool { self.0 & ffi::ZC_USER_SPOTLIGHTED != 0 }
    pub fn attendee(self) -> bool { self.0 & ffi::ZC_USER_ATTENDEE != 0 }
    pub fn raised_hand(self) -> bool { self.0 & ffi::ZC_USER_RAISED_HAND != 0 }
    pub fn audio_muted(self) -> bool { self.0 & ffi::ZC_USER_AUDIO_MUTED != 0 }
    pub fn talking(self) -> bool { self.0 & ffi::ZC_USER_TALKING != 0 }
    pub fn in_waiting_room(self) -> bool { self.0 & ffi::ZC_USER_IN_WAITING_ROOM != 0 }
}

/// Meeting metadata (password / join URL are intentionally not exposed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeetingInfo {
    pub error_code: i32,
    pub meeting_type: i32,
    pub topic: String,
    pub meeting_number: String,
}

/// An event from the Zoom Plugin SDK. All data is owned (copied out of the
/// adapter buffer inside the callback), so events can cross threads freely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// `OnAuthResult`; raw `ZMToolSuiteProxyAuthResult`.
    AuthResult { code: i32, thread_id: u32 },
    /// `OnIPCConnectStatusChanged`; raw `ZMToolSuiteIPCConnectStatus` + message.
    IpcStatus { code: i32, message: String, thread_id: u32 },
    /// `OnMeetingStatusChanged`; raw `ZMToolSuiteMeetingStatus`.
    MeetingStatus { code: i32, thread_id: u32 },
    /// `OnUserStatusChanged`; `action` = raw `ZMToolSuiteUserAction`,
    /// `error_code` = raw `ZMToolSuiteProxyErrors`.
    UserStatus { action: i32, error_code: i32, instance: i32, users: Vec<Participant>, thread_id: u32 },
    /// Completion of a documented async SDK operation. `code` is raw error_code;
    /// `value` carries a role or count for the corresponding operation.
    AsyncResult { operation: String, code: i32, value: i64, thread_id: u32 },
    /// Any other listener callback, identified by name (for exploration).
    Other { name: String, code: i32, value: i64, instance: i32, thread_id: u32 },
    /// Produced only by self-tests.
    SelfTest { code: i32, value: i64, text: String },
    /// Unrecognised kind (ABI drift); kept rather than dropped.
    Unknown { kind: u32, code: i32, value: i64, text: String },
}

/// Copy a (ptr,len) UTF-8 buffer into an owned String.
///
/// # Safety
/// `ptr` must be null or valid for `len` bytes for the duration of the call.
pub(crate) unsafe fn copy_text(ptr: *const std::ffi::c_char, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    // SAFETY: guaranteed by caller (adapter contract: valid during callback).
    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    String::from_utf8_lossy(bytes).into_owned()
}

/// # Safety
/// `users` must be null or valid for `count` elements, each with valid names.
pub(crate) unsafe fn copy_users(users: *const ffi::zc_user, count: usize) -> Vec<Participant> {
    if users.is_null() || count == 0 {
        return Vec::new();
    }
    // SAFETY: caller contract.
    let slice = unsafe { std::slice::from_raw_parts(users, count) };
    slice
        .iter()
        .map(|u| Participant {
            user_id: u.user_id,
            // SAFETY: adapter contract — name valid for callback duration.
            name: unsafe { copy_text(u.name, u.name_len) },
            flags: ParticipantFlags(u.flags),
            audio_type: u.audio_type,
        })
        .collect()
}

/// # Safety
/// `ev` must point to a valid `zc_event` for the duration of the call.
pub(crate) unsafe fn convert(ev: *const ffi::zc_event) -> Option<Event> {
    // SAFETY: caller contract.
    let ev = unsafe { ev.as_ref()? };
    // SAFETY: adapter contract — text valid for callback duration.
    let text = unsafe { copy_text(ev.text, ev.text_len) };
    let thread_id = ev.thread_id;
    Some(match ev.kind {
        ffi::ZC_EVENT_AUTH_RESULT => Event::AuthResult { code: ev.code, thread_id },
        ffi::ZC_EVENT_IPC_STATUS => Event::IpcStatus { code: ev.code, message: text, thread_id },
        ffi::ZC_EVENT_MEETING_STATUS => Event::MeetingStatus { code: ev.code, thread_id },
        ffi::ZC_EVENT_USER_STATUS => Event::UserStatus {
            action: ev.code,
            error_code: ev.value as i32,
            instance: ev.instance,
            // SAFETY: adapter contract.
            users: unsafe { copy_users(ev.users, ev.user_count) },
            thread_id,
        },
        ffi::ZC_EVENT_ASYNC_RESULT => Event::AsyncResult { operation: text, code: ev.code, value: ev.value, thread_id },
        ffi::ZC_EVENT_OTHER => Event::Other {
            name: text,
            code: ev.code,
            value: ev.value,
            instance: ev.instance,
            thread_id,
        },
        ffi::ZC_EVENT_SELFTEST => Event::SelfTest { code: ev.code, value: ev.value, text },
        kind => Event::Unknown { kind, code: ev.code, value: ev.value, text },
    })
}

/// The single event callback trampoline handed to C++. `user` is a
/// `*const Sender<Event>` owned by the caller for at least as long as the
/// adapter may invoke the callback.
///
/// Panic boundary: nothing may unwind into C++. Any panic is caught & logged.
pub(crate) unsafe extern "C" fn trampoline(user: *mut c_void, ev: *const ffi::zc_event) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if user.is_null() {
            return;
        }
        // SAFETY: `user` is a live &Sender<Event> per the registration contract.
        let tx = unsafe { &*(user as *const Sender<Event>) };
        // SAFETY: `ev` is valid for the duration of this callback.
        if let Some(event) = unsafe { convert(ev) } {
            // Receiver may be gone during shutdown; that is fine.
            let _ = tx.send(event);
        }
    }));
    if result.is_err() {
        log::error!("panic caught in Zoom SDK callback trampoline (event dropped)");
    }
}

/// Synchronous list callback: `user` is `*mut Vec<Participant>` on the caller's stack.
pub(crate) unsafe extern "C" fn users_sink(user: *mut c_void, users: *const ffi::zc_user, count: usize) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if user.is_null() {
            return;
        }
        // SAFETY: caller passes a live, exclusive &mut Vec<Participant>.
        let out = unsafe { &mut *(user as *mut Vec<Participant>) };
        // SAFETY: adapter contract.
        *out = unsafe { copy_users(users, count) };
    }));
}

/// Synchronous meeting-info callback: `user` is `*mut Option<MeetingInfo>`.
pub(crate) unsafe extern "C" fn meeting_info_sink(user: *mut c_void, info: *const ffi::zc_meeting_info) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: caller contract.
        let (Some(out), Some(info)) = (unsafe { (user as *mut Option<MeetingInfo>).as_mut() }, unsafe {
            info.as_ref()
        }) else {
            return;
        };
        *out = Some(MeetingInfo {
            error_code: info.error_code,
            meeting_type: info.meeting_type,
            // SAFETY: adapter contract.
            topic: unsafe { copy_text(info.topic, info.topic_len) },
            // SAFETY: adapter contract.
            meeting_number: unsafe { copy_text(info.meeting_number, info.meeting_number_len) },
        });
    }));
}
