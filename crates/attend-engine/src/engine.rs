use crate::*;
use chrono::{DateTime, Utc};
use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingMeta {
    #[serde(default)]
    pub meeting_id: String,
    #[serde(default)]
    pub meeting_topic: String,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    #[serde(default)]
    pub roster: Vec<RosterPerson>,
    #[serde(default)]
    pub aliases: Vec<AliasMapping>,
    #[serde(default)]
    pub profiles: Vec<MeetingProfile>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    #[serde(default)]
    pub mode: SessionMode,
    #[serde(default)]
    pub selected_profile_id: Option<String>,
    #[serde(default)]
    pub late_overrides: HashMap<String, bool>,
    #[serde(default)]
    pub return_overrides: HashMap<String, bool>,
    #[serde(default, rename = "reconciliationDecisions", alias = "decisions")]
    pub decisions: Vec<ReconciliationDecision>,
    #[serde(default)]
    pub manual_snapshots: Vec<Value>,
    #[serde(default)]
    pub raise_hand_captures: Vec<RaiseHandCapture>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct EngineOptions {
    pub waiting_room_counts_as_presence: bool,
    /// Companion sessions stop at the end of the tracked meeting instance.
    /// AttendFlow compatibility leaves this disabled for captured golden parity.
    #[serde(default)]
    pub stop_at_instance_end: bool,
}
impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            waiting_room_counts_as_presence: false,
            stop_at_instance_end: true,
        }
    }
}
impl EngineOptions {
    pub fn attendflow_compat() -> Self {
        Self {
            waiting_room_counts_as_presence: true,
            stop_at_instance_end: false,
        }
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid event: {0}")]
    InvalidEvent(String),
    #[error("multiple roster matches for Zoom name {0:?}")]
    MultipleRosterMatches(String),
}

/// Convert a Zoom Apps recording fixture to raw engine events. Fixture event timestamps are seconds.
pub fn adapt_zoom_apps_events(fixture: &Value) -> Result<Vec<Value>, EngineError> {
    let events = fixture
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| EngineError::InvalidEvent("missing events array".into()))?;
    let mut out = Vec::new();
    // AttendFlow bootstraps the record with the first SDK roster responses at
    // the replay reference time. These responses are recorded in `timeline`,
    // separately from callback events, and are essential when a participant
    // was already present before recording began.
    if let Some(timeline) = fixture.get("timeline").and_then(Value::as_array) {
        let first_response = |method: &str| {
            timeline
                .iter()
                .filter(|entry| entry["category"] == "sdk_call" && entry["type"] == method)
                .filter_map(|entry| entry["payload"]["response"]["participants"].as_array())
                .next()
        };
        let started = fixture
            .get("startedAt")
            .and_then(Value::as_str)
            .unwrap_or("1970-01-01T00:00:00.000Z");
        for (method, location) in [
            ("getMeetingParticipants", "meeting"),
            ("getWaitingRoomParticipants", "waiting-room"),
        ] {
            if let Some(participants) = first_response(method) {
                for participant in participants {
                    let p = participant_value(participant, &json!(started), false);
                    out.push(json!({
                        "eventType": if location == "meeting" { "joinedBefore" } else { "waitingRoomJoinedBefore" },
                        "timestamp": started,
                        "participantUUID": p["participantUUID"],
                        "screenName": p["screenName"],
                        "role": p["role"],
                        "location": location,
                    }));
                }
            }
        }
        // Later paired SDK responses are the snapshots applied by the live
        // repository after a debounced callback batch or during a roster poll.
        // Keep each complete meeting + waiting-room response as one replacement.
        let mut pending = Vec::<(i64, Value)>::new();
        for entry in timeline
            .iter()
            .filter(|entry| entry["category"] == "sdk_call")
        {
            let method = entry["type"].as_str().unwrap_or("");
            let location = match method {
                "getMeetingParticipants" => "meeting",
                "getWaitingRoomParticipants" => "waiting-room",
                _ => continue,
            };
            let participants = entry["payload"]["response"]["participants"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let relative = entry["relativeMs"].as_i64().unwrap_or(0);
            pending.extend(
                participants
                    .into_iter()
                    .map(|p| (relative, json!({"participant":p,"location":location}))),
            );
        }
        let first_relative = timeline
            .iter()
            .filter(|e| {
                e["category"] == "sdk_call"
                    && matches!(
                        e["type"].as_str(),
                        Some("getMeetingParticipants" | "getWaitingRoomParticipants")
                    )
            })
            .filter_map(|e| e["relativeMs"].as_i64())
            .min()
            .unwrap_or(i64::MIN);
        pending.sort_by_key(|(relative, _)| *relative);
        let mut groups: Vec<(i64, Vec<Value>)> = Vec::new();
        for (relative, participant) in pending {
            if let Some((at, values)) = groups.last_mut() {
                if relative - *at <= 5 {
                    values.push(participant);
                    continue;
                }
            }
            groups.push((relative, vec![participant]));
        }
        for (relative, values) in groups {
            let snapshot_at = started_ms(started).unwrap_or(0) + relative;
            // The first SDK snapshot seeds the record at reference time; it is
            // already represented by joinedBefore events above.
            if relative == first_relative {
                continue;
            }
            let participants: Vec<Value> = values
                .iter()
                .map(|v| {
                    let mut p = participant_value(
                        &v["participant"],
                        &json!(snapshot_at as f64 / 1000.0),
                        false,
                    );
                    p["location"] = json!(v["location"]);
                    p
                })
                .collect();
            out.push(json!({"eventType":"rosterSnapshot","timestamp":iso_from_millis(snapshot_at),"participants":participants}));
        }
    }
    for e in events {
        let kind = e.get("type").and_then(Value::as_str).unwrap_or("");
        let data = &e["data"];
        let timestamp = data
            .get("timestamp")
            .or_else(|| e.get("timestamp"))
            .cloned()
            .unwrap_or(Value::Null);
        match kind {
            "onParticipantChange" => {
                if let Some(list) = data.get("participants").and_then(Value::as_array) {
                    for p in list {
                        let mut v = participant_value(
                            p,
                            &timestamp,
                            p.get("status").and_then(Value::as_str).unwrap_or("join") == "leave",
                        );
                        v["eventType"] = json!(if p["status"] == "leave" {
                            "leave"
                        } else {
                            "join"
                        });
                        out.push(v);
                    }
                }
            }
            "onWaitingRoomParticipantJoin" | "onWaitingRoomParticipantLeave" => {
                let p = &data["participant"];
                let leave = kind.ends_with("Leave");
                let action = p.get("action").and_then(Value::as_str).unwrap_or("");
                if !leave && action == "putFromMainMeeting" {
                    let mut left = participant_value(p, &timestamp, true);
                    left["eventType"] = json!("leave");
                    left["location"] = json!("meeting");
                    out.push(left);
                }
                let mut v = participant_value(p, &timestamp, leave);
                v["location"] = json!("waiting-room");
                v["eventType"] = json!(if leave {
                    "waitingRoomLeave"
                } else if action == "joinMeeting" {
                    "waitingRoomJoinFromOutside"
                } else {
                    "waitingRoomJoinFromMeeting"
                });
                out.push(v);
            }
            "onMeeting" if data["action"] == "ended" => {
                out.push(json!({"eventType":"instance_ended","timestamp":timestamp}));
            }
            _ => {}
        }
    }
    out.sort_by_key(|event| timestamp_ms(&event["timestamp"]).unwrap_or(0));
    Ok(out)
}
fn started_ms(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis())
}
fn iso_from_millis(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn participant_value(p: &Value, timestamp: &Value, leave: bool) -> Value {
    // Zoom Apps recordings mix seconds and epoch milliseconds depending on the
    // recording generation. The 13 digit values are milliseconds.
    let value = timestamp.as_f64().unwrap_or(0.0);
    let sec = if value.abs() > 100_000_000_000.0 {
        value / 1000.0
    } else {
        value
    };
    json!({"eventType":if leave{"leave"}else{"join"},"timestamp":iso_from_seconds(sec),"participantUUID":p.get("participantUUID").and_then(Value::as_str).unwrap_or("unknown"),"screenName":p.get("screenName").and_then(Value::as_str).unwrap_or(""),"role":p.get("role").and_then(Value::as_str).unwrap_or("attendee"),"location":"meeting"})
}

/// Companion bridge adapter; bridge `at` values are epoch milliseconds, and UUIDs are instance scoped.
pub fn adapt_bridge_events(events: &[Value]) -> Result<Vec<Value>, EngineError> {
    let mut out = Vec::new();
    let mut names = HashMap::<String, String>::new();
    let mut live_roster = HashMap::<String, Value>::new();
    for e in events {
        let kind = e.get("kind").and_then(Value::as_str).unwrap_or("");
        let at = e.get("at").and_then(Value::as_f64).unwrap_or(0.0) / 1000.0;
        let ts = iso_from_seconds(at);
        let instance = e.get("instanceId").and_then(Value::as_str).unwrap_or("");
        if kind == "users" {
            let action = e.get("action").and_then(Value::as_str).unwrap_or("");
            if let Some(users) = e.get("users").and_then(Value::as_array) {
                let is_snapshot = matches!(action, "sync_all" | "syncAll");
                let previous_roster = if is_snapshot {
                    Some(live_roster.clone())
                } else {
                    None
                };
                if is_snapshot {
                    live_roster.clear();
                }
                let mut present = HashSet::<String>::new();
                for u in users {
                    let uid = u
                        .get("userId")
                        .map(|x| {
                            if let Some(s) = x.as_str() {
                                s.into()
                            } else {
                                x.to_string()
                            }
                        })
                        .unwrap_or_default();
                    let waiting = u["waitingRoom"].as_bool().unwrap_or(false);
                    names.insert(
                        format!("{instance}:{uid}"),
                        u["name"].as_str().unwrap_or("").into(),
                    );
                    let uuid = format!("{instance}:{uid}");
                    present.insert(uuid.clone());
                    let previous_location = previous_roster
                        .as_ref()
                        .and_then(|roster| roster.get(&uuid))
                        .and_then(|p| p["location"].as_str());
                    if action == "left" {
                        live_roster.remove(&uuid);
                    } else {
                        live_roster.insert(uuid.clone(), json!({"participantUUID":uuid,"screenName":u["name"].as_str().unwrap_or(""),"role":if u["host"].as_bool().unwrap_or(false){"host"}else{"attendee"},"location":if waiting{"waiting-room"}else{"meeting"}}));
                    }
                    let et = match action {
                        "left" if waiting => "waitingRoomLeave",
                        "left" => "leave",
                        "add" if waiting => "waitingRoomJoinFromOutside",
                        "add" => "join",
                        "update" => "update",
                        _ if is_snapshot
                            && previous_location
                                == Some(if waiting { "waiting-room" } else { "meeting" }) =>
                        {
                            "update"
                        }
                        _ => "snapshot",
                    };
                    out.push(json!({"eventType":et,"timestamp":ts,"participantUUID":format!("{instance}:{uid}"),"screenName":u["name"].as_str().unwrap_or(""),"role":if u["host"].as_bool().unwrap_or(false){"host"}else{"attendee"},"location":if waiting{"waiting-room"}else{"meeting"}}));
                }
                if let Some(previous) = previous_roster {
                    for (uuid, participant) in previous {
                        if !present.contains(&uuid) {
                            let waiting = participant["location"] == "waiting-room";
                            out.push(json!({"eventType":if waiting{"waitingRoomLeave"}else{"leave"},"timestamp":ts,"participantUUID":uuid,"screenName":participant["screenName"],"role":participant["role"],"location":participant["location"]}));
                        }
                    }
                }
                let snapshot: Vec<Value> = live_roster.values().cloned().collect();
                out.push(
                    json!({"eventType":"rosterSnapshot","timestamp":ts,"participants":snapshot}),
                );
            }
        } else if kind == "hand" {
            if let Some(uid) = e.get("userId") {
                let uid = uid
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| uid.to_string());
                let uuid = format!("{instance}:{uid}");
                out.push(json!({"eventType":"hand","timestamp":ts,"participantUUID":uuid,"screenName":names.get(&uuid).cloned().unwrap_or_default(),"raised":e["raised"].as_bool().unwrap_or(false)}));
            }
        } else if kind == "instance_ended" {
            out.push(json!({"eventType":"instance_ended","timestamp":ts}));
        }
    }
    Ok(out)
}

