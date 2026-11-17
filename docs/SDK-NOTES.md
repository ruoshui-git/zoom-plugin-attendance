# Zoom Plugin SDK 7.2.0.3001 — notes from the package (x64)

Authoritative sources: `export_h/*.h` and `demo/PSDKTest/*.cpp` in the
downloaded package. Re-diff both on every SDK upgrade.

## Lifecycle (ToolSuiteProxyInterface.h)

```cpp
extern "C" void InitZMToolSuite();
extern "C" bool StartToolSuiteAuth(ZMToolSuiteProxyAuthContext ctx); // {accessToken, domain} as const wchar_t*
extern "C" void UninitZMToolSuite();
void ZMToolSuiteProxy::SetToolSuiteProxyListener(IZMToolSuiteProxyListener*);  // C++-mangled export
```

Sample order: Init → StartToolSuiteAuth → SetListener(this); Uninit → SetListener(nullptr).
`IZMToolSuiteProxyListener` has **34 pure virtuals** (all implemented in
`crates/zoom-bridge/native/zc_sdk.cpp`; unmodelled ones are forwarded as
`Event::Other { name, ... }`).

Enums (raw values): `AUTH_RESULT_{SUCCESS=0,REQUEST_ERROR,VERIFY_USER_FAILED,FAILED}`;
`IPC_STATUS_{IDLE=0,CONNECT_TIMEOUT,CONNECTED,DISCONNECTED}`;
`ZMToolSuiteProxyErrors`: **0 = Unknown, 1 = Success**, then NotImplemented,
WrongUsage, InvalidArgument, InternalError, NoPermission, RateLimitReached,
OperationTimeout, VersionIncompatible. `ZMToolSuiteMeetingInstance`:
None=0, **Default=1**, GreenRoom=2, NewBO=3.

## Empirical observations (2026-11-17, Zoom Workplace 7.2.5, not in a meeting)

- `InitZMToolSuite` → `StartToolSuiteAuth` → listener: `AUTH_RESULT_SUCCESS` then
  `IPC_STATUS_CONNECTED` within ~1 s, delivered on the initialising thread (message pump).
- Requires the SDK `bin\` set beside the exe (otherwise silent: no callbacks).
- `meeting::GetMeetingToolkit(Default)` returns **null when not in a meeting** (bridge reports
  `SdkRejected`). Expect toolkits to become available after `MEETING_STATUS_INMEETING`.
- Token from the PKCE **public client** with only `plugin_sdk:read:connection_meta` is sufficient for the handshake.

## Participant-related surface (for the attendance question)

| API | Shape | Notes / open questions |
|---|---|---|
| `meeting::GetMeetingToolkit(inst)->GetParticipantsList()` | **synchronous**, returns `IToolSuiteVector<ZMToolSuiteUser>*` | SDK-owned (sample never frees). **Key experiment: does it return the full list for a plain attendee (non-host)?** No role check is documented in the header. |
| `GetMeetingToolkit(inst)->GetAttendeeList()` | sync vector | Webinar attendees; role requirements unknown. |
| `GetMeetingToolkit(inst)->GetSpotlightedUserList()` | sync vector | |
| `GetMeetingToolkit(inst)->GetMeetingStatus()` / `GetMeetingProperty()` | sync | Property includes topic, meeting number, **password and join URL** (bridge deliberately does not expose those two). |
| `GetParticipantsToolkit(inst)->GetUserByUserID(id)`, `GetMySelf()` | sync `ZMToolSuiteUser` | |
| `GetParticipantsToolkit(inst)->GetUserRoleByUserID(id, cb)` | async | Host/CoHost/Attendee/Panelist/BO moderator |
| `GetWaitingRoomToolkit(inst)->GetWaitingRoomUserList()` | sync vector | Likely host-only. |
| Listener `OnUserStatusChanged(ZMToolSuiteUserList, inst)` | event | `eUserAction` ∈ NO_ACTION, **SYNC_ALL**, ADD_USER, LEFT_USER, UPDATE_USER + `vUserList` + `error_code` → join/leave tracking. Bridge: `Event::UserStatus`. |
| Listener `OnUserRaisedHand/OnUserLowerHand(inst, userId)` | event | |

`ZMToolSuiteUser` fields: `bHost, bVideoOn, iUserID, strUserName, bMyself,
bSpotlighted, bAttendee, bRaisedHand, bAudioMuted, bTalking, bInWaitingRoom,
audioType`. **No email, no persistent/registrant id, no join time.** `iUserID` is
meeting-scoped; join/leave times must be timestamped by the companion when events arrive.

Bridge already exposes: `Session::participants`, `myself`, `meeting_info`,
`meeting_status`, and `Event::UserStatus` (CLI: `zoom-companion participants --watch N`).

## Chat surface

`IChatToolkit`: `SendChat(ZMToolSuiteChatInfo{content, userId, type}, cb)`,
`GetUnreadChatCount(cb)`, `SetParticipantsChatPrivilege(type, cb)`,
`IsChatShowing(cb)`. Listener: `OnUnreadChatCountChange(count, inst)`,
`OnChatWindowStatusChange(bool, inst)`.

**There is no documented API or callback that delivers incoming chat message
content.** Saving incoming chat is therefore not supported by the documented
Plugin SDK surface in 7.2.0.3001. (The DLL also exports undocumented C symbols
such as `SetConfChatCallBack`, `SetConfEventCallBack`, `SendMsgToZoomClient`;
these are not part of the public headers and are out of scope — do not use them.)
Documented alternatives to evaluate: Zoom's own "save chat" / auto-save chat
setting in Workplace, or REST/webhook features for the account, depending on role.

## Other toolkits (for reference)

PreMeeting (`StartMeeting`, `JoinMeeting`, `ShowSettingWindow`, `NavigateToURL`),
Audio, Video, Share, Recording, WaitingRoom, Reaction, UIAction
(`Switch2MeetingComponent` can show the Participants / Chat panels), Annotation,
Webinar, QA, ClosedCaption, Setting. Most mutating calls return `bool`
(= request submitted) and report completion through a `std::function` with
`BaseResult.error_code`.

## Runtime package (x64 `bin/`)

`zToolSuiteIPCProxy.dll` + Zoom deps (`cmmbase, cmmbiz, Cmmlib, util, tp, zNet,
zNetUtils, zLooper, zContext, ZMDB, ZoomTask, cares, libcurl, libcrypto-3-zm,
libssl-3-zm`) + VC++ runtime + api-ms-win-* forwarders + `win10rt.7z`.
Ship the whole folder beside the exe. All SDK exports are functions (no data),
so `/DELAYLOAD` is safe.
