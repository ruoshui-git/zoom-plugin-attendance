# New desktop attendance app — architecture and plan

Status: draft 2026-11-17. Working name **AttendFlow Desktop** (codename `zoom-companion`).
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
│ attend-store (SQLite): events, meetings, roster, aliases,     │
│                        profiles, settings, annotations        │
│ HTTP 127.0.0.1:47812: /api/* (REST JSON), /bridge (WS live),  │
│                        static UI                              │
│ window: WebView2 (wry) or default browser                     │
└───────────────────────────────────────────────────────────────┘
                 ▲ REST + WS (BRIDGE-PROTOCOL.md)
┌──────── apps/desktop-ui (Svelte 5 + Vite + TS) ───────────────┐
│ engine/ = AttendFlow session-engine + models (copied, tested) │
│ live view · records · roster/aliases/profiles · import/export │
└───────────────────────────────────────────────────────────────┘
```

Decisions:
- **Raw events are the source of truth**; attendance is derived deterministically, so
  engine fixes re-derive history. Manual corrections are stored as annotations on top.
- **One meeting instance = one record.** No Main/Meeting split, no postMessage sync,
  no heartbeat, no per-surface databases (those exist only because of Zoom Apps).
- **Engine in TypeScript (copied from AttendFlow)** for v1 parity speed and identical
  semantics; its tests come along. Porting to Rust is optional later.
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
  `GET/PUT /api/roster`, `/api/aliases`, `/api/profiles`, `/api/settings`;
  `GET /api/meetings` (instances, newest first); `GET /api/meetings/:id/events?sinceSeq=`;
  `GET/PUT /api/meetings/:id/annotations`; `POST /api/import/attendflow` (backup JSON).
- Live: `/bridge` as specified in BRIDGE-PROTOCOL.md.
