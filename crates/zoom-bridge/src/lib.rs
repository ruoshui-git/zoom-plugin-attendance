//! Safe Rust interface to the Zoom Plugin SDK for Windows.
//!
//! Layering:
//! ```text
//! Rust app ── zoom_bridge (safe API, this crate)
//!          ── ffi.rs (extern "C" decls; ALL unsafe FFI lives in this crate)
//!          ── native/*.cpp (thin C++ adapter, catches exceptions, UTF-8<->UTF-16)
//!          ── zToolSuiteIPCProxy.dll (official SDK, delay-loaded)
//!          ── IPC ── Zoom Workplace client
//! ```
//! If the SDK was not present at build time the adapter is a stub
//! ([`sdk_linked`] returns false) and [`Session::start`] fails cleanly.

mod dll;
pub mod events;
mod ffi;
pub mod session;
pub mod status;

pub use dll::{ensure_sdk_dll_loaded, sdk_runtime_beside_exe, DllError, SDK_DLL_NAME};
pub use events::{Event, MeetingInfo, Participant, ParticipantFlags};
pub use status::MeetingInstance;
pub use session::{MeetingViewType, Session, SessionConfig};

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("zoom-bridge was built without the Zoom Plugin SDK (stub mode)")]
    SdkNotLinked,
    #[error("a Zoom SDK session is already active in this process")]
    AlreadyActive,
    #[error("invalid argument passed to adapter")]
    InvalidArg,
    #[error("SDK call made in the wrong state")]
    WrongState,
    #[error("the SDK rejected the request")]
    SdkRejected,
    #[error("C++ exception caught at the adapter boundary")]
    Exception,
    #[error("adapter buffer too small")]
    BufferTooSmall,
    #[error("unknown adapter status {0}")]
    Unknown(i32),
    #[error(transparent)]
    Dll(#[from] DllError),
    #[error("thread error: {0}")]
    Thread(String),
}

pub(crate) fn check(code: i32) -> Result<(), BridgeError> {
    match code {
        ffi::ZC_OK => Ok(()),
        ffi::ZC_ERR_NOT_AVAILABLE => Err(BridgeError::SdkNotLinked),
        ffi::ZC_ERR_INVALID_ARG => Err(BridgeError::InvalidArg),
        ffi::ZC_ERR_WRONG_STATE => Err(BridgeError::WrongState),
        ffi::ZC_ERR_SDK_REJECTED => Err(BridgeError::SdkRejected),
        ffi::ZC_ERR_EXCEPTION => Err(BridgeError::Exception),
        ffi::ZC_ERR_BUFFER_TOO_SMALL => Err(BridgeError::BufferTooSmall),
        other => Err(BridgeError::Unknown(other)),
    }
}

/// ABI version reported by the compiled C++ adapter.
pub fn adapter_abi_version() -> u32 {
    // SAFETY: no arguments, no side effects.
    unsafe { ffi::zc_abi_version() }
}

/// ABI version this Rust crate was written against.
pub const RUST_ABI_VERSION: u32 = ffi::ZC_ABI_VERSION;

/// True if the adapter was compiled & linked against the real SDK.
pub fn sdk_linked() -> bool {
    // SAFETY: no arguments, no side effects.
    unsafe { ffi::zc_sdk_linked() == 1 }
}

/// SDK `bin/` directory used at build time (development fallback for DLLs).
pub fn build_time_sdk_bin_dir() -> Option<PathBuf> {
    let s = env!("ZC_SDK_BIN_DIR");
    (!s.is_empty()).then(|| PathBuf::from(s))
}

/// SDK root used at build time, if any.
pub fn build_time_sdk_root() -> Option<PathBuf> {
    let s = env!("ZC_SDK_ROOT");
    (!s.is_empty()).then(|| PathBuf::from(s))
}

/// Interop self-tests usable at runtime (`zoom-companion selftest`) and in
/// unit tests. They do not touch the SDK.
pub mod selftest {
    use super::*;
    use std::sync::mpsc;

    /// UTF-8 -> UTF-16 (C++) -> UTF-8 round trip.
    pub fn utf8_roundtrip(s: &str) -> Result<String, BridgeError> {
        let mut out = vec![0u8; s.len() * 3 + 16];
        let mut out_len = 0usize;
        // SAFETY: all buffers valid for the given lengths.
        check(unsafe {
            ffi::zc_selftest_utf8_roundtrip(
                s.as_ptr().cast(),
                s.len(),
                out.as_mut_ptr().cast(),
                out.len(),
                &mut out_len,
            )
        })?;
        out.truncate(out_len);
        String::from_utf8(out).map_err(|_| BridgeError::InvalidArg)
    }

    /// Raw bytes variant (to test rejection of invalid UTF-8).
    pub fn utf8_roundtrip_bytes(b: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let mut out = vec![0u8; b.len() * 3 + 16];
        let mut out_len = 0usize;
        // SAFETY: all buffers valid for the given lengths.
        check(unsafe {
            ffi::zc_selftest_utf8_roundtrip(
                b.as_ptr().cast(),
                b.len(),
                out.as_mut_ptr().cast(),
                out.len(),
                &mut out_len,
            )
        })?;
        out.truncate(out_len);
        Ok(out)
    }

    /// C++ invokes the Rust trampoline with `text`; returns the received event.
    pub fn callback_roundtrip(text: &str) -> Result<Event, BridgeError> {
        let (tx, rx) = mpsc::channel::<Event>();
        let user = &tx as *const mpsc::Sender<Event> as *mut std::ffi::c_void;
        // SAFETY: `tx` outlives the synchronous call.
        check(unsafe {
            ffi::zc_selftest_emit(crate::events::trampoline, user, text.as_ptr().cast(), text.len())
        })?;
        rx.try_recv().map_err(|_| BridgeError::Thread("no event received".into()))
    }

