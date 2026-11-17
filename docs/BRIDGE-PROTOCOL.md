# Companion bridge protocol v1 (zoom-companion `serve` ⇄ AttendFlow "companion backend")

Status: spec, 2026-11-17. Both sides implement exactly this; change it here first.

## Why

AttendFlow (repo `zoom-tracker`) is a Zoom App whose live capture needs host/co-host.
The Plugin SDK gives a **non-host** attendee the full participant list and live
join/leave events (verified, see SDK-NOTES "In-meeting observations"). `zoom-companion
serve` owns the SDK session and streams a faithful, timestamped, replayable event log;
AttendFlow runs in a normal browser and maps that log to the Zoom-Apps-shaped events
its existing session engine already consumes.

## Transport and security

- `zoom-companion serve [--port 47812] [--ui <dir>] [--open] [--allow-origin <origin>]...`
- Binds **127.0.0.1 only**. HTTP `GET /…` serves the AttendFlow build from `--ui` (SPA
  fallback to `index.html`, correct MIME types, `Cache-Control: no-store` for `index.html`).
- WebSocket at `ws://127.0.0.1:<port>/bridge`, UTF-8 JSON text frames, one message per frame.
- **Origin check:** reject the upgrade (HTTP 403) unless `Origin` equals
  `http://127.0.0.1:<port>` or `http://localhost:<port>` or one of `--allow-origin`
  (dev: `http://localhost:5173`). Prevents other web pages from reading meeting data.
- Multiple simultaneous clients are allowed (Main + Meeting surfaces, reloads). Each
  gets the same broadcast stream.
- Timestamps: `at` = Unix epoch **milliseconds** (companion wall clock when the SDK
  callback was received). The companion is the only clock; clients never stamp events.

## Meeting instances

- An **instance** starts when the companion first observes `MEETING_STATUS_INMEETING`
  with a known meeting number, and ends on `ENDED`/`FAILED`/`IDLE` after being in-meeting,
  or when the meeting number changes.
- `instanceId = "pz-<meetingNumber>-<startedAtMs>"`; stable for the whole instance.
- Persistence: `%LOCALAPPDATA%\zoom-companion\meetings\<instanceId>.jsonl` (every event,
  append + flush) and `current.json` `{instanceId, meetingNumber, startedAt, lastSeq,
  lastSeenAt}`. If the companion restarts and finds the same meeting number in-meeting
  with `lastSeenAt` within 15 min, it **resumes** that instance (same id, seq continues).
- `seq` is a per-instance monotonically increasing integer starting at 1.

## Data types

```jsonc
// User: one participant as the Plugin SDK reports it (copied, never SDK memory)
{ "userId": 16778240,            // meeting-scoped; changes on rejoin and on waiting-room admission
  "name": "毛若水",
  "host": true, "myself": true, "waitingRoom": false,
  "audioMuted": true, "videoOn": false, "raisedHand": false,
  "talking": false, "spotlighted": false }

// Meeting
{ "instanceId": "pz-1234509876-1794893640000", "meetingNumber": "1234509876",
  "topic": "…", "meetingType": 1, "status": 3, "statusName": "MEETING_STATUS_INMEETING",
  "startedAt": 1794893640000, "myself": User | null, "isHost": true }
```

## Server → client

| `type` | Fields | When |
|---|---|---|
| `hello` | `protocol: 1`, `companionVersion`, `connected` (SDK IPC up), `instanceId \| null`, `seqHead` | on connect |
| `state` | `meeting: Meeting \| null`, `participants: User[]` (incl. waiting-room users, flagged), `seqHead`, `at` | reply to `subscribe`, and whenever the instance changes |
| `event` | `seq`, `at`, `instanceId`, `kind`, payload (below) | live, and replayed after `subscribe` |
| `reply` | `id`, `ok`, `result` or `error: {code, message}` | reply to `request` |
| `record_updated` | `instanceId` | debounced ~500 ms after a stored event or record state change; refetch `/api/meetings/:id/record` |

## Desktop REST API

All API routes are served by the same loopback server and apply the `/bridge` Origin
allowlist when an `Origin` header is present. `serve --fake` starts the API/UI without
initializing the Zoom SDK, for local UI work and integration tests.

- `GET /api/config/{roster,aliases,profiles,settings}` → `{revision, value}`.
- `PUT /api/config/{key}` with `{revision, value}` → updated `{revision, value}`;
  stale revisions return HTTP 409 with `{error:"stale_revision",revision}`.
- `GET /api/meetings` → newest first; each item has `instanceId`, `meetingNumber`,
  `topic`, `startedAt`, nullable `endedAt`/`endReason`, `isHost`, and `lastSeq`.
