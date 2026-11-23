// zc_stub.cpp - compiled INSTEAD of zc_sdk.cpp when the Zoom Plugin SDK is not
// present. Every SDK entry point reports ZC_ERR_NOT_AVAILABLE so the Rust
// application can build, run its tests, and report a clear diagnostic.
#include "zc_util.hpp"

extern "C" int32_t zc_sdk_linked(void) { return 0; }
extern "C" int32_t zc_init(zc_event_fn, void *) { return ZC_ERR_NOT_AVAILABLE; }
extern "C" int32_t zc_start_auth(const char *, size_t, const char *, size_t) {
    return ZC_ERR_NOT_AVAILABLE;
}
extern "C" int32_t zc_uninit(void) { return ZC_ERR_NOT_AVAILABLE; }
extern "C" int32_t zc_get_meeting_status(int32_t, int32_t *) { return ZC_ERR_NOT_AVAILABLE; }
extern "C" int32_t zc_get_participants(int32_t, zc_users_fn, void *) {
    return ZC_ERR_NOT_AVAILABLE;
}
extern "C" int32_t zc_get_myself(int32_t, zc_users_fn, void *) { return ZC_ERR_NOT_AVAILABLE; }
extern "C" int32_t zc_get_meeting_info(int32_t, zc_meeting_info_fn, void *) {
    return ZC_ERR_NOT_AVAILABLE;
}
#define ZC_STUB(name, sig) extern "C" int32_t name sig { return ZC_ERR_NOT_AVAILABLE; }
ZC_STUB(zc_get_attendees, (int32_t, zc_users_fn, void *))
ZC_STUB(zc_get_spotlighted, (int32_t, zc_users_fn, void *))
ZC_STUB(zc_get_waiting_room, (int32_t, zc_users_fn, void *))
ZC_STUB(zc_get_user, (int32_t, uint32_t, zc_users_fn, void *))
ZC_STUB(zc_start_meeting, (int64_t, int32_t, int32_t))
ZC_STUB(zc_join_meeting, (const char *, size_t, int64_t, const char *, size_t))
ZC_STUB(zc_leave_meeting, (int32_t))
ZC_STUB(zc_end_meeting, (int32_t))
ZC_STUB(zc_get_user_role, (int32_t, uint32_t))
ZC_STUB(zc_make_host, (int32_t, uint32_t))
ZC_STUB(zc_reclaim_host, (int32_t))
ZC_STUB(zc_assign_cohost, (int32_t, uint32_t))
ZC_STUB(zc_revoke_cohost, (int32_t, uint32_t))
ZC_STUB(zc_get_unread_chat_count, (int32_t))
ZC_STUB(zc_admit_user, (int32_t, uint32_t))
ZC_STUB(zc_put_into_waiting_room, (int32_t, uint32_t))
ZC_STUB(zc_expel_user, (int32_t, uint32_t))
ZC_STUB(zc_admit_all, (int32_t))
ZC_STUB(zc_enable_waiting_room, (int32_t, int32_t))
ZC_STUB(zc_is_waiting_room_on, (int32_t))
ZC_STUB(zc_get_meeting_view_type, (int32_t))
ZC_STUB(zc_switch_meeting_view, (int32_t, int32_t))
ZC_STUB(zc_wall_view_page, (int32_t, int32_t))
#undef ZC_STUB
