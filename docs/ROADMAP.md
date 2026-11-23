# AttendFlow Desktop — roadmap (data collection & analysis)

Owner's ideas from 2026-11-23, organised and checked against the repo, the Plugin SDK 7.2.0.3001
headers, and a live test run the same day (see "Live test results" below). **All requirements
below are implemented** on branch `roadmap-2026-11` (see "Implementation status"). Before calling
something a gap or regression, also check `docs/PARITY.md`.

## Implementation status (2026-11-23, branch `roadmap-2026-11`)

| Group | Status | Still to verify live (owner) |
|---|---|---|
| R-A1 camera | Done: per-row camera on/off minutes and %, live camera dot and camera-off count, manual snapshots keep `videoOn`, export columns; rebuilt from old logs | — |
| R-A2 mic proxy + audio type | Done: "Mic opened N× · M min unmuted", no-audio / phone markers, live column off by default, export columns | `audioType` only exists in logs from now on |
| R-A3 hands | Done: lower on leave (`reason: left`), flag is source of truth, re-read after lower-all, feedback 0/1 ignored; replay of `host-lab.jsonl` closes all 4 leave-with-hand cases once | — |
| R-A4 feedback / reactions | Done: `feedback` / `reaction` log events, record tab with log, per-person counts and "Yes since" check-in | Real meeting with Yes/No/Away clicks |
| R-B1 new record | Done: confirm → `manual_split`, new instance seeded as joined-before; new record resumes after restart | — |
| R-B2 rename | Done: `customName` in record state, list/detail/live titles, search, export file name + JSON, backup | — |
| R-C1/C2 compact + pin | Done: `/api/window`, compact 380×260 view, pin with accent edge + badge + toast, mode/size/position/pin remembered (verified on the native window). No tray exists, so no tray menu | — |
| R-D proof screenshots | Done (experimental): UIAction bridge (ABI 3), Windows Graphics Capture, tile-load wait, page-repeat stop, PNGs under `captures\<instance>`, Proof tab | **Untested in a meeting by design.** Check `GET /api/proof/windows` on your machine (meeting window class) before the first try |


Ground rules for everything below:

- **Data first, actions later.** No in-meeting actions (remove, rename, chat, admit, spotlight) for
  now. Collect and present more data; the owner interprets it.
- **Data never changes attendance status.** E.g. camera-off people are *shown*, not marked absent.
  Status/score derivation must stay identical to AttendFlow (parity goldens in
  `crates/attend-engine/tests/parity`). New fields are optional additions.
- **Follow AttendFlow's design system.** Use the tokens in `apps/desktop-ui/src/style.css` (which
  mirror AttendFlow, `../zoom-tracker/src/styles` + its design demo page) and the existing common
  components (`StatusBadge`, `DataTable`, `Toast`, `Tooltip`, `ToggleSwitch`, …). No new look.
- **Works as a non-host** unless stated. Everything in section A was observed for non-hosts in a
  real meeting (`%LOCALAPPDATA%\zoom-companion\meetings\pz-84552879566-*.jsonl`, 55 min, 18 people).

Status legend: **Ready** = data/API confirmed, just build it · **Needs decision** · **Blocked**.

---

## A. Data collection and analysis (do first)

### A1. Camera on/off in attendance — **Ready**

Why: some meetings treat camera-off as absent; the owner decides that, the app only records it.

- Source: `videoOn` on every participant (`ZMToolSuiteUser.bVideoOn`), refreshed by `UPDATE_USER`
  within ~0.1 s of a change (live test). Already written to the meeting JSONL logs (`serve.rs:48`),
  so **records rebuilt from logs get camera data retroactively**.
- Auto attendance table + record detail: camera-on minutes and % of present time per person; optional
  camera timeline in Entry/Exit.
- Manual attendance (`manualSnapshots[].participants`): camera on/off at capture time.
- Exports (CSV/XLSX/JSON): same columns.
- Engine: derive camera intervals from `users` events alongside presence intervals; keep them out
  of status/score so parity goldens stay unchanged.
- Limit: only known from the first snapshot the app sees; on/off flag only (a frozen or virtual
  camera counts as on).

### A2. Who spoke, how often, how long — **Blocked for real speech; proxy Ready**

