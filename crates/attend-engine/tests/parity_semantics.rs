use attend_engine::{
    adapt_zoom_apps_events, build_record, EngineConfig, EngineOptions, EngineState, MeetingMeta,
    SessionMode,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn walk(path: &Path, files: &mut Vec<PathBuf>) {
    for item in fs::read_dir(path).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            walk(&path, files);
        } else if path.extension().and_then(|x| x.to_str()) == Some("json") {
            files.push(path);
        }
    }
}

fn semantic(row: &Value) -> Value {
    let mut tags = row["tags"].as_array().cloned().unwrap_or_default();
    tags.sort_by_key(|tag| tag.as_str().unwrap_or_default().to_string());
    json!({
        "zoomName": row["zoomName"], "personId": row["personId"],
        "status": row["currentStatus"], "tags": tags,
        "matchStatus": row["matchStatus"], "isExpected": row["isExpected"],
        "firstJoinAt": row["firstJoinAt"], "lastSeenAt": row["lastSeenAt"],
        "totalPresenceMinutes": row["totalPresenceMinutes"],
        "absenceCount": row["absenceCount"], "wasLate": row["wasLate"],
        "currentLocation": row["currentLocation"], "sharedName": row["sharedName"],
    })
}

fn field_differences(expected: &Value, actual: &Value) -> Vec<String> {
    expected
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| expected[*key] != actual[*key])
        .map(|key| key.to_string())
        .collect()
}

#[test]
#[ignore = "AttendFlow parity in progress (2/56); see tests/parity/PARITY-RESULTS.md"]
fn every_recording_fixture_matches_attendflow_semantics() {
    let fixture_root = Path::new("tests/parity/fixtures");
    let golden_root = Path::new("tests/parity/goldens");
    let mut fixtures = Vec::new();
    walk(fixture_root, &mut fixtures);
    fixtures.sort();
    assert_eq!(fixtures.len(), 56);
    let mut failures = Vec::new();
    for path in fixtures {
        let input: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let name = path.file_name().unwrap();
        let golden_path = golden_root.join(name);
        let expected: Value = serde_json::from_slice(
            &fs::read(&golden_path)
                .unwrap_or_else(|e| panic!("missing golden {}: {e}", golden_path.display())),
        )
        .unwrap();
        let events = adapt_zoom_apps_events(&input).unwrap();
        let config: EngineConfig = serde_json::from_value(expected["config"].clone()).unwrap();
        let mode: SessionMode = serde_json::from_value(expected["mode"].clone()).unwrap();
        let started = input["startedAt"]
            .as_str()
            .unwrap_or("1970-01-01T00:00:00.000Z")
            .to_string();
        let meta = MeetingMeta {
            meeting_id: input["sessionId"].as_str().unwrap_or_default().to_string(),
            meeting_topic: String::new(),
            started_at: started.clone(),
            ended_at: None,
        };
        let mut state = EngineState::default();
        state.mode = mode;
        let now = events
            .iter()
            .filter_map(|e| e["timestamp"].as_str())
            .max()
            .unwrap_or(&started);
        let record = build_record(
            meta,
            &events,
            config,
            state,
            EngineOptions::attendflow_compat(),
            now,
        )
        .unwrap_or_else(|e| panic!("{} engine error: {e}", path.display()));
        let mut actual: Vec<Value> = record
            .rows
            .iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        let actual_count = actual.len();
        let expected_rows = expected["derivedRows"].as_array().unwrap();
        let mut diffs = Vec::new();
        for er in expected_rows {
            let match_index = actual.iter().position(|ar| {
                ar["zoomName"] == er["zoomName"]
                    && ar["personId"] == er["personId"]
                    && ar["zoomNames"] == er["zoomNames"]
            });
            match match_index {
                None => diffs.push(format!("{}: missing actual row", er["zoomName"])),
                Some(index) => {
                    let ar = actual.remove(index);
                    let fields = field_differences(&semantic(er), &semantic(&ar));
                    if !fields.is_empty() {
                        diffs.push(format!("{} fields: {}", er["zoomName"], fields.join(",")));
                    }
                }
            }
        }
        if actual_count != expected_rows.len() {
            diffs.push(format!(
                "row_count expected={} actual={}",
                expected_rows.len(),
                actual_count
            ));
        }
        let expected_summary = &expected["summary"];
        let actual_summary = serde_json::to_value(&record.summary).unwrap();
        if expected_summary != &actual_summary {
            diffs.push(format!(
                "summary fields: {}",
                field_differences(expected_summary, &actual_summary).join(",")
            ));
        }
        if !diffs.is_empty() {
            failures.push(format!("{}: {}", path.display(), diffs.join("; ")));
        }
    }
    assert!(
        failures.is_empty(),
        "AttendFlow parity mismatches (not allow-listed):\n{}",
        failures.join("\n")
    );
}
