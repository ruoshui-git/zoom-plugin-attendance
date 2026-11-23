use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaStats {
    pub camera_on_ms: u64,
    pub camera_known_ms: u64,
    pub unmute_count: u32,
    pub unmuted_ms: u64,
    pub audio_known_ms: u64,
    pub last_audio_type: Option<i32>,
    pub ever_no_audio: bool,
    pub phone: bool,
    pub video_on: Option<bool>,
    pub audio_muted: Option<bool>,
}

#[derive(Default)]
struct Track { stats: MediaStats, video: Option<bool>, muted: Option<bool>, seen: bool, cursor: u64, video_since: Option<u64>, unmuted_since: Option<u64>, present: bool }
fn close(t: &mut Track, at: u64, departed: bool) {
    if let Some(s) = t.video_since.take() { t.stats.camera_on_ms += at.saturating_sub(s); }
    if let Some(s) = t.unmuted_since.take() { t.stats.unmuted_ms += at.saturating_sub(s); }
    if t.seen { t.stats.camera_known_ms += at.saturating_sub(t.cursor); t.stats.audio_known_ms += at.saturating_sub(t.cursor); }
    t.present = false;
    if departed { t.video = None; t.muted = None; t.stats.video_on=None; t.stats.audio_muted=None; }
}

pub fn media_by_participant(events: &[Value], end_ms: u64) -> HashMap<String, MediaStats> {
    let mut tracks: HashMap<String, Track> = HashMap::new();
    let mut sorted: Vec<&Value> = events.iter().collect(); sorted.sort_by_key(|e| e["at"].as_u64().unwrap_or(0));
    for e in sorted {
        let at = e["at"].as_u64().unwrap_or(0).min(end_ms);
        match e["kind"].as_str().unwrap_or("") {
            "users" => {
                let instance = e["instanceId"].as_str().unwrap_or(""); let action=e["action"].as_str().unwrap_or("");
                for u in e["users"].as_array().into_iter().flatten() {
                    let Some(uid)=u["userId"].as_u64() else { continue };
                    let key=format!("{instance}:{uid}"); let t=tracks.entry(key).or_default();
                    if action=="left" { close(t,at,true); continue; }
                    let v=u["videoOn"].as_bool(); let m=u["audioMuted"].as_bool();
                    if !t.seen { t.seen=true; t.present=true; t.cursor=at; t.video=v; t.muted=m; if v==Some(true){t.video_since=Some(at)}; }
                    else if t.present {
                        if v.is_some() && v!=t.video { if let Some(s)=t.video_since.take(){t.stats.camera_on_ms+=at.saturating_sub(s)}; if v==Some(true){t.video_since=Some(at)}; t.video=v; }
                        if m.is_some() && m!=t.muted { if t.muted==Some(true)&&m==Some(false){t.stats.unmute_count+=1;t.unmuted_since=Some(at)}; if t.muted==Some(false)&&m==Some(true){if let Some(s)=t.unmuted_since.take(){t.stats.unmuted_ms+=at.saturating_sub(s)}}; t.muted=m; }
                        t.stats.camera_known_ms+=at.saturating_sub(t.cursor); t.stats.audio_known_ms+=at.saturating_sub(t.cursor); t.cursor=at;
                    }
                    if let Some(kind)=u.get("audioType").and_then(Value::as_i64).map(|v|v as i32) { t.stats.last_audio_type=Some(kind); t.stats.ever_no_audio|=kind==0; t.stats.phone|=kind==2; }
                }
            },
            "instance_ended" => for t in tracks.values_mut(){if t.present{t.stats.video_on=t.video;t.stats.audio_muted=t.muted;close(t,at,false)}},
            _=>()
        }
    }
    for t in tracks.values_mut(){if t.present{t.stats.video_on=t.video; t.stats.audio_muted=t.muted; close(t,end_ms,false);}}
    tracks.into_iter().map(|(k,t)|(k,t.stats)).collect()
}

