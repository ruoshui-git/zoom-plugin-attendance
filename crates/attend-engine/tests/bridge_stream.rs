use attend_engine::{
    adapt_bridge_events, build_record, EngineConfig, EngineOptions, EngineState, MeetingMeta,
};
use serde_json::{json, Value};

fn snapshot_record(events: &[Value]) -> attend_engine::Record {
    let normalized = adapt_bridge_events(events).unwrap();
    let first = normalized.first().unwrap()["timestamp"].as_str().unwrap();
    let last = normalized.last().unwrap()["timestamp"].as_str().unwrap();
    build_record(
        MeetingMeta {
            meeting_id: events[0]["instanceId"].as_str().unwrap().into(),
            meeting_topic: "resume".into(),
            started_at: first.into(),
            ended_at: None,
        },
        &normalized,
        EngineConfig::default(),
        EngineState::default(),
        EngineOptions::default(),
        last,
    )
    .unwrap()
}

#[test]
fn resumed_sync_all_deduplicates_joins_and_closes_absent_users() {
    use attend_engine::MovementType;
    let instance = "resume-repro";
    let base = 1_795_172_436_792_i64;
    let person = json!({"userId":16778240,"name":"毛若水","host":true,"waitingRoom":false});
    let mut repeated = vec![
        json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base,"users":[person]}),
    ];
    for at in [base + 72_505, base + 183_757, base + 598_558] {
        repeated.push(json!({"kind":"instance_resumed","instanceId":instance,"at":at}));
        repeated.push(json!({"kind":"users","action":"sync_all","instanceId":instance,"at":at,"users":[person]}));
    }
    repeated.push(json!({"kind":"instance_ended","instanceId":instance,"at":base+670_074}));
    let repeated_record = snapshot_record(&repeated);
    let one_record = snapshot_record(&[repeated[0].clone(), repeated.last().unwrap().clone()]);
    let joins = repeated_record
        .movement_events
        .iter()
        .filter(|m| m.kind == MovementType::JoinedBefore)
        .count();
    assert_eq!(joins, 1);
    assert_eq!(
        repeated_record.rows[0].total_presence_minutes,
        one_record.rows[0].total_presence_minutes
    );

    let departed = snapshot_record(&[
        json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base,"users":[person]}),
        json!({"kind":"instance_resumed","instanceId":instance,"at":base+300_000}),
        json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base+300_000,"users":[]}),
        json!({"kind":"instance_ended","instanceId":instance,"at":base+600_000}),
    ]);
    assert!(departed
        .movement_events
        .iter()
        .any(|m| m.kind == MovementType::Leave));
    assert_eq!(
        departed.rows[0].current_status,
        attend_engine::AttendanceStatus::LeftEarly
    );
    assert!(departed.rows[0].total_presence_minutes < 10);

    let new_user = snapshot_record(&[
        json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base,"users":[]}),
        json!({"kind":"instance_resumed","instanceId":instance,"at":base+60_000}),
        json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base+60_000,"users":[{"userId":8,"name":"New User"}]}),
        json!({"kind":"instance_ended","instanceId":instance,"at":base+120_000}),
    ]);
    assert_eq!(
        new_user
            .movement_events
            .iter()
            .filter(|m| m.kind == MovementType::JoinedBefore)
            .count(),
        1
    );
}

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