pub fn build_record(
    meta: MeetingMeta,
    raw_events: &[Value],
    config: EngineConfig,
    state: EngineState,
    options: EngineOptions,
    now: &str,
) -> Result<Record, EngineError> {
    let profiles = config.profiles.clone();
    let chosen = state
        .selected_profile_id
        .as_deref()
        .and_then(|id| profiles.iter().find(|p| p.id == id))
        .or_else(|| select_profile(&meta.meeting_id, &meta.meeting_topic, &profiles));
    let late = chosen.map(|p| p.late_after_minutes).unwrap_or(10);
    let return_window = chosen.map(|p| p.return_window_minutes).unwrap_or(8);
    let groups = chosen
        .map(|p| p.included_subgroups.clone())
        .unwrap_or_default();
    let mut events = Vec::<MovementEvent>::new();
    let mut participants = Vec::<SessionParticipant>::new();
    let mut name_changes = Vec::<NameChange>::new();
    let mut uuid_map = HashMap::<String, String>::new();
    let mut observed_roster: Option<HashMap<String, ZoomParticipant>> = None;
    let mut snapshot_active = HashSet::<String>::new();
    let mut end = meta.ended_at.clone();
    let mut hands = Vec::new();
    let mut ordered = raw_events.to_vec();
    ordered.sort_by_key(|e| timestamp_ms(&e["timestamp"]).unwrap_or(0));
    let mut ended = false;
    for (idx, e) in ordered.iter().enumerate() {
        let ty = e["eventType"]
            .as_str()
            .or_else(|| e["kind"].as_str())
            .unwrap_or("");
        let ts = event_timestamp(e);
        if ty == "instance_ended" {
            end = Some(ts);
            ended = options.stop_at_instance_end;
            continue;
        }
        if ended {
            continue;
        }
        if ty == "rosterSnapshot" {
            let mut snapshot = HashMap::new();
            let mut next_active = HashSet::new();
            if let Some(items) = e["participants"].as_array() {
                for item in items {
                    let uuid = item["participantUUID"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    if uuid.is_empty() {
                        continue;
                    }
                    let name = item["screenName"].as_str().unwrap_or_default().to_string();
                    let loc = if item["location"] == "waiting-room" {
                        ParticipantLocation::WaitingRoom
                    } else {
                        ParticipantLocation::Meeting
                    };
                    let role = item["role"].as_str().unwrap_or("attendee").to_string();
                    let ts = ts.clone();
                    let Some(id) = uuid_map.get(&uuid).cloned() else {
                        continue;
                    };
                    if let Some(p) = participants
                        .iter_mut()
                        .find(|p| p.session_participant_id == id)
                    {
                        if p.current_screen_name != name {
                            name_changes.push(NameChange {
                                id: format!("rename-{}", name_changes.len() + 1),
                                timestamp: ts.clone(),
                                session_participant_id: id.clone(),
                                participant_uuid: uuid.clone(),
                                previous_screen_name: p.current_screen_name.clone(),
                                next_screen_name: name.clone(),
                            });
                            p.current_screen_name = name.clone();
                            if !p.screen_names.contains(&name) {
                                p.screen_names.push(name.clone());
                            }
                        }
                        p.current_participant_uuid = uuid.clone();
                        p.current_location = loc.clone();
                        p.role = role.clone();
                        p.last_seen_at = ts;
                    }
                    snapshot.insert(
                        uuid.clone(),
                        ZoomParticipant {
                            participant_uuid: uuid,
                            screen_name: name.clone(),
                            screen_name_pinyin: build_pinyin(&name),
                            role,
                            location: loc,
                        },
                    );
                    next_active.insert(id);
                }
            }
            snapshot_active = next_active;
            observed_roster = Some(snapshot);
            continue;
        }
        if ty == "hand" {
            if let Some(id) = e["participantUUID"].as_str() {
                hands.push(RaiseHandEntry {
                    id: format!("hand-{idx}"),
                    participant_uuid: id.into(),
                    screen_name: e["screenName"].as_str().unwrap_or("").into(),
                    timestamp: ts.clone(),
                    action: if e["raised"].as_bool().unwrap_or(false) {
                        "raise"
                    } else {
                        "lower"
                    }
                    .into(),
                    sequence: idx as u64,
                    received_at: timestamp_ms(&e["timestamp"]).unwrap_or(0),
                });
            }
            continue;
        }
        if !matches!(
            ty,
            "join"
                | "leave"
                | "waitingRoomJoinFromOutside"
                | "waitingRoomLeave"
                | "waitingRoomJoinFromMeeting"
                | "joinedBefore"
                | "waitingRoomJoinedBefore"
                | "update"
                | "snapshot"
        ) {
            continue;
        }
        let uuid = e["participantUUID"].as_str().unwrap_or("").to_string();
        if uuid.is_empty() {
            continue;
        }
        let name = e["screenName"].as_str().unwrap_or("").to_string();
        let loc = if e["location"] == "waiting-room" {
            ParticipantLocation::WaitingRoom
        } else {
            ParticipantLocation::Meeting
        };
        let role = e["role"].as_str().unwrap_or("attendee").to_string();
        if ty == "update" {
            if let Some(id) = uuid_map.get(&uuid).cloned() {
                if let Some(p) = participants
                    .iter_mut()
                    .find(|p| p.session_participant_id == id)
                {
                    if p.current_screen_name != name {
                        name_changes.push(NameChange {
                            id: format!("rename-{}", name_changes.len() + 1),
                            timestamp: ts.clone(),
                            session_participant_id: id,
                            participant_uuid: uuid,
                            previous_screen_name: p.current_screen_name.clone(),
                            next_screen_name: name.clone(),
                        });
                        p.current_screen_name = name.clone();
                        if !p.screen_names.contains(&name) {
                            p.screen_names.push(name.clone());
                        }
                    }
                    p.current_location = loc;
                    p.role = role;
                    p.last_seen_at = ts;
                }
            }
            continue;
        }
        let participant_id = if let Some(id) = uuid_map.get(&uuid) {
            id.clone()
        } else {
            let mut activity = HashMap::<String, bool>::new();
            for event in &events {
                if let Some(id) = &event.session_participant_id {
                    match event.kind {
                        MovementType::Join
                        | MovementType::JoinedBefore
                        | MovementType::WaitingRoomJoinFromOutside
                        | MovementType::WaitingRoomJoinedBefore
                        | MovementType::WaitingRoomJoinFromMeeting => {
                            activity.insert(id.clone(), true);
                        }
                        MovementType::Leave | MovementType::WaitingRoomLeave => {
                            activity.insert(id.clone(), false);
                        }
                    }
                }
            }
            let active: HashSet<String> = activity
                .into_iter()
                .filter_map(|(id, yes)| yes.then_some(id))
                .chain(snapshot_active.iter().cloned())
                .collect();
            let by_name: Vec<_> = participants
                .iter()
                .filter(|p| {
                    normalize_name(&p.current_screen_name) == normalize_name(&name)
                        && !active.contains(&p.session_participant_id)
                })
                .collect();
            let by_roster: Vec<_> = candidate_people(&name, &config.roster, &config.aliases);
            let target = if by_roster.len() == 1 {
                let id = by_roster[0].person_id;
                participants
                    .iter()
                    .filter(|p| {
                        !active.contains(&p.session_participant_id)
                            && candidate_people(
                                &p.current_screen_name,
                                &config.roster,
                                &config.aliases,
                            )
                            .iter()
                            .any(|rp| rp.person_id == id)
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            let existing = if by_name.len() == 1 {
                Some(by_name[0])
            } else if target.len() == 1 {
                Some(target[0])
            } else {
                None
            };
            if let Some(old) = existing {
                let id = old.session_participant_id.clone();
                uuid_map.insert(uuid.clone(), id.clone());
                id
            } else {
                let id = format!("sp-{}", participants.len() + 1);
                let resolution = if by_name.len() > 1 || target.len() > 1 {
                    "ambiguous"
                } else {
                    "new_identity"
                };
                participants.push(SessionParticipant {
                    session_participant_id: id.clone(),
                    current_participant_uuid: uuid.clone(),
                    participant_uuids: vec![uuid.clone()],
                    current_screen_name: name.clone(),
                    screen_names: vec![name.clone()],
                    role: role.clone(),
                    current_location: loc.clone(),
                    first_seen_at: ts.clone(),
                    last_seen_at: ts.clone(),
                    identity_resolution: resolution.into(),
                    has_ambiguous_name: resolution == "ambiguous",
                });
                uuid_map.insert(uuid.clone(), id.clone());
                id
            }
        };
        if let Some(p) = participants
            .iter_mut()
            .find(|p| p.session_participant_id == participant_id)
        {
            if p.current_screen_name != name && ty == "update" {
                name_changes.push(NameChange {
                    id: format!("rename-{}", name_changes.len() + 1),
                    timestamp: ts.clone(),
                    session_participant_id: participant_id.clone(),
                    participant_uuid: uuid.clone(),
                    previous_screen_name: p.current_screen_name.clone(),
                    next_screen_name: name.clone(),
                });
            }
            if !p.participant_uuids.contains(&uuid) {
                p.participant_uuids.push(uuid.clone())
            }
            if !p.screen_names.contains(&name) {
                p.screen_names.push(name.clone())
            }
            p.current_participant_uuid = uuid.clone();
            p.current_screen_name = name.clone();
            p.current_location = loc.clone();
            p.role = role.clone();
            p.last_seen_at = ts.clone();
        }
        let mt = match ty {
            "leave" => MovementType::Leave,
            "waitingRoomJoinFromOutside" => MovementType::WaitingRoomJoinFromOutside,
            "waitingRoomLeave" => MovementType::WaitingRoomLeave,
            "waitingRoomJoinFromMeeting" => MovementType::WaitingRoomJoinFromMeeting,
            "joinedBefore" => MovementType::JoinedBefore,
            "waitingRoomJoinedBefore" => MovementType::WaitingRoomJoinedBefore,
            "snapshot" if loc == ParticipantLocation::WaitingRoom => {
                MovementType::WaitingRoomJoinedBefore
            }
            "snapshot" => MovementType::JoinedBefore,
            _ => MovementType::Join,
        };
        events.push(MovementEvent {
            id: format!("event-{}", idx + 1),
            timestamp: ts,
            kind: mt,
            session_participant_id: Some(participant_id),
            participant: ZoomParticipant {
                participant_uuid: uuid,
                screen_name: name.clone(),
                screen_name_pinyin: build_pinyin(&name),
                role,
                location: loc,
            },
        });
    }
    // Flag identities when their active intervals overlap with the same
    // normalized name. A historical name reuse after a participant leaves is
    // a rejoin candidate, not a shared-name collision.
    let mut active_names = HashMap::<String, String>::new();
    let mut ambiguous_ids = HashSet::<String>::new();
    for event in &events {
        let Some(id) = &event.session_participant_id else {
            continue;
        };
        let name = normalize_name(&event.participant.screen_name);
        match event.kind {
            MovementType::Join
            | MovementType::JoinedBefore
            | MovementType::WaitingRoomJoinFromOutside
            | MovementType::WaitingRoomJoinFromMeeting
            | MovementType::WaitingRoomJoinedBefore => {
                for (active_id, active_name) in &active_names {
                    if active_id != id && active_name == &name {
                        ambiguous_ids.insert(id.clone());
                        ambiguous_ids.insert(active_id.clone());
                    }
                }
                active_names.insert(id.clone(), name);
            }
            MovementType::Leave | MovementType::WaitingRoomLeave => {
                active_names.remove(id);
            }
        }
    }
    for participant in &mut participants {
        if ambiguous_ids.contains(&participant.session_participant_id) {
            participant.has_ambiguous_name = true;
        }
    }
    let roster = observed_roster
        .as_ref()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_else(|| latest_roster(&events));
    let record_session = SessionRecord {
        session_id: meta.meeting_id.clone(),
        origin_session_key: meta.meeting_id.clone(),
        record_source: "zoom-companion".into(),
        started_at: meta.started_at.clone(),
        ended_at: end.clone(),
        is_active: end.is_none(),
        mode: state.mode.clone(),
        meeting_uuid: Some(meta.meeting_id.clone()),
        meeting_id: Some(meta.meeting_id.clone()),
        meeting_topic: Some(meta.meeting_topic.clone()),
        guessed_profile_id: chosen.map(|p| p.id.clone()),
        selected_profile_id: state.selected_profile_id.clone(),
        selected_included_subgroups: groups,
        late_threshold_minutes: late,
        return_window_minutes: return_window,
        late_overrides: state.late_overrides.clone(),
        return_overrides: state.return_overrides.clone(),
        session_participants: participants.clone(),
        movement_events: events.clone(),
        name_change_events: name_changes
            .iter()
            .map(|change| {
                let movement = events.iter().rev().find(|event| {
                    event.participant.participant_uuid == change.participant_uuid
                        && event.timestamp <= change.timestamp
                });
                SessionNameChangeEvent {
                    id: change.id.clone(),
                    timestamp: change.timestamp.clone(),
                    session_participant_id: change.session_participant_id.clone(),
                    participant_uuid: change.participant_uuid.clone(),
                    previous_screen_name: change.previous_screen_name.clone(),
                    next_screen_name: change.next_screen_name.clone(),
                    role: movement
                        .map(|event| event.participant.role.clone())
                        .unwrap_or_default(),
                    location: movement
                        .map(|event| event.participant.location.clone())
                        .unwrap_or_default(),
                    source: "bridge_update".into(),
                }
            })
            .collect(),
        latest_roster: roster.clone(),
        manual_snapshots: state.manual_snapshots.clone(),
        reconciliation_decisions: state.decisions.clone(),
        raise_hand_entries: hands.clone(),
        raise_hand_captures: state.raise_hand_captures.clone(),
        ..SessionRecord::default()
    };
    let derived = derive(&record_session, &config, &state.decisions, now, &options)?;
    Ok(Record {
        meeting_id: meta.meeting_id,
        meeting_topic: meta.meeting_topic,
        started_at: meta.started_at,
        ended_at: end,
        mode: state.mode,
        selected_profile_id: state.selected_profile_id,
        guessed_profile_id: chosen.map(|p| p.id.clone()),
        participants,
        movement_events: events,
        name_change_events: name_changes,
        latest_roster: roster,
        rows: derived.rows,
        summary: derived.summary,
        raise_hand_entries: hands,
        raise_hand_captures: state.raise_hand_captures,
    })
}

/// Derive attendance from a fully ingested AttendFlow-shaped session record.
/// The reference time is explicit so historical and live calculations are
/// deterministic and can be compared against fixture goldens.
pub fn derive(
    session: &SessionRecord,
    config: &EngineConfig,
    decisions: &[ReconciliationDecision],
    reference_time: &str,
    options: &EngineOptions,
) -> Result<Derived, EngineError> {
    // Derivation consumes persisted identity/event records as-is. Replaying them
    // through build_record would re-run ingestion and manufacture new IDs.
    #[derive(Default)]
    struct State {
        first_join: Option<String>,
        last_seen: Option<String>,
        last_leave: Option<String>,
        open_since: Option<i64>,
        total_ms: i64,
        absence_count: u32,
        returned: bool,
        present: bool,
        location: Option<ParticipantLocation>,
        first_waiting: Option<String>,
        waiting_since: Option<i64>,
        waiting_total_ms: i64,
    }

    let late_minutes = session.late_threshold_minutes;
    let return_minutes = session.return_window_minutes;
    let start_ms = timestamp_ms(&json!(session.started_at)).unwrap_or(0);
    let reference_ms = timestamp_ms(&json!(reference_time)).unwrap_or(start_ms);
    let mut participants = session.session_participants.clone();
    let mut states: HashMap<String, State> = participants
        .iter()
        .map(|p| {
            (
                p.session_participant_id.clone(),
                State {
                    first_join: options
                        .waiting_room_counts_as_presence
                        .then(|| p.first_seen_at.clone()),
                    last_seen: Some(p.last_seen_at.clone()),
                    location: Some(p.current_location.clone()),
                    ..State::default()
                },
            )
        })
        .collect();
    let end_cutoff = session
        .ended_at
        .as_deref()
        .and_then(|ended| timestamp_ms(&json!(ended)));
    let mut events: Vec<_> = session
        .movement_events
        .iter()
        .filter(|event| {
            !options.stop_at_instance_end
                || end_cutoff.is_none_or(|cutoff| {
                    timestamp_ms(&json!(event.timestamp)).unwrap_or(i64::MIN) <= cutoff
                })
        })
        .collect();
    let priority = |kind: &MovementType| match kind {
        MovementType::JoinedBefore
        | MovementType::WaitingRoomJoinedBefore
        | MovementType::WaitingRoomJoinFromOutside => 10,
        MovementType::WaitingRoomLeave => 20,
        MovementType::Join => 30,
        MovementType::Leave => 40,
        MovementType::WaitingRoomJoinFromMeeting => 50,
    };
    events.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| priority(&a.kind).cmp(&priority(&b.kind)))
            .then_with(|| a.id.cmp(&b.id))
    });
    for event in &events {
        let Some(id) = event.session_participant_id.as_deref() else {
            continue;
        };
        let Some(state) = states.get_mut(id) else {
            continue;
        };
        let t = timestamp_ms(&json!(event.timestamp)).unwrap_or(0);
        let waiting = event.participant.location == ParticipantLocation::WaitingRoom;
        let presence_join = matches!(
            event.kind,
            MovementType::JoinedBefore
                | MovementType::Join
                | MovementType::WaitingRoomJoinFromOutside
                | MovementType::WaitingRoomJoinFromMeeting
                | MovementType::WaitingRoomJoinedBefore
        );
        let counts_as_presence = !waiting || options.waiting_room_counts_as_presence;
        if let Some(p) = participants
            .iter_mut()
            .find(|p| p.session_participant_id == id)
        {
            p.current_participant_uuid = event.participant.participant_uuid.clone();
            if !p
                .participant_uuids
                .contains(&event.participant.participant_uuid)
            {
                p.participant_uuids
                    .push(event.participant.participant_uuid.clone());
            }
            p.current_screen_name = event.participant.screen_name.clone();
            if !p.screen_names.contains(&event.participant.screen_name) {
                p.screen_names.push(event.participant.screen_name.clone());
            }
            p.role = event.participant.role.clone();
            p.current_location = event.participant.location.clone();
        }
        if presence_join {
            if waiting && state.first_waiting.is_none() {
                state.first_waiting = Some(event.timestamp.clone());
            }
            if !counts_as_presence {
                if state.waiting_since.is_none() {
                    state.waiting_since = Some(t);
                }
                state.last_seen = Some(event.timestamp.clone());
                continue;
            }
            if state.first_join.is_none() {
                state.first_join = Some(event.timestamp.clone());
            }
            if !state.present {
                if let Some(waiting_since) = state.waiting_since.take() {
                    state.waiting_total_ms += (t - waiting_since).max(0);
                }
                if let Some(last_leave) = &state.last_leave {
                    let gap = t - timestamp_ms(&json!(last_leave)).unwrap_or(t);
                    if gap > return_minutes as i64 * 60_000 {
                        state.returned = true;
                    }
                }
                state.open_since = Some(t);
            }
            state.present = true;
            state.last_seen = Some(event.timestamp.clone());
            state.location = Some(event.participant.location.clone());
        } else if matches!(
            event.kind,
            MovementType::Leave | MovementType::WaitingRoomLeave
        ) {
            state.last_seen = Some(event.timestamp.clone());
            let paired_admission = event.kind == MovementType::WaitingRoomLeave
                && events.iter().any(|candidate| {
                    candidate.kind == MovementType::Join
                        && candidate.session_participant_id.as_deref() == Some(id)
                        && candidate.participant.location == ParticipantLocation::Meeting
                        && (timestamp_ms(&json!(candidate.timestamp)).unwrap_or(0) - t).abs()
                            <= 5_000
                });
            if event.kind == MovementType::WaitingRoomLeave {
                if let Some(waiting_since) = state.waiting_since.take() {
                    state.waiting_total_ms += (t - waiting_since).max(0);
                }
            }
            if let Some(open) = state.open_since.take() {
                state.total_ms += (t - open).max(0);
                if !paired_admission {
                    state.absence_count += 1;
                }
            }
            state.present = false;
            state.location = None;
            state.last_leave = Some(event.timestamp.clone());
        }
    }
    for participant in &mut participants {
        let Some(state) = states.get_mut(&participant.session_participant_id) else {
            continue;
        };
        if let Some(returned) = session
            .return_overrides
            .get(&participant.session_participant_id)
        {
            state.returned = *returned;
        }
        if let Some(latest) = session
            .latest_roster
            .iter()
            .find(|p| participant.participant_uuids.contains(&p.participant_uuid))
        {
            participant.current_participant_uuid = latest.participant_uuid.clone();
            if !participant
                .participant_uuids
                .contains(&latest.participant_uuid)
            {
                participant
                    .participant_uuids
                    .push(latest.participant_uuid.clone());
            }
            participant.current_screen_name = latest.screen_name.clone();
            if !participant.screen_names.contains(&latest.screen_name) {
                participant.screen_names.push(latest.screen_name.clone());
            }
            participant.role = latest.role.clone();
            participant.current_location = latest.location.clone();
            if latest.location == ParticipantLocation::WaitingRoom
                && !options.waiting_room_counts_as_presence
            {
                state.present = false;
                state.location = None;
                if state.waiting_since.is_none() {
                    state.waiting_since = Some(reference_ms);
                }
            } else {
                state.present = true;
                state.location = Some(latest.location.clone());
                if state.open_since.is_none() {
                    state.open_since = Some(reference_ms);
                }
            }
        } else if state.present {
            state.present = false;
            state.location = None;
        }
        if state.present {
            if let Some(open) = state.open_since {
                state.total_ms += (reference_ms - open).max(0);
            }
        }
        if let Some(waiting_since) = state.waiting_since {
            if !options.waiting_room_counts_as_presence {
                let cutoff = session
                    .ended_at
                    .as_deref()
                    .and_then(|ended| timestamp_ms(&json!(ended)))
                    .unwrap_or(reference_ms)
                    .min(reference_ms);
                state.waiting_total_ms += (cutoff - waiting_since).max(0);
            }
        }
    }
    let mut rows = Vec::new();
    for participant in &participants {
        let state = &states[&participant.session_participant_id];
        let first = state.first_join.clone().or_else(|| {
            options
                .waiting_room_counts_as_presence
                .then(|| participant.first_seen_at.clone())
        });
        let seen = state.first_join.is_some()
            || (options.waiting_room_counts_as_presence && !participant.last_seen_at.is_empty());
        let first_ms = state
            .first_join
            .as_deref()
            .and_then(|s| timestamp_ms(&json!(s)))
            .unwrap_or_else(|| {
                if options.waiting_room_counts_as_presence {
                    timestamp_ms(&json!(participant.first_seen_at)).unwrap_or(start_ms)
                } else {
                    start_ms
                }
            });
        let was_late = session
            .late_overrides
            .get(&participant.session_participant_id)
            .copied()
            .unwrap_or(first_ms > start_ms + late_minutes as i64 * 60_000);
        let status = if state.present {
            if state.returned {
                AttendanceStatus::ReturnedAfterLongAbsence
            } else if was_late {
                AttendanceStatus::Late
            } else {
                AttendanceStatus::Present
            }
        } else if seen {
            AttendanceStatus::LeftEarly
        } else {
            AttendanceStatus::Unmatched
        };
        let mut tags = Vec::new();
        if seen && state.present && !was_late && !state.returned {
            tags.push(AttendanceTag::FullPresent);
        }
        if seen {
            tags.push(AttendanceTag::Seen);
        }
        if seen && was_late {
            tags.push(AttendanceTag::Late);
        }
        if seen && !state.present {
            tags.push(AttendanceTag::LeftEarly);
        }
        if seen && state.present && state.returned {
            tags.push(AttendanceTag::Returned);
        }
        let tag_priority = |tag: &AttendanceTag| match tag {
            AttendanceTag::LeftEarly => 0,
            AttendanceTag::Late => 1,
            AttendanceTag::Returned => 2,
            AttendanceTag::FullPresent => 3,
            AttendanceTag::Seen => 4,
            AttendanceTag::Missing => 5,
            AttendanceTag::Ignored => 6,
            AttendanceTag::Guest => 7,
            AttendanceTag::Unmatched => 8,
        };
        tags.sort_by_key(tag_priority);
        rows.push(AttendanceRow {
            entity_id: format!("participant:{}", participant.session_participant_id),
            session_participant_id: Some(participant.session_participant_id.clone()),
            participant_uuid: Some(participant.current_participant_uuid.clone()),
            participant_uuids: participant.participant_uuids.clone(),
            zoom_name: participant.current_screen_name.clone(),
            zoom_names: participant.screen_names.clone(),
            role: Some(participant.role.clone()),
            person_id: None,
            canonical_name: None,
            pinyin: None,
            group: None,
            current_location: if state.present {
                state.location.clone()
            } else {
                None
            },
            match_status: MatchStatus::NotApplicable,
            is_expected: false,
            has_been_seen: seen,
            is_currently_present: state.present,
            first_join_at: first,
            first_waiting_room_at: if options.waiting_room_counts_as_presence {
                None
            } else {
                state.first_waiting.clone()
            },
            last_seen_at: state
                .last_seen
                .clone()
                .or_else(|| Some(participant.last_seen_at.clone())),
            current_status: status,
            was_late,
            total_presence_minutes: ((state.total_ms.max(0) as f64 / 60_000.0).round() as u64),
            waiting_room_minutes: if options.waiting_room_counts_as_presence {
                0
            } else {
                (state.waiting_total_ms.max(0) as f64 / 60_000.0).round() as u64
            },
            absence_count: state.absence_count,
            returned_after_long_absence: state.returned,
            tags,
            shared_name: participant.has_ambiguous_name
                || is_likely_shared_name(&participant.current_screen_name),
            split_person_ids: vec![],
            split_canonical_names: vec![],
            notes: None,
        });
    }
    if session.mode == SessionMode::Roster {
        rows = derive_roster_rows(rows, &participants, config, decisions, session)?;
    }
    // AttendFlow uses zh-CN locale collation for the displayed label.
    rows.sort_by(|a, b| {
        let left = a.canonical_name.as_deref().unwrap_or(&a.zoom_name);
        let right = b.canonical_name.as_deref().unwrap_or(&b.zoom_name);
        let left_cjk = left.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c));
        let right_cjk = right
            .chars()
            .any(|c| ('\u{3400}'..='\u{9fff}').contains(&c));
        (if left_cjk { 0 } else { 1 }, pinyin_sort_key(left), left).cmp(&(
            if right_cjk { 0 } else { 1 },
            pinyin_sort_key(right),
            right,
        ))
    });
    let summary = compute_summary(&rows, &session.mode, session.ended_at.is_some());
    Ok(Derived { rows, summary })
}

