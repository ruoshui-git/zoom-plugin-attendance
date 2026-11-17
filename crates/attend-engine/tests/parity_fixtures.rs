use serde_json::Value;
use std::{fs, path::Path};

fn walk(path: &Path, files: &mut Vec<std::path::PathBuf>) {
    for item in fs::read_dir(path).unwrap() {
        let path = item.unwrap().path();
        if path.is_dir() {
            walk(&path, files);
        } else if path.extension().and_then(|x| x.to_str()) == Some("json") {
            files.push(path);
        }
    }
}

#[test]
fn reference_fixture_inputs_are_valid_and_adaptable() {
    let root = Path::new("tests/parity/fixtures");
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    assert_eq!(
        files.len(),
        56,
        "fixture inventory changed; update parity metadata"
    );
    for path in files {
        let input: Value = serde_json::from_slice(&fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let result = attend_engine::adapt_zoom_apps_events(&input);
        assert!(result.is_ok(), "{}: {:?}", path.display(), result.err());
    }
}

#[test]
fn reference_goldens_capture_full_session_records_and_roster_variants() {
    let root = Path::new("tests/parity/goldens");
    let mut files = Vec::new();
    walk(root, &mut files);
    assert_eq!(files.len(), 56, "golden inventory changed");
    for path in files {
        let golden: Value = serde_json::from_slice(&fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        serde_json::from_value::<attend_engine::SessionRecord>(golden["sessionRecord"].clone())
            .unwrap_or_else(|e| panic!("{} sessionRecord schema: {e}", path.display()));
        let session = &golden["sessionRecord"];
        for field in [
            "startedAt",
            "endedAt",
            "isActive",
            "mode",
            "lateThresholdMinutes",
            "returnWindowMinutes",
            "selectedProfileId",
            "guessedProfileId",
            "selectedIncludedSubgroups",
            "selectedGroupSortOrder",
            "sessionParticipants",
            "movementEvents",
            "nameChangeEvents",
            "latestRoster",
            "manualSnapshots",
            "reconciliationDecisions",
            "raiseHandEntries",
            "raiseHandCaptures",
        ] {
            assert!(
                session.get(field).is_some(),
                "{} missing sessionRecord.{field}",
                path.display()
            );
        }
        assert!(
            golden["referenceTime"].is_string(),
            "{} missing referenceTime",
            path.display()
        );
        assert!(
            golden["derivedRows"].is_array(),
            "{} missing derivedRows",
            path.display()
        );
        assert!(
            golden["summary"].is_object(),
            "{} missing summary",
            path.display()
        );
        assert_eq!(
            golden["rosterVariants"].as_array().map(Vec::len),
            Some(8),
            "{} roster variants",
            path.display()
        );
    }
}

#[test]
fn derive_accepts_a_captured_session_without_replaying_zoom_apps_fixtures() {
    let path = Path::new("tests/parity/goldens/simple-meeting.json");
    let golden: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let session: attend_engine::SessionRecord =
        serde_json::from_value(golden["sessionRecord"].clone()).unwrap();
    let config: attend_engine::EngineConfig = serde_json::from_value(golden["config"].clone()).unwrap();
    let decisions = session.reconciliation_decisions.clone();
    let actual = attend_engine::derive(
        &session,
        &config,
        &decisions,
        golden["referenceTime"].as_str().unwrap(),
        &attend_engine::EngineOptions::attendflow_compat(),
    )
    .unwrap();
    assert_eq!(actual.rows.len(), golden["derivedRows"].as_array().unwrap().len());
}

#[test]
fn pinyin_matches_tiny_pinyin_for_fixture_and_config_names() {
    #[derive(serde::Deserialize)]
    struct Golden {
        name: String,
        expected: String,
    }
    let data: Vec<Golden> =
        serde_json::from_slice(&fs::read("tests/parity/pinyin-goldens.json").unwrap()).unwrap();
    let mut seen = 0;
    for item in data.iter().filter(|item| item.name.chars().count() > 1) {
        seen += 1;
        assert_eq!(
            attend_engine::build_pinyin(&item.name),
            item.expected,
            "pinyin mismatch for {:?}",
            item.name
        );
    }
    assert!(seen >= 200, "pinyin corpus unexpectedly shrank: {seen}");
}
