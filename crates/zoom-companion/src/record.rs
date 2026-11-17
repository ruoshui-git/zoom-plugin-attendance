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
