# AttendFlow parity golden generator

`replay.test.ts` mounts AttendFlow's real `Meeting.svelte` surface, replays each copied
recording with `ZoomSdkMock`, waits for the mock and the 1-second repository debounce,
then serializes Dexie's final derived rows, real `computeAttendanceSummary`, participant
history, movements, and name-change events. It uses the source snapshot's own setup and
replay helpers. No tracked source files in that snapshot are changed.

Run from this crate's repository checkout:

```powershell
.\crates\attend-engine\tests\parity\generator\run.ps1 `
  -AttendFlowRoot 'C:\path\to\zt-main'
```

The supplied AttendFlow snapshot has its `node_modules` linked and supports
`npx vitest run`. Goldens are overwritten deterministically except for generated
AttendFlow session IDs, which are not used in semantic comparisons.
