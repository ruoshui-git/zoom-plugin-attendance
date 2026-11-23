# AttendFlow → desktop app parity checklist

The Rust engine crate is in progress at `crates/attend-engine`. AttendFlow is split at
the parity boundary into ingestion (Zoom-Apps SDK and debounce/poll behavior producing
a SessionRecord) and derivation (SessionRecord + config + decisions producing rows and
summary). Only derivation is a hard gate for the desktop app. The 33/56 Zoom-Apps
adapter comparison is informational and stays ignored. Goldens now include the full
SessionRecord, derivation reference time, and roster-mode output variants. The Rust
derive function and exact all-row comparison are not yet at parity; see the crate
README and generator documentation for current scope.

**How to use this file.** It is the single record of AttendFlow vs the desktop app:
what is ported, what is open, what is skipped or not applicable, and what the desktop
app adds. If AttendFlow (as shipped, `featureSet = 'mvp'`) has a feature *or an
interaction* (keyboard, focus, busy state, feedback, click-through, flow) that is not
in the desktop app and not listed under "Not carried over", it is a gap.

Source: AttendFlow `main` @ 405abd5 (inventory 2026-11-17). Phase column: P1 / P2 / P3,
**N/A** = exists only because of Zoom Apps constraints (replaced by a simpler mechanism),
**OFF** = feature-flagged off in AttendFlow's `mvp` build (optional, P3).

## Engine and data (must be semantically identical)

| Feature | AttendFlow source | Phase | Rust status |
|---|---|---|
| Name normalization (trim, lowercase, strip whitespace incl. U+3000, punctuation) | lib/name-utils.ts | P1 | ✓ |
| Pinyin (tiny-pinyin, toneless) for search, sort (zh-CN collation), roster auto-fill | name-utils.ts | P1 | partial: name corpus differs on individual characters; roster row ordering matches all captured variants |
| Identity stitching: uuid_direct → unique inactive name → unique inactive roster/alias target → new / ambiguous | session-engine.ts resolveSessionParticipantForRecord | P1 | ingestion concern; bridge admission/rejoin covered, duplicate/active-conflict parity incomplete |
| Movement events (7 types) incl. waiting-room ones; admission pairing ±5 s | session-engine.ts | P1 | partial: bridge event stream tests; Zoom Apps replay differs |
| Presence intervals, absenceCount, returnedAfterLongAbsence (return window), wasLate (late threshold) | buildAttendanceStates, resolveCurrentStatus | P1 | partial |
| Statuses present / missing / late / leftEarly / returned / guest / unmatched; tags seen, full_present, late, left_early, returned, missing, ignored | lib/attendance-status.ts | P1 | ✓: generic and roster parity goldens |
| Summary counts (generic vs roster counting rules) | computeAttendanceSummary | P1 | ✓: generic and roster parity goldens |
| Score % = presence / duration, grades ≥90 / ≥75 / ≥50 / >0 / 0 | lib/attendance-score.ts | P1 | ✓ |
| Roster matching: canonical name / pinyin / alias; >1 candidate = error | matchZoomNameToRoster | P1 | ✓: roster parity goldens |
| Decisions: match, guest, split (≥2 people), name_change_keep_match, revert | repository.ts + engine | P2 | ✓: match, guest, split, keep-match precedence covered in 448 variants; revert is candidate fallback |
| Shared-name detection + split suggestions | isLikelySharedName, getSplitSuggestionsForZoomName | P2 | partial: shared-name detection parity covered; token parsing/suggestions remain separate |
| Name-change events + pending-rename gating | engine | P1 (log) / P2 (gating UI) | partial: log only |
| Profile auto-selection: meetingIds exact → topic keyword → default → first active | guessMeetingProfile | P1 | ✓ |
| Profile fields: name, description, included subgroups (`*`), late, return, meetingIds, topic keywords, group sort order, active; default profile `default-all` (locked, factory reset 10 / 8) | MeetingProfileWorkspace | P2 | ✓: table, filters/sort, editor, bulk actions, wildcard checks/help, import preview/commit, selected live profile + overrides |
| isExpected via included subgroups; out-of-scope ⚠ | engine | P1 | ✓: wildcard profile scope covered in roster parity goldens |
| Manual snapshots (capture current list) | ManualAttendanceTab | P2 | ✓: live and record tabs; timestamped participant, location and match status snapshots persisted in revisioned meeting state; newest snapshot expanded |
| Raise-hand entries + captures (dedup raise/lower) | RaiseHandPanel | P2 (better: SDK gives raised flag at connect) | ✓: current raised users, searchable raise/lower log with transition dedup, startup snapshot raises, persisted capture; capture disabled when empty |
| **Deliberate deviation:** waiting-room time is shown separately and is NOT presence (owner 2026-11-17); AttendFlow counts it. Engine flag reproduces AttendFlow for the parity harness | applyJoinLikeEvent | P1 | ✓: option and separate row fields |
| **Deliberate deviation:** no new record after a meeting ends (AttendFlow phantom-session bug) | Meeting.svelte effect | P1 | ✓: ignores subsequent events |