#[test]
fn companion_edge_scenarios_cover_presence_identity_roster_and_owner_policy() {
    let instance = "scenario-matrix";
    let base = 1_794_893_640_000_i64;
    let add = |user_id, name: &str, waiting_room: bool| json!({"userId":user_id,"name":name,"waitingRoom":waiting_room});
    let events = vec![
        json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[add(1,"Direct",false),add(2,"Stay",false),add(3,"Twin",false),add(4,"Twin",false)]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+180_000,"users":[add(1,"Direct",false),add(3,"Twin",false)]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+181_000,"users":[add(5,"Twin",false)]}),
        json!({"kind":"users","action":"update","instanceId":instance,"at":base+182_000,"users":[add(5,"Twin Renamed",false)]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+240_000,"users":[add(4,"Twin",false)]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+241_000,"users":[add(6,"Twin",false)]}),
        json!({"kind":"users","action":"update","instanceId":instance,"at":base+242_000,"users":[add(6,"Twin After Rejoin",false)]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+243_000,"users":[add(7,"Waiting Rename",true)]}),
        json!({"kind":"users","action":"update","instanceId":instance,"at":base+244_000,"users":[add(7,"Waiting Renamed",true)]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+300_000,"users":[add(7,"Waiting Renamed",true)]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+360_000,"users":[add(8,"Mover",false)]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+420_000,"users":[add(8,"Mover",false)]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+420_100,"users":[add(9,"Mover",true)]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+480_000,"users":[add(9,"Mover",true)]}),
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+480_050,"users":[add(10,"Mover",false)]}),
        json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base+540_000,"users":[add(2,"Stay",false),add(5,"Twin Renamed",false),add(6,"Twin After Rejoin",false),add(10,"Mover",false)]}),
        json!({"kind":"hand","instanceId":instance,"at":base+541_000,"userId":10,"raised":true}),
        json!({"kind":"hand","instanceId":instance,"at":base+542_000,"userId":10,"raised":false}),
    ];
    let record = derive_companion(&events, EngineOptions::default());
    assert_eq!(
        record.rows.len(),
        6,
        "expected unique identities after joins/rejoins"
    );
    let named = |name: &str| {
        record
            .rows
            .iter()
            .find(|row| row.zoom_name == name)
            .unwrap()
    };
    assert_eq!(
        named("Direct").current_status,
        attend_engine::AttendanceStatus::LeftEarly
    );
    assert_eq!(
        named("Stay").current_status,
        attend_engine::AttendanceStatus::Present
    );
    assert_eq!(
        named("Stay").first_join_at.as_deref(),
        Some("2026-11-17T05:34:00.000Z")
    );
    assert!(
        record
            .rows
            .iter()
            .filter(|row| row.zoom_name.contains("Twin"))
            .count()
            >= 2
    );
    assert!(
        record.name_change_events.len() >= 3,
        "renames while present and after rejoin are logged"
    );
    assert_eq!(record.raise_hand_entries.len(), 2);
    let waiting = named("Waiting Renamed");
    assert_eq!(waiting.first_join_at, None);
    assert!(waiting.first_waiting_room_at.is_some());
    assert!(waiting.waiting_room_minutes >= 1);
    let mover = named("Mover");
    assert_eq!(
        mover.current_location,
        Some(attend_engine::ParticipantLocation::Meeting)
    );
    assert!(mover.total_presence_minutes >= 2);

    let roster = EngineConfig {
        roster: vec![attend_engine::RosterPerson {
            person_id: 42,
            canonical_name: "Known Guest".into(),
            pinyin: String::new(),
            group: "class".into(),
            active: true,
            created_at: String::new(),
            updated_at: String::new(),
        }],
        aliases: vec![attend_engine::AliasMapping {
            id: None,
            zoom_name: "Guest Alias".into(),
            person_id: 42,
            notes: String::new(),
            last_used_at: None,
        }],
        profiles: vec![],
    };
    let matched = build_record(
        MeetingMeta { meeting_id: instance.into(), meeting_topic: String::new(), started_at: "2026-11-17T05:34:00.000Z".into(), ended_at: None },
        &adapt_bridge_events(&[json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[add(77,"Guest Alias",false),add(78,"Unmatched Guest",false)]})]).unwrap(),
        roster,
        EngineState { mode: attend_engine::SessionMode::Roster, ..EngineState::default() },
        EngineOptions::default(),
        "2026-11-17T05:39:00.000Z",
    ).unwrap();
    assert!(matched.rows.iter().any(|row| row.person_id == Some(42)));
    assert!(matched
        .rows
        .iter()
        .any(|row| row.zoom_name == "Unmatched Guest"));
}

#[test]
fn companion_late_leave_many_participants_and_non_host_streams() {
    let instance = "scenario-scale";
    let base = 1_794_893_640_000_i64;
    let late_events = [
        json!({"kind":"users","action":"add","instanceId":instance,"at":base+13*60_000,"users":[{"userId":1,"name":"Late Guest"}]}),
        json!({"kind":"users","action":"left","instanceId":instance,"at":base+16*60_000,"users":[{"userId":1,"name":"Late Guest"}]}),
    ];
    let late = build_record(
        MeetingMeta {
            meeting_id: instance.into(),
            meeting_topic: String::new(),
            started_at: "2026-11-17T05:34:00.000Z".into(),
            ended_at: None,
        },
        &adapt_bridge_events(&late_events).unwrap(),
        EngineConfig::default(),
        EngineState::default(),
        EngineOptions::default(),
        "2026-11-17T05:50:00.000Z",
    )
    .unwrap();
    assert_eq!(late.rows.len(), 1);
    assert_eq!(
        late.rows[0].first_join_at.as_deref(),
        Some("2026-11-17T05:47:00.000Z")
    );
    assert_eq!(late.rows[0].total_presence_minutes, 3);
    assert!(late.rows[0].was_late);
    assert!(late.rows[0]
        .tags
        .contains(&attend_engine::AttendanceTag::Late));

    let users: Vec<Value> = (0..80)
        .map(|id| json!({"userId":id+100,"name":format!("Participant {id}"),"waitingRoom":false}))
        .collect();
    let many = derive_companion(
        &[
            json!({"kind":"users","action":"sync_all","instanceId":instance,"at":base,"users":users}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(many.participants.len(), 80);
    assert_eq!(many.rows.len(), 80);

    // A non-host SDK view has no waiting-room lifecycle at all. The user's first
    // observed in-meeting add is their actual admission/presence start.
    let non_host = derive_companion(
        &[
            json!({"kind":"users","action":"add","instanceId":instance,"at":base,"users":[{"userId":9,"name":"Direct Admission"}]}),
        ],
        EngineOptions::default(),
    );
    assert_eq!(non_host.rows.len(), 1);
    assert_eq!(non_host.rows[0].waiting_room_minutes, 0);
    assert_eq!(
        non_host.rows[0].first_join_at.as_deref(),
        Some("2026-11-17T05:34:00.000Z")
    );
    assert_eq!(non_host.rows[0].total_presence_minutes, 0);
}
