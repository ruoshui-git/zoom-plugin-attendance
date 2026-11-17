# AttendFlow derivation parity

`tests/derivation_parity.rs` is the hard derivation gate. It loads the 56 captured
GENERIC-mode goldens, deserializes each complete `SessionRecord` and its config,
then calls `derive` with the recorded reference time and
`EngineOptions::attendflow_compat()`. It compares the full row structs in order and
the summary structs. Fixture errors report the changed row fields.

The roster gate is `tests/derivation_roster_parity.rs`. For each captured record it
recreates the generator's roster config, selected profile, and one decision for the
first participant, then compares every roster row field and ordering plus summary.
It covers all 448 variants (56 records × two example-config languages × four
decision actions). The test also checks that every key present in golden rows is
represented by the Rust row struct.

Both comparisons intentionally start from persisted records. Replaying the event
stream through ingestion would create new session participant identifiers and
would not test derivation independently of SDK/repository batching.

Companion ingestion now constructs that same `SessionRecord` shape and calls the
same `derive` function. It does not calculate rows or summaries itself. The default
owner options exclude waiting-room intervals from presence and stop event processing
at the meeting instance end. `EngineOptions::attendflow_compat()` keeps captured
AttendFlow behavior for the generic and roster golden gates.

AttendFlow quirks represented by this port include its movement-event tie order,
latest-roster identity refresh, rounded elapsed minutes, ±5-second waiting-room
leave/admission pairing, status-tag priority, expanded ISO years from legacy
recordings, guest/match/split decision precedence, split row fan-out, active-roster
missing rows, and Chinese syllable-by-syllable row collation. The Rust engine uses
the supplied reference time so active-session results remain deterministic.

Run the crate verification with:

```powershell
cargo test -p attend-engine
```

The Zoom-Apps replay comparison remains informational and separate from these gates.