- **Speaking count or speaking time from the SDK: not possible.** Live test: ~2.5 min of verified
  speech (other guests saw the active-speaker switch) and the `talking` flag never turned on, neither
  in events nor in a 1 s participant-list poll. It only changed together with other fields (e.g. it
  read `talking` once at the moment of muting). The real non-host log shows the same: 10–18 min
  "talking" stretches and "talking while muted". The Plugin SDK has no active-speaker event.
- AttendFlow (Apps SDK) has `onActiveSpeakerChange`, but it reports one coarse "current speaker"
  (what speaker view follows), not per-turn timing, and AttendFlow doesn't use it. Not available here.
- **Proxy we can build: mic activity.** `audioMuted` changes are reliable (146 in the real meeting,
  ~instant in the test). Per person: number of unmutes and total unmuted minutes. Label it honestly
  ("Mic opened 4× · 6 min unmuted"), never "spoke". Good proxy where people stay muted until they
  speak; meaningless for people who never mute.
- Also cheap: `audioType` (none / computer / **telephone**) → "joined without audio" and phone
  dial-in markers.
- Revisit if a later SDK adds an active-speaker or audio-level callback (re-diff headers on upgrade).

### A3. Raised-hand correctness — **Ready (smaller than feared)**

Live test results:

- Host/co-host "lower all hands" (web client: "Clear All Feedback") sends, for **every** raised
  hand, an `UPDATE_USER` with the flag cleared **and** `OnUserLowerHand`, then `OnAllHandsLowered`
  (3× in a row). The current code (engine `engine.rs:355`, UI `apps/desktop-ui/src/lib/record-tools.ts:47`) already handles
  this correctly through the per-user lower events.
- **Real gap: leaving with a hand up.** Only `LEFT_USER` arrives (flag still raised); no lower
  event. Seen 4× (run 1 leave + `H3`). The live "Current hand raises" list is fine (it reads present
  users), but the record's hand log keeps an open "raise" with no "lower" for that person.
- Fix plan: (1) on `LEFT_USER` with `raisedHand`, emit a synthetic lower (reason `left`); (2) make
  the participant flag the source of truth: on every `users` event diff `raisedHand` against the
  last known state and emit raise *and* lower transitions (UI `deriveHandEntries` currently only
  adds raises from flags); (3) on `OnAllHandsLowered`, re-read the participant list and diff — a
  safety net, since per-user lowers already arrive. Existing dedup by transition stays.
- Also seen: raising/lowering a hand also fires `OnEmojiFeedbackReceived` (type 1 Hand / 0 None);
  ignore those for hand tracking to avoid double entries.

### A4. Nonverbal feedback and reactions — **Ready (new, optional)**

Not in the original list, but confirmed live and cheap; fits "more data":

- `OnEmojiFeedbackReceived(sender, type)`: Yes=2, No=3, Slower=5, Faster=4, I'm away (coffee)=9,
  cleared=0 (enum `IPCFeedbackReactionType`), ~50 ms latency, sender id included.
  Use: quick check-ins ("click ✅ Yes to confirm you're here") with a per-person tally, and an
  "away" log.
- `OnEmojiReactionReceived(sender, type)`: clap, thumbs up, heart, … (type 2 = thumbs up confirmed).
  Use: per-person engagement count.
- Limit: feedback state is not in the participant list, so feedback set before the app connected is
  invisible.

---

## B. Record control

### B1. Manually start a new record — **Ready (decided 2026-11-23)**

For a break that should become two separate meetings.

- The tracker already has clean end/start steps (`serve.rs:303` `end(reason)`, used for
  `meeting_changed`). A "Start new record" button: end the current instance with reason
  `manual_split`, start a new instance (`pz-<meeting>-<startedAt>`) seeded with everyone present as
  joined-before, like AttendFlow's reload split.
- Keep the guarantee that an ended instance is never resumed (`serve.rs:1716` test).
- Owner decisions: **ask for confirmation**; **no automatic "Part 2" label**.

### B2. Rename a record — **Ready (decided 2026-11-23)**

