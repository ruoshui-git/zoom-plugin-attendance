//! Raw declarations mirroring `native/zc_bridge.h`.
//!
//! This module is the ONLY place that declares foreign functions. Keep it in
//! sync with the header (the `abi_version_matches` test guards drift).

#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_char, c_void};

pub const ZC_ABI_VERSION: u32 = 2;

pub const ZC_OK: i32 = 0;
pub const ZC_ERR_NOT_AVAILABLE: i32 = 1;
pub const ZC_ERR_INVALID_ARG: i32 = 2;
pub const ZC_ERR_WRONG_STATE: i32 = 3;
pub const ZC_ERR_SDK_REJECTED: i32 = 4;
pub const ZC_ERR_EXCEPTION: i32 = 5;
pub const ZC_ERR_BUFFER_TOO_SMALL: i32 = 6;

pub const ZC_EVENT_AUTH_RESULT: u32 = 1;
pub const ZC_EVENT_IPC_STATUS: u32 = 2;
pub const ZC_EVENT_MEETING_STATUS: u32 = 3;
pub const ZC_EVENT_USER_STATUS: u32 = 10;
pub const ZC_EVENT_OTHER: u32 = 50;
pub const ZC_EVENT_ASYNC_RESULT: u32 = 20;
pub const ZC_EVENT_SELFTEST: u32 = 999;

pub const ZC_USER_HOST: u32 = 1 << 0;
pub const ZC_USER_VIDEO_ON: u32 = 1 << 1;
pub const ZC_USER_MYSELF: u32 = 1 << 2;
pub const ZC_USER_SPOTLIGHTED: u32 = 1 << 3;
pub const ZC_USER_ATTENDEE: u32 = 1 << 4;
pub const ZC_USER_RAISED_HAND: u32 = 1 << 5;
pub const ZC_USER_AUDIO_MUTED: u32 = 1 << 6;
pub const ZC_USER_TALKING: u32 = 1 << 7;
pub const ZC_USER_IN_WAITING_ROOM: u32 = 1 << 8;

#[repr(C)]
pub struct zc_user {
    pub user_id: u32,
    pub flags: u32,
    pub audio_type: i32,
    pub name: *const c_char,
    pub name_len: usize,
}

#[repr(C)]
pub struct zc_event {
    pub kind: u32,
    pub code: i32,
    pub value: i64,
    pub text: *const c_char,
    pub text_len: usize,
    pub instance: i32,
    pub thread_id: u32,
    pub users: *const zc_user,
    pub user_count: usize,
}

#[repr(C)]
pub struct zc_meeting_info {
    pub error_code: i32,
    pub meeting_type: i32,
    pub topic: *const c_char,
    pub topic_len: usize,
    pub meeting_number: *const c_char,
    pub meeting_number_len: usize,
}

pub type zc_event_fn = unsafe extern "C" fn(user: *mut c_void, ev: *const zc_event);
pub type zc_users_fn = unsafe extern "C" fn(user: *mut c_void, users: *const zc_user, count: usize);
pub type zc_meeting_info_fn = unsafe extern "C" fn(user: *mut c_void, info: *const zc_meeting_info);

extern "C" {
    pub fn zc_abi_version() -> u32;
    pub fn zc_sdk_linked() -> i32;
    pub fn zc_init(cb: zc_event_fn, user: *mut c_void) -> i32;
    pub fn zc_start_auth(
        token: *const c_char,
        token_len: usize,
        domain: *const c_char,
        domain_len: usize,
    ) -> i32;
    pub fn zc_uninit() -> i32;

    pub fn zc_get_meeting_status(instance: i32, out_status: *mut i32) -> i32;
    pub fn zc_get_participants(instance: i32, cb: zc_users_fn, user: *mut c_void) -> i32;
    pub fn zc_get_myself(instance: i32, cb: zc_users_fn, user: *mut c_void) -> i32;
    pub fn zc_get_meeting_info(instance: i32, cb: zc_meeting_info_fn, user: *mut c_void) -> i32;
    pub fn zc_get_attendees(instance: i32, cb: zc_users_fn, user: *mut c_void) -> i32;
    pub fn zc_get_spotlighted(instance: i32, cb: zc_users_fn, user: *mut c_void) -> i32;
    pub fn zc_get_waiting_room(instance: i32, cb: zc_users_fn, user: *mut c_void) -> i32;
    pub fn zc_get_user(instance: i32, user_id: u32, cb: zc_users_fn, user: *mut c_void) -> i32;
    pub fn zc_start_meeting(number: i64, no_video: i32, no_audio: i32) -> i32;
    pub fn zc_join_meeting(name: *const c_char, name_len: usize, number: i64, password: *const c_char, password_len: usize) -> i32;
    pub fn zc_leave_meeting(instance: i32) -> i32;
    pub fn zc_end_meeting(instance: i32) -> i32;
    pub fn zc_get_user_role(instance: i32, user_id: u32) -> i32;
    pub fn zc_make_host(instance: i32, user_id: u32) -> i32;
    pub fn zc_reclaim_host(instance: i32) -> i32;
    pub fn zc_assign_cohost(instance: i32, user_id: u32) -> i32;
    pub fn zc_revoke_cohost(instance: i32, user_id: u32) -> i32;
    pub fn zc_get_unread_chat_count(instance: i32) -> i32;
    pub fn zc_admit_user(instance: i32, user_id: u32) -> i32;
    pub fn zc_admit_all(instance: i32) -> i32;
    pub fn zc_enable_waiting_room(instance: i32, enable: i32) -> i32;
    pub fn zc_is_waiting_room_on(instance: i32) -> i32;

    pub fn zc_selftest_utf8_roundtrip(
        input: *const c_char,
        in_len: usize,
        out: *mut c_char,
        out_cap: usize,
        out_len: *mut usize,
    ) -> i32;
    pub fn zc_selftest_emit(
        cb: zc_event_fn,
        user: *mut c_void,
        text: *const c_char,
        text_len: usize,
    ) -> i32;
    pub fn zc_selftest_emit_async(cb: zc_event_fn, user: *mut c_void, operation: *const c_char, operation_len: usize, code: i32, value: i64) -> i32;
    pub fn zc_selftest_throw() -> i32;
    pub fn zc_selftest_emit_users(
        cb: zc_event_fn,
        user: *mut c_void,
        names: *const *const c_char,
        name_lens: *const usize,
        count: usize,
    ) -> i32;
}