    pub fn async_result_roundtrip(operation: &str, code: i32, value: i64) -> Result<Event, BridgeError> {
        let (tx, rx) = mpsc::channel::<Event>();
        let user = &tx as *const mpsc::Sender<Event> as *mut std::ffi::c_void;
        check(unsafe { ffi::zc_selftest_emit_async(crate::events::trampoline, user, operation.as_ptr().cast(), operation.len(), code, value) })?;
        rx.try_recv().map_err(|_| BridgeError::Thread("no async event received".into()))
    }

    /// C++ builds a USER_STATUS event (via UTF-16 copies, like SDK users).
    pub fn users_roundtrip(names: &[&str]) -> Result<Event, BridgeError> {
        let (tx, rx) = mpsc::channel::<Event>();
        let user = &tx as *const mpsc::Sender<Event> as *mut std::ffi::c_void;
        let ptrs: Vec<*const std::ffi::c_char> = names.iter().map(|n| n.as_ptr().cast()).collect();
        let lens: Vec<usize> = names.iter().map(|n| n.len()).collect();
        // SAFETY: arrays have `names.len()` valid elements; `tx` outlives the call.
        check(unsafe {
            ffi::zc_selftest_emit_users(crate::events::trampoline, user, ptrs.as_ptr(), lens.as_ptr(), names.len())
        })?;
        rx.try_recv().map_err(|_| BridgeError::Thread("no event received".into()))
    }

    /// C++ throws internally; must surface as BridgeError::Exception.
    pub fn exception_boundary() -> Result<(), BridgeError> {
        // SAFETY: no arguments.
        check(unsafe { ffi::zc_selftest_throw() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_matches() {
        assert_eq!(adapter_abi_version(), RUST_ABI_VERSION);
    }

    #[test]
    fn meeting_view_type_conversions() {
        for (raw, view) in [(0, MeetingViewType::ActiveSpeaker), (1, MeetingViewType::Wall), (2, MeetingViewType::MultiSpeaker), (3, MeetingViewType::Other)] {
            assert_eq!(MeetingViewType::from_i32(raw), view);
            assert_eq!(view.to_i32(), raw);
        }
        assert_eq!(MeetingViewType::from_i32(99), MeetingViewType::Other);
    }

    #[test]
    fn utf8_roundtrip_unicode_names() {
        for name in ["Alice", "张三", "李小龍（香港）", "José Ñúñez", "👩‍💻 Dev", "Ελένη", ""] {
            assert_eq!(selftest::utf8_roundtrip(name).unwrap(), name);
        }
    }

    #[test]
    fn invalid_utf8_rejected() {
        let err = selftest::utf8_roundtrip_bytes(&[0x66, 0xff, 0xfe]).unwrap_err();
        assert!(matches!(err, BridgeError::InvalidArg));
    }

    #[test]
    fn callback_delivers_owned_event() {
        let ev = selftest::callback_roundtrip("参会者 王五").unwrap();
        assert_eq!(
            ev,
            Event::SelfTest { code: 42, value: -7, text: "参会者 王五".into() }
        );
    }

    #[test]
    fn async_result_event_copies_operation_and_scalars() {
        match selftest::async_result_roundtrip("GetUserRoleByUserID", 1, 5).unwrap() {
            Event::AsyncResult { operation, code, value, thread_id } => { assert_eq!(operation, "GetUserRoleByUserID"); assert_eq!(code, 1); assert_eq!(value, 5); assert_ne!(thread_id, 0); }
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn user_status_event_copies_unicode_participants() {
        let names = ["主持人 张伟", "Ana María", "😀 Emoji", ""];
        match selftest::users_roundtrip(&names).unwrap() {
            Event::UserStatus { action, error_code, instance, users, thread_id } => {
                assert_eq!(status::user_action_name(action), "USER_ACTION_SYNC_ALL");
                assert_eq!(status::proxy_error_name(error_code), "kZMToolSuiteProxyErrorsSuccess");
                assert_eq!(instance, MeetingInstance::Default as i32);
                assert_ne!(thread_id, 0);
                let got: Vec<&str> = users.iter().map(|u| u.name.as_str()).collect();
                assert_eq!(got, names);
                assert!(users[0].flags.host() && users[0].flags.myself());
                assert!(users[1].flags.audio_muted() && !users[1].flags.host());
            }
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn empty_user_list() {
        match selftest::users_roundtrip(&[]).unwrap() {
            Event::UserStatus { users, .. } => assert!(users.is_empty()),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn cpp_exception_does_not_unwind_into_rust() {
        assert!(matches!(selftest::exception_boundary(), Err(BridgeError::Exception)));
    }

    #[test]
    fn sdk_dll_loads_when_linked() {
        if !sdk_linked() {
            return;
        }
        let dir = build_time_sdk_bin_dir().expect("bin dir recorded at build time");
        let p = ensure_sdk_dll_loaded(Some(&dir)).expect("SDK DLL + dependencies load");
        assert!(p.ends_with(SDK_DLL_NAME), "{}", p.display());
    }

    #[test]
    fn stub_mode_session_fails_cleanly() {
        if sdk_linked() {
            return; // covered by the SDK integration test instead
        }
        let r = Session::start(SessionConfig {
            access_token: "not-a-real-token".into(),
            domain: "https://zoom.us".into(),
            dll_fallback_dir: None,
        });
        assert!(matches!(r, Err(BridgeError::SdkNotLinked)));
    }
}