Owner wants to give any record a custom name for easier reference later (e.g. after a split).
AttendFlow has no record names, so this is a desktop addition (add it to `docs/PARITY.md` "Desktop
additions").

---

## C. Window and UX

### C1. Compact live panel — **Ready**

Like Zoom's minimized meeting window: a narrow strip with present/expected counts, current hands,
latest joins/leaves, and camera-off count (A1). Toggle full ↔ compact.

- Window is tao 0.30 + wry (`serve.rs:853`, currently 1280×820). tao supports resize, min size and
  hiding decorations. The UI talks to the shell over local HTTP (no wry IPC), so window commands need
  an `EventLoopProxy` user event triggered by a new API route.
- Remember the last mode/position per user (local setting).

### C2. Always on top, with a clear indicator — **Ready**

- tao `set_always_on_top`. Same plumbing as C1.
- Indicator: pin button with an obvious active state, a coloured window edge/badge while pinned, a
  toast on toggle. Pairs naturally with C1.

### C3. Design system — rule for all of the above (see ground rules).

---

## D. Attendance proof screenshots — **Build, untested (owner tests in real meetings)**

Owner decision 2026-11-23: implement it without an agent live test; the owner will test it in the
real app on real data. The UI must say the feature is experimental and not guaranteed to work.

Proof that people were present (owner's normal workflow; removal stays manual, see ground rules).

- The SDK has **no** frame/snapshot API. Method: switch Zoom to Gallery
  (`IUIActionToolkit::Switch2MeetingView(Wall)`), step pages (`Switch2WallViewPrevOrNextPage`),
  capture the Zoom meeting window with Windows Graphics Capture, and store each image with the
  participant list and camera states at that moment.
- **Waiting for tiles to load** (owner: after a page flip many tiles are black for a few seconds):
  the SDK gives no "rendered" signal, so judge from the image — capture every ~0.5 s and keep the
  frame once two consecutive frames are nearly identical **and** the count of near-black tiles has
  stopped falling; time out at ~8–10 s and save anyway. Store wait time, black-tile count, and the
  SDK's camera-on count for that moment as a cross-check.
- One person on their own: no tile geometry from the SDK. Start with whole-page captures tagged
  with the person; spotlight-then-capture is host-only and visible to everyone (out of scope now).
- Switching to Gallery changes only the user's own Zoom view (allowed under "no actions").
- Privacy: images stay local; consider a visible "captured" note.
- Not tested by an agent (by owner decision). The UIAction calls are not wired into the bridge yet.

---

## Out of scope for now

Removing/expelling, putting into waiting room, renaming to roster names, sending chat, auto-admit,
spotlight, lower all hands from the app. The SDK has calls for these (mostly host/co-host); revisit
after data collection.

## Suggested order

1. **A3** hand fix (wrong data today, small).
2. **A1** camera data (+ **A2** mic-activity proxy and audio type; same plumbing).
3. **B1 + B2** new record, rename record.
4. **C2 + C1** pin and compact panel.
5. **A4** feedback/reactions (optional).
6. **D** proof screenshots (experimental).

The handoff-ready version of all this is the **Requirements** section below.

---

## Requirements (handoff)

Self-contained requirement list for an implementing agent. IDs are stable; reference them in commits
(e.g. `R-A1.3`). Written against `ui-parity` at `61b3b50`; **re-check the cited code first**, other
agents commit to this branch in parallel.

### General (apply to every requirement)

- **G1** Data only. Implement no in-meeting action (expel, waiting room, rename participant, chat,
  admit, spotlight, lower hands) even though the SDK has them.
- **G2** New data never changes attendance status, score, tags or the parity goldens. All new
  record fields are optional (`#[serde(default)]` / optional in TS) so old records, AttendFlow
  imports and backups still load. `cargo test --workspace` parity tests must stay green.
- **G3** UI follows AttendFlow's design system: tokens in `apps/desktop-ui/src/style.css`, existing
  components in `apps/desktop-ui/src/lib/common/`, toasts and busy states through the existing
  helpers (`runBusy`, `toast`). Every new string in zh **and** en (`i18n-core.ts` / `t()`).
- **G4** Works as a non-host unless the requirement says otherwise.
- **G5** New SDK calls go through the adapter pattern in `docs/HANDOFF.md`: `zc_sdk.cpp` +
  `zc_bridge.h` + `crates/zoom-bridge/src/ffi.rs` + a `Session` method (bump the adapter ABI if the
  C ABI changes). Listener callbacks already arrive as `Event::Other { name, code, value }`.
- **G6** Done means: Rust + UI unit tests for new logic (`scripts/test.ps1`, `pnpm test` and
  `pnpm check` in `apps/desktop-ui`), screenshots of every changed screen in zh and en
  (`apps/desktop-ui/scripts/screenshots.mjs`), and `docs/PARITY.md` "Desktop additions" updated.

### R-A1 Camera on/off

User story: as a teacher, I see who had their camera off and for how long, so I can decide myself
whether that counts as absent.

| ID | Requirement |
|---|---|
| R-A1.1 | Track each participant's camera state from `ZMToolSuiteUser.bVideoOn` (bridge flag `ZC_USER_VIDEO_ON`, JSON `videoOn` in `users` events: `SYNC_ALL`/`ADD_USER`/`UPDATE_USER`). Close the open interval on `LEFT_USER`, instance end, and meeting end. |
| R-A1.2 | Derive per attendance row: camera-on minutes, camera-off minutes, camera-on % of present time; merged across the row's ids/rejoins like presence minutes. Unknown time (before the first snapshot) counts as neither. |
| R-A1.3 | Live status table and record detail attendance table: add camera columns (on minutes, %), sortable; live view also shows a current camera on/off indicator per present person. |
| R-A1.4 | Manual attendance capture (`manualSnapshots[].participants`): store `videoOn` per person; show it as a column in the manual records table. |
| R-A1.5 | Exports (all record export formats): add camera columns to the attendance sheet and to manual snapshot rows. |
| R-A1.6 | Rebuilding a record from its JSONL log yields camera data for meetings recorded before this change (the logs already contain `videoOn`). |
| R-A1.7 | Attendance status/score unchanged (G2). |

### R-A2 Mic activity (proxy for speaking) and audio type

The SDK cannot tell who spoke (live-tested: `bTalking` stays false during real speech). Do **not**
use `bTalking` anywhere.

| ID | Requirement |
|---|---|
| R-A2.1 | Per row: unmute count (transitions of `bAudioMuted` true→false, flag `ZC_USER_AUDIO_MUTED`, JSON `audioMuted`) and total unmuted minutes. The first snapshot's state is not an unmute. |
| R-A2.2 | Per row: audio type from `ZMToolSuiteUser.audioType` (`ZMToolSuiteAudioType`: 0 None, 1 VoIP/computer, 2 Telephony). Show "no audio" and "phone" markers; keep the last known value and whether it was ever None. |
| R-A2.3 | Label honestly: "Mic opened N× · M min unmuted" (zh equivalent). Never "spoke" or "speaking time". Tooltip explains it is mic activity, not speech. |
| R-A2.4 | Show in record detail and exports; optional column in the live table (off by default). |

### R-A3 Raised-hand correctness

Current state at `61b3b50`: `serve.rs` emits `hand` from `OnUserRaisedHand`/`OnUserLowerHand` and
emits `all_hands_lowered` from `OnAllHandsLowered`; engine (`engine.rs`, `kind == "hand"`) uses only
`hand`; UI `deriveHandEntries` (`record-tools.ts`) adds raises from flags but never lowers.

| ID | Requirement |
|---|---|
| R-A3.1 | When a participant leaves (`LEFT_USER`) with `bRaisedHand` set, record a lower for that hand at the leave time, marked as caused by leaving (e.g. `reason: "left"`), in both the live log and rebuilt records. |
| R-A3.2 | Treat the participant flag as the source of truth: on every `users` event, compare `raisedHand` with the last known state per user id and record raise **and** lower transitions. Dedup with the explicit events so one physical change gives exactly one entry. |
| R-A3.3 | On `OnAllHandsLowered` (`all_hands_lowered`), re-read `GetParticipantsList` and apply R-A3.2 as a safety net. Per-user lowers normally arrive first (live-tested); no duplicates. |
| R-A3.4 | Ignore `OnEmojiFeedbackReceived` types 1 (Hand) and 0 (None) for hand tracking (they echo raise/lower). |
| R-A3.5 | Hand-log UI shows "Lowered (left meeting)" for R-A3.1 entries. Existing captures and "Current hand raises" behaviour unchanged. |
| R-A3.6 | Tests: replay fixtures for raise→leave, lower-all (per-user lowers + 3× all-lowered), and flag-only lower. Use `docs/live-tests/2026-11-23/host-lab.jsonl` as reference data. |

### R-A4 Nonverbal feedback and emoji reactions (optional, lower priority)

| ID | Requirement |
|---|---|
| R-A4.1 | Persist `OnEmojiFeedbackReceived(sender_id, IPCFeedbackReactionType)` as a `feedback` log event `{userId, type}` (types: 2 Yes, 3 No, 4 Fast, 5 Slow, 9 Coffee/"I'm away", 0 cleared; ignore 1 per R-A3.4). |
| R-A4.2 | Persist `OnEmojiReactionReceived(sender_id, SendEmojiReactionParam.type)` as a `reaction` log event (`IPCEmojiReactionType`: 1 clap, 2 thumbs up, 3 heart, 4 joy, 5 open mouth, 6 tada). Bridge currently forwards only the type in `code` and the sender in `value`; that is enough. |
| R-A4.3 | Record detail: feedback/reaction log tab (time, name, type) with per-person counts; "Yes" tally usable as a check-in ("who clicked Yes since T"). |
| R-A4.4 | Note in UI: feedback set before the app connected is not visible (no field in the participant list). |

### R-B1 Manually start a new record

| ID | Requirement |
|---|---|
| R-B1.1 | Live view: "Start new record" button, enabled only while a meeting record is active. |
| R-B1.2 | Confirmation dialog (existing `ConfirmDialog`) explaining that the current record ends now and a new one starts; Esc/Enter behave like other confirms. |
| R-B1.3 | On confirm: end the current instance with reason `manual_split` (tracker `end(reason)` in `serve.rs`), start a new instance for the same meeting, seeded with everyone currently present as joined-before (same semantics as AttendFlow's reload split). |
| R-B1.4 | The ended record is never resumed (keep `ended_instance_is_never_resumed` passing); a companion restart resumes the **new** record. |
| R-B1.5 | No automatic name or "Part 2" label; both records keep the default title (topic + start time). Toast on success/failure. |
| R-B1.6 | Waiting-room minutes, hands and camera intervals split cleanly at the boundary (no interval spans both records). |

### R-B2 Rename a record

| ID | Requirement |
|---|---|
| R-B2.1 | Any record (live or past, including imported AttendFlow records) can be given a custom name; clearing it restores the default (topic + start time). |
| R-B2.2 | Store the name in the record's revisioned meeting state (`/api/meetings/:id/state`, e.g. `customName`), so it survives rebuilds from logs, restarts and resume. |
| R-B2.3 | Where the record title appears (records list, record detail header, live header), show the custom name and keep topic/start time visible as secondary text. Records-list search matches the custom name. |
| R-B2.4 | Included in exports (record title) and in full backup/restore; AttendFlow backup import ignores it gracefully. |
| R-B2.5 | Edit affordance: inline edit or small dialog in record detail (and live header), trimmed, max ~100 chars, toast on save. |

### R-C1 Compact live panel

| ID | Requirement |
|---|---|
| R-C1.1 | Toggle between full window and a compact mode, similar in spirit to Zoom's minimized meeting window. |
| R-C1.2 | Compact content: present / expected counts, late / missing counts (roster mode), current raised hands, camera-off count (R-A1), last few joins/leaves; click-through to full mode. |
| R-C1.3 | Window: tao `Window::set_inner_size` / `set_min_inner_size` and optionally `set_decorations(false)` with a custom drag area. The UI talks to the shell over local HTTP, so add an API route that sends an `EventLoopProxy` user event to the window loop (`serve.rs`, `WindowBuilder` near the `app` command). |
| R-C1.4 | Remember mode, size and position per user across restarts (local setting). Restoring to full returns to the previous full size. |
| R-C1.5 | Usable at ~360×240 logical px with zh and en text, AttendFlow tokens (G3). |

### R-C2 Always on top

| ID | Requirement |
|---|---|
| R-C2.1 | Pin toggle (tao `Window::set_always_on_top`) through the same route/proxy as R-C1.3; available in both full and compact modes. |
| R-C2.2 | Clear indicator while pinned: pin button in an active state, plus a visible window-level marker (accent edge or badge), plus a toast on change. |
| R-C2.3 | Remember the pin state across restarts (setting); tray menu shows and toggles it if the tray has a menu. |

### R-D Attendance proof screenshots (experimental, owner tests live)

| ID | Requirement |
|---|---|
| R-D.1 | Wire `IUIActionToolkit` (`meeting::GetUIActionToolkit()`): `GetMeetingViewType`, `Switch2MeetingView(IPCMeetingViewType_Wall)`, `Switch2WallViewPrevOrNextPage(bool nextPage)`; forward `OnMeetingViewTypeChange` (already arrives as `Event::Other`). |
| R-D.2 | "Capture attendance proof" action in the live view: remember the current view type, switch to gallery, capture page 1, step pages until a page repeats or a page cap (e.g. 20) is reached, then restore the original view. Never touches other participants. |
| R-D.3 | Capture the Zoom meeting window with Windows Graphics Capture (works when partly covered; fails when minimized → clear error). Find the meeting window robustly (verify class/title on the owner's machine; Workplace UI is Spanish). |
| R-D.4 | Tile-load wait per page: capture every ~0.5 s; accept when two consecutive frames are nearly identical **and** the count of near-black tile regions has stopped decreasing; time out at ~10 s and keep the last frame. |
| R-D.5 | Store per page: PNG, timestamp, wait time, near-black tile estimate, and the SDK participant list with `videoOn` at that moment. Files under `%LOCALAPPDATA%\zoom-companion\captures\<instanceId>\`, referenced from meeting state; deleted with the record. |
| R-D.6 | Record detail: "Proof" tab with thumbnails, time, page n/N and the participant list; open full size. |
| R-D.7 | UI labels the feature **experimental / not guaranteed to work** (zh + en) next to the button and in the Proof tab, and shows any failure as a toast with the reason. |
| R-D.8 | No agent live test (owner decision); unit-test the pure parts (frame comparison, black-tile counting, page-repeat detection) with fixture images. |

### Out of scope (do not build)

Expel (`IWaitingRoomToolkit::ExpelUser`), `PutIntoWaitingRoom`, `ChangeUserName`, `SendChat`,
`AdmitToMeeting`, `SpotlightVideo`, `LowerAllHands`; speaking time/count from `bTalking`; the
undocumented `Zoom3rdSDK` DLL exports (`SetConfChatCallBack` etc.).

---

## Live test results (2026-11-23)

Unattended run by an agent: owner's PMI with owner's Zoom Workplace as **host** (lab console), four
web-client guests (`tools/guest-bot/scenario.mjs`), 1 s participant-list polling. Raw data, timeline
and how to reproduce: [`docs/live-tests/2026-11-23/`](live-tests/2026-11-23/README.md).

| Question | Result |
|---|---|
| Raise hand reaches host? | Yes: `UPDATE_USER` (flag) + `OnUserRaisedHand` + `OnEmojiFeedbackReceived(1)`, within a few ms of the click. |
| Host/co-host lowers all hands | Per hand: `UPDATE_USER` (flag cleared) + `OnUserLowerHand` + `OnEmojiFeedbackReceived(0)`; then `OnAllHandsLowered` ×3. ~85 ms after the click. Current code handles it. |
| Leave with hand raised | Only `LEFT_USER` with raised flag; **no lower event** (4 cases). → A3 fix. |
| Nonverbal feedback | `OnEmojiFeedbackReceived(sender, type)`: Yes=2, I'm away=9; ~50 ms. |
| Emoji reaction | `OnEmojiReactionReceived(sender, 2)` for thumbs up; ~40 ms. |
| Camera off / on | `UPDATE_USER` with video bit cleared/set; ~90 / ~150 ms. |
| Mute / unmute | `UPDATE_USER` muted bit, ~50–150 ms; 1 s poll agrees. |
| `talking` flag during real speech | **Never true** while unmuted and speaking (2 windows, ~2.5 min, speech verified from another guest's view). Read `talking` once, at a mute moment. Polling does not refresh it. → A2 blocked. |
| Host calls used | `StartMeeting`, `EnableWaitingRoomFeature` (off, then back on), `AssignCoHost` ×2, `EndMeeting`: all `code 1` (success). |

Not tested: non-host variants of these exact steps (the earlier real non-host log shows the same
hand/video/mute behaviour), breakout rooms, screen-share events, gallery capture (D).