## Parity run status (2026-11-17)

Desktop record detail and live activity tabs were added on 2026-11-17. Record
exports are served from Rust (`zoom-companion/src/record_export.rs`): version-3
session bundle, localized summary CSV with trailing waiting-room minutes, and
manual-record CSV. CSV output includes UTF-8 BOM, AttendFlow column order, and
score/group/name ordering. Export writes `exportedAt` and clears `needsExport` in
revisioned state. Rust export/state tests and UI filter/sort/event/hand tests cover
these additions; attendance derivation remains unchanged.

### Desktop implementation checklist

- [x] Record detail header, overview cards, local Generic/Roster attendance view,
  filters, search, sort, score grades, and read-only Name Matching tab.
- [x] Attendance Settings, Name Changes, Entry/Exit, Raise Hand, and Manual
  Attendance Records tabs; matching activity tabs are available in live view.
- [x] Manual snapshots and raise-hand captures persist in meeting state; initial
  raised-hand flags are included and repeated raise/lower events are deduplicated.
- [x] Rust per-record JSON, summary CSV, manual-record CSV, export-all downloads,
  localized column headers, UTF-8 BOM, sorting, and exported status updates.
- [x] Rust state/export coverage and UI filter/sort/event/hand helper tests pass;
  `scripts/test.ps1`, `pnpm check`, `pnpm test --run`, and `pnpm build` pass.

The 56 generated outputs contain a full SessionRecord, exact reference time,
generic rows/summary, and eight roster variants (two example-config languages and
four decision actions). Rust exposes `derive(SessionRecord, config, decisions,
reference_time, options)`. Generic derivation parity passes 56/56 and roster-mode
parity passes 448/448, with exact row fields, row order, and summaries. The full
upstream test port and companion scenario catalogue remain unfinished. The Zoom
Apps harness remains ignored and informational at 33/56; its discrepancies include
batch-timed roster observations and identity resolution. No failures are allow-listed.
Per-fixture results remain listed in
[`crates/attend-engine/tests/parity/PARITY-RESULTS.md`](../crates/attend-engine/tests/parity/PARITY-RESULTS.md).

## Live meeting view (AttendFlow Meeting surface)

