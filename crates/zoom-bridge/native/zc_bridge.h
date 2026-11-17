/*
 * zc_bridge.h - minimal C ABI between Rust and the Zoom Plugin SDK (C++).
 *
 * Design rules (see docs/ARCHITECTURE.md):
 *  - Only plain C types cross this boundary. All strings are UTF-8 with an
 *    explicit length (never rely on NUL termination of SDK data).
 *  - Every exported function catches all C++ exceptions and returns a status.
 *  - SDK-owned memory is never handed to Rust: event payloads are copied into
 *    adapter-owned buffers that are valid ONLY for the duration of the
 *    callback. Rust must copy what it needs before returning.
 *  - One global SDK session per process (the SDK itself is a process-global
 *    singleton: InitZMToolSuite / UninitZMToolSuite).
 *  - Callbacks may arrive on an SDK thread; the Rust side must be thread-safe.
 */
#ifndef ZC_BRIDGE_H
#define ZC_BRIDGE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define ZC_ABI_VERSION 1u

/* Return codes of zc_* functions. */
enum {
    ZC_OK = 0,
    ZC_ERR_NOT_AVAILABLE = 1, /* adapter built without the Zoom SDK (stub)   */
    ZC_ERR_INVALID_ARG = 2,   /* null pointer / bad UTF-8 / bad length        */
    ZC_ERR_WRONG_STATE = 3,   /* e.g. zc_start_auth before zc_init            */
    ZC_ERR_SDK_REJECTED = 4,  /* SDK call returned false / error code         */
    ZC_ERR_EXCEPTION = 5,     /* C++ exception caught at the boundary         */
    ZC_ERR_BUFFER_TOO_SMALL = 6
};

/* Event kinds delivered through zc_event_fn. `code` carries the raw SDK
 * enum value; `text` carries an optional UTF-8 string (e.g. error message). */
enum {
    ZC_EVENT_AUTH_RESULT = 1,    /* code = ZMToolSuiteProxyAuthResult        */
    ZC_EVENT_IPC_STATUS = 2,     /* code = ZMToolSuiteIPCConnectStatus, text */
    ZC_EVENT_MEETING_STATUS = 3, /* code = ZMToolSuiteMeetingStatus          */
    ZC_EVENT_USER_STATUS = 10,   /* OnUserStatusChanged: code = ZMToolSuiteUserAction,
                                    value = BaseResult.error_code, users/user_count */
    ZC_EVENT_OTHER = 50,         /* any other listener callback; text = name,
                                    code/value = first scalar args          */
    ZC_EVENT_SELFTEST = 999
};

/* Participant flags (zc_user.flags), mirroring ZMToolSuiteUser bools. */
enum {
    ZC_USER_HOST = 1u << 0,
    ZC_USER_VIDEO_ON = 1u << 1,
    ZC_USER_MYSELF = 1u << 2,
    ZC_USER_SPOTLIGHTED = 1u << 3,
    ZC_USER_ATTENDEE = 1u << 4,
    ZC_USER_RAISED_HAND = 1u << 5,
    ZC_USER_AUDIO_MUTED = 1u << 6,
    ZC_USER_TALKING = 1u << 7,
    ZC_USER_IN_WAITING_ROOM = 1u << 8
};

/* A participant, copied from ZMToolSuiteUser. `name` is adapter-owned UTF-8,
 * valid only during the callback that delivers it. */
typedef struct zc_user {
    uint32_t user_id;
    uint32_t flags;
    int32_t audio_type; /* ZMToolSuiteAudioType */
    const char *name;
    size_t name_len;
} zc_user;

typedef struct zc_event {
    uint32_t kind;
    int32_t code;
    int64_t value;
    const char *text; /* UTF-8, NOT NUL-terminated necessarily; may be NULL */
    size_t text_len;
    int32_t instance;  /* ZMToolSuiteMeetingInstance, 0 if n/a */
    uint32_t thread_id; /* Win32 thread id the SDK invoked the listener on */
    const zc_user *users; /* ZC_EVENT_USER_STATUS only */
    size_t user_count;
} zc_event;

typedef void (*zc_event_fn)(void *user, const zc_event *ev);

/* Synchronous list result: `users` valid only during the callback. */
typedef void (*zc_users_fn)(void *user, const zc_user *users, size_t count);

/* Meeting info (password and join URL deliberately not exposed). */
typedef struct zc_meeting_info {
    int32_t error_code;   /* BaseResult.error_code (raw) */
    int32_t meeting_type; /* ZMToolSuiteMeetingType */
    const char *topic;
    size_t topic_len;
    const char *meeting_number;
    size_t meeting_number_len;
} zc_meeting_info;
typedef void (*zc_meeting_info_fn)(void *user, const zc_meeting_info *info);

/* Introspection */
uint32_t zc_abi_version(void);
/* 1 if compiled and linked against the real Zoom Plugin SDK, 0 for stub. */
int32_t zc_sdk_linked(void);

/* Lifecycle (mirrors PSDKTestDlg.cpp ordering).
 * zc_init: stores the callback (must stay valid until zc_uninit returns) and
 *          calls InitZMToolSuite().
 * zc_start_auth: StartToolSuiteAuth({token, domain}) then
 *          SetToolSuiteProxyListener(adapter). Token/domain are UTF-8; the
 *          UTF-16 token copy is wiped after the call.
 * zc_uninit: UninitZMToolSuite(), SetToolSuiteProxyListener(nullptr), then
 *          clears the callback. After return no more callbacks.
 * All SDK calls should be made from the thread that called zc_init. */
int32_t zc_init(zc_event_fn cb, void *user);
int32_t zc_start_auth(const char *token, size_t token_len,
                      const char *domain, size_t domain_len);
int32_t zc_uninit(void);

/* Queries (synchronous IPC calls into Zoom Workplace). */
int32_t zc_get_meeting_status(int32_t instance, int32_t *out_status);
/* IMeetingToolkit::GetParticipantsList(); the SDK-owned vector is copied
 * before `cb` runs. Returns ZC_ERR_SDK_REJECTED if the SDK returned null. */
int32_t zc_get_participants(int32_t instance, zc_users_fn cb, void *user);
/* IParticipantsToolkit::GetMySelf() */
int32_t zc_get_myself(int32_t instance, zc_users_fn cb, void *user);
/* IMeetingToolkit::GetMeetingProperty() */
int32_t zc_get_meeting_info(int32_t instance, zc_meeting_info_fn cb, void *user);

/* Self-tests: exercise UTF-8 <-> UTF-16 conversion and the callback path
 * without the SDK. Used by the Rust test-suite. */
int32_t zc_selftest_utf8_roundtrip(const char *in, size_t in_len,
                                   char *out, size_t out_cap, size_t *out_len);
int32_t zc_selftest_emit(zc_event_fn cb, void *user,
                         const char *text, size_t text_len);
/* Throws a C++ exception internally; must return ZC_ERR_EXCEPTION. */
int32_t zc_selftest_throw(void);
/* Emits a ZC_EVENT_USER_STATUS event with the given UTF-8 names (array of
 * ptr/len pairs), built through the same UTF-16 copy path as SDK users. */
int32_t zc_selftest_emit_users(zc_event_fn cb, void *user, const char *const *names,
                               const size_t *name_lens, size_t count);

#ifdef __cplusplus
}
#endif

#endif /* ZC_BRIDGE_H */