fn pinyin_sort_key(value: &str) -> Vec<String> {
    value
        .chars()
        .map(|c| {
            c.to_pinyin()
                .map(|p| p.plain().to_lowercase())
                .unwrap_or_else(|| c.to_lowercase().collect())
        })
        .collect()
}

fn normalize_group(group: &str) -> String {
    let group = group.trim();
    if group.is_empty() {
        "__UNGROUPED__".into()
    } else {
        group.into()
    }
}

fn derive_roster_rows(
    generic_rows: Vec<AttendanceRow>,
    participants: &[SessionParticipant],
    config: &EngineConfig,
    decisions: &[ReconciliationDecision],
    session: &SessionRecord,
) -> Result<Vec<AttendanceRow>, EngineError> {
    let profile = config
        .profiles
        .iter()
        .find(|p| Some(&p.id) == session.selected_profile_id.as_ref())
        .or_else(|| config.profiles.iter().find(|p| p.is_default))
        .or_else(|| config.profiles.first());
    let groups = profile
        .map(|p| p.included_subgroups.as_slice())
        .unwrap_or(&[]);
    let mut output = Vec::new();
    let mut seen_people = HashSet::new();
    for mut row in generic_rows {
        let Some(participant) = participants.iter().find(|p| {
            Some(p.session_participant_id.as_str()) == row.session_participant_id.as_deref()
        }) else {
            continue;
        };
        let decision = decisions
            .iter()
            .rev()
            .find(|d| d.session_participant_id == participant.session_participant_id);
        let mut status = MatchStatus::Unmatched;
        let mut people: Vec<&RosterPerson> = Vec::new();
        if let Some(d) = decision {
            match d.action {
                DecisionAction::Guest => status = MatchStatus::Guest,
                DecisionAction::Match | DecisionAction::NameChangeKeepMatch => {
                    if let Some(person) = config
                        .roster
                        .iter()
                        .find(|p| Some(p.person_id) == d.person_id)
                    {
                        people.push(person);
                        status = MatchStatus::Matched;
                    }
                }
                DecisionAction::Split => {
                    for id in &d.split_person_ids {
                        if let Some(person) = config.roster.iter().find(|p| p.person_id == *id) {
                            if !people.iter().any(|p| p.person_id == person.person_id) {
                                people.push(person);
                            }
                        }
                    }
                    if !people.is_empty() {
                        status = MatchStatus::Split;
                    }
                }
                DecisionAction::NameChangeRevertMatch => {}
            }
        }
        if people.is_empty() && status != MatchStatus::Guest {
            let candidates = candidate_people(
                &participant.current_screen_name,
                &config.roster,
                &config.aliases,
            );
            if candidates.len() > 1 {
                return Err(EngineError::MultipleRosterMatches(
                    participant.current_screen_name.clone(),
                ));
            }
            if candidates.len() == 1 {
                people.push(candidates[0]);
                status = MatchStatus::Matched;
            }
        }
        let is_present = row.is_currently_present;
        let base_status =
            if is_present && row.current_status == AttendanceStatus::ReturnedAfterLongAbsence {
                AttendanceStatus::ReturnedAfterLongAbsence
            } else if is_present {
                AttendanceStatus::Present
            } else {
                AttendanceStatus::LeftEarly
            };
        let matched = status == MatchStatus::Matched;
        let split = status == MatchStatus::Split;
        let counted = matched || split;
        if let Some(person) = people.first().copied().filter(|_| matched) {
            seen_people.insert(person.person_id);
            row.entity_id = participant.session_participant_id.clone();
            row.person_id = Some(person.person_id);
            row.canonical_name = Some(person.canonical_name.clone());
            row.pinyin = Some(person.pinyin.clone());
            row.group = Some(normalize_group(&person.group));
            row.match_status = MatchStatus::Matched;
            row.is_expected = profile_includes_group(groups, &normalize_group(&person.group));
        } else if split {
            let ids: Vec<u64> = people.iter().map(|p| p.person_id).collect();
            let names: Vec<String> = people.iter().map(|p| p.canonical_name.clone()).collect();
            for person in &people {
                seen_people.insert(person.person_id);
            }
            let mut fanout = Vec::new();
            for person in people {
                let mut split_row = row.clone();
                split_row.entity_id = format!(
                    "{}:{}",
                    participant.session_participant_id, person.person_id
                );
                split_row.person_id = Some(person.person_id);
                split_row.canonical_name = Some(person.canonical_name.clone());
                split_row.pinyin = Some(person.pinyin.clone());
                split_row.group = Some(normalize_group(&person.group));
                split_row.match_status = MatchStatus::Split;
                split_row.is_expected =
                    profile_includes_group(groups, &normalize_group(&person.group));
                split_row.split_person_ids = ids.clone();
                split_row.split_canonical_names = names.clone();
                split_row.current_status = roster_current_status(&split_row, base_status.clone());
                split_row.tags = roster_tags(&split_row, true);
                fanout.push(split_row);
            }
            output.extend(fanout);
            continue;
        } else {
            row.entity_id = if status == MatchStatus::Guest {
                format!("guest:{}", participant.session_participant_id)
            } else {
                format!("participant:{}", participant.session_participant_id)
            };
            row.person_id = None;
            row.canonical_name = None;
            row.pinyin = None;
            row.group = None;
            row.match_status = status.clone();
            row.is_expected = false;
            row.split_person_ids.clear();
            row.split_canonical_names.clear();
        }
        row.current_status = if status == MatchStatus::Guest {
            AttendanceStatus::Guest
        } else {
            roster_current_status(&row, base_status)
        };
        if status == MatchStatus::Guest {
            row.was_late = false;
        }
        row.tags = roster_tags(&row, counted);
        output.push(row);
    }
    if profile.is_some() {
        for person in config.roster.iter().filter(|p| p.active) {
            let group = normalize_group(&person.group);
            if seen_people.contains(&person.person_id) || !profile_includes_group(groups, &group) {
                continue;
            }
            output.push(AttendanceRow {
                entity_id: person.person_id.to_string(),
                session_participant_id: None,
                participant_uuid: None,
                participant_uuids: vec![],
                zoom_name: person.canonical_name.clone(),
                zoom_names: vec![],
                role: None,
                person_id: Some(person.person_id),
                canonical_name: Some(person.canonical_name.clone()),
                pinyin: Some(person.pinyin.clone()),
                group: Some(group),
                current_location: None,
                match_status: MatchStatus::Matched,
                is_expected: true,
                has_been_seen: false,
                is_currently_present: false,
                first_join_at: None,
                first_waiting_room_at: None,
                last_seen_at: None,
                current_status: AttendanceStatus::Missing,
                was_late: false,
                total_presence_minutes: 0,
                waiting_room_minutes: 0,
                absence_count: 0,
                returned_after_long_absence: false,
                tags: vec![AttendanceTag::Missing],
                shared_name: false,
                split_person_ids: vec![],
                split_canonical_names: vec![],
                notes: None,
            });
        }
    }
    Ok(output)
}

