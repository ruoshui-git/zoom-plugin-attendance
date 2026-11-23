// zc_sdk.cpp - real adapter over the Zoom Plugin SDK (compiled with ZC_HAVE_SDK).
//
// Verified against zoom-plugin-sdk-windows-7.2.0.3001 headers and the bundled
// PSDKTest sample. Ownership notes from the package:
//  * IMeetingToolkit::GetParticipantsList() returns IToolSuiteVector<ZMToolSuiteUser>*
//    which the sample never frees => SDK-owned; we copy immediately.
//  * ZMToolSuiteUser::strUserName is a const wchar_t* into SDK memory => copied.
//  * OnUserStatusChanged receives ZMToolSuiteUserList by value holding an
//    SDK-owned vector pointer => copied within the callback.
#include "zc_users.hpp"
#include "zc_util.hpp"

#include <mutex>

#include "ToolSuiteProxyInterface.h"
#include "ToolSuitePreMeetingInterface.h"
#include "ToolSuiteParticipantsInterface.h"
#include "ToolSuiteWaitingRoomInterface.h"
#include "ToolSuiteChatInterface.h"
#include "ToolSuiteUIActionInterface.h"

using namespace ZMToolSuiteProxy;

namespace {

std::mutex g_mu;
zc_event_fn g_cb = nullptr;
void *g_user = nullptr;
bool g_inited = false;

void Emit(zc_event &ev) {
    ev.thread_id = ::GetCurrentThreadId();
    std::lock_guard<std::mutex> lk(g_mu);
    if (g_cb) g_cb(g_user, &ev);
}

void EmitSimple(uint32_t kind, int32_t code, int64_t value = 0, int32_t instance = 0,
                const std::string *text = nullptr) {
    zc_event ev{};
    ev.kind = kind;
    ev.code = code;
    ev.value = value;
    ev.instance = instance;
    if (text) {
        ev.text = text->data();
        ev.text_len = text->size();
    }
    Emit(ev);
}

void EmitOther(const char *name, int64_t a = 0, int64_t b = 0, int32_t instance = 0) {
    std::string n(name);
    EmitSimple(ZC_EVENT_OTHER, (int32_t)a, b, instance, &n);
}

zc::OwnedUser CopyUser(const ZMToolSuiteUser &u) {
    zc::OwnedUser o;
    o.user_id = u.iUserID;
    o.audio_type = (int32_t)u.audioType;
    o.name = zc::WideToUtf8(u.strUserName);
    uint32_t f = 0;
    if (u.bHost) f |= ZC_USER_HOST;
    if (u.bVideoOn) f |= ZC_USER_VIDEO_ON;
    if (u.bMyself) f |= ZC_USER_MYSELF;
    if (u.bSpotlighted) f |= ZC_USER_SPOTLIGHTED;
    if (u.bAttendee) f |= ZC_USER_ATTENDEE;
    if (u.bRaisedHand) f |= ZC_USER_RAISED_HAND;
    if (u.bAudioMuted) f |= ZC_USER_AUDIO_MUTED;
    if (u.bTalking) f |= ZC_USER_TALKING;
    if (u.bInWaitingRoom) f |= ZC_USER_IN_WAITING_ROOM;
    o.flags = f;
    return o;
}

void CopyVector(IToolSuiteVector<ZMToolSuiteUser> *v, zc::UserBatch &out) {
    if (!v) return;
    const int n = v->GetCount();
    for (int i = 0; i < n; ++i) out.Add(CopyUser(v->GetItem(i)));
}

#define LISTENER_GUARD(body) \
    try {                    \
        body;                \
    } catch (...) {          \
    }

// Implements every pure virtual of IZMToolSuiteProxyListener for 7.2.0.3001.
// Callbacks not explicitly modelled are forwarded as ZC_EVENT_OTHER so the
// capability experiments can observe them without adapter changes.
class Listener final : public IZMToolSuiteProxyListener {
public:
    void OnAuthResult(ZMToolSuiteProxyAuthResult r) override {
        LISTENER_GUARD(EmitSimple(ZC_EVENT_AUTH_RESULT, (int32_t)r))
    }
    void OnIPCConnectStatusChanged(ZMToolSuiteIPCConnectStatus s, std::string msg) override {
        // errorMessage encoding is not documented; treat as UTF-8 (lossy in Rust).
        LISTENER_GUARD(EmitSimple(ZC_EVENT_IPC_STATUS, (int32_t)s, 0, 0, &msg))
    }
    void OnUserStatusChanged(ZMToolSuiteUserList list, ZMToolSuiteMeetingInstance inst) override {
        LISTENER_GUARD({
            zc::UserBatch batch;
            CopyVector(list.vUserList, batch);
            zc_event ev{};
            ev.kind = ZC_EVENT_USER_STATUS;
            ev.code = (int32_t)list.eUserAction;
            ev.value = (int64_t)list.error_code;
            ev.instance = (int32_t)inst;
            ev.users = batch.Data();
            ev.user_count = batch.Size();
            Emit(ev);
        })
    }
    void OnMeetingStatusChanged(ZMToolSuiteMeetingStatus s) override {
        LISTENER_GUARD(EmitSimple(ZC_EVENT_MEETING_STATUS, (int32_t)s))
    }
    void OnMeetingShareOptionsChanged(ZMToolSuiteShareOptions &o) override {
        LISTENER_GUARD(EmitOther("OnMeetingShareOptionsChanged", o.bShareSound, o.bOptimizeVideo))
    }
    void OnUserRaisedHand(ZMToolSuiteMeetingInstance i, unsigned int uid) override {
        LISTENER_GUARD(EmitOther("OnUserRaisedHand", 0, uid, i))
    }
    void OnUserLowerHand(ZMToolSuiteMeetingInstance i, unsigned int uid) override {
        LISTENER_GUARD(EmitOther("OnUserLowerHand", 0, uid, i))
    }
    void OnRecordingStatusChanged(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnRecordingStatusChanged", 0, 0, i))
    }
    void OnEmojiReactionReceived(unsigned int sender, SendEmojiReactionParam t,
                                 ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnEmojiReactionReceived", t.type, sender, i))
    }
    void OnEmojiReactionReceivedInWebinar(IToolSuiteVector<WebinarEmojiContent> *c,
                                          ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnEmojiReactionReceivedInWebinar", 0, c ? c->GetCount() : 0, i))
    }
    void OnEmojiFeedbackReceived(unsigned int sender, IPCFeedbackReactionType t,
                                 ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnEmojiFeedbackReceived", t, sender, i))
    }
    void OnAllHandsLowered(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnAllHandsLowered", 0, 0, i))
    }
    void OnGroupLayoutUpdate(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnGroupLayoutUpdate", 0, 0, i))
    }
    void OnMyVideoStatusChange(bool on, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnMyVideoStatusChange", on, 0, i))
    }
    void OnStartShareAudio(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnStartShareAudio", 0, 0, i))
    }
    void OnStopShareAudio(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnStopShareAudio", 0, 0, i))
    }
    void OnStartViewAudio(unsigned int uid, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnStartViewAudio", 0, uid, i))
    }
    void OnStopViewAudio(unsigned int uid, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnStopViewAudio", 0, uid, i))
    }
    void OnShareSendStatusChanged(unsigned int uid, bool paused, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnShareSendStatusChanged", paused, uid, i))
    }
    void OnStartSendShare(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnStartSendShare", 0, 0, i))
    }
    void OnStopSendShare(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnStopSendShare", 0, 0, i))
    }
    void OnShareContentTypeChanged(ZMToolSuiteProxyShareContentType t, unsigned int uid,
                                   ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnShareContentTypeChanged", t, uid, i))
    }
    void OnShareStatusChanged(unsigned int uid, ZMToolSuiteShareStatus s,
                              ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnShareStatusChanged", s, uid, i))
    }
    void OnSharedVideoEnded(ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnSharedVideoEnded", 0, 0, i))
    }
    void OnVideoFileSharePlayError(ZMToolSuiteProxySharePlayError e, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnVideoFileSharePlayError", e, 0, i))
    }
    void OnMyAudioStatusChanged(bool muted, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnMyAudioStatusChanged", muted, 0, i))
    }
    void OnOriginalSoundStatusChange(bool on, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnOriginalSoundStatusChange", on, 0, i))
    }
    void OnUnreadChatCountChange(unsigned int count, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnUnreadChatCountChange", 0, count, i))
    }
    void OnChatWindowStatusChange(bool on, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnChatWindowStatusChange", on, 0, i))
    }
    void OnMeetingViewTypeChange(IPCMeetingViewType t, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnMeetingViewTypeChange", t, 0, i))
    }
    void OnFullScreenStatusChange(bool fs, ZMToolSuitMonitorType m, ZMToolSuiteMeetingInstance i) override {
        LISTENER_GUARD(EmitOther("OnFullScreenStatusChange", fs, m, i))
    }
};

// Process-lifetime listener: never destroyed, so the SDK can never hold a
// dangling pointer even if a late callback races with shutdown.
Listener &TheListener() {
    static Listener *l = new Listener();
    return *l;
}

ZMToolSuiteMeetingInstance Inst(int32_t i) {
    return i == 0 ? ZMToolSuiteMeetingInstance_Default : (ZMToolSuiteMeetingInstance)i;
}

int32_t CopyToolkitVector(IToolSuiteVector<ZMToolSuiteUser> *v, zc_users_fn cb, void *user) {
    if (!v) return ZC_ERR_SDK_REJECTED;
    zc::UserBatch batch;
    CopyVector(v, batch);
    cb(user, batch.Data(), batch.Size());
    return ZC_OK;
}

void Async(const char *op, const BaseResult &r, int64_t value = 0) {
    std::string name(op);
    EmitSimple(ZC_EVENT_ASYNC_RESULT, (int32_t)r.error_code, value, 0, &name);
}

}  // namespace