- `GET /api/meetings/:id` → meeting metadata; `DELETE` removes an ended meeting and
  its events/state, while active meetings return 409.
- `GET /api/meetings/:id/events` → ordered event objects, with payload fields at the
  top level alongside `seq`, `at`, `instanceId`, and `kind`.
- `GET /api/meetings/:id/record` → `{meeting, rows, summary, durationMinutes, score,
  mode}`. Rows follow AttendFlow `AttendanceRow` camelCase fields and add
  `waitingRoomMinutes`; summary uses `seen`, `fullPresent`, `late`, `returned`,
  `leftEarly`, `absent`, `ignored`, and `total`.
- `GET /api/meetings/:id/state` → `{revision,value}`. `PUT` uses the same revisioned
  body and response as config.
- `POST /api/import/attendflow-backup` accepts backup v3 JSON. It stores
  `rosterPeople`, `aliasMappings`, and `attendanceProfiles` as the three config docs,
  ignores session records, and reports imported item counts and `sessionsIgnored`.

SQLite at `%LOCALAPPDATA%\\zoom-companion\\data.db` stores meeting metadata, events,
config docs, and record state. Existing `meetings/*.jsonl` files are imported
idempotently at startup and remain the append-only backup.

Event kinds (payload fields are siblings of `kind`):

| `kind` | Payload | Source |
|---|---|---|
| `instance_started` | `meeting: Meeting` | first INMEETING of an instance (seq 1); followed by a `users sync_all` seed (`source: "snapshot"`) |
| `instance_resumed` | `meeting: Meeting` | companion restarted and resumed the same instance (same id, seq continues); followed by a `users sync_all` seed. Not a new meeting. |
| `users` | `action: "sync_all" \| "add" \| "left" \| "update"`, `users: User[]` | `OnUserStatusChanged` (verbatim; no inference) |
| `hand` | `userId`, `raised: bool` | `OnUserRaisedHand` / `OnUserLowerHand` |
| `all_hands_lowered` | — | `OnAllHandsLowered` |
| `meeting_status` | `status`, `statusName` | `OnMeetingStatusChanged` |
| `self_changed` | `myself: User`, `isHost` | myself flags changed (host transfer etc.) |
| `ipc` | `connected: bool`, `detail` | IPC lost / re-established (companion keeps retrying every 5 s) |
| `instance_ended` | `reason: "ended" \| "left" \| "failed" \| "meeting_changed"` | last event of an instance |

Notes from live testing that clients must handle:
- After (re)connect the SDK sends `sync_all` with the full list; treat it as a snapshot.
- Waiting-room arrivals are visible **only to the host** (`add` with `waitingRoom: true`).
  Admission = `left` (old id, waitingRoom) + `add` (new id, not waitingRoom), same ~100 ms.
  A non-host sees just the `add` at admission time.
- Killed clients produce `left` ~3 min late (Zoom server timeout).
- A renamed user arrives as `update` with a new `name`, same `userId`.

## Client → server

| `type` | Fields | Effect |
|---|---|---|
| `subscribe` | `instanceId?`, `sinceSeq?` | Server sends `state`, then replays logged events of the **current** instance with `seq > sinceSeq` (all of them if `sinceSeq` omitted or `instanceId` differs), then streams live. |
| `request` | `id` (string), `method`, `params` | One `reply` per request. |

Request methods (all run on the SDK thread; errors use `code` = `not_in_meeting`,
`sdk_rejected`, `not_supported`, `bad_params`, `internal`):

| `method` | `params` | Result |
|---|---|---|
| `getState` | — | same shape as `state` |
| `getParticipants` | — | `User[]` from a fresh `GetParticipantsList` |
| `getWaitingRoom` | — | `User[]` (`[]` when not host) |
| `admit` | `userId` | `{submitted: true}`; completion appears as `users` events |
| `admitAll` | — | `{submitted: true}` |
| `putIntoWaitingRoom` | `userId` | `{submitted: true}` |
| `expel` | `userId` | `{submitted: true}` |

## AttendFlow mapping (client side, informative)

- `participantUUID = "<instanceId>:<userId>"`; `meetingUUID = instanceId`;
  `meetingID = meetingNumber`; Zoom-Apps `timestamp` = `at / 1000` (seconds).
- `users add` (not waiting) → `onParticipantChange {status:"join"}`; `left` → `"leave"`;
  `add` with waitingRoom → `onWaitingRoomParticipantJoin {action:"joinMeeting"}`;
  `left` with waitingRoom → `onWaitingRoomParticipantLeave`; `hand` → `onFeedbackReaction
  {feedback:"raiseHand"}` / `onRemoveFeedbackReaction`; `instance_ended` → `onMeeting
  {action:"ended"}`.
- Capture is allowed regardless of Zoom role in companion mode.
