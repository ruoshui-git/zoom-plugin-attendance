use attend_engine::{
    derive, AliasMapping, AttendanceRow, AttendanceSummary, EngineConfig, EngineOptions,
    MeetingProfile, ReconciliationDecision, RosterPerson, SessionMode, SessionRecord,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Golden {
    fixture: String,
    session_record: SessionRecord,
    reference_time: String,
    roster_variants: Vec<Variant>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Variant {
    language: String,
    action: String,
    config: VariantConfig,
    derived_rows: Vec<Value>,
    summary: AttendanceSummary,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VariantConfig {
    roster_people: Vec<RosterPerson>,
    alias_mappings: Vec<AliasMapping>,
    profiles: Vec<MeetingProfile>,
}

#[test]
fn roster_attendflow_derivation_matches_all_variants() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/goldens");
    let mut files: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 56);
    let mut count = 0;
    let mut failures = Vec::new();
    for path in files {
        let golden: Golden = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(golden.roster_variants.len(), 8, "{}", golden.fixture);
        for variant in golden.roster_variants {
            count += 1;
            let config = EngineConfig {
                roster: variant.config.roster_people,
                aliases: variant.config.alias_mappings,
                profiles: variant.config.profiles,
            };
            let mut session = golden.session_record.clone();
            session.mode = SessionMode::Roster;
            session.selected_profile_id = config.profiles.first().map(|p| p.id.clone());
            let picked = session.session_participants.first();
            let decisions = picked
                .map(|p| {
                    vec![ReconciliationDecision {
                        id: format!("golden-{}", variant.action),
                        session_participant_id: p.session_participant_id.clone(),
                        zoom_name: p.current_screen_name.clone(),
                        zoom_name_at_decision: p.current_screen_name.clone(),
                        action: match variant.action.as_str() {
                            "match" => attend_engine::DecisionAction::Match,
                            "guest" => attend_engine::DecisionAction::Guest,
                            "split" => attend_engine::DecisionAction::Split,
                            "name_change_keep_match" => {
                                attend_engine::DecisionAction::NameChangeKeepMatch
                            }
                            other => panic!("unknown golden decision action {other}"),
                        },
                        person_id: if matches!(
                            variant.action.as_str(),
                            "match" | "name_change_keep_match"
                        ) {
                            config.roster.first().map(|p| p.person_id)
                        } else {
                            None
                        },
                        canonical_name: if matches!(
                            variant.action.as_str(),
                            "match" | "name_change_keep_match"
                        ) {
                            config.roster.first().map(|p| p.canonical_name.clone())
                        } else {
                            None
                        },
                        split_person_ids: if variant.action == "split" {
                            config.roster.iter().take(2).map(|p| p.person_id).collect()
                        } else {
                            vec![]
                        },
                        split_canonical_names: if variant.action == "split" {
                            config
                                .roster
                                .iter()
                                .take(2)
                                .map(|p| p.canonical_name.clone())
                                .collect()
                        } else {
                            vec![]
                        },
                        notes: None,
                        created_at: session.started_at.clone(),
                    }]
                })
                .unwrap_or_default();
            let actual = derive(
                &session,
                &config,
                &decisions,
                &golden.reference_time,
                &EngineOptions::attendflow_compat(),
            );
            let expected: Vec<AttendanceRow> =
                serde_json::from_value(Value::Array(variant.derived_rows.clone())).unwrap();
            let expected_keys = variant
                .derived_rows
                .iter()
                .flat_map(|r| r.as_object().unwrap().keys().cloned())
                .collect::<BTreeSet<_>>();
            match actual {
                Ok(actual) => {
                    let struct_keys = actual
                        .rows
                        .iter()
                        .flat_map(|r| {
                            serde_json::to_value(r)
                                .unwrap()
                                .as_object()
                                .unwrap()
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                        })
                        .collect::<BTreeSet<_>>();
                    assert!(
                        expected_keys.is_subset(&struct_keys),
                        "AttendanceRow misses golden row keys: {:?}",
                        expected_keys.difference(&struct_keys).collect::<Vec<_>>()
                    );
                    if actual.rows != expected || actual.summary != variant.summary {
                        let diffs = (0..actual.rows.len().max(expected.len()))
                            .filter_map(|i| {
                                let got = actual
                                    .rows
                                    .get(i)
                                    .map(|r| serde_json::to_value(r).unwrap())
                                    .unwrap_or(Value::Null);
                                let exp = expected
                                    .get(i)
                                    .map(|r| serde_json::to_value(r).unwrap())
                                    .unwrap_or(Value::Null);
                                (got != exp).then(|| {
                                    let fields = got
                                        .as_object()
                                        .into_iter()
                                        .flat_map(|o| o.keys())
                                        .chain(exp.as_object().into_iter().flat_map(|o| o.keys()))
                                        .cloned()
                                        .collect::<BTreeSet<_>>();
                                    let differences = fields
                                        .into_iter()
                                        .filter_map(|field| {
                                            (got[&field] != exp[&field]).then(|| {
                                                format!(
                                                    "{field}={:?}->{:?}",
                                                    exp[&field], got[&field]
                                                )
                                            })
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    format!(
                                        "row[{i}] expected {} {} actual {} {}: {differences}",
                                        exp["entityId"],
                                        exp["zoomName"],
                                        got["entityId"],
                                        got["zoomName"]
                                    )
                                })
                            })
                            .take(5)
                            .collect::<Vec<_>>()
                            .join(" | ");
                        failures.push(format!(
                            "{} [{} {}]: rows {}/{}, summary {:?}/{:?}; {}",
                            golden.fixture,
                            variant.language,
                            variant.action,
                            actual.rows.len(),
                            expected.len(),
                            actual.summary,
                            variant.summary,
                            diffs
                        ));
                    }
                }
                Err(error) => failures.push(format!(
                    "{} [{} {}]: {error}",
                    golden.fixture, variant.language, variant.action
                )),
            }
        }
    }
    assert_eq!(count, 448);
    assert!(
        failures.is_empty(),
        "Roster AttendFlow parity failures ({}):\n{}",
        failures.len(),
        failures
            .iter()
            .take(80)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