extern "C" int32_t zc_sdk_linked(void) { return 1; }

extern "C" int32_t zc_init(zc_event_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (cb == nullptr) return ZC_ERR_INVALID_ARG;
    {
        std::lock_guard<std::mutex> lk(g_mu);
        if (g_inited) return ZC_ERR_WRONG_STATE;
        g_cb = cb;
        g_user = user;
        g_inited = true;
    }
    InitZMToolSuite();
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_start_auth(const char *token, size_t token_len, const char *domain,
                                 size_t domain_len) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (token == nullptr || token_len == 0) return ZC_ERR_INVALID_ARG;
    std::wstring wtoken = zc::Utf8ToWide(token, token_len);
    std::wstring wdomain =
        domain_len ? zc::Utf8ToWide(domain, domain_len) : std::wstring(L"https://zoom.us");
    ZMToolSuiteProxyAuthContext ctx;
    ctx.accessToken = wtoken.c_str();
    ctx.domain = wdomain.c_str();
    bool ok = StartToolSuiteAuth(ctx);
    zc::Wipe(wtoken);
    SetToolSuiteProxyListener(&TheListener());  // same order as PSDKTestDlg.cpp
    return ok ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}

extern "C" int32_t zc_uninit(void) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    UninitZMToolSuite();
    SetToolSuiteProxyListener(nullptr);
    std::lock_guard<std::mutex> lk(g_mu);
    g_cb = nullptr;
    g_user = nullptr;
    g_inited = false;
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_get_meeting_status(int32_t instance, int32_t *out_status) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!out_status) return ZC_ERR_INVALID_ARG;
    meeting::IMeetingToolkit *mt = meeting::GetMeetingToolkit(Inst(instance));
    if (!mt) return ZC_ERR_SDK_REJECTED;
    *out_status = (int32_t)mt->GetMeetingStatus();
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_get_participants(int32_t instance, zc_users_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    meeting::IMeetingToolkit *mt = meeting::GetMeetingToolkit(Inst(instance));
    if (!mt) return ZC_ERR_SDK_REJECTED;
    IToolSuiteVector<ZMToolSuiteUser> *v = mt->GetParticipantsList();
    if (!v) return ZC_ERR_SDK_REJECTED;
    zc::UserBatch batch;
    CopyVector(v, batch);
    cb(user, batch.Data(), batch.Size());
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_get_myself(int32_t instance, zc_users_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    meeting::IParticipantsToolkit *pt = meeting::GetParticipantsToolkit(Inst(instance));
    if (!pt) return ZC_ERR_SDK_REJECTED;
    zc::UserBatch batch;
    batch.Add(CopyUser(pt->GetMySelf()));
    cb(user, batch.Data(), batch.Size());
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_get_meeting_info(int32_t instance, zc_meeting_info_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    meeting::IMeetingToolkit *mt = meeting::GetMeetingToolkit(Inst(instance));
    if (!mt) return ZC_ERR_SDK_REJECTED;
    ZMToolSuiteGetMeetingProperty p = mt->GetMeetingProperty();
    std::string topic = zc::WideToUtf8(p.strMeetingTopic);
    std::string number = zc::WideToUtf8(p.strCurrentMeetingNum);
    zc_meeting_info info{};
    info.error_code = (int32_t)p.error_code;
    info.meeting_type = (int32_t)p.eMeetingType;
    info.topic = topic.data();
    info.topic_len = topic.size();
    info.meeting_number = number.data();
    info.meeting_number_len = number.size();
    cb(user, &info);
    return ZC_OK;
    ZC_GUARD_END
}

extern "C" int32_t zc_get_attendees(int32_t instance, zc_users_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    auto *mt = meeting::GetMeetingToolkit(Inst(instance));
    return mt ? CopyToolkitVector(mt->GetAttendeeList(), cb, user) : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_get_spotlighted(int32_t instance, zc_users_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    auto *mt = meeting::GetMeetingToolkit(Inst(instance));
    return mt ? CopyToolkitVector(mt->GetSpotlightedUserList(), cb, user) : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_get_waiting_room(int32_t instance, zc_users_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt ? CopyToolkitVector(wt->GetWaitingRoomUserList(), cb, user) : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_get_user(int32_t instance, uint32_t user_id, zc_users_fn cb, void *user) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!cb) return ZC_ERR_INVALID_ARG;
    auto *pt = meeting::GetParticipantsToolkit(Inst(instance));
    if (!pt) return ZC_ERR_SDK_REJECTED;
    zc::UserBatch batch;
    batch.Add(CopyUser(pt->GetUserByUserID(user_id)));
    cb(user, batch.Data(), batch.Size());
    return ZC_OK;
    ZC_GUARD_END
}
extern "C" int32_t zc_start_meeting(int64_t number, int32_t no_video, int32_t no_audio) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *pm = premeeting::GetPreMeetingToolkit();
    if (!pm) return ZC_ERR_SDK_REJECTED;
    StartMeetingParam p{}; p.meetingNumber = number; p.isNoVideo = no_video != 0; p.isNoAudio = no_audio != 0;
    return pm->StartMeeting(p, [](const BaseResult &r) { try { Async("StartMeeting", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_join_meeting(const char *name, size_t name_len, int64_t number, const char *password, size_t password_len) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (!name || !password) return ZC_ERR_INVALID_ARG;
    auto *pm = premeeting::GetPreMeetingToolkit();
    if (!pm) return ZC_ERR_SDK_REJECTED;
    std::wstring wname = zc::Utf8ToWide(name, name_len), wpass = zc::Utf8ToWide(password, password_len);
    JoinMeetingParam p{}; p.displayName = wname.c_str(); p.meetingNumber = number; p.password = wpass.c_str();
    bool ok = false;
    try { ok = pm->JoinMeeting(p, [](const BaseResult &r) { try { Async("JoinMeeting", r); } catch (...) {} }); }
    catch (...) { zc::Wipe(wpass); throw; }
    zc::Wipe(wpass);
    return ok ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_leave_meeting(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *mt = meeting::GetMeetingToolkit(Inst(instance));
    return mt && mt->LeaveMeeting([](const BaseResult &r) { try { Async("LeaveMeeting", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_end_meeting(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *mt = meeting::GetMeetingToolkit(Inst(instance));
    return mt && mt->EndMeeting([](const BaseResult &r) { try { Async("EndMeeting", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_get_user_role(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *pt = meeting::GetParticipantsToolkit(Inst(instance));
    return pt && pt->GetUserRoleByUserID(id, [](const GetUserRoleByUserIDResult &r) { try { Async("GetUserRoleByUserID", r, (int64_t)r.role); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_make_host(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *pt = meeting::GetParticipantsToolkit(Inst(instance));
    return pt && pt->MakeHost(id, [](const BaseResult &r) { try { Async("MakeHost", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_reclaim_host(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *pt = meeting::GetParticipantsToolkit(Inst(instance));
    return pt && pt->ReclaimHost([](const BaseResult &r) { try { Async("ReclaimHost", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_assign_cohost(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *pt = meeting::GetParticipantsToolkit(Inst(instance));
    return pt && pt->AssignCoHost(id, [](const BaseResult &r) { try { Async("AssignCoHost", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_revoke_cohost(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *pt = meeting::GetParticipantsToolkit(Inst(instance));
    return pt && pt->RevokeCoHost(id, [](const BaseResult &r) { try { Async("RevokeCoHost", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_get_unread_chat_count(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *ct = meeting::GetChatToolkit(Inst(instance));
    return ct && ct->GetUnreadChatCount([](const GetUnreadChatCountResult &r) { try { Async("GetUnreadChatCount", r, r.uCount); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_admit_user(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt && wt->AdmitToMeeting(id, [](const BaseResult &r) { try { Async("AdmitToMeeting", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_put_into_waiting_room(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt && wt->PutIntoWaitingRoom(id, [](const BaseResult &r) { try { Async("PutIntoWaitingRoom", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_expel_user(int32_t instance, uint32_t id) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt && wt->ExpelUser(id, [](const BaseResult &r) { try { Async("ExpelUser", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_admit_all(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt && wt->AdmitAllUserToMeeting([](const BaseResult &r) { try { Async("AdmitAllUserToMeeting", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_enable_waiting_room(int32_t instance, int32_t enable) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt && wt->EnableWaitingRoomFeature(enable != 0, [](const BaseResult &r) { try { Async("EnableWaitingRoomFeature", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_is_waiting_room_on(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *wt = meeting::GetWaitingRoomToolkit(Inst(instance));
    return wt && wt->IsWaitingRoomFeatureOn([](const BoolValueResult &r) { try { Async("IsWaitingRoomFeatureOn", r, r.bValue ? 1 : 0); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_get_meeting_view_type(int32_t instance) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *toolkit = meeting::GetUIActionToolkit();
    return toolkit && toolkit->GetMeetingViewType([](const GetMeetingViewTypeResult &r) { try { Async("GetMeetingViewType", r, (int64_t)r.type); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_switch_meeting_view(int32_t instance, int32_t view_type) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    if (view_type < IPCMeetingViewType_ActiveSpeaker || view_type > IPCMeetingViewType_Other) return ZC_ERR_INVALID_ARG;
    auto *toolkit = meeting::GetUIActionToolkit();
    return toolkit && toolkit->Switch2MeetingView((IPCMeetingViewType)view_type, [](const BaseResult &r) { try { Async("Switch2MeetingView", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
extern "C" int32_t zc_wall_view_page(int32_t instance, int32_t next) {
    ZC_GUARD_BEGIN
    if (!g_inited) return ZC_ERR_WRONG_STATE;
    auto *toolkit = meeting::GetUIActionToolkit();
    return toolkit && toolkit->Switch2WallViewPrevOrNextPage(next != 0, [](const BaseResult &r) { try { Async("Switch2WallViewPrevOrNextPage", r); } catch (...) {} }) ? ZC_OK : ZC_ERR_SDK_REJECTED;
    ZC_GUARD_END
}