fn roster_current_status(row: &AttendanceRow, base: AttendanceStatus) -> AttendanceStatus {
    if row.is_expected && !row.has_been_seen {
        AttendanceStatus::Missing
    } else if row.is_currently_present && row.was_late {
        AttendanceStatus::Late
    } else if row.is_currently_present {
        base
    } else if row.has_been_seen {
        AttendanceStatus::LeftEarly
    } else {
        AttendanceStatus::Unmatched
    }
}

fn roster_tags(row: &AttendanceRow, counted: bool) -> Vec<AttendanceTag> {
    let mut tags = Vec::new();
    if counted && row.has_been_seen {
        tags.push(AttendanceTag::Seen);
    }
    if counted
        && row.has_been_seen
        && row.is_currently_present
        && !row.was_late
        && row.current_status != AttendanceStatus::ReturnedAfterLongAbsence
    {
        tags.push(AttendanceTag::FullPresent);
    }
    if counted && row.has_been_seen && row.was_late {
        tags.push(AttendanceTag::Late);
    }
    if counted
        && row.has_been_seen
        && !row.is_currently_present
        && row.current_status == AttendanceStatus::LeftEarly
    {
        tags.push(AttendanceTag::LeftEarly);
    }
    if counted
        && row.current_status == AttendanceStatus::ReturnedAfterLongAbsence
        && row.is_currently_present
    {
        tags.push(AttendanceTag::Returned);
    }
    if counted && row.current_status == AttendanceStatus::Missing {
        tags.push(AttendanceTag::Missing);
    }
    if !counted && row.has_been_seen {
        tags.push(AttendanceTag::Ignored);
    }
    if row.match_status == MatchStatus::Guest {
        tags.push(AttendanceTag::Guest);
    }
    if row.match_status == MatchStatus::Unmatched {
        tags.push(AttendanceTag::Unmatched);
    }
    let priority = |tag: &AttendanceTag| match tag {
        AttendanceTag::LeftEarly => 0,
        AttendanceTag::Late => 1,
        AttendanceTag::Returned => 2,
        AttendanceTag::FullPresent => 3,
        AttendanceTag::Seen => 4,
        AttendanceTag::Missing => 5,
        AttendanceTag::Ignored => 6,
        AttendanceTag::Guest => 7,
        AttendanceTag::Unmatched => 8,
    };
    tags.sort_by_key(priority);
    tags
}

