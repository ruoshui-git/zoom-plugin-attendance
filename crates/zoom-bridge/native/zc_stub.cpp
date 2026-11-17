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
