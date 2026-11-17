use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct Artifact {
    pub filename: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
fn csv(headers: &[&str], rows: &[Vec<String>]) -> Vec<u8> {
    let mut out = vec![0xef, 0xbb, 0xbf];
    out.extend_from_slice(headers.join(",").as_bytes());
    out.extend_from_slice(b"\r\n");
    for row in rows {
        out.extend_from_slice(
            row.iter()
                .map(|v| csv_escape(v))
                .collect::<Vec<_>>()
                .join(",")
                .as_bytes(),
        );
        out.extend_from_slice(b"\r\n");
    }
    out
}
fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
fn number(v: &Value, key: &str) -> f64 {
    v[key].as_f64().unwrap_or(0.0)
}
fn bool_text(v: &Value, key: &str, en: bool) -> String {
    if v[key].as_bool().unwrap_or(false) {
        (if en { "Yes" } else { "是" }).into()
    } else {
        String::new()
    }
}
fn timestamp(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|d| d.format("%Y%m%d-%H%M%S").to_string())
        .unwrap_or_else(|| "unknown".into())
}
fn tags(row: &Value, en: bool) -> String {
    row["tags"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|tag| match (tag, en) {
                    ("seen", true) => "Seen",
                    ("seen", false) => "到过",
                    ("full_present", true) => "Full Present",
                    ("full_present", false) => "全程到场",
                    ("late", true) => "Late",
                    ("late", false) => "迟到",
                    ("left_early", true) => "Left Early",
                    ("left_early", false) => "早退",
                    ("returned", true) => "Returned",
                    ("returned", false) => "返回",
                    ("missing", true) => "Missing",
                    ("missing", false) => "缺席",
                    ("ignored", true) => "Ignored",
                    ("ignored", false) => "不计入",
                    (other, _) => other,
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .unwrap_or_default()
}
fn grade(score: f64, en: bool) -> &'static str {
    if score >= 90.0 {
        if en {
            "Excellent"
        } else {
            "优秀"
        }
    } else if score >= 75.0 {
        if en {
            "Good"
        } else {
            "良好"
        }
    } else if score >= 50.0 {
        if en {
            "Moderate"
        } else {
            "一般"
        }
    } else if score > 0.0 {
        if en {
            "Poor"
        } else {
            "较差"
        }
    } else if en {
        "Absent"
    } else {
        "缺席"
    }
}

pub fn artifacts(
    meeting: &Value,
    record: &Value,
    state: &Value,
    events: &[Value],
    exported_at: &str,
    language: &str,
    kind: &str,
) -> Vec<Artifact> {
    let en = language == "en";
    let id = text(meeting, "instanceId");
    let started_ms = meeting["startedAt"].as_u64().unwrap_or_default();
    let started_at = chrono::DateTime::from_timestamp_millis(started_ms as i64)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();
    let ended_at = meeting["endedAt"]
        .as_u64()
        .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64).map(|d| d.to_rfc3339()));
    let stamp = timestamp(started_ms);
    let session = json!({"version":3,"id":null,"sessionId":id,"originSessionKey":id,"sessionRevision":meeting["lastSeq"].as_u64().unwrap_or_default(),"recordSource":if meeting["source"]=="imported" {"main_import"} else {"meeting_live"},"startedAt":started_at,"endedAt":ended_at,"isActive":ended_at.is_none(),"needsExport":false,"mode":record["mode"],"meetingUUID":meeting["instanceId"],"meetingId":meeting["meetingNumber"],"meetingTopic":meeting["topic"],"privilegeAcquiredAt":started_at,"guessedProfileId":record["record"]["guessedProfileId"],"selectedProfileId":record["record"]["selectedProfileId"],"selectedIncludedSubgroups":record["record"]["selectedIncludedSubgroups"].as_array().cloned().unwrap_or_default(),"selectedGroupSortOrder":record["record"]["selectedGroupSortOrder"],"lateThresholdMinutes":record["record"]["lateAfterMinutes"].as_u64().unwrap_or(10),"returnWindowMinutes":record["record"]["returnWindowMinutes"].as_u64().unwrap_or(8),"sessionParticipants":record["record"]["participants"].as_array().cloned().unwrap_or_default(),"movementEvents":record["record"]["movementEvents"].as_array().cloned().unwrap_or_default(),"nameChangeEvents":record["record"]["nameChangeEvents"].as_array().cloned().unwrap_or_default(),"latestRoster":record["record"]["latestRoster"].as_array().cloned().unwrap_or_default(),"rosterObservations":record["record"]["rosterObservations"].as_array().cloned().unwrap_or_default(),"manualSnapshots":state["manualSnapshots"].as_array().cloned().unwrap_or_default(),"reconciliationDecisions":state["reconciliationDecisions"].as_array().cloned().unwrap_or_default(),"derivedRows":record["rows"],"exportedAt":exported_at,"raiseHandEntries":state["raiseHandEntries"].as_array().cloned().unwrap_or_else(||hand_entries(events)),"raiseHandCaptures":state["raiseHandCaptures"].as_array().cloned().unwrap_or_default(),"lastSubmittedAt":Value::Null,"meeting":meeting,"summary":record["summary"]});
    let mut files = Vec::new();
    if kind == "json" || kind == "all" {
        let content = json!({"version":3,"exportedAt":exported_at,"session":session}).to_string();
        files.push(Artifact {
            filename: format!("session-{stamp}-{id}.json"),
            content_type: "application/json; charset=utf-8",
            bytes: content.into_bytes(),
        });
    }
    if kind == "summary" || kind == "all" {
        let headers = if en {
            vec![
                "#",
                "id",
                "canonicalName",
                "zoomName",
                "zoomNameHistory",
                "group",
                "role",
                "statusTags",
                "location",
                "matchStatus",
                "attendanceScore",
                "attendanceGrade",
                "firstJoinAt",
                "lastSeenAt",
                "totalPresenceMinutes",
                "absenceCount",
                "sharedName",
                "notes",
                "waitingRoomMinutes",
            ]
        } else {
            vec![
                "#",
                "固定编号",
                "姓名",
                "Zoom名称",
                "改名历史",
                "组别",
                "角色",
                "状态标签",
                "当前位置",
                "匹配状态",
                "出勤分数",
                "出勤等级",
                "首次进入时间",
                "最后出现时间",
                "累计在线分钟",
                "离开次数",
                "是否疑似共用名称",
                "备注",
                "等候室分钟",
            ]
        };
        let mut rows = record["rows"].as_array().cloned().unwrap_or_default();
        rows.sort_by(|a, b| {
            number(b, "score")
                .total_cmp(&number(a, "score"))
                .then_with(|| text(a, "group").cmp(text(b, "group")))
                .then_with(|| text(a, "pinyin").cmp(text(b, "pinyin")))
                .then_with(|| text(a, "canonicalName").cmp(text(b, "canonicalName")))
                .then_with(|| text(a, "zoomName").cmp(text(b, "zoomName")))
        });
        let data = rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                vec![
                    (i + 1).to_string(),
                    r["personId"]
                        .as_i64()
                        .map(|x| x.to_string())
                        .unwrap_or_default(),
                    text(r, "canonicalName").into(),
                    text(r, "zoomName").into(),
                    r["zoomNames"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .unwrap_or_default(),
                    text(r, "group").into(),
                    text(r, "role").into(),
                    tags(r, en),
                    text(r, "currentLocation").into(),
                    text(r, "matchStatus").into(),
                    format!("{:.0}", number(r, "score")),
                    grade(number(r, "score"), en).into(),
                    text(r, "firstJoinAt").into(),
                    text(r, "lastSeenAt").into(),
                    format!("{:.2}", number(r, "totalPresenceMinutes")),
                    r["absenceCount"].as_u64().unwrap_or_default().to_string(),
                    bool_text(r, "sharedName", en),
                    text(r, "notes").into(),
                    format!("{:.2}", number(r, "waitingRoomMinutes")),
                ]
            })
            .collect::<Vec<_>>();
        files.push(Artifact {
            filename: format!("session-{stamp}-{id}.csv"),
            content_type: "text/csv; charset=utf-8",
            bytes: csv(&headers, &data),
        });
    }
    if kind == "manual" || kind == "all" {
        let headers = if en {
            vec![
                "manualAttendanceRecordedAt",
                "zoomName",
                "participantUUID",
                "location",
                "matchedPersonId",
                "matchedCanonicalName",
                "matchStatus",
            ]
        } else {
            vec![
                "手动考勤时间",
                "Zoom名称",
                "参与者标识",
                "位置",
                "匹配固定编号",
                "匹配姓名",
                "匹配状态",
            ]
        };
        let data = state["manualSnapshots"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|s| {
                s["participants"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(move |p| {
                        vec![
                            text(s, "timestamp").into(),
                            text(p, "screenName").into(),
                            text(p, "participantUUID").into(),
                            text(p, "location").into(),
                            p["matchedPersonId"]
                                .as_i64()
                                .map(|n| n.to_string())
                                .unwrap_or_default(),
                            text(p, "matchedCanonicalName").into(),
                            text(p, "matchStatus").into(),
                        ]
                    })
            })
            .collect::<Vec<_>>();
        files.push(Artifact {
            filename: format!("session-manual-attendance-records-{stamp}-{id}.csv"),
            content_type: "text/csv; charset=utf-8",
            bytes: csv(&headers, &data),
        });
    }
    files
}

