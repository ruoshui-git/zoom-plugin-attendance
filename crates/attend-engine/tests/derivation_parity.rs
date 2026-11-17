use attend_engine::{
    derive, AttendanceRow, AttendanceSummary, EngineConfig, EngineOptions, SessionRecord,
};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Golden {
    fixture: String,
    mode: String,
    config: EngineConfig,
    session_record: SessionRecord,
    reference_time: String,
    derived_rows: Vec<AttendanceRow>,
    summary: AttendanceSummary,
}

#[test]
fn generic_attendflow_derivation_matches_all_goldens() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/goldens");
    let mut files: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    assert_eq!(
        files.len(),
        56,
        "the derivation parity gate must cover all 56 fixtures"
    );
    let mut failures = Vec::new();
    for path in files {
        let golden: Golden = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            golden.mode, "generic",
            "{} is outside this test's GENERIC scope",
            golden.fixture
        );
        let actual = derive(
            &golden.session_record,
            &golden.config,
            &golden.session_record.reconciliation_decisions,
            &golden.reference_time,
            &EngineOptions::attendflow_compat(),
        )
        .unwrap();
        if actual.rows != golden.derived_rows || actual.summary != golden.summary {
            let row_diffs: Vec<String> = (0..actual.rows.len().max(golden.derived_rows.len()))
                .filter_map(|i| {
                    let got = actual.rows.get(i);
                    let expected = golden.derived_rows.get(i);
                    (got != expected).then(|| {
                        let got = got
                            .map(|row| serde_json::to_value(row).unwrap())
                            .unwrap_or_default();
                        let expected = expected
                            .map(|row| serde_json::to_value(row).unwrap())
                            .unwrap_or_default();
                        let fields: Vec<_> = expected
                            .as_object()
                            .into_iter()
                            .flat_map(|obj| obj.keys())
                            .chain(got.as_object().into_iter().flat_map(|obj| obj.keys()))
                            .collect();
                        let mut fields = fields.into_iter().map(String::as_str).collect::<Vec<_>>();
                        fields.sort();
                        fields.dedup();
                        let differences: Vec<_> = fields
                            .into_iter()
                            .filter_map(|key| {
                                (expected[key] != got[key])
                                    .then(|| format!("{key}={:?}->{:?}", expected[key], got[key]))
                            })
                            .collect();
                        format!(
                            "row[{i}] expected {} {} actual {} {}: {}",
                            expected["entityId"],
                            expected["zoomName"],
                            got["entityId"],
                            got["zoomName"],
                            differences.join(",")
                        )
                    })
                })
                .take(5)
                .collect();
            failures.push(format!(
                "{}: rows={}/{} summary={:?}/{:?}; {}",
                golden.fixture,
                actual.rows.len(),
                golden.derived_rows.len(),
                actual.summary,
                golden.summary,
                row_diffs.join(" | ")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "AttendFlow derivation parity failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
