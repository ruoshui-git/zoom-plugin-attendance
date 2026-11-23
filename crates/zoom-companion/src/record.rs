use attend_engine::{
    adapt_bridge_events, build_record, EngineConfig, EngineOptions, EngineState, MeetingMeta,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Boundary for the Rust attendance engine. The real attend-engine crate can replace
/// StubRecordProvider without changing HTTP handlers or the UI contract.
pub trait RecordProvider: Send + Sync {
    fn build_record(
        &self,
        meeting: &Value,
        events: &[Value],
        config: &Value,
        state: &Value,
        now: u64,
    ) -> Value;

    fn try_build_record(
        &self,
        meeting: &Value,
        events: &[Value],
        config: &Value,
        state: &Value,
        now: u64,
    ) -> Result<Value, String> {
        Ok(self.build_record(meeting, events, config, state, now))
    }
}
#[derive(Default)]
pub struct StubRecordProvider;
#[derive(Default)]
struct Participant {
    name: String,
    first: u64,
    last: Option<u64>,
    joined: Option<u64>,
    presence_ms: u64,
    waiting_join: Option<u64>,
    waiting_ms: u64,
    location: String,
}
fn iso(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}
impl RecordProvider for StubRecordProvider {
    fn build_record(
        &self,
        meeting: &Value,
        events: &[Value],
        _config: &Value,
        state: &Value,
        now: u64,
    ) -> Value {
        let id = meeting["instanceId"].as_str().unwrap_or_default();
        let mut people: BTreeMap<u64, Participant> = BTreeMap::new();
        for e in events {
            let at = e["at"].as_u64().unwrap_or(now);
            let action = e["action"].as_str().unwrap_or("");
            if let Some(users) = e["users"].as_array() {
                for u in users {
                    let Some(uid) = u["userId"].as_u64() else {
                        continue;
                    };
                    let name = u["name"].as_str().unwrap_or("Unknown").to_string();
                    let p = people.entry(uid).or_default();
                    p.name = name;
                    let waiting = u["waitingRoom"].as_bool().unwrap_or(false);
                    if action == "add" || action == "sync_all" {
                        if waiting {
                            p.location = "waiting-room".into();
                            if p.waiting_join.is_none() {
                                p.waiting_join = Some(at);
                            }
                        } else {
                            p.location = "meeting".into();
                            if p.first == 0 {
                                p.first = at;
                            }
                            if p.joined.is_none() {
                                p.joined = Some(at);
                            }
                        }
                    }
                    if action == "left" {
                        if waiting {
                            if let Some(joined) = p.waiting_join.take() {
                                p.waiting_ms =
                                    p.waiting_ms.saturating_add(at.saturating_sub(joined));
                            }
                        } else if let Some(joined) = p.joined.take() {
                            p.presence_ms = p.presence_ms.saturating_add(at.saturating_sub(joined));
                        }
                        p.last = Some(at)
                    }
                }
            }
        }
        let start = meeting["startedAt"].as_u64().unwrap_or(now);
        let duration_ms = now.saturating_sub(start).max(1);
        let duration = duration_ms as f64 / 60000.0;
        let rows: Vec<Value> = people.into_iter().map(|(uid, p)| {
            let total_ms = p.presence_ms.saturating_add(p.joined.map_or(0, |j| now.saturating_sub(j)));
            let waiting_ms = p.waiting_ms.saturating_add(p.waiting_join.map_or(0, |j| now.saturating_sub(j)));
            let minutes = total_ms as f64 / 60000.0;
            let score = (minutes / duration * 100.0).clamp(0.0, 100.0);
            let present = p.joined.is_some();
            let status = if present { "present" } else { "leftEarly" };
            let name = p.name;
            json!({"entityId":format!("{id}:{uid}"),"sessionParticipantId":format!("{uid}"),"participantUUID":format!("{id}:{uid}"),"participantUUIDs":[format!("{id}:{uid}")],"zoomName":name,"zoomNames":[name],"role":"participant","matchStatus":"not-applicable","isExpected":true,"hasBeenSeen":true,"isCurrentlyPresent":present,"firstJoinAt":if p.first>0{Some(iso(p.first))}else{None},"lastSeenAt":iso(p.last.unwrap_or(now)),"currentStatus":status,"currentLocation":p.location,"wasLate":false,"totalPresenceMinutes":minutes,"waitingRoomMinutes":waiting_ms as f64/60000.0,"absenceCount":0,"sharedName":false,"score":score,"tags":["seen"]})
        }).collect();
        let count = rows.len();
        let present = rows
            .iter()
            .filter(|r| r["isCurrentlyPresent"] == true)
            .count();
        json!({"meeting":meeting,"rows":rows,"summary":{"seen":count,"fullPresent":present,"late":0,"returned":0,"leftEarly":count.saturating_sub(present),"absent":0,"ignored":0,"total":count},"durationMinutes":duration,"score":null,"mode":state["mode"].as_str().unwrap_or("generic")})
    }
}

#[derive(Default)]
pub struct EngineRecordProvider;

impl RecordProvider for EngineRecordProvider {
    fn build_record(
        &self,
        meeting: &Value,
        events: &[Value],
        config: &Value,
        state: &Value,
        now: u64,
    ) -> Value {
        self.try_build_record(meeting, events, config, state, now)
            .unwrap_or_else(|error| json!({"error":error}))
    }

    fn try_build_record(
        &self,
        meeting: &Value,
        events: &[Value],
        config: &Value,
        state: &Value,
        now: u64,
    ) -> Result<Value, String> {
        let started_ms = meeting["startedAt"].as_u64().unwrap_or(now);
        let started_at = iso(started_ms);
        // An ended record is derived at its end time (AttendFlow derives finalized
        // sessions at endedAt); a live one at `now`.
        let ended_ms = meeting["endedAt"].as_u64();
        let reference_ms = ended_ms.map_or(now, |end| end.min(now)).max(started_ms);
        let meta = MeetingMeta {
            // AttendFlow profile auto-selection matches profile.meetingIds against the
            // Zoom meeting number, not our instance id.
            meeting_id: meeting["meetingNumber"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            meeting_topic: meeting["topic"].as_str().unwrap_or_default().to_string(),
            started_at: started_at.clone(),
            ended_at: meeting["endedAt"].as_u64().map(iso),
        };
        let mut normalized = adapt_bridge_events(events).map_err(|error| error.to_string())?;
        if let Some(end) = &meta.ended_at {
            if !normalized
                .iter()
                .any(|event| event["eventType"] == "instance_ended")
            {
                normalized.push(json!({"eventType":"instance_ended","timestamp":end}));
            }
        }
        let mut config_value = json!({
            "roster": config.get("roster").cloned().filter(|v| v.is_array()).unwrap_or_else(|| json!([])),
            "aliases": config.get("aliases").cloned().filter(|v| v.is_array()).unwrap_or_else(|| json!([])),
            "profiles": config.get("profiles").cloned().filter(|v| v.is_array()).unwrap_or_else(|| json!([])),
        });
        if let Some(profiles) = config_value["profiles"].as_array_mut() {
            let auto=profiles.iter().filter(|p|p["active"].as_bool().unwrap_or(true)).find(|p|p["meetingIds"].as_array().is_some_and(|ids|ids.iter().any(|id|id.as_str()==Some(meta.meeting_id.as_str()))))
                .or_else(||profiles.iter().filter(|p|p["active"].as_bool().unwrap_or(true)).find(|p|p["topicKeywords"].as_array().is_some_and(|ks|ks.iter().filter_map(Value::as_str).any(|kw|!kw.is_empty()&&meta.meeting_topic.to_lowercase().contains(&kw.to_lowercase())))))
                .or_else(||profiles.iter().find(|p|p["isDefault"]==true&&p["active"].as_bool().unwrap_or(true)))
                .or_else(||profiles.iter().find(|p|p["active"].as_bool().unwrap_or(true)));
            let selected=state["selectedProfileId"].as_str().or_else(||auto.and_then(|p|p["id"].as_str())).map(str::to_owned);
            if let Some(selected) = selected {
                if let Some(profile) = profiles.iter_mut().find(|p| p["id"].as_str() == Some(selected.as_str())) {
                    if let Some(v) = state["lateAfterMinutesOverride"].as_i64() { profile["lateAfterMinutes"] = json!(v.max(0)); }
                    if let Some(v) = state["returnWindowMinutesOverride"].as_i64() { profile["returnWindowMinutes"] = json!(v.max(1)); }
                }
            }
        }
        let engine_config: EngineConfig = serde_json::from_value(config_value)
            .map_err(|error| format!("invalid attendance config: {error}"))?;
        let engine_state: EngineState = serde_json::from_value(state.clone())
            .map_err(|error| format!("invalid attendance state: {error}"))?;
        let record = build_record(
            meta,
            &normalized,
            engine_config,
            engine_state,
            EngineOptions::default(),
            &iso(reference_ms),
        )
        .map_err(|error| error.to_string())?;
        let duration_minutes = (reference_ms.saturating_sub(started_ms) as f64 / 60_000.0).max(0.0);
        let media = attend_engine::media_by_participant(events, reference_ms);
        let mut rows = serde_json::to_value(&record.rows).map_err(|error| error.to_string())?;
        if let Some(rows) = rows.as_array_mut() {
            for row in rows.iter_mut() {
                // Desktop deviation (waiting-room time is not attendance): someone who
                // only ever waited and was never admitted is not "unmatched"; label it.
                let never_in_meeting = !row["hasBeenSeen"].as_bool().unwrap_or(false)
                    && row["firstJoinAt"].is_null();
                let waited = row["waitingRoomMinutes"].as_f64().unwrap_or(0.0) > 0.0
                    || !row["firstWaitingRoomAt"].is_null();
                if never_in_meeting && waited && row["currentStatus"] != "missing" {
                    row["currentStatus"] = json!("waitingRoomOnly");
                    row["tags"] = json!(["waiting_room_only"]);
                }
                let mut ids = row["participantUUIDs"].as_array().cloned().unwrap_or_default();
                if let Some(id)=row["participantUUID"].as_str(){ids.push(json!(id));}
                let stats: Vec<_> = ids.iter().filter_map(Value::as_str).filter_map(|id|media.get(id)).collect();
                if !stats.is_empty() {
                    let m=attend_engine::merge_media(&stats);
                    row["cameraOnMinutes"]=json!((m.camera_on_ms as f64/60000.0).round());
                    row["cameraOffMinutes"]=json!(((m.camera_known_ms.saturating_sub(m.camera_on_ms)) as f64/60000.0).round());
                    row["cameraOnPercent"]=if m.camera_known_ms==0 {Value::Null} else {json!((m.camera_on_ms as f64*100.0/m.camera_known_ms as f64).round())};
                    row["unmutedMinutes"]=json!((m.unmuted_ms as f64/60000.0).round());
                    row["media"]=json!(m);
                }
            }
            for row in rows {
                let minutes = row["totalPresenceMinutes"].as_u64().unwrap_or(0);
                row["score"] = json!(
                    attend_engine::score(minutes, duration_minutes.round() as u64).percentage
                );
            }
        }
        let privilege = derive_privilege(meeting, events, reference_ms);
        Ok(json!({
            "meeting": meeting,
            "privilege": privilege,
            "rows": rows,
            "summary": record.summary,
            "durationMinutes": duration_minutes,
            "score": null,
            "mode": record.mode,
            "record": record,
        }))
    }
}

fn derive_privilege(meeting: &Value, events: &[Value], reference_ms: u64) -> Value {
    let start = meeting["startedAt"].as_u64().unwrap_or(reference_ms);
    let end = meeting["endedAt"].as_u64().unwrap_or(reference_ms).min(reference_ms).max(start);
    // role_changed events (host/co-host aware) are authoritative. self_changed carries only the
    // host flag and also fires on mute/video changes, so it is used only for meetings recorded
    // before role tracking existed.
    let has_roles = events.iter().any(|e| e["kind"] == "role_changed");
    let mut changes: Vec<(u64, String)> = events.iter().filter_map(|e| {
        let at = e["at"].as_u64()?;
        match e["kind"].as_str()? {
            "role_changed" => Some((at, match e["role"].as_str()? { "host" => "host", "cohost" => "cohost", _ => "none" }.to_string())),
            "self_changed" if !has_roles && e["isHost"] == true => Some((at, "host".into())),
            "self_changed" if !has_roles && e["isHost"] == false => Some((at, "none".into())),
            _ => None,
        }
    }).collect();
    changes.sort_by_key(|c| c.0);
    let initial_host = events.iter().find(|e| matches!(e["kind"].as_str(), Some("instance_started" | "instance_resumed")))
        .and_then(|e| e["meeting"]["isHost"].as_bool()).unwrap_or(meeting["isHost"] == true);
    let mut current = if initial_host { "host".to_string() } else { "none".to_string() };
    let mut cursor = start;
    let mut intervals = Vec::new();
    let mut acquired_at = None;
    for (at, role) in changes.into_iter().filter(|(at, _)| *at >= start && *at <= end) {
        let at = at.max(cursor);
        if current != "none" && at > cursor { intervals.push(json!({"from":cursor,"to":at})); }
        if current == "none" && role != "none" && acquired_at.is_none() { acquired_at = Some(at); }
        current = role;
        cursor = at;
    }
    if current != "none" && end > cursor { intervals.push(json!({"from":cursor,"to":end})); }
    let covered = intervals.first().is_some_and(|i| i["from"] == start) && intervals.last().is_some_and(|i| i["to"] == end) && intervals.windows(2).all(|w| w[0]["to"] == w[1]["from"]);
    if initial_host && acquired_at.is_none() { acquired_at = Some(start); }
    json!({"current":current,"acquiredAt":acquired_at,"intervals":intervals,"coveredWholeMeeting":covered})
}

#[cfg(test)]
mod tests {
    #[test]
    fn privilege_intervals_from_role_and_legacy_host_events() {
        let m = |host: bool| json!({"startedAt":100,"endedAt":1000,"isHost":host});
        let ev = |kind: &str, at: u64, extra: Value| { let mut e = json!({"kind":kind,"at":at}); for (k, v) in extra.as_object().unwrap() { e[k] = v.clone(); } e };
        // Never privileged.
        let p = derive_privilege(&m(false), &[], 2000);
        assert_eq!(p["current"], "none"); assert_eq!(p["acquiredAt"], Value::Null); assert_eq!(p["coveredWholeMeeting"], false);
        // Host for the whole meeting.
        let p = derive_privilege(&m(true), &[], 2000);
        assert_eq!(p["intervals"], json!([{"from":100,"to":1000}])); assert_eq!(p["coveredWholeMeeting"], true); assert_eq!(p["acquiredAt"], 100);
        // Co-host gained midway, then lost.
        let events = [ev("role_changed", 300, json!({"role":"cohost"})), ev("role_changed", 700, json!({"role":"attendee"}))];
        let p = derive_privilege(&m(false), &events, 2000);
        assert_eq!(p["intervals"], json!([{"from":300,"to":700}])); assert_eq!(p["acquiredAt"], 300); assert_eq!(p["current"], "none"); assert_eq!(p["coveredWholeMeeting"], false);
        // With role events present, a self_changed without the host flag (e.g. a mute) does not end co-host.
        let events = [ev("role_changed", 300, json!({"role":"cohost"})), ev("self_changed", 400, json!({"isHost":false}))];
        let p = derive_privilege(&m(false), &events, 2000);
        assert_eq!(p["intervals"], json!([{"from":300,"to":1000}])); assert_eq!(p["current"], "cohost");
        // Legacy meetings (no role events) use the host flag transitions.
        let events = [ev("self_changed", 500, json!({"isHost":true}))];
        let p = derive_privilege(&m(false), &events, 2000);
        assert_eq!(p["intervals"], json!([{"from":500,"to":1000}])); assert_eq!(p["acquiredAt"], 500);
    }
    use super::*;
    use std::{fs, path::PathBuf};

    #[test]
    fn fake_host_two_guests_produces_one_row_per_person_after_admission() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/fake/host-two-guests.jsonl");
        let text = fs::read_to_string(path).unwrap();
        let mut events = Vec::new();
        let instance = "fake-host-two-guests";
        let start = 1_794_893_640_000_u64;
        for line in text.lines() {
            let line: Value = serde_json::from_str(line).unwrap();
            if line["event"] == "user_status" {
                let action = line["action"].as_u64().unwrap_or_default();
                let users: Vec<Value> = line["users"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|user| {
                        json!({
                            "userId": user["user_id"],
                            "name": user["name"],
                            "waitingRoom": user["flags"].as_u64().unwrap_or_default() & 256 != 0,
                            "host": user["flags"].as_u64().unwrap_or_default() & 1 != 0,
                        })
                    })
                    .collect();
                let event_action = match action {
                    1 => "sync_all",
                    2 => "add",
                    3 => "left",
                    4 => "update",
                    _ => continue,
                };
                events.push(json!({
                    "kind":"users",
                    "action":event_action,
                    "instanceId":instance,
                    "at":start + events.len() as u64 * 60_000,
                    "users":users,
                }));
            }
        }
        let provider = EngineRecordProvider;
        let record = provider
            .try_build_record(
                &json!({"instanceId":instance,"meetingNumber":"1","topic":"Fake","startedAt":start,"isHost":true}),
                &events,
                &json!({"roster":[],"aliases":[],"profiles":[]}),
                &json!({"mode":"generic"}),
                start + 20 * 60_000,
            )
            .unwrap();
        assert_eq!(record["rows"].as_array().unwrap().len(), 4);
        let admitted = record["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["zoomName"] == "Waiting Guest")
            .unwrap();
        assert_eq!(admitted["participantUUIDs"].as_array().unwrap().len(), 2);
        assert_eq!(admitted["firstJoinAt"], "2026-11-17T05:37:00.000Z");
        assert_eq!(record["summary"]["total"], 4);
    }

    #[test]
    fn engine_provider_returns_config_errors_without_panicking() {
        let error = EngineRecordProvider
            .try_build_record(
                &json!({"instanceId":"i","startedAt":0}),
                &[],
                &json!({"roster":[{"personId":"bad"}]}),
                &json!({"mode":"generic"}),
                0,
            )
            .unwrap_err();
        assert!(error.starts_with("invalid attendance config:"));
    }

    #[test]
    fn ended_record_derives_at_end_and_labels_never_admitted_waiters() {
        let instance = "pz-1-1000";
        let start = 1_794_893_640_000_u64;
        let user = |id: u64, name: &str, waiting: bool, host: bool| {
            json!({"userId":id,"name":name,"waitingRoom":waiting,"host":host})
        };
        let ev = |seq: u64, at: u64, action: &str, users: Vec<Value>| {
            json!({"seq":seq,"kind":"users","action":action,"instanceId":instance,"at":at,"users":users})
        };
        let events = vec![
            ev(1, start, "sync_all", vec![user(1, "Host", false, true)]),
            // Waits 5 minutes and is never admitted.
            ev(2, start + 60_000, "add", vec![user(2, "Waiter", true, false)]),
            // Admitted guest: left(waiting id) + add(new id) within 100 ms.
            ev(3, start + 120_000, "add", vec![user(3, "Guest", true, false)]),
            ev(4, start + 180_000, "left", vec![user(3, "Guest", true, false)]),
            ev(5, start + 180_050, "add", vec![user(4, "Guest", false, false)]),
            ev(6, start + 360_000, "left", vec![user(2, "Waiter", true, false)]),
        ];
        let ended = start + 10 * 60_000;
        let meeting = json!({"instanceId":instance,"meetingNumber":"1","topic":"T","startedAt":start,
            "endedAt":ended,"endReason":"companion_offline","isHost":true});
        // `now` is long after the end: derivation must still stop at endedAt.
        let now = ended + 5 * 3_600_000;
        let record = EngineRecordProvider
            .try_build_record(&meeting, &events, &json!({}), &json!({"mode":"generic"}), now)
            .unwrap();
        let rows = record["rows"].as_array().unwrap();
        let row = |name: &str| {
            rows.iter()
                .find(|r| r["zoomName"] == name)
                .unwrap_or_else(|| panic!("no row for {name}: {rows:?}"))
        };
        assert_eq!(rows.iter().filter(|r| r["zoomName"] == "Guest").count(), 1, "admitted guest is one person");
        assert_eq!(row("Host")["totalPresenceMinutes"], json!(10), "host presence ends at endedAt");
        assert_eq!(row("Waiter")["currentStatus"], json!("waitingRoomOnly"));
        assert_eq!(row("Waiter")["totalPresenceMinutes"], json!(0));
        assert!(row("Guest")["firstJoinAt"].as_str().is_some(), "guest first join is admission time");
        assert_eq!(record["durationMinutes"].as_f64().unwrap().round(), 10.0);
    }

    #[test]
    fn persisted_match_and_split_decisions_rederive_roster_rows() {
        let start=1_794_893_640_000_u64;
        let meeting=json!({"instanceId":"reconcile-engine","meetingNumber":"1","topic":"Roster","startedAt":start,"isHost":false});
        let events=vec![json!({"kind":"users","action":"sync_all","instanceId":"reconcile-engine","at":start,"users":[{"userId":7,"name":"Shared Zoom"}]})];
        let config=json!({"roster":[
            {"personId":1,"canonicalName":"Ada Lovelace","pinyin":"ada lovelace","group":"A","active":true},
            {"personId":2,"canonicalName":"Grace Hopper","pinyin":"grace hopper","group":"B","active":true}
        ],"aliases":[],"profiles":[]});
        let provider=EngineRecordProvider;
        let unmatched=provider.try_build_record(&meeting,&events,&config,&json!({"mode":"roster"}),start+60_000).unwrap();
        assert!(unmatched["rows"].as_array().unwrap().iter().any(|r|r["matchStatus"]=="unmatched"));
        let match_state=json!({"mode":"roster","decisions":[{"id":"d1","sessionParticipantId":"sp-1","zoomName":"Shared Zoom","zoomNameAtDecision":"Shared Zoom","action":"match","personId":1,"canonicalName":"Ada Lovelace","createdAt":"2026-11-17T00:00:00Z"}]});
        let matched=provider.try_build_record(&meeting,&events,&config,&match_state,start+60_000).unwrap();
        assert!(matched["rows"].as_array().unwrap().iter().any(|r|r["matchStatus"]=="matched"&&r["canonicalName"]=="Ada Lovelace"),"matched result: {}",matched["rows"]);
        let split_state=json!({"mode":"roster","decisions":[{"id":"d2","sessionParticipantId":"sp-1","zoomName":"Shared Zoom","zoomNameAtDecision":"Shared Zoom","action":"split","splitPersonIds":[1,2],"splitCanonicalNames":["Ada Lovelace","Grace Hopper"],"createdAt":"2026-11-17T00:00:00Z"}]});
        let split=provider.try_build_record(&meeting,&events,&config,&split_state,start+60_000).unwrap();
        let rows=split["rows"].as_array().unwrap();
        assert_eq!(rows.iter().filter(|r|r["matchStatus"]=="split").count(),2,"a split decision fans one participant out to both selected roster people");
    }

}
