# AttendFlow derivation parity

`tests/derivation_parity.rs` is the hard derivation gate. It loads the 56 captured
GENERIC-mode goldens, deserializes each complete `SessionRecord` and its config,
then calls `derive` with the recorded reference time and
`EngineOptions::attendflow_compat()`. It compares the full row structs in order and
the summary structs. Fixture errors report the changed row fields.

The comparison intentionally starts from persisted records. Replaying the event
stream through ingestion would create new session participant identifiers and
would not test derivation independently of SDK/repository batching. The extra
roster variants embedded in goldens are not part of this test.

AttendFlow quirks represented by this port include its movement-event tie order,
latest-roster identity refresh, rounded elapsed minutes, ±5-second waiting-room
leave/admission pairing, status-tag priority, and expanded ISO years from legacy
recordings. The Rust engine uses the supplied reference time so active-session
results remain deterministic.

Run the crate verification with:

```powershell
cargo test -p attend-engine
```

The Zoom-Apps replay comparison and roster-mode variants remain separate work.