fn latest_roster(events: &[MovementEvent]) -> Vec<ZoomParticipant> {
    let mut m = HashMap::new();
    for e in events {
        if e.kind == MovementType::Join || e.kind == MovementType::WaitingRoomJoinFromOutside {
            m.insert(
                e.participant.participant_uuid.clone(),
                e.participant.clone(),
            );
        } else if e.kind == MovementType::Leave || e.kind == MovementType::WaitingRoomLeave {
            m.remove(&e.participant.participant_uuid);
        }
    }
    m.into_values().collect()
}
fn compute_summary(rows: &[AttendanceRow], mode: &SessionMode, ended: bool) -> AttendanceSummary {
    let mut s = AttendanceSummary::default();
    for r in rows {
        let counted = *mode == SessionMode::Generic
            || matches!(r.match_status, MatchStatus::Matched | MatchStatus::Split);
        if counted && r.has_been_seen {
            s.seen += 1
        }
        if counted
            && r.has_been_seen
            && r.is_currently_present
            && !r.was_late
            && r.current_status != AttendanceStatus::ReturnedAfterLongAbsence
        {
            s.full_present += 1
        }
        if counted && r.has_been_seen && r.was_late {
            s.late += 1
        }
        if counted
            && r.current_status == AttendanceStatus::ReturnedAfterLongAbsence
            && r.is_currently_present
        {
            s.returned += 1
        }
        if counted
            && r.has_been_seen
            && !r.is_currently_present
            && r.current_status == AttendanceStatus::LeftEarly
        {
            s.left_early += 1
        }
        if *mode == SessionMode::Roster && counted && r.current_status == AttendanceStatus::Missing
        {
            s.absent += 1
        }
        if *mode == SessionMode::Roster
            && !counted
            && (if ended {
                r.has_been_seen
            } else {
                r.is_currently_present
            })
        {
            s.ignored += 1
        }
        if if *mode == SessionMode::Generic {
            r.has_been_seen
        } else {
            counted
        } {
            s.total += 1
        }
    }
    s
}

