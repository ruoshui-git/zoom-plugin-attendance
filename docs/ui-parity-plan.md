# AttendFlow Desktop UI parity plan

## Surface inventory and current desktop mapping

| AttendFlow surface | Desktop equivalent today | Phase 2 direction / gap |
|---|---|---|
| Meeting primary tabs (attendance, operations, raise hand, settings) | One Live sidebar page; `RecordActivity` below the attendance table | Add primary tabs within Live; current content is vertically stacked |
| Attendance status, name matching, name changes | Live table, `NameMatchingPanel`; matching also has its own sidebar page | Attendance secondary tabs; keep matching on its own secondary view and retain direct entry from summary |
| Entry/exit log, manual attendance, raise-hand captures | `RecordActivity` | Secondary tabs; compact event and capture views |
| Sortable filter stack / draggable filters | No equivalent; fixed summary buttons and text input | Reorderable filter tabs, persist order locally; status filters as compact chips |
| Meeting records list | Records page in `App.svelte` | Compact searchable/selectable table and bulk actions |
| Record detail and attendance/events/snapshots/raise-hand/name-change tabs | Inline record detail branch and `RecordActivity` | Detail tabs; improve density and keep export actions accessible |
| Configuration workspaces: roster, aliases, profiles | `ConfigWorkspace`, `MeetingProfileWorkspace` | Sidebar Configuration page with compact workspace navigation |
| Settings | Inline settings branch | Sidebar Settings page; keep language, example data, backup and local-data actions |
| Filters / filter input | Inline search and fixed summary buttons | Shared `ChipFilter`, `FilterInput`, sortable filter stack |
| Dialogs | Browser `confirm`/`prompt`, custom import overlay | Shared `ConfirmDialog` and `SlideOverPanel`; replace native prompts as page work proceeds |
| Toasts | Inline toast message | Shared `Toast`, preserve translated messages |
| Export tray | Direct export buttons/actions | Shared tray/dropdown pattern in record details; desktop API exports remain unchanged |
| Common status, data table, tooltip, toggles, form actions | Inline markup and global CSS | Shared primitives in `src/lib/common/` |

## Information architecture

Keep the desktop sidebar as the top-level navigation for Live, Records, Configuration, and Settings. The Live page will adopt AttendFlow's meeting-client hierarchy: primary tabs for Attendance, Operations, Raise hand, and Settings, with secondary tabs within Attendance for Status, Name matching, Name changes, Entry/exit, and Manual attendance as applicable. Name matching stays reachable as a Live secondary view; the existing sidebar shortcut can remain during transition. Records uses a list then a selected detail view with detail tabs. Exclude MainClient/MeetingClient communication, sync-review, Zoom Apps SDK/install/privilege flows, cloud upload/download, and design demos.

## Concrete parity gaps

- Live meeting details, profile controls, eight summary cards, participant search/table, and activity are stacked, so essential attendance content is not compact.
- The desktop has no primary/secondary tab hierarchy and duplicates name matching in sidebar and Live content.
- Summary-card filters are fixed buttons; there is no draggable/reorderable filter stack or shared chip filter behavior.
- Records and record details are inline conditional markup with limited tab structure; export and bulk actions need clearer grouping.
- Configuration and Settings are embedded in a very long component. Spacing, table density, labels and controls are not consistently shared.
- Native `alert`/`confirm`/`prompt` calls bypass the app's visual language; import preview is the only custom panel pattern.
- Backend gap: the current desktop API exposes meeting records, events, state, config and exports, but does not expose a separate live participant stream/history contract for every original meeting subview. Phase 2 should map subviews to available record/events/state data; do not change Rust endpoints.

## Phase 2 workstreams and file ownership

Each workstream owns its listed files and may add files only under its owned page directory. Shared shell, common primitives, API/store modules and global styles are frozen; phase 2 may make additive changes to shared files only, and must coordinate any required shared API with the owner before use.

### A — Live meeting page, including name matching

- `apps/desktop-ui/src/pages/LivePage.svelte`
- `apps/desktop-ui/src/pages/live/**`
- `apps/desktop-ui/src/lib/NameMatchingPanel.svelte`
- `apps/desktop-ui/src/lib/RecordActivity.svelte`

### B — Records list and record detail

- `apps/desktop-ui/src/pages/RecordsPage.svelte`
- `apps/desktop-ui/src/pages/records/**`
- `apps/desktop-ui/src/lib/record-tools.ts`
- `apps/desktop-ui/src/lib/formatting.ts`

### C — Configuration and Settings

- `apps/desktop-ui/src/pages/ConfigurationPage.svelte`
- `apps/desktop-ui/src/pages/SettingsPage.svelte`
- `apps/desktop-ui/src/pages/configuration/**`
- `apps/desktop-ui/src/lib/ConfigWorkspace.svelte`
- `apps/desktop-ui/src/lib/MeetingProfileWorkspace.svelte`
- `apps/desktop-ui/src/lib/ProfileOverrideForm.svelte`

## Phase 2 frozen shared files

`apps/desktop-ui/src/App.svelte`, `apps/desktop-ui/src/main.ts`, `apps/desktop-ui/src/lib/api.ts`, `apps/desktop-ui/src/lib/stores.ts`, `apps/desktop-ui/src/lib/common/**`, `apps/desktop-ui/src/style.css`, `apps/desktop-ui/src/lib/i18n-core.ts`, and package/config files are shared/frozen. Only additive changes are allowed during phase 2; page owners should not reformat or restructure these files. Coordinate shared changes through the integration owner. No Rust/backend changes are in scope.