| Feature | Phase |
|---|---|
| Header: connection, stop/start record, language toggle, UI size (5 steps) | P1 (connection, language) / P2 |
| Summary strip (Seen, Full Present, Late, Returned, Left Early, Absent, Ignored, Total), click = filter | P1 |
| Generic / Roster mode switch | P1 |
| Status table: #, name (+Zoom name, ⚠ out-of-scope, shared note), status chips, location, group, first join, last entry/exit, minutes; status-then-pinyin sort; row tints; resizable columns | P1 (core columns) / P2 (resize, chip overflow) |
| Filters: search (name/pinyin), A→Z/Z→A, roster scope chips, group chips (counts, select all, reverse, drag reorder) | P2 |
| Duplicate-name banner, `#XXXX` disambiguators | P2 |
| Name Matching panel (pending / name changes / resolved; match, guest, split, keep match, revert, manual match) | P2 | ✓: roster live panel, record summary, revisioned decisions and atomic alias effects |
| Name Changes log (search) | P2 |
| Settings & Info (record info, per-record profile / late / return overrides with Apply/Reset) | P2 |
| Manual Attendance tab | P2 |
| Entry/Exit log (7 event-type chips, search) | P2 |
| Raise Hand tab | P2 |
| Stop dialog / stopped state / start new record | P2 (record = meeting instance; "stop" = stop counting) |

## Records, configuration, settings (AttendFlow Main surface)