pub fn hand_entries(events: &[Value]) -> Vec<Value> {
    let mut names = std::collections::HashMap::<String, String>::new();
    let mut raised = std::collections::HashMap::<String, bool>::new();
    let mut entries = Vec::new();
    for event in events {
        if event["kind"] == "users" {
            for user in event["users"].as_array().into_iter().flatten() {
                let id = user["userId"].to_string().trim_matches('"').to_owned();
                if let Some(name) = user["name"].as_str() {
                    names.insert(id.clone(), name.to_owned());
                }
                if user["raisedHand"].as_bool().unwrap_or(false)
                    && !raised.get(&id).copied().unwrap_or(false)
                {
                    raised.insert(id.clone(), true);
                    entries.push(hand_entry(
                        event,
                        &id,
                        names.get(&id).map(String::as_str).unwrap_or("Unknown"),
                        true,
                    ));
                }
            }
        } else if event["kind"] == "hand" {
            let id = event["userId"].to_string().trim_matches('"').to_owned();
            let is_raised = event["raised"].as_bool().unwrap_or(false);
            if raised.get(&id).copied() != Some(is_raised) {
                raised.insert(id.clone(), is_raised);
                entries.push(hand_entry(
                    event,
                    &id,
                    names.get(&id).map(String::as_str).unwrap_or("Unknown"),
                    is_raised,
                ));
            }
        }
    }
    entries
}
fn hand_entry(event: &Value, id: &str, name: &str, raised: bool) -> Value {
    let at = event["at"].as_u64().unwrap_or_default();
    let timestamp = chrono::DateTime::from_timestamp_millis(at as i64)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();
    json!({"id":format!("{}-{id}",event["seq"]),"participantUUID":id,"screenName":name,"timestamp":timestamp,"action":if raised {"raise"} else {"lower"},"sequence":event["seq"],"receivedAt":at})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_headers_bom_score_grade_and_order() {
        let m = json!({"instanceId":"id","startedAt":0});
        let r = json!({"rows":[{"zoomName":"B","group":"z","score":50,"totalPresenceMinutes":5},{"zoomName":"A","group":"a","score":90,"totalPresenceMinutes":9}]});
        let f = artifacts(&m, &r, &json!({}), &[], "now", "en", "summary").remove(0);
        assert_eq!(&f.bytes[..3], &[0xef, 0xbb, 0xbf]);
        let s = String::from_utf8(f.bytes[3..].to_vec()).unwrap();
        assert!(s.starts_with("#,id,canonicalName,zoomName,zoomNameHistory,group,role,statusTags,location,matchStatus,attendanceScore,attendanceGrade,firstJoinAt,lastSeenAt,totalPresenceMinutes,absenceCount,sharedName,notes,waitingRoomMinutes\r\n"));
        assert!(s.find("Excellent").unwrap() < s.find("Moderate").unwrap());
        assert!(s.find(",A,").unwrap() < s.find(",B,").unwrap());
    }
    #[test]
    fn chinese_headers_and_manual_csv_are_bom_prefixed() {
        let m = json!({"instanceId":"id","startedAt":0});
        let summary = artifacts(
            &m,
            &json!({"rows":[]}),
            &json!({}),
            &[],
            "x",
            "zh-CN",
            "summary",
        )
        .remove(0);
        assert_eq!(&summary.bytes[..3], &[0xef, 0xbb, 0xbf]);
        let summary_text = String::from_utf8(summary.bytes[3..].to_vec()).unwrap();
        assert!(summary_text.starts_with("#,固定编号,姓名,Zoom名称,改名历史,组别,角色,状态标签,当前位置,匹配状态,出勤分数,出勤等级,首次进入时间,最后出现时间,累计在线分钟,离开次数,是否疑似共用名称,备注,等候室分钟\r\n"));
        let f=artifacts(&m,&json!({}),&json!({"manualSnapshots":[{"timestamp":"t","participants":[{"screenName":"Ada","participantUUID":"u","location":"meeting","matchStatus":"matched"}]}]}),&[],"x","zh-CN","manual").remove(0);
        assert_eq!(&f.bytes[..3], &[0xef, 0xbb, 0xbf]);
        let s = String::from_utf8(f.bytes).unwrap();
        assert!(s.starts_with(
            "\u{feff}手动考勤时间,Zoom名称,参与者标识,位置,匹配固定编号,匹配姓名,匹配状态\r\n"
        ));
        assert!(s.contains("Ada"));
    }
    #[test]
    fn json_bundle_shape() {
        let fs = artifacts(
            &json!({"instanceId":"i","startedAt":0}),
            &json!({"rows":[]}),
            &json!({}),
            &[],
            "stamp",
            "en",
            "json",
        );
        let v: Value = serde_json::from_slice(&fs[0].bytes).unwrap();
        assert_eq!(v["version"], 3);
        assert_eq!(v["exportedAt"], "stamp");
        assert!(v["session"].is_object());
        assert_eq!(v["session"]["sessionId"], "i");
        assert_eq!(v["session"]["needsExport"], false);
        assert!(v["session"]["startedAt"]
            .as_str()
            .unwrap()
            .starts_with("1970-"));
    }
    #[test]
    fn hand_log_deduplicates_repeated_events_and_keeps_startup_raises() {
        let entries = hand_entries(&[
            json!({"kind":"users","seq":1,"at":10,"users":[{"userId":7,"name":"Ada","raisedHand":true}]}),
            json!({"kind":"hand","seq":2,"at":11,"userId":7,"raised":true}),
            json!({"kind":"hand","seq":3,"at":12,"userId":7,"raised":false}),
        ]);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["action"], "raise");
        assert_eq!(entries[0]["screenName"], "Ada");
        assert_eq!(entries[1]["action"], "lower");
    }
}