pub fn merge_media(stats: &[&MediaStats]) -> MediaStats {
    let mut out=MediaStats::default();
    for s in stats { out.camera_on_ms+=s.camera_on_ms;out.camera_known_ms+=s.camera_known_ms;out.unmute_count+=s.unmute_count;out.unmuted_ms+=s.unmuted_ms;out.audio_known_ms+=s.audio_known_ms; if s.last_audio_type.is_some(){out.last_audio_type=s.last_audio_type};out.ever_no_audio|=s.ever_no_audio;out.phone|=s.phone;if s.video_on.is_some(){out.video_on=s.video_on};if s.audio_muted.is_some(){out.audio_muted=s.audio_muted}; }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn ev(at:u64, action:&str, users:Value)->Value { json!({"kind":"users","at":at,"instanceId":"i","action":action,"users":users}) }
    fn user(id:u64,v:bool,m:bool)->Value { json!({"userId":id,"videoOn":v,"audioMuted":m}) }
    #[test] fn camera_on_off_intervals_and_percent(){let e=[ev(0,"sync_all",json!([user(1,true,true)])),ev(60_000,"update",json!([user(1,false,true)])),ev(120_000,"update",json!([user(1,true,true)]))];let s=media_by_participant(&e,180_000);let m=&s["i:1"];assert_eq!(m.camera_on_ms,120_000);assert_eq!(m.camera_known_ms,180_000);}
    #[test] fn first_snapshot_muted_state_is_not_unmute(){let e=[ev(0,"sync_all",json!([user(1,true,false)]))];assert_eq!(media_by_participant(&e,60_000)["i:1"].unmute_count,0);}
    #[test] fn unmute_count_and_minutes(){let e=[ev(0,"sync_all",json!([user(1,true,true)])),ev(10_000,"update",json!([user(1,true,false)]))];let m=&media_by_participant(&e,70_000)["i:1"];assert_eq!(m.unmute_count,1);assert_eq!(m.unmuted_ms,60_000);}
    #[test] fn left_and_instance_end_close_intervals(){let e=[ev(0,"sync_all",json!([user(1,true,false)])),ev(30_000,"left",json!([user(1,true,false)]))];let m=&media_by_participant(&e,90_000)["i:1"];assert_eq!(m.camera_on_ms,30_000);assert_eq!(m.video_on,None);let e=[ev(0,"sync_all",json!([user(1,true,false)])),json!({"kind":"instance_ended","at":40_000})];assert_eq!(media_by_participant(&e,90_000)["i:1"].camera_on_ms,40_000);}
    #[test] fn still_present_keeps_current_state(){let e=[ev(0,"sync_all",json!([user(1,true,false)]))];let m=&media_by_participant(&e,10)["i:1"];assert_eq!(m.video_on,Some(true));assert_eq!(m.audio_muted,Some(false));}
    #[test] fn missing_audio_type_is_unknown_not_none(){let e=[ev(0,"sync_all",json!([user(1,true,true)]))];let m=&media_by_participant(&e,10)["i:1"];assert_eq!(m.last_audio_type,None);assert!(!m.ever_no_audio);}
    #[test] fn phone_and_no_audio_markers(){let mut u=user(1,true,true);u["audioType"]=json!(0);let mut p=user(2,true,true);p["audioType"]=json!(2);let m=media_by_participant(&[ev(0,"sync_all",json!([u,p]))],10);assert!(m["i:1"].ever_no_audio);assert!(m["i:2"].phone);}
    #[test] fn rejoin_ids_merge(){let a=MediaStats{camera_on_ms:10,ever_no_audio:true,..Default::default()};let b=MediaStats{camera_on_ms:20,phone:true,..Default::default()};let m=merge_media(&[&a,&b]);assert_eq!(m.camera_on_ms,30);assert!(m.phone&&m.ever_no_audio);}
}