| Feature | Phase |
|---|---|
| Home cards (records, configuration, settings), active meeting strip | P1 (minimal) / P2 |
| Records list (topic, start, end, mode, # attended, status; date-prefix search; bulk delete/export; import with preview) | P1 (list) / P2 | partial: date-prefix search, bulk delete (active refusal), JSON export; AttendFlow session/record import preview and read-only imported rows |
| Record detail: overview cards, auto attendance (status / name matching / settings / name changes), entry/exit, raise hand, manual records; attendance table generic/roster columns, score pills, tag/group filters, sorts | P1 (attendance table) / P2 | partial: read-only name matching summary and unresolved count |
| Per-record exports: JSON bundle v3, summary CSV (zh/en columns), manual-records CSV; export-all | P2 |
| Roster CRUD, bulk edit/delete, roster.csv import (zh/en headers, UTF-8/GB18030, preview, cascade) / export | P2 | implemented: Rust preview/commit/export APIs, UTF-8/BOM/GB18030 decode, bilingual CSV columns, CRUD and bulk UI, filtered selection and cascade handling |
| Name matching (aliases) CRUD, bulk, CSV import/export with validation rules | P2 | implemented: Rust preview/commit/export APIs and validation; roster autocomplete, filters and bulk UI |
| Attendance profiles CRUD, bulk, JSON import/export, wildcard help | P2 | ✓: Rust validation, locked default/factory reset, bulk editing/deletion, JSON preview/commit, selected export, bilingual editor/help |
| Settings: export language, UI sizes, filter defaults, example data (zh/en), full backup v3 export/restore, clear data | P2 | partial: backup export, config restore preview, clear local data, conflict-atomic AttendFlow example data seed; language/scale/filter persistence remain |
| Import AttendFlow full backup v3 + session bundles (migration path) | P2 | partial: profile/record import paths; full backup import remains separate |
| Bilingual UI zh-CN / en (inline `t(zh, en)`), export language separate | P1 (framework) / P2 |

## Open parity gaps (audit 2026-11-23)

Found by diffing every bilingual UI string between the apps, then reading both sides,
including interaction code. Tick items off as they land.

Found by diffing every bilingual UI string between the apps, then reading both sides.
Tick items off here as they land.

### Feedback, dialogs, interaction

- [x] Toasts: success/error/warning/info types with colors and icons, 5 s auto-dismiss, progress bar, pause on hover, close button, stacking.
- [x] Feedback on every action (AttendFlow: ~110 toast calls; desktop: ~19). Saves, deletes, imports, exports, captures, mode and profile switches, name-matching actions.
- [x] Busy states: buttons disabled with "…ing" labels while a request runs (AttendFlow: 25 components; desktop: name matching only). Prevents double submits.
- [x] No native `alert()` (12) or `confirm()` (5): use toasts and `ConfirmDialog`.
- [x] Bug: `SettingsPage` mounts a second `ConfirmDialog`, so each confirm on Settings opens twice.
- [x] `ConfirmDialog`: Esc cancels, Enter confirms, focus moves into the dialog.
- [x] Import panels and the import preview overlay close on Esc.
- [x] Export menu closes on outside click and Esc.
- [x] UI size slider applies on release, not on every drag step; the slider stays put on screen.
- [x] Roster person picker: one type-ahead field with ↑/↓/Enter/Esc and "No results" (desktop: a text box plus a separate dropdown).
- [x] Hover tooltips on icon-only buttons; full-text tooltips on truncated table cells.
- [x] Resizable table columns (owner: include).
- [x] Resizable record-detail slide-over (drag and keyboard).

### Live meeting

- [x] 全程 (Full Present) shown in generic mode too (summary, filter, row badge replaces 到过).
- [x] Summary hides zero counts except 总计.
- [x] Summary visible above every live tab; clicking a count opens Status filtered by it; clicking 不计入 opens Name Matching.
- [x] Remove the stray ↕ in the 状态 header (name sort lives in the filter panel as A→Z/Z→A).
- [x] Duplicate-name banner (⚠ 重复姓名).
- [x] Generic mode: "Zoom 名称" column title and `name · abcd` suffixes for same-name people (the `#XXXX` form is only in the name-matching panel).
- [x] Roster mode: ⚠️ marker for people outside the profile's groups; "疑似重名或共用账号名" note.
- [x] Row tints per status, waiting-room highlight. (AttendFlow flashes the table only after ↻ Refresh, which is not applicable; no flash.)
- [x] 分组 column in roster mode only; times as HH:mm:ss; header "在线分钟".
- [x] Entry/Exit: one chip per event type with counts, and "Clear filters".
- [x] Pinyin search in Entry/Exit, Raise Hand and Name Changes.
- [x] Name Changes as a searchable Previous/New table (live and record detail).
- [x] Settings & Info: record ID, current mode, plain-language late/return rules, Restore default; fix the "Apply profile overrides" button that sends no values.
- [x] Manual attendance: Participant ID and match-status columns.
- [x] Name matching: one-click "拆分共用 (N)" suggestions (shared name and past meetings), "Current match:" line.

- [x] Host/co-host privilege: record role and when it was acquired; show it in the record overview (AttendFlow "权限获取时间 / Privilege Acquired"); clear notes in Live, record detail and exports that waiting-room data is missing while the user is not host/co-host. Host and co-host count as the same privilege (owner 2026-11-23).

### Records

- [x] Record overview: duration, profile name (not ID), revision, export status, meeting UUID, operator-action counts (manual records, decisions, unmatched, guests).
- [x] Record import: several files or pasted JSON.
- [x] Records list uses the same "Bulk Select" mode as the Configuration tables.
- [x] Remove the Mode, Source and Status dropdown filters from the Records list (added by Codex in `63dd3a8` without an owner decision; owner 2026-11-23: remove).

### Configuration

- [x] Import preview: summary counts (total, adds, updates, deletes, rejected, warnings, applicable) and readable Before/After instead of raw JSON; scroll to the preview when it opens.

### Settings

- [x] Restore full backup: preview with backup version, date and "current → backup" counts per dataset, then full replacement (records included).
- [x] Clear local data resets everything (see "Changed on purpose").

## Not carried over

### Skipped by owner decision

| AttendFlow feature | Decision | Date |
|---|---|---|
| Stop / Start attendance record (stop dialog, stopped state, "start new record") | Left out for now. Capture starts automatically and a record is one meeting instance. Revisit if a "stop counting" switch is wanted. | 2026-11-23 |
| Home page (cards, backup reminder, "N records not downloaded" reminder) | Skipped. The sidebar replaces the home cards; the Records list shows Downloaded/Ended status. | 2026-11-23 |
| A mid-meeting panel reload closes the record and starts a new one | Companion keeps the instance; a UI reload resumes it. | 2026-11-17 |
| Waiting-room time counts as attendance | Separate field (`waitingRoomMinutes`); non-hosts cannot see the waiting room. | 2026-11-17 |
| A record can restart after the meeting ends (phantom session) | An ended instance is never reopened. | 2026-11-17 |
| Archive analytics, Operations tab, special participants, blacklist actions | OFF in shipped AttendFlow (`showArchiveAnalytics`, `showMeetingOperations`, `enableProfileSpecialParticipants`, `enableProfileBlacklist`); optional P3 modules. | — |
| Design-system demo page | Developer-only in AttendFlow. | — |

### Not applicable to a desktop app

| AttendFlow mechanism | Why / replacement |
|---|---|
| Install page, Zoom App install flow, Firebase | Not a Zoom App; installer/package and in-app Zoom sign-in instead |
| Main ↔ Meeting sync: Sync button, "Settings connected" status, postMessage, heartbeat, fallback closure, sessionRevision/ack/cleanup | One process, one store; live updates over `/bridge` |
| Alias mutation outbox, Sync Review workspace, processed alias requests | Single writer: decisions write aliases directly (same validation rules) |
| Download center / export tray (prepare, retry, copy to clipboard), remote download worker | Direct download from the local server |
| Host/co-host gating, limited mode, "Insufficient privilege" screen | Plugin SDK works as a plain attendee |
| Collapse/expand window and window-mode toggles | Zoom side-panel sizing; normal desktop window |
| Separate Meeting-panel and Settings-panel UI sizes, quick size switch in the meeting header | One window, one UI size in Settings |
| "↻ Refresh current roster" in the status table | The list updates live |
| SDK event recording controls | Zoom Apps developer tool; per-instance JSONL logs instead |
| "Back up regularly, Zoom may clear your data" warning | Data is in SQLite under `%LOCALAPPDATA%` |
| Separate Dexie DBs, orphaned-session cleanup | SQLite (`attend-store`); one record per meeting instance |

### Changed on purpose

| AttendFlow | Desktop |
|---|---|
| Home + breadcrumb + page tabs | Sidebar navigation (Live, Records, Configuration, Settings) |
| Settings "Clear local data" | Same scope (owner 2026-11-23: match the original). Clearing only records is done from the Records list: select all, Delete selected |
| Records list: start date/time prefix search, no sorting | Search also matches topic, meeting number and end time; click a column header to sort (added by Codex in `63dd3a8`, kept by owner 2026-11-23) |

## Desktop additions

- Works as a plain attendee: full participant list and live join/leave without host/co-host
  (Plugin SDK; SDK-NOTES "In-meeting observations").
- Background capture without the UI open; the record survives UI reloads and companion
  restarts (resume, including quiet meetings via a 30 s heartbeat).
- Real join times from when the companion first sees the meeting.
- Waiting-room location and minutes (host), kept separate from attendance.
- Raised-hand state at connect.
- One engine (`attend-engine`, Rust) for capture and records; derivation matches
  AttendFlow exactly (parity goldens above).
- SQLite storage that survives browser resets; append-only JSONL logs as backup.
- Import of AttendFlow backups and session bundles (migration path).
- In-app Zoom sign-in/sign-out with token revocation; Zoom connection banner.
- Native window (WebView2); browser access via `zoom-companion serve`.
- Always-on-top pin and a compact live window; window mode, size, position and pin are remembered.
- Hand log closes raises when a participant leaves with a hand up, and lowers from the participant
  flag (lower-all, flag-only).
- Custom record names (rename any record; default title stays visible).
- Start a new record mid-meeting (manual split; same semantics as AttendFlow's reload split, with confirmation).
- Camera on/off minutes and %, mic unmute count and unmuted minutes (a proxy, not speech), audio
  type markers (no audio / phone) per person; also rebuilt from logs recorded before this change.
- Nonverbal feedback (Yes/No/Faster/Slower/Away) and emoji reactions per person, with a Yes
  check-in tally.
- Planned (P3): auto-admit roster matches from the waiting room (host), unknown-arrival
  alerts, autostart, notifications.