pub fn score(presence_minutes: u64, duration_minutes: u64) -> AttendanceScore {
    let pct = if duration_minutes == 0 {
        0
    } else {
        ((presence_minutes.min(duration_minutes) as f64 / duration_minutes as f64) * 100.0).round()
            as u8
    };
    let grade = match pct {
        90..=100 => AttendanceGrade::Excellent,
        75..=89 => AttendanceGrade::Good,
        50..=74 => AttendanceGrade::Moderate,
        1..=49 => AttendanceGrade::Poor,
        _ => AttendanceGrade::Absent,
    };
    AttendanceScore {
        percentage: pct,
        grade,
    }
}
fn event_timestamp(e: &Value) -> String {
    if let Some(s) = e["timestamp"].as_str() {
        s.to_string()
    } else {
        iso_from_seconds(e["timestamp"].as_f64().unwrap_or(0.0))
    }
}
fn timestamp_ms(v: &Value) -> Option<i64> {
    if let Some(s) = v.as_str() {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.timestamp_millis())
            .or_else(|| expanded_iso_millis(s))
    } else {
        v.as_i64()
            .map(|x| {
                if x.abs() > 100_000_000_000 {
                    x
                } else {
                    x * 1000
                }
            })
            .or_else(|| {
                v.as_f64().map(|x| {
                    if x.abs() > 100_000_000_000.0 {
                        x as i64
                    } else {
                        (x * 1000.0) as i64
                    }
                })
            })
    }
}

