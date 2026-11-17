# New desktop attendance app — architecture and plan

Status: P1 foundation implementation 2026-11-17. Working name **AttendFlow Desktop** (codename `zoom-companion`).
Owner decisions: AttendFlow stays a Zoom App and must keep working with only the Apps
SDK; this app is built separately in this repo and may duplicate AttendFlow logic.

## Goals

1. Everything AttendFlow does (parity checklist: [PARITY.md](PARITY.md)).
2. Without host/co-host: the Plugin SDK gives a plain attendee the full list and live
   join/leave (SDK-NOTES "In-meeting observations").
3. More than a Zoom App can do: background capture with no UI open, real join times
   from the moment the companion sees the meeting, one record per meeting even across
   UI reloads/companion restarts, waiting-room handling when host, raise-hand tracking.

## Architecture

```text
┌────────────── zoom-companion.exe (one process) ──────────────┐
│ SDK capture (zoom-bridge)  → instance tracker → event store   │
│ attend-engine: identity, matching, presence, statuses, score  │
│                (Rust port of AttendFlow semantics)            │
│ attend-store (SQLite): events, meetings, roster, aliases,     │
│                        profiles, settings, record state       │
│ HTTP 127.0.0.1:47812: /api/* (REST JSON), /bridge (WS live),  │
│                        static UI                              │
│ window: WebView2 (wry/tao), tray; browser tab for dev          │
└───────────────────────────────────────────────────────────────┘
                 ▲ REST + WS (BRIDGE-PROTOCOL.md)
┌──────── apps/desktop-ui (Svelte 5 + Vite + TS) ───────────────┐
│ presentation only: renders computed records from /api         │
│ live view · records · roster/aliases/profiles · import/export │
└───────────────────────────────────────────────────────────────┘
```

Decisions:
- **Raw events are the source of truth**; attendance is derived deterministically, so
  engine fixes re-derive history. Manual corrections are stored as annotations on top.
- **One meeting instance = one record.** No Main/Meeting split, no postMessage sync,
  no heartbeat, no per-surface databases (those exist only because of Zoom Apps).
- **Engine in Rust (`crates/attend-engine`)**, owner decision 2026-11-17: background
  capture and P3 features (auto-admit roster matches, alerts, scheduled exports) need
  matching/identity in the backend; one engine, one source of truth. Its boundary is
  `ingest` (bridge events → SessionRecord) then `derive(SessionRecord, config, decisions,
  reference_time, options) → { rows, summary }`. The app receives the companion event
  stream directly; it does not reproduce Zoom-Apps batches, debounced flushes, or roster
  polls. Therefore exact AttendFlow derivation is the hard gate. Zoom-Apps ingestion
  parity remains an ignored informational metric; bridge ingestion is covered by an
  explicit companion scenario suite. Current derivation parity work is in progress.
- **UI: Svelte in a WebView2 window** (owner decision): dense bilingual data UI, good CJK
  input/rendering, AttendFlow screens adaptable; presentation only (no engine in the UI).
- **Waiting-room time is NOT attendance** (owner decision): non-hosts cannot see the
  waiting room, so counting it would make host and non-host records disagree. Presence
  minutes, first join and lateness use in-meeting time only; waiting-room minutes are a
  separate field (host-only data). Engine flag `waiting_room_counts_as_presence`
  (default false; true only to reproduce AttendFlow goldens).
- **No post-end restart**: a record is a meeting instance; an ended instance is never
  reopened or followed by an implicit new record (AttendFlow's phantom-session bug).
- **Storage in the companion (SQLite, bundled rusqlite)** under
  `%LOCALAPPDATA%\zoom-companion\data.db`, not browser IndexedDB, so data survives
  browser resets and is reachable without the UI. The per-instance JSONL logs remain as
  an append-only backup.
- **Same UI stack as AttendFlow** (Svelte 5, Vite, TS) so components and i18n can be
  copied and adapted rather than rewritten.
- **Import AttendFlow's export/backup formats** so existing roster/aliases/profiles and
  records move over.

## Phases

| Phase | Scope | Done when |
|---|---|---|
| P1 Foundation | `attend-store` crate + `/api` (config CRUD, meetings list, events by meeting); UI skeleton (Svelte) with live meeting view fed by `/bridge`; engine copied with its tests; meeting history list | live view shows correct present/left with durations in a real meeting; history survives restart |
| P2 Parity | everything in PARITY.md: roster/alias/profile management + imports, attendance table + statuses, record detail tabs, manual attendance, name-change log, raise-hand, reconciliation, analytics, exports, zh/en | parity checklist ticked, AttendFlow fixtures produce identical derived rows |
| P3 Beyond | auto-admit roster matches from waiting room (host), unknown-arrival alerts, raise-hand queue, notifications, WebView2 window + tray, autostart, installer/package | owner sign-off |

## Interfaces (P1)

- REST under `/api`, JSON, same Origin rules as `/bridge`:
  config `GET/PUT /api/config/{roster,aliases,profiles,settings}` (revisioned, 409 on
  stale); `GET /api/meetings` (instances, newest first); `GET /api/meetings/:id/events`;
  `GET /api/meetings/:id/record` (**computed by attend-engine**: participants, rows,
  statuses, tags, summary, score); `GET/PUT /api/meetings/:id/state` (mode, profile,
  late/return overrides, decisions, manual snapshots, raise-hand captures);
  `POST /api/import/attendflow-backup`.
- Live: `/bridge` as in BRIDGE-PROTOCOL.md plus a `record_updated` notification so the
  UI refetches (or receives) the recomputed record.

## As-built P1 notes

- `crates/attend-store` provides SQLite migrations and revisioned config/state docs.
- `zoom-companion serve` exposes the routes above; `serve --fake` starts the HTTP/UI
  process without starting the SDK worker. `zoom-companion app` hosts the UI in a Wry /
  Tao native window titled **AttendFlow Desktop**.
- `apps/desktop-ui` is a Svelte 5/Vite presentation-only client. Live record changes
  arrive as `record_updated`; the client refetches the record. Generic/roster mode is
  persisted in per-meeting state; language selection is stored in local UI settings.
- The companion defines `RecordProvider`; `StubRecordProvider` currently derives
  generic participant rows from stored `users` events, with no roster matching. It
  returns AttendFlow camelCase `AttendanceRow` fields and summary keys, including
  `waitingRoomMinutes`. Replace it with the `attend-engine` implementation when that
  crate is ready; no attendance calculations belong in the UI.
- `serve --fake` seeds two demo participants in an isolated temp database and JSONL
  folder. It does not read or write the normal `%LOCALAPPDATA%` store and never starts
  an SDK session.
- Build scripts build the UI into `target/{debug,release}/ui`, and package it under
  `ui/` beside the executable. The app resolves that directory relative to its exe.
