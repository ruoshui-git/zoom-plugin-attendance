use attend_engine::{
    adapt_bridge_events, build_record, EngineConfig, EngineOptions, EngineState, MeetingMeta,
};
use serde_json::{json, Value};

fn derive_companion(events: &[Value], options: EngineOptions) -> attend_engine::Record {
    let normalized = adapt_bridge_events(events).unwrap();
    let first = normalized.first().unwrap()["timestamp"].as_str().unwrap();
    let now = normalized.last().unwrap()["timestamp"].as_str().unwrap();
    build_record(
        MeetingMeta {
            meeting_id: events[0]["instanceId"]
                .as_str()
                .unwrap_or("instance")
                .into(),
            meeting_topic: "Bridge scenario".into(),
            started_at: first.into(),
            ended_at: None,
        },
        &normalized,
        EngineConfig::default(),
        EngineState::default(),
        options,
        now,
    )
    .unwrap()
}

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

// AttendFlow fixture catalog: duplicate-display-names, leave-then-rejoin-same-name.
#[test]
fn companion_scenarios_preserve_duplicate_and_rejoin_identity() {
    let instance = "scenario-identity";
    let base = 1_794_893_640_000_i64;
    let duplicate = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":1,"name":"Sam Lee"},{"userId":2,"name":"Sam Lee"}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(duplicate.participants.len(), 2);
    assert!(duplicate.participants.iter().all(|p| p.has_ambiguous_name));

    let rejoin = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":7,"name":"Riley Chen"}]}),
            json!({"kind":"users","action":"left","instanceId":instance,"at":base+60_000,"users":[{"userId":7,"name":"Riley Chen"}]}),
            json!({"kind":"users","action":"add","instanceId":instance,"at":base+61_000,"users":[{"userId":8,"name":"Riley Chen"}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(rejoin.participants.len(), 1);
    assert_eq!(rejoin.participants[0].participant_uuids.len(), 2);
    assert_eq!(rejoin.rows.len(), 1);
}

// AttendFlow fixture catalog: rename-while-present, meeting-end-with-active-participants.
#[test]
fn companion_updates_rename_without_fake_movement_and_end_the_instance() {
    let instance = "scenario-update-end";
    let base = 1_794_893_640_000_i64;
    let renamed = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":1,"name":"Taylor"}]}),
            json!({"kind":"users","action":"update","instanceId":instance,"at":base+30_000,"users":[{"userId":1,"name":"Taylor New"}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(renamed.movement_events.len(), 1);
    assert_eq!(renamed.name_change_events.len(), 1);
    assert_eq!(renamed.participants[0].current_screen_name, "Taylor New");

    let ended = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":1,"name":"Taylor"}]}),
            json!({"kind":"instance_ended","instanceId":instance,"at":base+60_000}),
            json!({"kind":"users","action":"add","instanceId":instance,"at":base+120_000,"users":[{"userId":2,"name":"After End"}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(ended.participants.len(), 1);
    assert_eq!(ended.rows.len(), 1);
}

// AttendFlow fixture catalog: waiting-room-leave-without-admit, waiting-room-multiple-admit-burst.
#[test]
fn companion_waiting_room_admission_excludes_waiting_time_and_stitches_ids() {
    let instance = "scenario-waiting";
    let base = 1_794_893_640_000_i64;
    let unadmitted = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":1,"name":"Waiting Person","waitingRoom":true}]}),
            json!({"kind":"users","action":"left","instanceId":instance,"at":base+60_000,"users":[{"userId":1,"name":"Waiting Person","waitingRoom":true}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(unadmitted.participants.len(), 1);
    assert!(unadmitted.rows[0].first_join_at.is_none());
    assert_eq!(unadmitted.rows[0].total_presence_minutes, 0);

    let admitted = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":1,"name":"Riley Chen","waitingRoom":true}]}),
            json!({"kind":"users","action":"left","instanceId":instance,"at":base+60_000,"users":[{"userId":1,"name":"Riley Chen","waitingRoom":true}]}),
            json!({"kind":"users","action":"add","instanceId":instance,"at":base+61_000,"users":[{"userId":2,"name":"Riley Chen"}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(admitted.participants.len(), 1);
    assert_eq!(admitted.participants[0].participant_uuids.len(), 2);
    assert_eq!(
        admitted.rows[0].current_location,
        Some(attend_engine::ParticipantLocation::Meeting)
    );
}

// AttendFlow fixture catalog: burst-mixed-join-leave, many-participants, manual-snapshot, raise-hand.
#[test]
fn companion_burst_sync_snapshot_and_hand_events_keep_roster_state() {
    let instance = "scenario-burst";
    let base = 1_794_893_640_000_i64;
    let record = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":1,"name":"A"},{"userId":2,"name":"B"},{"userId":3,"name":"C"}]}),
            json!({"kind":"users","action":"left","instanceId":instance,"at":base+60_000,"users":[{"userId":2,"name":"B"}]}),
            json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base+90_000,"users":[{"userId":1,"name":"A"},{"userId":3,"name":"C"},{"userId":4,"name":"D"}]}),
            json!({"kind":"hand","instanceId":instance,"at":base+95_000,"userId":4,"raised":true}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(record.participants.len(), 4);
    assert_eq!(record.latest_roster.len(), 3);
    assert_eq!(record.raise_hand_entries.len(), 1);
    assert_eq!(record.raise_hand_entries[0].action, "raise");
}