// Luxon accepts ISO years outside chrono's RFC3339 parser range (for example
// the expanded-year strings produced by legacy recordings). Keep those values
// comparable instead of silently treating them as the session start.
fn expanded_iso_millis(value: &str) -> Option<i64> {
    let value = value.strip_prefix('+')?;
    let (date, time) = value.split_once('T')?;
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    let (clock, zone) = if let Some(clock) = time.strip_suffix('Z') {
        (clock, 0i64)
    } else {
        let split = time
            .char_indices()
            .skip(1)
            .find(|(_, ch)| *ch == '+' || *ch == '-')?;
        let (clock, offset) = time.split_at(split.0);
        let sign = if offset.starts_with('-') { -1i64 } else { 1i64 };
        let offset = offset.get(1..)?;
        let (hours, minutes) = offset.split_once(':')?;
        (
            clock,
            sign * (hours.parse::<i64>().ok()? * 60 + minutes.parse::<i64>().ok()?) * 60_000,
        )
    };
    let mut clock_parts = clock.split(':');
    let hour: i64 = clock_parts.next()?.parse().ok()?;
    let minute: i64 = clock_parts.next()?.parse().ok()?;
    let seconds = clock_parts.next()?;
    let (second, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    let second: i64 = second.parse().ok()?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        let digits = format!("{fraction:0<3}");
        digits.get(..3)?.parse::<i64>().ok()?
    };
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * adjusted_month + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + fraction - zone)
}
fn iso_from_seconds(sec: f64) -> String {
    DateTime::<Utc>::from_timestamp_millis((sec * 1000.0) as i64)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoring_bands() {
        assert_eq!(score(9, 10).grade, AttendanceGrade::Excellent);
        assert_eq!(score(4, 10).grade, AttendanceGrade::Poor);
        assert_eq!(score(0, 10).percentage, 0);
    }
    #[test]
    fn bridge_maps_uuid_and_waiting_room() {
        let e = json!([{"kind":"users","action":"add","at":100000,"instanceId":"i1","users":[{"userId":7,"name":"A","waitingRoom":true}]}]);
        let o = adapt_bridge_events(e.as_array().unwrap()).unwrap();
        assert_eq!(o[0]["participantUUID"], "i1:7");
        assert_eq!(o[0]["eventType"], "waitingRoomJoinFromOutside");
    }
    #[test]
    fn waiting_room_is_separate_by_default_and_counted_in_compat_mode() {
        let ts = |minute: u32| format!("2026-01-01T00:{minute:02}:00Z");
        let events = vec![
            json!({"eventType":"waitingRoomJoinFromOutside","timestamp":ts(1),"participantUUID":"i:u","screenName":"A","location":"waiting-room"}),
            json!({"eventType":"waitingRoomLeave","timestamp":ts(2),"participantUUID":"i:u","screenName":"A","location":"waiting-room"}),
            json!({"eventType":"join","timestamp":ts(2),"participantUUID":"i:u","screenName":"A","location":"meeting"}),
        ];
        let meta = MeetingMeta {
            meeting_id: "m".into(),
            meeting_topic: String::new(),
            started_at: ts(0),
            ended_at: None,
        };
        let default = build_record(
            meta.clone(),
            &events,
            EngineConfig::default(),
            EngineState::default(),
            EngineOptions::default(),
            &ts(5),
        )
        .unwrap();
        assert_eq!(default.rows[0].waiting_room_minutes, 1);
        assert_eq!(default.rows[0].total_presence_minutes, 3);
        assert_eq!(
            default.rows[0].first_waiting_room_at.as_deref(),
            Some(ts(1).as_str())
        );
        let compat = build_record(
            meta,
            &events,
            EngineConfig::default(),
            EngineState::default(),
            EngineOptions::attendflow_compat(),
            &ts(5),
        )
        .unwrap();
        assert_eq!(compat.rows[0].total_presence_minutes, 4);
        assert_eq!(compat.rows[0].waiting_room_minutes, 0);
    }

    #[test]
    fn derivation_stops_at_instance_end_only_in_companion_mode() {
        let participant = SessionParticipant {
            session_participant_id: "sp-1".into(),
            current_participant_uuid: "m:1".into(),
            participant_uuids: vec!["m:1".into()],
            current_screen_name: "A".into(),
            screen_names: vec!["A".into()],
            role: "attendee".into(),
            current_location: ParticipantLocation::Meeting,
            first_seen_at: "2026-01-01T00:00:00.000Z".into(),
            last_seen_at: "2026-01-01T00:02:00.000Z".into(),
            identity_resolution: "new_identity".into(),
            has_ambiguous_name: false,
        };
        let session = SessionRecord {
            started_at: "2026-01-01T00:00:00.000Z".into(),
            ended_at: Some("2026-01-01T00:01:00.000Z".into()),
            session_participants: vec![participant],
            movement_events: vec![MovementEvent {
                id: "after-end".into(),
                timestamp: "2026-01-01T00:02:00.000Z".into(),
                kind: MovementType::Join,
                session_participant_id: Some("sp-1".into()),
                participant: ZoomParticipant {
                    participant_uuid: "m:1".into(),
                    screen_name: "A".into(),
                    screen_name_pinyin: String::new(),
                    role: "attendee".into(),
                    location: ParticipantLocation::Meeting,
                },
            }],
            ..SessionRecord::default()
        };
        let owner = derive(
            &session,
            &EngineConfig::default(),
            &[],
            "2026-01-01T00:03:00.000Z",
            &EngineOptions::default(),
        )
        .unwrap();
        let compat = derive(
            &session,
            &EngineConfig::default(),
            &[],
            "2026-01-01T00:03:00.000Z",
            &EngineOptions::attendflow_compat(),
        )
        .unwrap();
        assert!(!owner.rows[0].has_been_seen);
        assert!(compat.rows[0].has_been_seen);
    }
}
