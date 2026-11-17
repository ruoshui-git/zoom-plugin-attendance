// zc_common.cpp - SDK-independent parts of the adapter (always compiled).
#include "zc_users.hpp"
#include "zc_util.hpp"

#include <cstring>
#include <stdexcept>

extern "C" uint32_t zc_abi_version(void) { return ZC_ABI_VERSION; }

extern "C" int32_t zc_selftest_utf8_roundtrip(const char *in, size_t in_len, char *out,
                                              size_t out_cap, size_t *out_len) {
    ZC_GUARD_BEGIN
    if (out_len == nullptr) return ZC_ERR_INVALID_ARG;
    std::wstring w = zc::Utf8ToWide(in, in_len);
    std::string back = zc::WideToUtf8(w);
    *out_len = back.size();
    if (back.size() > out_cap) return ZC_ERR_BUFFER_TOO_SMALL;
    if (!back.empty()) std::memcpy(out, back.data(), back.size());
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_selftest_emit(zc_event_fn cb, void *user, const char *text,
                                    size_t text_len) {
    ZC_GUARD_BEGIN
    if (cb == nullptr) return ZC_ERR_INVALID_ARG;
    // Round-trip through UTF-16 exactly like SDK strings will be.
    std::string owned = zc::WideToUtf8(zc::Utf8ToWide(text, text_len));
    zc_event ev{};
    ev.kind = ZC_EVENT_SELFTEST;
    ev.code = 42;
    ev.value = -7;
    ev.text = owned.data();
    ev.text_len = owned.size();
    cb(user, &ev);
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_selftest_emit_users(zc_event_fn cb, void *user, const char *const *names,
                                          const size_t *name_lens, size_t count) {
    ZC_GUARD_BEGIN
    if (cb == nullptr || (count > 0 && (names == nullptr || name_lens == nullptr)))
        return ZC_ERR_INVALID_ARG;
    zc::UserBatch batch;
    for (size_t i = 0; i < count; ++i) {
        // Simulate SDK data: a NUL-terminated wchar_t* name.
        std::wstring w = zc::Utf8ToWide(names[i], name_lens[i]);
        zc::OwnedUser u;
        u.user_id = (uint32_t)(16778240 + i);
        u.flags = (i == 0) ? (ZC_USER_HOST | ZC_USER_MYSELF) : ZC_USER_AUDIO_MUTED;
        u.name = zc::WideToUtf8(w.c_str());
        batch.Add(std::move(u));
    }
    zc_event ev{};
    ev.kind = ZC_EVENT_USER_STATUS;
    ev.code = 1; /* USER_ACTION_SYNC_ALL */
    ev.value = 1; /* kZMToolSuiteProxyErrorsSuccess */
    ev.instance = 1;
    ev.thread_id = ::GetCurrentThreadId();
    ev.users = batch.Data();
    ev.user_count = batch.Size();
    cb(user, &ev);
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_selftest_throw(void) {
    ZC_GUARD_BEGIN
    throw std::runtime_error("selftest");
    ZC_GUARD_END
}
