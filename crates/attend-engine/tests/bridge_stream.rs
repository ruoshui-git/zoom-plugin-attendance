use attend_engine::{
    adapt_bridge_events, build_record, EngineConfig, EngineOptions, EngineState, MeetingMeta,
};
use serde_json::{json, Value};

#[test]
fn companion_stream_stitches_admission_rename_hand_resume_and_delayed_leave() {
    let instance = "pz-123-1794893640000";
    let base = 1_794_893_640_000_i64;
    let events: Vec<Value> = vec![
        json!({"kind":"instance_started","instanceId":instance,"at":base,"meeting":{}}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+1000,"users":[{"userId":7,"name":"Riley Chen","waitingRoom":true}]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+2000,"users":[{"userId":7,"name":"Riley Chen","waitingRoom":true}]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+2100,"users":[{"userId":8,"name":"Riley Chen","waitingRoom":false}]}),
        json!({"kind":"users","action":"update","instanceId":instance,"at":base+3000,"users":[{"userId":8,"name":"Riley Chen (guest)","waitingRoom":false}]}),
        json!({"kind":"hand","instanceId":instance,"at":base+4000,"userId":8,"raised":true}),
        json!({"kind":"hand","instanceId":instance,"at":base+5000,"userId":8,"raised":false}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+180_000,"users":[{"userId":8,"name":"Riley Chen (guest)","waitingRoom":false}]}),
        json!({"kind":"instance_resumed","instanceId":instance,"at":base+181_000,"meeting":{}}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+182_000,"users":[{"userId":9,"name":"Riley Chen (guest)","waitingRoom":false}]}),
    ];
    let normalized = adapt_bridge_events(&events).unwrap();
    assert_eq!(
        normalized
            .iter()
            .filter(|e| e["eventType"] == "hand")
            .count(),
        2
    );
    let last = normalized.last().unwrap()["timestamp"]
        .as_str()
        .unwrap()
        .to_string();
    let record = build_record(
        MeetingMeta {
            meeting_id: instance.into(),
            meeting_topic: "Bridge replay".into(),
            started_at: normalized[0]["timestamp"].as_str().unwrap().into(),
            ended_at: None,
        },
        &normalized,
        EngineConfig::default(),
        EngineState::default(),
        EngineOptions::default(),
        &last,
    )
    .unwrap();
    assert_eq!(
        record.participants.len(),
        1,
        "admission and rejoin must preserve identity"
    );
    let person = &record.participants[0];
    assert_eq!(
        person.participant_uuids,
        [
            format!("{instance}:7"),
            format!("{instance}:8"),
            format!("{instance}:9")
        ]
    );
    assert_eq!(person.current_screen_name, "Riley Chen (guest)");
    assert_eq!(record.name_change_events.len(), 1);
    assert_eq!(record.raise_hand_entries.len(), 2);
    assert_eq!(
        record
            .movement_events
            .last()
            .unwrap()
            .session_participant_id
            .as_deref(),
        Some(person.session_participant_id.as_str())
    );
}

#[test]
fn default_waiting_room_fixture_keeps_waiting_time_out_of_presence() {
    let path = "tests/parity/fixtures/zoomsdk-recordings-1/waiting-room-join-then-leave-without-admit-1775069523877.json";
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let events = attend_engine::adapt_zoom_apps_events(&fixture).unwrap();
    let started = fixture["startedAt"].as_str().unwrap();
    let record = build_record(
        MeetingMeta {
            meeting_id: fixture["sessionId"].as_str().unwrap_or_default().into(),
            meeting_topic: "waiting room".into(),
            started_at: started.into(),
            ended_at: None,
        },
        &events,
        EngineConfig::default(),
        EngineState::default(),
        EngineOptions::default(),
        started,
    )
    .unwrap();
    let row = record
        .rows
        .iter()
        .find(|r| r.zoom_name == "赵美玲")
        .expect("fixture attendee row");
    // The source replay only waits seven seconds, which rounds to zero minutes;
    // the companion still records the waiting-room entry separately.
    assert_eq!(row.waiting_room_minutes, 0);
    assert_eq!(row.total_presence_minutes, 0);
    assert!(row.first_join_at.is_none());
    assert!(row.first_waiting_room_at.is_some());
}
