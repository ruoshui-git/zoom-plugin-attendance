# AttendFlow → desktop app parity checklist

Source: AttendFlow `main` @ 405abd5 (inventory 2026-11-17). Phase column: P1 / P2 / P3,
**N/A** = exists only because of Zoom Apps constraints (replaced by a simpler mechanism),
**OFF** = feature-flagged off in AttendFlow's `mvp` build (optional, P3).

## Engine and data (must be semantically identical)

| Feature | AttendFlow source | Phase |
|---|---|---|
| Name normalization (trim, lowercase, strip whitespace incl. U+3000, punctuation) | lib/name-utils.ts | P1 |
| Pinyin (tiny-pinyin, toneless) for search, sort (zh-CN collation), roster auto-fill | name-utils.ts | P1 |
| Identity stitching: uuid_direct → unique inactive name → unique inactive roster/alias target → new / ambiguous | session-engine.ts resolveSessionParticipantForRecord | P1 |
| Movement events (7 types) incl. waiting-room ones; admission pairing ±5 s | session-engine.ts | P1 |
| Presence intervals, absenceCount, returnedAfterLongAbsence (return window), wasLate (late threshold) | buildAttendanceStates, resolveCurrentStatus | P1 |
| Statuses present / missing / late / leftEarly / returned / guest / unmatched; tags seen, full_present, late, left_early, returned, missing, ignored | lib/attendance-status.ts | P1 |
| Summary counts (generic vs roster counting rules) | computeAttendanceSummary | P1 |
| Score % = presence / duration, grades ≥90 / ≥75 / ≥50 / >0 / 0 | lib/attendance-score.ts | P1 |
| Roster matching: canonical name / pinyin / alias; >1 candidate = error | matchZoomNameToRoster | P1 |
| Decisions: match, guest, split (≥2 people), name_change_keep_match, revert | repository.ts + engine | P2 |
| Shared-name detection + split suggestions | isLikelySharedName, getSplitSuggestionsForZoomName | P2 |
| Name-change events + pending-rename gating | engine | P1 (log) / P2 (gating UI) |
| Profile auto-selection: meetingIds exact → topic keyword → default → first active | guessMeetingProfile | P1 |
| Profile fields: name, description, included subgroups (`*`), late, return, meetingIds, topic keywords, group sort order, active; default profile `default-all` (locked, factory reset 10 / 8) | MeetingProfileWorkspace | P2 |
| isExpected via included subgroups; out-of-scope ⚠ | engine | P1 |
| Manual snapshots (capture current list) | ManualAttendanceTab | P2 |
| Raise-hand entries + captures (dedup raise/lower) | RaiseHandPanel | P2 (better: SDK gives raised flag at connect) |
| Waiting-room time counts as presence (AttendFlow behaviour; owner to confirm) | applyJoinLikeEvent | P1 (configurable later) |

## Live meeting view (AttendFlow Meeting surface)

| Feature | Phase |
|---|---|
| Header: connection, stop/start record, language toggle, UI size (5 steps) | P1 (connection, language) / P2 |
| Summary strip (Seen, Full Present, Late, Returned, Left Early, Absent, Ignored, Total), click = filter | P1 |
| Generic / Roster mode switch | P1 |
| Status table: #, name (+Zoom name, ⚠ out-of-scope, shared note), status chips, location, group, first join, last entry/exit, minutes; status-then-pinyin sort; row tints; resizable columns | P1 (core columns) / P2 (resize, chip overflow) |
| Filters: search (name/pinyin), A→Z/Z→A, roster scope chips, group chips (counts, select all, reverse, drag reorder) | P2 |
| Duplicate-name banner, `#XXXX` disambiguators | P2 |
| Name Matching panel (pending / name changes / resolved; match, guest, split, keep match, revert, manual match) | P2 |
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
| Records list (topic, start, end, mode, # attended, status; date-prefix search; bulk delete/export; import with preview) | P1 (list) / P2 |
| Record detail: overview cards, auto attendance (status / name matching / settings / name changes), entry/exit, raise hand, manual records; attendance table generic/roster columns, score pills, tag/group filters, sorts | P1 (attendance table) / P2 |
| Per-record exports: JSON bundle v3, summary CSV (zh/en columns), manual-records CSV; export-all | P2 |
| Roster CRUD, bulk edit/delete, roster.csv import (zh/en headers, UTF-8/GB18030, preview, cascade) / export | P2 |
| Name matching (aliases) CRUD, bulk, CSV import/export with validation rules | P2 |
| Attendance profiles CRUD, bulk, JSON import/export, wildcard help | P2 |
| Settings: export language, UI sizes, filter defaults, example data (zh/en), full backup v3 export/restore, clear data | P2 |
| Import AttendFlow full backup v3 + session bundles (migration path) | P2 |
| Bilingual UI zh-CN / en (inline `t(zh, en)`), export language separate | P1 (framework) / P2 |

## Not carried over (by design)

| AttendFlow mechanism | Why / replacement |
|---|---|
| Main ↔ Meeting postMessage sync, heartbeat, fallback closure, sessionRevision/ack/cleanup | one process, one store |
| Alias mutation outbox + Sync Review | single writer: decisions write aliases directly (same validation rules) |
| Separate Dexie DBs, orphaned-session cleanup | SQLite in the companion; one record per meeting instance |
| Remote download worker + download center | native/browser download from the local server |
| Install page, Firebase | not needed |
| Host/co-host gating, limited mode | Plugin SDK works as attendee |
| Mid-meeting reload splits records | companion keeps the instance; UI reload resumes |

## Plugin-SDK extras (P3)

Background capture without UI; real join times from meeting start; waiting-room view +
admit / auto-admit roster matches (host); raised-hand state at connect; unknown-arrival
alerts; meeting history across restarts; OFF features (special participants, blacklist
actions, archive analytics) as optional modules.
