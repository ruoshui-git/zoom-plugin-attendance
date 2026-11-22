use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::record::{EngineRecordProvider, RecordProvider};
#[cfg(test)]
use crate::record::StubRecordProvider;
use attend_store::Store;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use zoom_bridge::status::{self, MeetingInstance};
use zoom_bridge::{Event, Participant, Session};

use anyhow::Context;
use crate::config::Config;
use crate::connection::{classify_start_error, ZoomLink, ZoomState, ZoomStatus};
use crate::{open_browser, start_session};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct User {
    #[serde(rename = "userId")]
    user_id: u32,
    name: String,
    host: bool,
    myself: bool,
    #[serde(rename = "waitingRoom")]
    waiting_room: bool,
    #[serde(rename = "audioMuted")]
    audio_muted: bool,
    #[serde(rename = "videoOn")]
    video_on: bool,
    #[serde(rename = "raisedHand")]
    raised_hand: bool,
    talking: bool,
    spotlighted: bool,
}
fn user(p: &Participant) -> User {
    let f = p.flags;
    User {
        user_id: p.user_id,
        name: p.name.clone(),
        host: f.host(),
        myself: f.myself(),
        waiting_room: f.in_waiting_room(),
        audio_muted: f.audio_muted(),
        video_on: f.video_on(),
        raised_hand: f.raised_hand(),
        talking: f.talking(),
        spotlighted: f.spotlighted(),
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Meeting {
    #[serde(rename = "instanceId")]
    instance_id: String,
    #[serde(rename = "meetingNumber")]
    meeting_number: String,
    topic: String,
    #[serde(rename = "meetingType")]
    meeting_type: i32,
    status: i32,
    #[serde(rename = "statusName")]
    status_name: String,
    #[serde(rename = "startedAt")]
    started_at: u64,
    myself: Option<User>,
    #[serde(rename = "isHost")]
    is_host: bool,
}
#[derive(Clone)]
struct LogEvent {
    seq: u64,
    at: u64,
    instance_id: String,
    kind: String,
    payload: Value,
}
impl LogEvent {
    fn wire(&self) -> Value {
        let mut v = json!({"type":"event","seq":self.seq,"at":self.at,"instanceId":self.instance_id,"kind":self.kind});
        if let (Some(a), Some(b)) = (v.as_object_mut(), self.payload.as_object()) {
            a.extend(b.clone())
        };
        v
    }
}
#[derive(Default)]
struct Tracker {
    meeting: Option<Meeting>,
    seq: u64,
    users: Vec<User>,
    history: VecDeque<LogEvent>,
    connected: bool,
    root: Option<PathBuf>,
    file: Option<std::fs::File>,
    resume: Option<Resume>,
    last_touch: u64,
}
/// A companion restart within this window of the last event resumes the same instance.
const RESUME_WINDOW_MS: u64 = 15 * 60 * 1000;
#[derive(Clone, Serialize, Deserialize)]
struct Resume {
    #[serde(rename = "instanceId")]
    instance_id: String,
    #[serde(rename = "meetingNumber")]
    meeting_number: String,
    #[serde(rename = "startedAt")]
    started_at: u64,
    #[serde(rename = "lastSeq")]
    last_seq: u64,
    #[serde(rename = "lastSeenAt")]
    last_seen_at: u64,
}
impl Tracker {
    fn new(root: Option<PathBuf>) -> Self {
        let resume = root
            .as_ref()
            .and_then(|r| fs::read(r.join("current.json")).ok())
            .and_then(|b| serde_json::from_slice(&b).ok());
        Self {
            root,
            resume,
            ..Default::default()
        }
    }
    fn state(&self) -> Value {
        json!({"type":"state","meeting":self.meeting,"participants":self.users,"seqHead":self.seq,"at":now()})
    }
    fn emit(&mut self, kind: &str, payload: Value) -> Option<Value> {
        let m = self.meeting.as_ref()?;
        self.seq += 1;
        let e = LogEvent {
            seq: self.seq,
            at: now(),
            instance_id: m.instance_id.clone(),
            kind: kind.into(),
            payload,
        };
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{}", e.wire());
            let _ = f.flush();
        }
        self.write_current();
        self.history.push_back(e.clone());
        while self.history.len() > 10000 {
            self.history.pop_front();
        }
        Some(e.wire())
    }
    /// Persists the resume point (instance, last seq, last seen = now).
    fn write_current(&mut self) {
        if let (Some(root), Some(m)) = (&self.root, &self.meeting) {
            let current = Resume {
                instance_id: m.instance_id.clone(),
                meeting_number: m.meeting_number.clone(),
                started_at: m.started_at,
                last_seq: self.seq,
                last_seen_at: now(),
            };
            let _ = fs::write(
                root.join("current.json"),
                serde_json::to_vec_pretty(&current).unwrap_or_default(),
            );
            self.last_touch = now();
        }
    }
    /// Liveness heartbeat while a meeting is live: quiet meetings (no joins/leaves for
    /// longer than the resume window) must stay resumable after a companion restart.
    fn touch(&mut self) {
        if self.meeting.is_some() && now().saturating_sub(self.last_touch) >= 30_000 {
            self.write_current();
        }
    }
    fn start(
        &mut self,
        number: String,
        topic: String,
        mt: i32,
        st: i32,
        me: Option<User>,
    ) -> Option<Value> {
        if self.meeting.is_some() {
            return None;
        }
        let resumed = self.resume.take().filter(|r| {
            r.meeting_number == number && now().saturating_sub(r.last_seen_at) <= RESUME_WINDOW_MS
        });
        let started = resumed.as_ref().map(|r| r.started_at).unwrap_or_else(now);
        let id = resumed
            .as_ref()
            .map(|r| r.instance_id.clone())
            .unwrap_or_else(|| format!("pz-{number}-{started}"));
        let m = Meeting {
            instance_id: id.clone(),
            meeting_number: number.clone(),
            topic,
            meeting_type: mt,
            status: st,
            status_name: status::meeting_status_name(st),
            started_at: started,
            is_host: me.as_ref().is_some_and(|u| u.host),
            myself: me,
        };
        self.meeting = Some(m.clone());
        self.seq = resumed.as_ref().map(|r| r.last_seq).unwrap_or(0);
        self.prepare_log(&m);
        if resumed.is_some() {
            self.load_history(&m.instance_id);
        }
        let kind = if resumed.is_some() {
            "instance_resumed"
        } else {
            "instance_started"
        };
        self.emit(kind, json!({"meeting":m}))
    }
    /// Starts an instance and records the current participant list as a `sync_all`
    /// seed: the SDK's own SYNC_ALL usually arrives before INMEETING (no instance yet).
    fn start_with_snapshot(
        &mut self,
        number: String,
        topic: String,
        mt: i32,
        st: i32,
        me: Option<User>,
    ) -> Vec<Value> {
        let mut out: Vec<Value> = self.start(number, topic, mt, st, me).into_iter().collect();
        if !out.is_empty() {
            let users = self.users.clone();
            out.extend(self.emit(
                "users",
                json!({"action":"sync_all","users":users,"source":"snapshot"}),
            ));
        }
        out
    }
    fn prepare_log(&mut self, m: &Meeting) {
        if let Some(root) = self.root.as_ref() {
            let _ = fs::create_dir_all(root);
            let p = root.join(format!("{}.jsonl", m.instance_id));
            self.file = OpenOptions::new().create(true).append(true).open(p).ok();
        }
    }
    fn load_history(&mut self, id: &str) {
        let Some(root) = &self.root else { return };
        let Ok(contents) = fs::read_to_string(root.join(format!("{id}.jsonl"))) else {
            return;
        };
        for line in contents
            .lines()
            .rev()
            .take(10000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if v["instanceId"] != id {
                continue;
            }
            let (Some(seq), Some(at), Some(kind)) =
                (v["seq"].as_u64(), v["at"].as_u64(), v["kind"].as_str())
            else {
                continue;
            };
            let mut payload = v.clone();
            if let Some(o) = payload.as_object_mut() {
                for k in ["type", "seq", "at", "instanceId", "kind"] {
                    o.remove(k);
                }
            }
            self.history.push_back(LogEvent {
                seq,
                at,
                instance_id: id.into(),
                kind: kind.into(),
                payload,
            });
        }
    }
    fn end(&mut self, reason: &str) -> Option<Value> {
        let v = self.emit("instance_ended", json!({"reason":reason}));
        // An ended meeting must never be resumed: the next meeting is a new record,
        // even with the same meeting number (owner rule: no continuation after end).
        self.resume = None;
        if let Some(root) = &self.root {
            let _ = fs::remove_file(root.join("current.json"));
        }
        self.meeting = None;
        self.users.clear();
        self.file = None;
        v
    }
    fn apply(
        &mut self,
        e: Event,
        meeting_data: Option<(String, String, i32, i32, Option<User>)>,
    ) -> Vec<Value> {
        let mut out = vec![];
        match e {
            Event::IpcStatus { code, message, .. } => {
                let c = code == status::IPC_STATUS_CONNECTED;
                self.connected = c;
                if let Some(v) = self.emit("ipc", json!({"connected":c,"detail":message})) {
                    out.push(v)
                } else {
                    out.push(json!({"type":"event","seq":self.seq,"at":now(),"instanceId":null,"kind":"ipc","connected":c,"detail":message}));
                }
            }
            Event::MeetingStatus { code, .. } => {
                let name = status::meeting_status_name(code);
                if let Some(v) =
                    self.emit("meeting_status", json!({"status":code,"statusName":name}))
                {
                    out.push(v)
                }
                if code != status::MEETING_STATUS_INMEETING && self.meeting.is_some() {
                    let r = match code {
                        7 => "ended",
                        6 => "failed",
                        _ => "left",
                    };
                    if let Some(v) = self.end(r) {
                        out.push(v)
                    }
                } else if code == status::MEETING_STATUS_INMEETING {
                    if let Some((num, topic, mt, st, me)) = meeting_data {
                        if self
                            .meeting
                            .as_ref()
                            .is_some_and(|m| m.meeting_number != num)
                        {
                            if let Some(v) = self.end("meeting_changed") {
                                out.push(v)
                            }
                        }
                        out.extend(self.start_with_snapshot(num, topic, mt, st, me));
                    }
                }
            }
            Event::UserStatus { action, users, .. } => {
                let us: Vec<_> = users.iter().map(user).collect();
                let mut self_change = None;
                match action {
                    1 => self.users = us.clone(),
                    2 => {
                        for u in &us {
                            if let Some(old) =
                                self.users.iter_mut().find(|x| x.user_id == u.user_id)
                            {
                                *old = u.clone()
                            } else {
                                self.users.push(u.clone())
                            }
                        }
                    }
                    3 => {
                        let ids: Vec<_> = us.iter().map(|u| u.user_id).collect();
                        self.users.retain(|u| !ids.contains(&u.user_id));
                    }
                    4 => {
                        for u in &us {
                            if let Some(old) =
                                self.users.iter_mut().find(|x| x.user_id == u.user_id)
                            {
                                *old = u.clone()
                            } else {
                                self.users.push(u.clone())
                            }
                            if u.myself {
                                if let Some(m) = self.meeting.as_mut() {
                                    if m.myself.as_ref() != Some(u) || m.is_host != u.host {
                                        m.myself = Some(u.clone());
                                        m.is_host = u.host;
                                        self_change = Some(u.clone());
                                    }
                                }
                            }
                        }
                    }
                    _ => return out,
                };
                let a = match action {
                    1 => "sync_all",
                    2 => "add",
                    3 => "left",
                    4 => "update",
                    _ => return out,
                };
                if let Some(v) = self.emit("users", json!({"action":a,"users":us})) {
                    out.push(v)
                }
                if let Some(me) = self_change {
                    if let Some(v) =
                        self.emit("self_changed", json!({"myself":me,"isHost":me.host}))
                    {
                        out.push(v)
                    }
                }
            }
            Event::Other { name, value, .. }
                if name == "OnUserRaisedHand" || name == "OnUserLowerHand" =>
            {
                let id = value as u32;
                let raised = name == "OnUserRaisedHand";
                if let Some(user) = self.users.iter_mut().find(|user| user.user_id == id) {
                    user.raised_hand = raised;
                }
                if let Some(v) = self.emit(
                    "hand",
                    json!({"userId":id,"raised":raised}),
                ) {
                    out.push(v)
                }
            }
            Event::Other { name, .. } if name == "OnAllHandsLowered" => {
                if let Some(v) = self.emit("all_hands_lowered", json!({})) {
                    out.push(v)
                }
            }
            _ => {}
        }
        out
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn meetings_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("zoom-companion")
        .join("meetings")
}
fn origin_allowed(origin: Option<&str>, port: u16, allowed: &[String]) -> bool {
    let Some(o) = origin else { return false };
    o == format!("http://127.0.0.1:{port}")
        || o == format!("http://localhost:{port}")
        || allowed.iter().any(|a| a == o)
}
fn replay(t: &Tracker, instance: Option<&str>, since: Option<u64>) -> Vec<Value> {
    let current = t.meeting.as_ref().map(|m| m.instance_id.as_str());
    let same = current == instance;
    t.history
        .iter()
        .filter(|e| {
            Some(e.instance_id.as_str()) == current
                && (!same || since.is_none() || e.seq > since.unwrap())
        })
        .map(LogEvent::wire)
        .collect()
}
#[derive(Clone)]
struct App {
    tx: broadcast::Sender<Value>,
    tracker: Arc<Mutex<Tracker>>,
    session: Arc<Mutex<Option<Session>>>,
    shutdown: Arc<AtomicBool>,
    port: u16,
    allowed: Vec<String>,
    ui: Option<PathBuf>,
    log_dir: PathBuf,
    store: Arc<Store>,
    provider: Arc<dyn RecordProvider>,
    record_updates: Arc<Mutex<std::collections::HashMap<String, u64>>>,
    link: Arc<ZoomLink>,
}
trait BridgeEventSource: Send {
    fn recv_event(&self, timeout: Duration) -> Result<Event, mpsc::RecvTimeoutError>;
}
struct ChannelEventSource(mpsc::Receiver<Event>);
impl BridgeEventSource for ChannelEventSource {
    fn recv_event(&self, timeout: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.0.recv_timeout(timeout)
    }
}
async fn root(State(a): State<App>, uri: Uri) -> Response {
    if let Some(dir) = &a.ui {
        let path = uri.path().trim_start_matches('/');
        let safe = std::path::Path::new(path)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
        let file = if path.is_empty() {
            dir.join("index.html")
        } else if safe {
            dir.join(path)
        } else {
            dir.join("__invalid_path__")
        };
        if let Ok(bytes) = fs::read(&file) {
            let mime = mime_guess::from_path(&file)
                .first_or_octet_stream()
                .to_string();
            let mut r = (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, mime)],
                bytes,
            )
                .into_response();
            if file.file_name().is_some_and(|x| x == "index.html") {
                r.headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
            }
            return r;
        }
        if let Ok(bytes) = fs::read(dir.join("index.html")) {
            let mut r = (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
                bytes,
            )
                .into_response();
            r.headers_mut()
                .insert("cache-control", "no-store".parse().unwrap());
            return r;
        }
    }
    let s = a
        .tracker
        .lock()
        .unwrap()
        .state()
        .to_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    Html(format!("<!doctype html><meta charset=utf-8><title>zoom-companion</title><h1>zoom-companion</h1><pre id=s>{}</pre><script>let s=document.querySelector('#s');let w=new WebSocket('ws://127.0.0.1:{}/bridge');w.onopen=()=>w.send(JSON.stringify({{type:'subscribe'}}));w.onmessage=e=>s.textContent+='\\n'+e.data</script>",s,a.port)).into_response()
}
async fn ws(State(a): State<App>, headers: HeaderMap, upgrade: WebSocketUpgrade) -> Response {
    let o = headers.get("origin").and_then(|v| v.to_str().ok());
    if !origin_allowed(o, a.port, &a.allowed) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade.on_upgrade(move |socket| client(socket, a))
}
async fn client(socket: WebSocket, a: App) {
    let (mut tx, mut rx) = socket.split();
    let hello = {
        let t = a.tracker.lock().unwrap();
        json!({"type":"hello","protocol":1,"companionVersion":env!("CARGO_PKG_VERSION"),"connected":t.connected,"instanceId":t.meeting.as_ref().map(|m|m.instance_id.clone()),"seqHead":t.seq})
    };
    if tx.send(Message::Text(hello.to_string())).await.is_err() {
        return;
    }
    let mut events = a.tx.subscribe();
    let mut subscribed = false;
    let mut cursor: Option<(String, u64)> = None;
    loop {
        tokio::select! {
            msg=rx.next()=>{
                let Some(Ok(Message::Text(text)))=msg else{break};
                let Ok(v)=serde_json::from_str::<Value>(&text)else{continue};
                match v["type"].as_str(){
                    Some("subscribe")=>{
                        let snapshot={let t=a.tracker.lock().unwrap();(t.state(),replay(&t,v["instanceId"].as_str(),v["sinceSeq"].as_u64()))};
                        let meeting_id=snapshot.0["meeting"]["instanceId"].as_str().map(str::to_owned);
                        let seq_head=snapshot.0["seqHead"].as_u64().unwrap_or(0);
                        cursor=meeting_id.map(|id|(id,seq_head));
                        subscribed=true;
                        let _=tx.send(Message::Text(snapshot.0.to_string())).await;
                        let zoom=a.link.status.lock().unwrap().wire();
                        let _=tx.send(Message::Text(zoom.to_string())).await;
                        for e in snapshot.1{if tx.send(Message::Text(e.to_string())).await.is_err(){return}}
                    },
                    Some("request")=>{
                        let id=v["id"].clone();let method=v["method"].as_str().unwrap_or("").to_owned();let params=v["params"].clone();let app=a.clone();
                        let result=tokio::task::spawn_blocking(move||request(&app,&method,&params)).await.unwrap_or_else(|e|Err(("internal",e.to_string())));
                        let r=match result{Ok(x)=>json!({"type":"reply","id":id,"ok":true,"result":x}),Err((c,m))=>json!({"type":"reply","id":id,"ok":false,"error":{"code":c,"message":m}})};
                        let _=tx.send(Message::Text(r.to_string())).await;
                    },_=>{}
                }
            },
            ev=events.recv()=>if let Ok(v)=ev{
                if !subscribed {continue}
                if v["type"]=="event" {
                    if let (Some(id),Some(seq))=(v["instanceId"].as_str(),v["seq"].as_u64()) {
                        if cursor.as_ref().is_some_and(|(old,last)|old==id&&seq<=*last){continue}
                        cursor=Some((id.to_owned(),seq));
                    }
                }
                if tx.send(Message::Text(v.to_string())).await.is_err(){break}
            }
        }
    }
}
fn request(a: &App, method: &str, p: &Value) -> Result<Value, (&'static str, String)> {
    if method == "getState" {
        return Ok(a.tracker.lock().unwrap().state());
    }
    let _m = a
        .tracker
        .lock()
        .unwrap()
        .meeting
        .clone()
        .ok_or(("not_in_meeting", "No active meeting".into()))?;
    let id = || {
        p["userId"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(("bad_params", "userId must be a u32".into()))
    };
    let guard = a.session.lock().unwrap();
    let s = guard
        .as_ref()
        .ok_or(("internal", "SDK session is reconnecting".into()))?;
    let res = match method {
        "getState" => return Ok(a.tracker.lock().unwrap().state()),
        "getParticipants" => s
            .participants(MeetingInstance::Default)
            .map(|u| json!(u.iter().map(user).collect::<Vec<_>>())),
        "getWaitingRoom" => s
            .waiting_room(MeetingInstance::Default)
            .or_else(|_| Ok(Vec::new()))
            .map(|u| json!(u.iter().map(user).collect::<Vec<_>>())),
        "admit" => s
            .admit(MeetingInstance::Default, id()?)
            .map(|_| json!({"submitted":true})),
        "admitAll" => s
            .admit_all(MeetingInstance::Default)
            .map(|_| json!({"submitted":true})),
        "putIntoWaitingRoom" => s
            .put_into_waiting_room(MeetingInstance::Default, id()?)
            .map(|_| json!({"submitted":true})),
        "expel" => s
            .expel(MeetingInstance::Default, id()?)
            .map(|_| json!({"submitted":true})),
        _ => return Err(("not_supported", "Unknown method".into())),
    };
    res.map_err(|e| ("sdk_rejected", e.to_string()))
}

pub fn run(
    port: u16,
    ui: Option<PathBuf>,
    open: bool,
    allowed: Vec<String>,
    fake: bool,
) -> anyhow::Result<()> {
    let ui = ui.or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("ui")))
    });
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let (tx, _) = broadcast::channel(1024);
        let data_dir = if fake {
            std::env::temp_dir().join(format!("zoom-companion-fake-{}", std::process::id()))
        } else {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("zoom-companion")
        };
        let log_dir = if fake {
            data_dir.join("meetings")
        } else {
            meetings_dir()
        };
        let app = App {
            tx: tx.clone(),
            tracker: Arc::new(Mutex::new(Tracker::new(Some(log_dir.clone())))),
            session: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
            port,
            allowed,
            ui,
            store: Arc::new(Store::open(data_dir.join("data.db"))?),
            provider: Arc::new(EngineRecordProvider),
            record_updates: Arc::new(Mutex::new(std::collections::HashMap::new())),
            link: Default::default(),
            log_dir,
        };
        let (profile_revision, profiles) = app.store.get_doc("profiles")?;
        if !profiles.as_array().is_some_and(|rows| rows.iter().any(|p| p["id"] == "default-all")) {
            let mut rows=profiles.as_array().cloned().unwrap_or_default();
            rows.push(json!({"id":"default-all","name":"All participants","description":"Factory default profile","includedSubgroups":[],"lateAfterMinutes":10,"returnWindowMinutes":8,"meetingIds":[],"topicKeywords":[],"groupSortOrder":[],"active":true,"isDefault":true}));
            let _=app.store.put_doc("profiles",profile_revision,&json!(rows),now())?;
        }
        import_logs(&app, &app.log_dir);
        // A resume point is only valid for a meeting whose end was never recorded and
        // that was seen within the resume window; that one stays open for resumption.
        let keep = {
            let mut t = app.tracker.lock().unwrap();
            let ended = t.resume.as_ref().is_some_and(|r| {
                app.store
                    .meeting(&r.instance_id)
                    .ok()
                    .flatten()
                    .is_some_and(|m| !m["endedAt"].is_null())
            });
            let expired = t
                .resume
                .as_ref()
                .is_some_and(|r| now().saturating_sub(r.last_seen_at) > RESUME_WINDOW_MS);
            if ended || expired {
                t.resume = None;
            }
            t.resume.as_ref().map(|r| r.instance_id.clone())
        };
        // Every other still-open record belongs to a process that ended before this one.
        app.store.close_open_meetings_except(now(), keep.as_deref())?;
        if fake {
            seed_fake_meeting(&app);
        }
        // Bind first: if another instance already owns the port this process must not
        // start a second SDK session (one Plugin SDK client per Zoom Workplace).
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("port {port} is in use (is AttendFlow Desktop already running?)"))?;
        let worker = app.clone();
        let stop = app.shutdown.clone();
        let worker_task = if fake {
            set_zoom(&app, ZoomStatus::new(ZoomState::Connected, "fake meeting"));
            None
        } else {
            Some(tokio::task::spawn_blocking(move || sdk_loop(worker)))
        };
        let url = format!("http://127.0.0.1:{port}/");
        if open {
            open_browser(&url)?;
        }
        let router = Router::new()
            .route("/bridge", get(ws))
            .route("/api/config/:key", get(api_get_config).put(api_put_config))
            .route("/api/names/pinyin", post(api_name_pinyin))
            .route(
                "/api/config/:key/import/preview",
                post(api_config_import_preview),
            )
            .route(
                "/api/config/:key/import/commit",
                post(api_config_import_commit),
            )
            .route("/api/config/:key/export", get(api_config_export))
            .route("/api/profiles/import/preview", post(api_profiles_import_preview))
            .route("/api/profiles/import/commit", post(api_profiles_import_commit))
            .route("/api/config/example-data", post(api_example_data_commit))
            .route("/api/meetings", get(api_meetings))
            .route("/api/zoom", get(api_zoom_status))
            .route("/api/zoom/sign-in", post(api_zoom_sign_in))
            .route("/api/zoom/sign-out", post(api_zoom_sign_out))
            .route("/api/records/export", get(api_export_records))
            .route("/api/records/import/preview", post(api_records_import_preview))
            .route("/api/records/import/commit", post(api_records_import_commit))
            .route("/api/records/delete", post(api_delete_records))
            .route(
                "/api/meetings/:id",
                get(api_meeting).delete(api_delete_meeting),
            )
            .route("/api/meetings/:id/events", get(api_events))
            .route("/api/meetings/:id/record", get(api_record))
            .route("/api/meetings/:id/export", get(api_record_export))
            .route(
                "/api/meetings/:id/state",
                get(api_get_state).put(api_put_state),
            )
            .route("/api/meetings/:id/reconciliation", post(api_reconciliation_decision))
            .route("/api/import/attendflow-backup", post(api_import_backup))
            .route(
                "/api/backup",
                get(api_export_backup).post(api_restore_backup),
            )
            .route(
                "/api/local-data",
                axum::routing::delete(api_clear_local_data),
            )
            .fallback(get(root))
            .layer(axum::middleware::from_fn_with_state(app.clone(), host_guard))
            .with_state(app);
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                stop.store(true, Ordering::SeqCst);
            })
            .await?;
        if let Some(worker_task) = worker_task {
            let _ = worker_task.await;
        }
        Ok::<_, anyhow::Error>(())
    })?;
    Ok(())
}

pub fn app() -> anyhow::Result<()> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
    };
    let exe = std::env::current_exe()?;
    let ui = exe
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("ui");
    let url = "http://127.0.0.1:47812/".to_owned();
    std::thread::spawn(move || {
        if let Err(e) = run(47812, Some(ui), false, vec![], false) {
            log::error!("app server stopped: {e:#}");
        }
    });
    for _ in 0..100 {
        if std::net::TcpStream::connect("127.0.0.1:47812").is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let event_loop = EventLoopBuilder::<String>::with_user_event().build();
    let title_proxy = event_loop.create_proxy();
    // Icon resource 1 is embedded by build.rs (packaging/assets/app.ico).
    let icon = {
        use tao::platform::windows::IconExtWindows;
        tao::window::Icon::from_resource(1, None).ok()
    };
    let window = WindowBuilder::new()
        .with_title("AttendFlow Desktop")
        .with_window_icon(icon)
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 820.0))
        .build(&event_loop)?;
    // WebView2 profile in LOCALAPPDATA: the default (next to the exe) is read-only
    // when installed under Program Files or as an MSIX package.
    let mut web_context = wry::WebContext::new(
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|d| d.join("zoom-companion").join("webview2")),
    );
    let _webview = wry::WebViewBuilder::new_with_web_context(&mut web_context)
        .with_url(&url)
        .with_document_title_changed_handler(move |title| {
            let _ = title_proxy.send_event(title);
        })
        .with_download_started_handler(|download_url, suggested_path| {
            if !download_url.contains("/api/config/roster/export")
                && !download_url.contains("/api/config/aliases/export")
            {
                return true;
            }
            let name = suggested_path
                .file_name()
                .map(|x| x.to_string_lossy().into_owned())
                .unwrap_or_else(|| "attendflow-export.csv".into());
            match rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name(name)
                .save_file()
            {
                Some(path) => {
                    *suggested_path = path;
                    true
                }
                None => false,
            }
        })
        .with_download_completed_handler(|url, path, success| {
            if url.contains("/api/config/") && url.contains("/export") {
                if success {
                    log::info!(
                        "configuration CSV saved to {}",
                        path.map(|p| p.display().to_string()).unwrap_or_default()
                    );
                } else {
                    log::warn!("configuration CSV download failed");
                }
            }
        })
        .build(&window)?;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(title) => window.set_title(&title),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
fn publish(a: &App, events: Vec<Value>) {
    let changed = events.iter().any(|e| {
        e["kind"] == "instance_started"
            || e["kind"] == "instance_resumed"
            || e["kind"] == "instance_ended"
    });
    let mut changed_id: Option<String> = None;
    for e in events {
        if e["type"] == "event" {
            if let Some(id) = e["instanceId"].as_str() {
                changed_id = Some(id.to_owned());
                let kind = e["kind"].as_str().unwrap_or("unknown");
                let mut payload = e.clone();
                if let Some(o) = payload.as_object_mut() {
                    for k in ["type", "seq", "at", "instanceId", "kind"] {
                        o.remove(k);
                    }
                }
                if matches!(kind, "instance_started" | "instance_resumed") {
                    if let Err(err) =
                        a.store
                            .upsert_meeting(&e["meeting"], None, e["seq"].as_u64().unwrap_or(0))
                    {
                        log::error!("persist meeting: {err}");
                    }
                    if kind == "instance_started" {
                        if let Err(err) = a.store.close_open_meetings_except(now(), Some(id)) {
                            log::error!("close previous meetings: {err}");
                        }
                    }
                } else if let Ok(Some(mut meta)) = a.store.meeting(id) {
                    if kind == "self_changed" {
                        meta["isHost"] = e["isHost"].clone();
                        meta["myself"] = e["myself"].clone();
                    }
                    let end = if kind == "instance_ended" {
                        Some((
                            e["at"].as_u64().unwrap_or_else(now),
                            e["reason"].as_str().unwrap_or("ended").to_owned(),
                        ))
                    } else {
                        None
                    };
                    if let Err(err) =
                        a.store
                            .upsert_meeting(&meta, end, e["seq"].as_u64().unwrap_or(0))
                    {
                        log::error!("update meeting: {err}");
                    }
                }
                if let Err(err) = a.store.append_event(
                    id,
                    e["seq"].as_u64().unwrap_or(0),
                    e["at"].as_u64().unwrap_or_else(now),
                    kind,
                    &payload,
                ) {
                    log::error!("persist event: {err}");
                }
            }
        }
        let _ = a.tx.send(e);
    }
    if let Some(id) = changed_id {
        emit_record_updated(a, &id);
    }
    if changed {
        let state = a.tracker.lock().unwrap().state();
        let _ = a.tx.send(state);
    }
}
fn meeting_data(a: &App) -> Option<(String, String, i32, i32, Option<User>)> {
    let g = a.session.lock().unwrap();
    let s = g.as_ref()?;
    if s.meeting_status(MeetingInstance::Default).ok() != Some(status::MEETING_STATUS_INMEETING) {
        return None;
    }
    let i = s.meeting_info(MeetingInstance::Default).ok()?;
    if i.meeting_number.is_empty() {
        return None;
    }
    let me = s.myself(MeetingInstance::Default).ok().map(|p| user(&p));
    Some((
        i.meeting_number,
        i.topic,
        i.meeting_type,
        status::MEETING_STATUS_INMEETING,
        me,
    ))
}
fn refresh_users(a: &App) {
    let g = a.session.lock().unwrap();
    let Some(s) = g.as_ref() else { return };
    if let Ok(mut ps) = s.participants(MeetingInstance::Default) {
        // GetParticipantsList already includes waiting-room users (flagged) for the host;
        // merge the waiting-room list without duplicating ids.
        for w in s.waiting_room(MeetingInstance::Default).unwrap_or_default() {
            if !ps.iter().any(|p| p.user_id == w.user_id) {
                ps.push(w);
            }
        }
        a.tracker.lock().unwrap().users = ps.iter().map(user).collect();
    }
}
fn sdk_loop(a: App) {
    while !a.shutdown.load(Ordering::SeqCst) {
        a.link.sign_out.store(false, Ordering::SeqCst);
        let mut pause_secs = 5;
        match start_session() {
            Ok((session, rx)) => {
                set_zoom(&a, ZoomStatus::new(ZoomState::Connecting, ""));
                let rx = ChannelEventSource(rx);
                *a.session.lock().unwrap() = Some(session);
                let until = std::time::Instant::now() + Duration::from_secs(30);
                let (mut auth, mut ipc, mut retry) = (false, false, false);
                while std::time::Instant::now() < until
                    && !(auth && ipc)
                    && !retry
                    && !a.shutdown.load(Ordering::SeqCst)
                    && !a.link.sign_out.load(Ordering::SeqCst)
                {
                    if let Ok(e) = rx.recv_event(Duration::from_millis(200)) {
                        match &e {
                            Event::AuthResult { code, .. } => {
                                auth = *code == status::AUTH_RESULT_SUCCESS;
                                if !auth {
                                    retry = true;
                                    // VERIFY_USER_FAILED: Zoom Workplace is signed in as a different user.
                                    let reason = if *code == 2 { "account_mismatch" } else { "auth_failed" };
                                    pause_secs = 15;
                                    // A token that is unexpired but revoked (app removed on the Zoom
                                    // website) also fails here: if Zoom refuses a refresh, the grant is
                                    // gone and the user has to sign in again.
                                    let grant_gone = reason == "auth_failed"
                                        && matches!(
                                            zoom_auth::TokenStore::open_default()
                                                .and_then(|s| zoom_auth::refresh_now(&s)),
                                            Err(zoom_auth::AuthError::NotSignedIn)
                                                | Err(zoom_auth::AuthError::TokenEndpoint { status: 400..=499, .. })
                                        );
                                    if grant_gone {
                                        if let Ok(store) = zoom_auth::TokenStore::open_default() {
                                            let _ = store.clear();
                                        }
                                        set_zoom(
                                            &a,
                                            ZoomStatus::new(ZoomState::SignedOut, status::auth_result_name(*code))
                                                .with_reason("session_expired"),
                                        );
                                        pause_secs = 2;
                                    } else {
                                        set_zoom(
                                            &a,
                                            ZoomStatus::new(ZoomState::Error, status::auth_result_name(*code))
                                                .with_reason(reason),
                                        );
                                    }
                                }
                            }
                            Event::IpcStatus { code, .. } => {
                                ipc = *code == status::IPC_STATUS_CONNECTED;
                                if *code == 1 || *code == 3 {
                                    retry = true;
                                    // CONNECT_TIMEOUT is also what Zoom Workplace answers when it is
                                    // signed out or signed in as a different user than the token's.
                                    set_zoom(
                                        &a,
                                        ZoomStatus::new(ZoomState::Error, status::ipc_status_name(*code))
                                            .with_reason("ipc_timeout"),
                                    );
                                    pause_secs = 15;
                                }
                            }
                            _ => {}
                        }
                        let out = a.tracker.lock().unwrap().apply(e, None);
                        publish(&a, out);
                    }
                }
                if !(auth && ipc) && !retry && !a.link.sign_out.load(Ordering::SeqCst) {
                    // Neither success nor an explicit failure within 30 s.
                    let st = if crate::zoom_workplace_running() {
                        ZoomStatus::new(ZoomState::Error, "no response from Zoom Workplace").with_reason("ipc_timeout")
                    } else {
                        ZoomStatus::new(ZoomState::ZoomNotRunning, "")
                    };
                    set_zoom(&a, st);
                }
                if auth && ipc && !retry && !a.link.sign_out.load(Ordering::SeqCst) {
                    set_zoom(&a, ZoomStatus::new(ZoomState::Connected, ""));
                    // The SDK toolkits remain null just after IPC connect; let the SYNC_ALL/status burst arrive first.
                    let burst = std::time::Instant::now() + Duration::from_secs(5);
                    while std::time::Instant::now() < burst && !a.shutdown.load(Ordering::SeqCst) {
                        if let Ok(e) = rx.recv_event(Duration::from_millis(100)) {
                            let data = if matches!(e,Event::MeetingStatus{code,..} if code==status::MEETING_STATUS_INMEETING)
                            {
                                meeting_data(&a)
                            } else {
                                None
                            };
                            if data.is_some() {
                                refresh_users(&a);
                            }
                            let out = a.tracker.lock().unwrap().apply(e, data);
                            publish(&a, out);
                        }
                    }
                    if let Some(data) = meeting_data(&a) {
                        refresh_users(&a);
                        let mut t = a.tracker.lock().unwrap();
                        if t.meeting.is_none() {
                            let out = t.start_with_snapshot(data.0, data.1, data.2, data.3, data.4);
                            drop(t);
                            publish(&a, out);
                        }
                    }
                    // A meeting can begin (or the toolkits become ready) after connect:
                    // INMEETING / user activity arms a start attempt, retried once per second.
                    let mut want_start = false;
                    let mut last_try = std::time::Instant::now() - Duration::from_secs(5);
                    loop {
                        if a.shutdown.load(Ordering::SeqCst) || a.link.sign_out.load(Ordering::SeqCst) {
                            break;
                        }
                        let no_meeting = {
                            let mut t = a.tracker.lock().unwrap();
                            t.touch();
                            t.meeting.is_none()
                        };
                        if no_meeting && want_start && last_try.elapsed() >= Duration::from_secs(1)
                        {
                            last_try = std::time::Instant::now();
                            if let Some(d) = meeting_data(&a) {
                                refresh_users(&a);
                                let out = a
                                    .tracker
                                    .lock()
                                    .unwrap()
                                    .start_with_snapshot(d.0, d.1, d.2, d.3, d.4);
                                publish(&a, out);
                                want_start = false;
                            }
                        }
                        match rx.recv_event(Duration::from_millis(250)) {
                            Ok(e) => {
                                let lost = matches!(&e,Event::IpcStatus{code,..} if *code!=status::IPC_STATUS_CONNECTED);
                                match &e {
                                    Event::MeetingStatus { code, .. } => {
                                        want_start = *code == status::MEETING_STATUS_INMEETING;
                                    }
                                    Event::UserStatus { .. } if no_meeting => want_start = true,
                                    _ => {}
                                }
                                let out = a.tracker.lock().unwrap().apply(e, None);
                                publish(&a, out);
                                if lost {
                                    set_zoom(&a, ZoomStatus::new(ZoomState::Connecting, "connection to Zoom Workplace lost"));
                                    pause_secs = 2;
                                    break;
                                }
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                            Err(_) => {
                                break;
                            }
                        }
                    }
                }
                if let Some(s) = a.session.lock().unwrap().take() {
                    let _ = s.shutdown();
                }
            }
            Err(e) => {
                // Logged at info level by set_zoom when the status changes; retries repeat every few seconds.
                log::debug!("serve session unavailable: {e:#}");
                let (st, clear_tokens) = classify_start_error(&e);
                if clear_tokens {
                    if let Ok(store) = zoom_auth::TokenStore::open_default() {
                        let _ = store.clear();
                    }
                }
                let signing_in = a.link.login_pending.load(Ordering::SeqCst) > 0;
                if !(st.state == ZoomState::SignedOut && signing_in) {
                    if st.state == ZoomState::Error {
                        pause_secs = 15;
                    }
                    set_zoom(&a, st);
                }
                let event = Event::IpcStatus {
                    code: 1,
                    message: e.to_string(),
                    thread_id: 0,
                };
                let out = a.tracker.lock().unwrap().apply(event, None);
                publish(&a, out);
            }
        }
        for _ in 0..pause_secs * 10 {
            if a.shutdown.load(Ordering::SeqCst) || a.link.wake.swap(false, Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if let Some(s) = a.session.lock().unwrap().take() {
        let _ = s.shutdown();
    }
}

/// DNS-rebinding guard: a page on another domain that resolves to 127.0.0.1 is
/// same-origin with itself, so browsers send no Origin on its GETs; its Host
/// header still names the attacker's domain. Requests without a Host header
/// (non-browser clients, tests) pass; `--allow-origin` hosts (the Vite dev proxy
/// keeps `Host: localhost:5173`) are allowed too.
fn host_allowed(host: Option<&str>, port: u16, allowed: &[String]) -> bool {
    match host {
        None => true,
        Some(h) => {
            h == format!("127.0.0.1:{port}")
                || h == format!("localhost:{port}")
                || allowed.iter().any(|o| o.split_once("://").map(|(_, rest)| rest) == Some(h))
        }
    }
}

async fn host_guard(
    State(a): State<App>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let host = req.headers().get("host").and_then(|v| v.to_str().ok());
    if !host_allowed(host, a.port, &a.allowed) {
        return StatusCode::MISDIRECTED_REQUEST.into_response();
    }
    next.run(req).await
}

/// State-changing Zoom endpoints require a same-app Origin (browsers always send
/// one on POST), so another website cannot sign the user in or out.
fn strict_origin(a: &App, h: &HeaderMap) -> Option<Response> {
    let o = h.get("origin").and_then(|v| v.to_str().ok());
    (!origin_allowed(o, a.port, &a.allowed)).then(|| StatusCode::FORBIDDEN.into_response())
}

async fn api_zoom_status(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    Json(a.link.status.lock().unwrap().wire()).into_response()
}

/// Starts the browser sign-in (PKCE, loopback on an ephemeral port) and returns at
/// once; progress arrives as `zoom_status` messages. One sign-in waits for its
/// redirect at a time: clicking again while it is pending reopens the same
/// authorize URL.
async fn api_zoom_sign_in(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = strict_origin(&a, &headers) {
        return r;
    }
    let cfg = match Config::load() {
        Ok(c) if !c.client_id.is_empty() => c,
        Ok(_) => return (StatusCode::CONFLICT, Json(json!({"error":"not_configured"}))).into_response(),
        Err(e) => return api_error(e),
    };
    let pending = {
        let mut url = a.link.login_url.lock().unwrap();
        if let Some(u) = url.as_ref() {
            let _ = open_browser(u);
            set_zoom(&a, ZoomStatus::new(ZoomState::SigningIn, ""));
            return (StatusCode::ACCEPTED, Json(json!({"started":true,"reopened":true}))).into_response();
        }
        match zoom_auth::begin_login(&cfg.oauth()) {
            Ok(p) => {
                *url = Some(p.authorize_url().to_owned());
                p
            }
            Err(e) => {
                log::warn!("sign-in could not start: {e}");
                set_zoom(&a, ZoomStatus::new(ZoomState::SignedOut, e.to_string()).with_reason("sign_in_failed"));
                return (StatusCode::CONFLICT, Json(json!({"error":e.to_string()}))).into_response();
            }
        }
    };
    a.link.login_pending.fetch_add(1, Ordering::SeqCst);
    set_zoom(&a, ZoomStatus::new(ZoomState::SigningIn, ""));
    log::info!("opening browser for Zoom authorization (client_id={}, redirect {})", cfg.client_id, pending.redirect_uri());
    if let Err(e) = open_browser(pending.authorize_url()) {
        log::warn!("{e}");
    }
    std::thread::spawn(move || {
        let result = zoom_auth::TokenStore::open_default()
            .and_then(|store| pending.finish(&store, Duration::from_secs(300)));
        *a.link.login_url.lock().unwrap() = None;
        a.link.login_pending.fetch_sub(1, Ordering::SeqCst);
        match result {
            Ok(t) if !t.has_scope(zoom_auth::PLUGIN_SDK_SCOPE) => set_zoom(
                &a,
                ZoomStatus::new(ZoomState::Error, format!("granted scopes: {}", t.scope)).with_reason("missing_scope"),
            ),
            Ok(_) => {
                set_zoom(&a, ZoomStatus::new(ZoomState::Connecting, ""));
                a.link.wake.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                log::warn!("sign-in failed: {e}");
                let reason = match e {
                    zoom_auth::AuthError::Denied(_) => "sign_in_denied",
                    zoom_auth::AuthError::Loopback(_) => "sign_in_timeout",
                    _ => "sign_in_failed",
                };
                set_zoom(&a, ZoomStatus::new(ZoomState::SignedOut, e.to_string()).with_reason(reason));
            }
        }
    });
    (StatusCode::ACCEPTED, Json(json!({"started":true}))).into_response()
}

/// Revokes the grant at Zoom (removing the app from the user's Zoom "Added apps"),
/// deletes the local tokens and drops the SDK session. Attendance data is kept.
async fn api_zoom_sign_out(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = strict_origin(&a, &headers) {
        return r;
    }
    a.link.sign_out.store(true, Ordering::SeqCst);
    let res = tokio::task::spawn_blocking(|| {
        zoom_auth::TokenStore::open_default().and_then(|store| zoom_auth::sign_out(&store))
    })
    .await;
    set_zoom(&a, ZoomStatus::new(ZoomState::SignedOut, ""));
    a.link.wake.store(true, Ordering::SeqCst);
    match res {
        Ok(Ok(revoked)) => Json(json!({"revoked":revoked})).into_response(),
        Ok(Err(e)) => api_error(e),
        Err(e) => api_error(e),
    }
}

/// Updates the Zoom connection status and pushes it to the UI when it changed.
fn set_zoom(a: &App, st: ZoomStatus) {
    let mut cur = a.link.status.lock().unwrap();
    if *cur != st {
        log::info!("zoom status: {} {} {}", st.state.as_str(), st.reason, st.detail);
        let _ = a.tx.send(st.wire());
        *cur = st;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FakeBridgeEventSource(mpsc::Receiver<Event>);
    impl BridgeEventSource for FakeBridgeEventSource {
        fn recv_event(&self, timeout: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
            self.0.recv_timeout(timeout)
        }
    }
    #[test]
    fn host_guard_blocks_rebinding() {
        assert!(host_allowed(Some("127.0.0.1:47812"), 47812, &[]));
        assert!(host_allowed(Some("localhost:47812"), 47812, &[]));
        assert!(host_allowed(None, 47812, &[]));
        assert!(!host_allowed(Some("evil.test:47812"), 47812, &[]));
        assert!(!host_allowed(Some("127.0.0.1:1"), 47812, &[]));
        let dev = ["http://localhost:5173".to_string()];
        assert!(host_allowed(Some("localhost:5173"), 47812, &dev));
        assert!(!host_allowed(Some("localhost:5174"), 47812, &dev));
    }
    #[test]
    fn origin_whitelist() {
        assert!(origin_allowed(Some("http://localhost:47812"), 47812, &[]));
        assert!(!origin_allowed(Some("https://evil.test"), 47812, &[]));
        assert!(origin_allowed(
            Some("http://localhost:5173"),
            47812,
            &["http://localhost:5173".into()]
        ));
    }
    // Ported cases from AttendFlow roster-import.test.ts and alias-import.test.ts:
    // BOM/GB18030 decoding, full-width comma, added rows, invalid nickname targets.
    #[test]
    fn roster_alias_import_csv_decoding_cases() {
        assert_eq!(
            decode_config_csv_bytes("\u{feff}id，姓名\r\n1，张三".as_bytes()),
            "id,姓名\r\n1,张三"
        );
        assert_eq!(
            decode_config_csv_bytes("id,姓名\n1,张三".as_bytes()),
            "id,姓名\n1,张三"
        );
        assert_eq!(
            decode_config_csv_bytes(&[b'i', b'd', b',', 0xd6, 0xd0]),
            "id,中"
        );
        #[cfg(windows)]
        assert_eq!(
            decode_config_csv_bytes(&[b'i', b'd', b',', 0x94, 0x39, 0xfc, 0x36]),
            "id,😀"
        );
        assert_eq!(
            csv_cells("id,name\n1,\"A, B\""),
            vec![vec!["id", "name"], vec!["1", "A, B"]]
        );
    }
    #[test]
    fn replay_filters_sequence_and_instance() {
        let mut t = Tracker::default();
        t.meeting = Some(Meeting {
            instance_id: "a".into(),
            meeting_number: "1".into(),
            topic: "".into(),
            meeting_type: 1,
            status: 3,
            status_name: "INMEETING".into(),
            started_at: 1,
            myself: None,
            is_host: false,
        });
        t.emit("users", json!({"n":1}));
        t.emit("users", json!({"n":2}));
        assert_eq!(
            t.history
                .iter()
                .filter(|e| e.seq > 1 && e.instance_id == "a")
                .count(),
            1
        );
        assert_eq!(t.history.iter().filter(|e| e.instance_id == "b").count(), 0);
    }
    #[test]
    fn jsonl_round_trip() {
        let x = LogEvent {
            seq: 1,
            at: 2,
            instance_id: "i".into(),
            kind: "hand".into(),
            payload: json!({"userId":8,"raised":true}),
        };
        let y: Value = serde_json::from_str(&x.wire().to_string()).unwrap();
        assert_eq!(y["seq"], 1);
        assert_eq!(y["raised"], true);
    }
    fn part(id: u32, name: &str, flags: u32) -> Participant {
        Participant {
            user_id: id,
            name: name.into(),
            flags: zoom_bridge::ParticipantFlags(flags),
            audio_type: 0,
        }
    }
    #[test]
    fn start_end_sequence() {
        let mut t = Tracker::default();
        let started = t.start("123".into(), "topic".into(), 1, 3, None).unwrap();
        assert_eq!(started["seq"], 1);
        let x = t.apply(
            Event::UserStatus {
                action: 2,
                error_code: 1,
                instance: 1,
                users: vec![part(2, "Guest", 0)],
                thread_id: 0,
            },
            None,
        );
        assert_eq!(x[0]["seq"], 2);
        let end = t.end("left").unwrap();
        assert_eq!(end["reason"], "left");
        assert_eq!(end["seq"], 3);
    }
    #[test]
    fn waiting_admission_pair_and_rename() {
        let mut t = Tracker::default();
        t.start("1".into(), "".into(), 1, 3, None);
        let l = t.apply(
            Event::UserStatus {
                action: 3,
                error_code: 1,
                instance: 1,
                users: vec![part(8, "Guest", 1 << 8)],
                thread_id: 0,
            },
            None,
        );
        let a = t.apply(
            Event::UserStatus {
                action: 2,
                error_code: 1,
                instance: 1,
                users: vec![part(9, "Guest", 0)],
                thread_id: 0,
            },
            None,
        );
        assert_eq!(l[0]["action"], "left");
        assert_eq!(l[0]["users"][0]["waitingRoom"], true);
        assert_eq!(a[0]["users"][0]["userId"], 9);
        let r = t.apply(
            Event::UserStatus {
                action: 4,
                error_code: 1,
                instance: 1,
                users: vec![part(9, "Renamed", 0)],
                thread_id: 0,
            },
            None,
        );
        assert_eq!(r[0]["users"][0]["name"], "Renamed");
    }
    #[test]
    fn host_transfer_emits_self_changed() {
        let mut t = Tracker::default();
        let me = User {
            user_id: 1,
            name: "me".into(),
            host: false,
            myself: true,
            waiting_room: false,
            audio_muted: false,
            video_on: false,
            raised_hand: false,
            talking: false,
            spotlighted: false,
        };
        t.start("1".into(), "".into(), 1, 3, Some(me));
        let out = t.apply(
            Event::UserStatus {
                action: 4,
                error_code: 1,
                instance: 1,
                users: vec![part(1, "me", 5)],
                thread_id: 0,
            },
            None,
        );
        assert!(out
            .iter()
            .any(|v| v["kind"] == "self_changed" && v["isHost"] == true));
    }
    #[test]
    fn meeting_change_closes_old_instance() {
        let mut t = Tracker::default();
        t.start("1".into(), "".into(), 1, 3, None);
        let out = t.apply(
            Event::MeetingStatus {
                code: 3,
                thread_id: 0,
            },
            Some(("2".into(), "".into(), 1, 3, None)),
        );
        assert!(out
            .iter()
            .any(|v| v["kind"] == "instance_ended" && v["reason"] == "meeting_changed"));
        assert!(out.iter().any(|v| v["kind"] == "instance_started"));
    }
    #[test]
    fn replay_since_and_mismatch() {
        let mut t = Tracker::default();
        t.start("1".into(), "".into(), 1, 3, None);
        t.emit("users", json!({"n":1}));
        t.emit("users", json!({"n":2}));
        let id = t.meeting.as_ref().unwrap().instance_id.clone();
        assert_eq!(replay(&t, Some(&id), Some(2)).len(), 1);
        assert_eq!(replay(&t, Some("old"), Some(99)).len(), 3);
    }
    #[test]
    fn resumes_same_id_and_sequence() {
        let path = std::env::temp_dir().join(format!("zc-test-{}", now()));
        let mut t = Tracker::new(Some(path.clone()));
        let first = t.start("7".into(), "".into(), 1, 3, None).unwrap();
        let id = first["instanceId"].as_str().unwrap().to_owned();
        t.emit("users", json!({"action":"add"}));
        drop(t);
        let mut r = Tracker::new(Some(path.clone()));
        let next = r.start("7".into(), "".into(), 1, 3, None).unwrap();
        assert_eq!(next["instanceId"], id);
        assert_eq!(next["seq"], 3);
        let _ = fs::remove_dir_all(path);
    }
    #[test]
    fn quiet_live_meeting_heartbeat_keeps_resume_point_fresh() {
        let path = std::env::temp_dir().join(format!("zc-test-touch-{}", now()));
        let mut t = Tracker::new(Some(path.clone()));
        t.start("7".into(), "".into(), 1, 3, None).unwrap();
        // Pretend the last write was long ago; a heartbeat must refresh lastSeenAt.
        t.last_touch = 0;
        let before = now();
        t.touch();
        let r: Resume =
            serde_json::from_slice(&fs::read(path.join("current.json")).unwrap()).unwrap();
        assert!(r.last_seen_at >= before);
        // Throttled: an immediate second touch does not rewrite.
        let stamp = t.last_touch;
        t.touch();
        assert_eq!(t.last_touch, stamp);
        let _ = fs::remove_dir_all(path);
    }
    #[test]
    fn config_writes_must_match_engine_schema() {
        // A string personId once got stored and then broke every meeting record (500).
        let bad = json!([{"personId":"example-1","canonicalName":"张三","pinyin":"zhangsan","group":"A","active":true}]);
        assert!(config_schema_error("roster", &bad).unwrap().contains("invalid roster data"));
        let good = json!([{"personId":1,"canonicalName":"张三","pinyin":"zhangsan","group":"A","active":true}]);
        assert!(config_schema_error("roster", &good).is_none());
        assert!(config_schema_error("aliases", &json!([{"zoomName":"小张","personId":"x"}])).is_some());
        assert!(config_schema_error("settings", &json!({"anything":true})).is_none());
    }
    #[test]
    fn ended_instance_is_never_resumed() {
        let path = std::env::temp_dir().join(format!("zc-test-end-{}", now()));
        let mut t = Tracker::new(Some(path.clone()));
        let first = t.start("7".into(), "".into(), 1, 3, None).unwrap();
        let id = first["instanceId"].as_str().unwrap().to_owned();
        t.emit("users", json!({"action":"add"}));
        t.end("ended");
        drop(t);
        // Same meeting number restarts right away (e.g. host ended then restarted the PMI).
        std::thread::sleep(Duration::from_millis(5));
        let mut r = Tracker::new(Some(path.clone()));
        let next = r.start("7".into(), "".into(), 1, 3, None).unwrap();
        assert_eq!(next["kind"], "instance_started");
        assert_ne!(next["instanceId"], id);
        let _ = fs::remove_dir_all(path);
    }
    #[tokio::test]
    async fn websocket_streams_fake_source_event() {
        let (tx, _) = broadcast::channel(16);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();
        let tracker = Arc::new(Mutex::new(Tracker::default()));
        let app = App {
            tx: tx.clone(),
            tracker: tracker.clone(),
            session: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
            port,
            allowed: vec![],
            ui: None,
            log_dir: std::env::temp_dir(),
            store: Arc::new(
                Store::open(std::env::temp_dir().join(format!("zc-api-{}.db", now()))).unwrap(),
            ),
            provider: Arc::new(StubRecordProvider),
            record_updates: Arc::new(Mutex::new(std::collections::HashMap::new())),
            link: Default::default(),
        };
        let router = Router::new()
            .route("/bridge", get(ws))
            .fallback(get(root))
            .with_state(app);
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut req = format!("ws://127.0.0.1:{port}/bridge")
            .into_client_request()
            .unwrap();
        req.headers_mut().insert(
            "origin",
            format!("http://localhost:{port}").parse().unwrap(),
        );
        let (mut ws, _) = tokio_tungstenite::connect_async(req).await.unwrap();
        assert!(ws
            .next()
            .await
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap()
            .contains("hello"));
        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            "{\"type\":\"subscribe\"}".into(),
        ))
        .await
        .unwrap();
        assert!(ws
            .next()
            .await
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap()
            .contains("state"));
        // The subscribe snapshot also carries the current Zoom connection status.
        assert!(ws
            .next()
            .await
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap()
            .contains("\"type\":\"zoom_status\""));
        let (fake_tx, fake_rx) = mpsc::channel();
        fake_tx
            .send(Event::IpcStatus {
                code: status::IPC_STATUS_CONNECTED,
                message: "fake connected".into(),
                thread_id: 0,
            })
            .unwrap();
        let fake = FakeBridgeEventSource(fake_rx);
        let raw = fake.recv_event(Duration::from_millis(1)).unwrap();
        for event in tracker.lock().unwrap().apply(raw, None) {
            let _ = tx.send(event);
        }
        let pushed = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap();
        assert!(pushed.contains("\"connected\":true"));
        task.abort();
    }
}
async fn api_get_config(
    State(a): State<App>,
    axum::extract::Path(key): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if !["roster", "aliases", "profiles", "settings"].contains(&key.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    if key=="profiles" {
        if let Ok((revision,value))=a.store.get_doc("profiles") {
            if !value.as_array().is_some_and(|rows|rows.iter().any(|p|p["id"]=="default-all")) {
                let mut rows=value.as_array().cloned().unwrap_or_default();rows.push(json!({"id":"default-all","name":"All participants","description":"Factory default profile","includedSubgroups":[],"lateAfterMinutes":10,"returnWindowMinutes":8,"meetingIds":[],"topicKeywords":[],"groupSortOrder":[],"active":true,"isDefault":true}));
                if let Ok(Ok(next))=a.store.put_doc("profiles",revision,&json!(rows),now()) { return Json(json!({"revision":next,"value":rows})).into_response(); }
            }
        }
    }
    match a.store.get_doc(&key) {
        Ok((revision, value)) => Json(json!({"revision":revision,"value":value})).into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_put_config(
    State(a): State<App>,
    axum::extract::Path(key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if !["roster", "aliases", "profiles", "settings"].contains(&key.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(rev) = body["revision"].as_u64() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let val = body.get("value").cloned().unwrap_or(Value::Null);
    if let Some(message) = validate_config(&a, &key, &val) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"validation_error","message":message})),
        )
            .into_response();
    }
    match a.store.put_doc(&key, rev, &val, now()) {
        Ok(Ok(revision)) => {
            if key == "roster" {
                prune_orphan_aliases(&a, &val);
            }
            Json(json!({"revision":revision,"value":val})).into_response()
        }
        Ok(Err(actual)) => Json(json!({"error":"stale_revision","revision":actual}))
            .into_response_with_status(StatusCode::CONFLICT),
        Err(e) => api_error(e),
    }
}
fn prune_orphan_aliases(a: &App, roster: &Value) {
    let ids: std::collections::HashSet<i64> = roster
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| x["personId"].as_i64().or_else(|| x["id"].as_i64()))
        .collect();
    if let Ok((revision, aliases)) = a.store.get_doc("aliases") {
        if let Some(rows) = aliases.as_array() {
            let filtered: Vec<Value> = rows
                .iter()
                .filter(|x| ids.contains(&x["personId"].as_i64().unwrap_or(-1)))
                .cloned()
                .collect();
            if filtered.len() != rows.len() {
                let _ = a
                    .store
                    .put_doc("aliases", revision, &json!(filtered), now());
            }
        }
    }
}
fn norm_config_name(value: &str) -> String {
    attend_engine::normalize_name(value)
}
async fn api_name_pinyin(_headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let Some(name) = body["name"].as_str() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    Json(json!({"pinyin":attend_engine::build_pinyin(name)})).into_response()
}
/// Every stored config document must deserialize into the attendance engine's types;
/// otherwise one malformed write (e.g. a string personId) breaks every record.
fn config_schema_error(key: &str, value: &Value) -> Option<String> {
    let result = match key {
        "roster" => serde_json::from_value::<Vec<attend_engine::RosterPerson>>(value.clone()).map(|_| ()),
        "aliases" => serde_json::from_value::<Vec<attend_engine::AliasMapping>>(value.clone()).map(|_| ()),
        "profiles" => serde_json::from_value::<Vec<attend_engine::MeetingProfile>>(value.clone()).map(|_| ()),
        _ => Ok(()),
    };
    result.err().map(|e| format!("invalid {key} data: {e}"))
}
fn validate_config(a: &App, key: &str, value: &Value) -> Option<String> {
    if let Some(message) = config_schema_error(key, value) {
        return Some(message);
    }
    let rows = value.as_array()?;
    if key == "roster" {
        let mut names = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        for p in rows {
            let name = p["canonicalName"]
                .as_str()
                .or_else(|| p["name"].as_str())
                .unwrap_or("")
                .trim();
            if name.is_empty() {
                return Some("canonical name is required".into());
            }
            let normalized = norm_config_name(name);
            if !names.insert(normalized) {
                return Some(format!("duplicate canonical name: {name}"));
            }
            let id = p["personId"]
                .as_i64()
                .or_else(|| p["id"].as_i64())
                .unwrap_or(-1);
            if id < 0 || !ids.insert(id) {
                return Some(format!("invalid or duplicate roster id: {id}"));
            }
        }
        let aliases = a.store.get_doc("aliases").ok()?.1;
        for alias in aliases.as_array().into_iter().flatten() {
            let name = alias["zoomName"]
                .as_str()
                .or_else(|| alias["alias"].as_str())
                .unwrap_or("");
            if names.contains(&norm_config_name(name)) {
                return Some(format!("canonical name conflicts with nickname: {name}"));
            }
        }
    } else if key == "aliases" {
        let roster = a.store.get_doc("roster").ok()?.1;
        let roster_ids: std::collections::HashSet<i64> = roster
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["personId"].as_i64().or_else(|| p["id"].as_i64()))
            .collect();
        let canonical: std::collections::HashSet<String> = roster
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["canonicalName"].as_str().or_else(|| p["name"].as_str()))
            .map(norm_config_name)
            .collect();
        let mut seen = std::collections::HashSet::new();
        for alias in rows {
            let name = alias["zoomName"]
                .as_str()
                .or_else(|| alias["alias"].as_str())
                .unwrap_or("")
                .trim();
            let norm = norm_config_name(name);
            if norm.is_empty() || !seen.insert(norm.clone()) {
                return Some(format!("nickname is empty or duplicated: {name}"));
            }
            if canonical.contains(&norm) {
                return Some(format!("nickname conflicts with canonical name: {name}"));
            }
            let person_id = alias["personId"]
                .as_i64()
                .or_else(|| alias["id"].as_i64())
                .unwrap_or(-1);
            if !roster_ids.contains(&person_id) {
                return Some(format!("roster person not found: {person_id}"));
            }
        }
    } else if key == "profiles" {
        let current = a.store.get_doc("profiles").ok()?.1;
        let factory = current.as_array().into_iter().flatten().find(|p| p["id"] == "default-all");
        if !rows.iter().any(|p|p["id"]=="default-all") { return Some("default-all profile is required".into()); }
        if factory.is_some() && !rows.iter().any(|p| p["id"] == "default-all") {
            return Some("default-all profile is locked".into());
        }
        let mut ids = std::collections::HashSet::new();
        for p in rows {
            let id = p["id"].as_str().unwrap_or("");
            let name = p["name"].as_str().unwrap_or("").trim();
            if id.is_empty() || name.is_empty() { return Some("profile id and name are required".into()); }
            if !ids.insert(id) { return Some(format!("duplicate profile id: {id}")); }
            if id == "default-all" {
                if let Some(old) = factory {
                    if old["name"] != p["name"] { return Some("default-all name is locked".into()); }
                }
                if p["name"].as_str().is_some_and(|n| n.trim().is_empty()) { return Some("default-all name is locked".into()); }
            }
            for field in ["lateAfterMinutes", "returnWindowMinutes"] {
                if !p[field].as_u64().is_some_and(|n| n > 0) { return Some(format!("{field} must be a positive integer")); }
            }
            for field in ["includedSubgroups", "meetingIds", "topicKeywords", "groupSortOrder"] {
                if !p[field].as_array().is_some_and(|a| a.iter().all(Value::is_string)) { return Some(format!("{field} must be an array of strings")); }
            }
            for field in ["includedSubgroups", "groupSortOrder"] {
                for pattern in p[field].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    if pattern.contains('?') || pattern.contains('[') || pattern.contains(']') || pattern.contains('{') || pattern.contains('}') || pattern.contains("**") {
                        return Some(format!("invalid wildcard pattern: {pattern}"));
                    }
                }
            }
        }
    }
    None
}

async fn api_profiles_import_preview(State(a): State<App>, headers: HeaderMap, Json(body): Json<Value>) -> Response {
    if let Some(r)=api_origin(&a,&headers){return r}
    let Some(incoming)=body["profiles"].as_array().or_else(||body.as_array()) else { return StatusCode::BAD_REQUEST.into_response() };
    let (revision,current)=match a.store.get_doc("profiles"){Ok(v)=>v,Err(e)=>return api_error(e)};
    let mut merged=current.as_array().cloned().unwrap_or_default();
    let mut changes=Vec::new();
    for (idx,raw) in incoming.iter().enumerate(){
        if raw["id"].as_str().unwrap_or("").is_empty() || raw["name"].as_str().unwrap_or("").trim().is_empty(){changes.push(json!({"row":idx+1,"action":"reject","reason":"id and name required"}));continue}
        let mut p=raw.clone(); if let Some(o)=p.as_object_mut(){o.remove("specialParticipants");o.remove("blacklist");for (key,default) in [("description",json!("")),("includedSubgroups",json!([])),("lateAfterMinutes",json!(10)),("returnWindowMinutes",json!(8)),("meetingIds",json!([])),("topicKeywords",json!([])),("groupSortOrder",json!([])),("active",json!(true))]{o.entry(key).or_insert(default);}}
        let id=p["id"].as_str().unwrap(); let pos=merged.iter().position(|x|x["id"]==id);
        match pos {Some(i)=>{if id=="default-all" && merged[i]["name"]!=p["name"]{changes.push(json!({"row":idx+1,"id":id,"action":"reject","reason":"default-all name is locked"}));}else{changes.push(json!({"row":idx+1,"id":id,"action":"update"}));merged[i]=p;}},None=>{changes.push(json!({"row":idx+1,"id":id,"action":"add"}));merged.push(p)}}
    }
    if let Some(msg)=validate_config(&a,"profiles",&json!(merged)){return (StatusCode::UNPROCESSABLE_ENTITY,Json(json!({"message":msg}))).into_response()}
    Json(json!({"revision":revision,"value":merged,"changes":changes})).into_response()
}
async fn api_profiles_import_commit(State(a): State<App>, headers: HeaderMap, Json(body): Json<Value>) -> Response {
    if let Some(r)=api_origin(&a,&headers){return r}
    let (Some(rev),Some(value))=(body["revision"].as_u64(),body.get("value")) else{return StatusCode::BAD_REQUEST.into_response()};
    if let Some(msg)=validate_config(&a,"profiles",value){return (StatusCode::UNPROCESSABLE_ENTITY,Json(json!({"message":msg}))).into_response()}
    match a.store.put_doc("profiles",rev,value,now()){Ok(Ok(revision))=>Json(json!({"revision":revision,"value":value})).into_response(),Ok(Err(actual))=>Json(json!({"error":"stale_revision","revision":actual})).into_response_with_status(StatusCode::CONFLICT),Err(e)=>api_error(e)}
}
async fn api_example_data_commit(State(a):State<App>,headers:HeaderMap,Json(body):Json<Value>)->Response{
    if let Some(r)=api_origin(&a,&headers){return r}
    let (rr,old_roster)=match a.store.get_doc("roster"){Ok(v)=>v,Err(e)=>return api_error(e)};
    let (ar,old_aliases)=match a.store.get_doc("aliases"){Ok(v)=>v,Err(e)=>return api_error(e)};
    let (pr,old_profiles)=match a.store.get_doc("profiles"){Ok(v)=>v,Err(e)=>return api_error(e)};
    let (Some(new_roster),Some(new_aliases),Some(new_profiles))=(body["roster"].as_array(),body["aliases"].as_array(),body["profiles"].as_array()) else{return StatusCode::BAD_REQUEST.into_response()};
    let roster_names:std::collections::HashSet<String>=old_roster.as_array().into_iter().flatten().filter_map(|p|p["canonicalName"].as_str()).map(norm_config_name).collect();
    if let Some(p)=new_roster.iter().find(|p|roster_names.contains(&norm_config_name(p["canonicalName"].as_str().unwrap_or("")))){return (StatusCode::CONFLICT,Json(json!({"message":format!("Example roster person {} conflicts with existing roster data",p["canonicalName"])}))).into_response()}
    let alias_names:std::collections::HashSet<String>=old_aliases.as_array().into_iter().flatten().filter_map(|p|p["zoomName"].as_str()).map(norm_config_name).collect();
    if let Some(p)=new_aliases.iter().find(|p|alias_names.contains(&norm_config_name(p["zoomName"].as_str().unwrap_or("")))){return (StatusCode::CONFLICT,Json(json!({"message":format!("Example alias {} conflicts with existing name matching",p["zoomName"])}))).into_response()}
    let new_canonical:std::collections::HashSet<String>=new_roster.iter().filter_map(|p|p["canonicalName"].as_str()).map(norm_config_name).collect();
    let old_canonical:std::collections::HashSet<String>=roster_names.clone();
    if let Some(p)=new_roster.iter().find(|p|alias_names.contains(&norm_config_name(p["canonicalName"].as_str().unwrap_or("")))){return (StatusCode::CONFLICT,Json(json!({"message":format!("Example roster person {} conflicts with existing name matching",p["canonicalName"])}))).into_response()}
    if let Some(p)=new_aliases.iter().find(|p|old_canonical.contains(&norm_config_name(p["zoomName"].as_str().unwrap_or("")))||new_canonical.contains(&norm_config_name(p["zoomName"].as_str().unwrap_or("")))){return (StatusCode::CONFLICT,Json(json!({"message":format!("Example alias {} conflicts with a canonical name",p["zoomName"])}))).into_response()}
    let profile_ids:std::collections::HashSet<String>=old_profiles.as_array().into_iter().flatten().filter_map(|p|p["id"].as_str().map(str::to_owned)).collect();
    if let Some(p)=new_profiles.iter().find(|p|profile_ids.contains(p["id"].as_str().unwrap_or(""))){return (StatusCode::CONFLICT,Json(json!({"message":format!("Example profile id {} already exists",p["id"])}))).into_response()}
    let mut aliases=old_aliases.as_array().cloned().unwrap_or_default();aliases.extend(new_aliases.iter().cloned());
    let mut roster=old_roster.as_array().cloned().unwrap_or_default();roster.extend(new_roster.iter().cloned());
    let mut profiles=old_profiles.as_array().cloned().unwrap_or_default();for profile in new_profiles{let mut p=profile.clone();if let Some(o)=p.as_object_mut(){o.remove("specialParticipants");o.remove("blacklist");}profiles.push(p)}
    for (key,val) in [("roster",json!(roster)),("aliases",json!(aliases)),("profiles",json!(profiles))]{if let Some(message)=config_schema_error(key,&val){return (StatusCode::UNPROCESSABLE_ENTITY,Json(json!({"message":message}))).into_response()}}
    let docs=vec![("roster".into(),rr,json!(roster)),("aliases".into(),ar,json!(aliases)),("profiles".into(),pr,json!(profiles))];
    match a.store.put_docs_atomic(&docs,now()){Ok(Ok(_))=>Json(json!({"added":{"roster":new_roster.len(),"aliases":new_aliases.len(),"profiles":new_profiles.len()}})).into_response(),Ok(Err((key,_)))=>(StatusCode::CONFLICT,Json(json!({"message":format!("{key} changed during example import; retry") }))).into_response(),Err(e)=>api_error(e)}
}

// Configuration CSV workflows are kept in the companion so preview and commit
// use the same normalization and validation as all other roster mutations.
fn csv_cells(input: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut cell)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => {}
            _ => cell.push(ch),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}
fn decode_config_csv_bytes(data: &[u8]) -> String {
    let bytes = data;
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let decoded = std::str::from_utf8(bytes)
        .map(str::to_owned)
        .unwrap_or_else(|_| {
            #[cfg(windows)]
            {
                use windows_sys::Win32::Globalization::MultiByteToWideChar;
                const CP_GB18030: u32 = 54936;
                unsafe {
                    let size = MultiByteToWideChar(
                        CP_GB18030,
                        0,
                        bytes.as_ptr(),
                        bytes.len() as i32,
                        std::ptr::null_mut(),
                        0,
                    );
                    if size > 0 {
                        let mut wide = vec![0u16; size as usize];
                        let written = MultiByteToWideChar(
                            CP_GB18030,
                            0,
                            bytes.as_ptr(),
                            bytes.len() as i32,
                            wide.as_mut_ptr(),
                            size,
                        );
                        if written > 0 {
                            return String::from_utf16_lossy(&wide[..written as usize]);
                        }
                    }
                }
            }
            let (s, _, _) = encoding_rs::GBK.decode(bytes);
            s.into_owned()
        });
    decoded.replace('，', ",")
}
fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.into()
    }
}
fn decode_query_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&value[i + 1..i + 3], 16) {
                out.push(hex);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
fn bool_csv(v: &Value) -> &'static str {
    if v == false {
        "false"
    } else {
        "true"
    }
}
fn config_export_csv(a: &App, key: &str, zh: bool, selected: &[String]) -> Result<String, String> {
    let rows = a.store.get_doc(key).map_err(|e| e.to_string())?.1;
    let fields: &[&str] = if key == "roster" {
        if zh {
            &["id", "姓名", "拼音", "分组", "启用", "删除"]
        } else {
            &["id", "canonicalName", "pinyin", "group", "active", "delete"]
        }
    } else if zh {
        &["固定编号", "姓名", "昵称", "删除", "最后使用时间"]
    } else {
        &[
            "personId",
            "canonicalName",
            "nickname",
            "delete",
            "lastUsedAt",
        ]
    };
    let mut out = String::from("\u{feff}");
    out.push_str(&fields.join(","));
    out.push_str("\r\n");
    let roster = if key == "aliases" {
        a.store.get_doc("roster").map_err(|e| e.to_string())?.1
    } else {
        Value::Null
    };
    for item in rows.as_array().into_iter().flatten() {
        let id = item["personId"]
            .as_i64()
            .or_else(|| item["id"].as_i64())
            .unwrap_or(0);
        let row_key = if key == "roster" {
            id.to_string()
        } else {
            item["id"]
                .as_i64()
                .map(|x| x.to_string())
                .unwrap_or_else(|| {
                    item["zoomName"]
                        .as_str()
                        .or_else(|| item["alias"].as_str())
                        .unwrap_or("")
                        .to_string()
                })
        };
        if !selected.is_empty() && !selected.contains(&row_key) {
            continue;
        }
        let values = if key == "roster" {
            vec![
                id.to_string(),
                item["canonicalName"].as_str().unwrap_or("").into(),
                item["pinyin"].as_str().unwrap_or("").into(),
                item["group"].as_str().unwrap_or("").into(),
                bool_csv(&item["active"]).into(),
                String::new(),
            ]
        } else {
            let pid = item["personId"].as_i64().unwrap_or(0);
            let cname = roster
                .as_array()
                .into_iter()
                .flatten()
                .find(|p| p["personId"].as_i64().or_else(|| p["id"].as_i64()) == Some(pid))
                .and_then(|p| p["canonicalName"].as_str())
                .unwrap_or("");
            vec![
                pid.to_string(),
                cname.into(),
                item["zoomName"]
                    .as_str()
                    .or_else(|| item["alias"].as_str())
                    .unwrap_or("")
                    .into(),
                String::new(),
                item["lastUsedAt"].as_str().unwrap_or("").into(),
            ]
        };
        out.push_str(
            &values
                .iter()
                .map(|v| csv_escape(v))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    Ok(out)
}
async fn api_config_export(
    State(a): State<App>,
    axum::extract::Path(key): axum::extract::Path<String>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if !["roster", "aliases"].contains(&key.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let zh = uri.query().unwrap_or("").contains("language=zh");
    let selected_query = uri
        .query()
        .unwrap_or("")
        .split("selected=")
        .nth(1)
        .unwrap_or("")
        .split('&')
        .next()
        .unwrap_or("");
    let selected = decode_query_component(selected_query)
        .split('|')
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect::<Vec<String>>();
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let language = if zh { "zh" } else { "en" };
    let stem = if key == "roster" {
        format!("roster-{language}-{}", stamp)
    } else {
        format!("zoom-name-matching-{language}-{}", stamp)
    };
    let filename = if selected.is_empty() {
        format!("{stem}.csv")
    } else {
        format!("{}-selected.csv", stem)
    };
    match config_export_csv(&a, &key, zh, &selected) {
        Ok(csv) => {
            let mut h = HeaderMap::new();
            h.insert(
                axum::http::header::CONTENT_TYPE,
                "text/csv; charset=utf-8".parse().unwrap(),
            );
            h.insert(
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\"")
                    .parse()
                    .unwrap(),
            );
            (h, csv).into_response()
        }
        Err(e) => api_error(e),
    }
}
async fn api_config_import_preview(
    State(a): State<App>,
    axum::extract::Path(key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if !["roster", "aliases"].contains(&key.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let bytes: Vec<u8> = if let Some(a) = body["csvBytes"].as_array() {
        a.iter()
            .filter_map(Value::as_u64)
            .map(|x| x as u8)
            .collect()
    } else if let Some(text) = body["csv"].as_str() {
        text.as_bytes().to_vec()
    } else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let parsed = csv_cells(&decode_config_csv_bytes(&bytes));
    if parsed.is_empty() {
        return Json(json!({"revision":0,"changes":[]})).into_response();
    }
    let norm_header = |s: &str| s.trim().to_lowercase().replace(char::is_whitespace, "");
    let headers: Vec<String> = parsed[0].iter().map(|x| norm_header(x)).collect();
    let existing = match a.store.get_doc(&key) {
        Ok(x) => x,
        Err(e) => return api_error(e),
    };
    let roster = if key == "aliases" {
        a.store.get_doc("roster").map(|x| x.1).unwrap_or(json!([]))
    } else {
        existing.1.clone()
    };
    let mut next = existing.1.as_array().cloned().unwrap_or_default();
    let mut changes = Vec::new();
    let mut import_alias_names = std::collections::HashSet::new();
    let mut next_id = next
        .iter()
        .filter_map(|x| x["personId"].as_i64().or_else(|| x["id"].as_i64()))
        .max()
        .unwrap_or(0)
        + 1;
    for (i, cells) in parsed.iter().enumerate().skip(1) {
        if cells.iter().all(|x| x.trim().is_empty()) {
            continue;
        }
        let get = |names: &[&str]| -> String {
            headers
                .iter()
                .position(|h| names.contains(&h.as_str()))
                .and_then(|idx| cells.get(idx))
                .cloned()
                .unwrap_or_default()
                .trim()
                .into()
        };
        let mut action = "Reject";
        let mut note = String::new();
        let mut before = Value::Null;
        let mut after = Value::Null;
        let mut pid = 0i64;
        if key == "roster" {
            let idtxt = get(&["id"]);
            let name = get(&["canonicalname/姓名", "canonicalname", "姓名"]);
            let delete = get(&["delete/删除", "delete", "删除"]);
            let id = idtxt.parse::<i64>().ok();
            let existing_pos = id.and_then(|id| {
                next.iter()
                    .position(|p| p["personId"].as_i64().or_else(|| p["id"].as_i64()) == Some(id))
            });
            let name = if name.is_empty() {
                existing_pos
                    .and_then(|p| next[p]["canonicalName"].as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                name
            };
            if ["1", "true", "yes", "y", "是"].contains(&delete.to_lowercase().as_str()) {
                if let Some(id) = id {
                    if let Some(pos) = next.iter().position(|p| {
                        p["personId"].as_i64().or_else(|| p["id"].as_i64()) == Some(id)
                    }) {
                        before = next.remove(pos);
                        action = "Delete";
                        pid = id;
                    } else {
                        note = "roster person not found".into();
                    }
                } else {
                    note = "id is required for delete".into();
                }
            } else if name.is_empty() {
                note = "canonical name is required".into();
            } else {
                let pos = existing_pos;
                let pinyin = get(&["pinyin/拼音", "pinyin", "拼音"]);
                let group = get(&["group/分组", "group", "分组"]);
                let active = get(&["active/启用", "active", "启用"]);
                let (obj, act) = if let Some(pos) = pos {
                    let mut o = next[pos].as_object().cloned().unwrap_or_default();
                    o.insert("canonicalName".into(), json!(name));
                    if !pinyin.is_empty() {
                        o.insert("pinyin".into(), json!(pinyin));
                    }
                    if !group.is_empty() {
                        o.insert("group".into(), json!(group));
                    }
                    if !active.is_empty() {
                        o.insert(
                            "active".into(),
                            json!(!["0", "false", "no", "n", "否", "停用"]
                                .contains(&active.to_lowercase().as_str())),
                        );
                    }
                    (o, "Update")
                } else {
                    let id = id.unwrap_or_else(|| {
                        let n = next_id;
                        next_id += 1;
                        n
                    });
                    let py = if pinyin.is_empty() {
                        attend_engine::build_pinyin(&name)
                    } else {
                        pinyin
                    };
                    let active_on = active.is_empty()
                        || !["0", "false", "no", "n", "否", "停用"]
                            .contains(&active.to_lowercase().as_str());
                    (
                        serde_json::Map::from_iter([
                            ("personId".into(), json!(id)),
                            ("canonicalName".into(), json!(name)),
                            ("pinyin".into(), json!(py)),
                            ("group".into(), json!(group)),
                            ("active".into(), json!(active_on)),
                        ]),
                        "Add",
                    )
                };
                pid = obj.get("personId").and_then(Value::as_i64).unwrap_or(0);
                after = Value::Object(obj.clone());
                action = act;
                before = pos.map(|p| next[p].clone()).unwrap_or(Value::Null);
                if let Some(pos) = pos {
                    next[pos] = after.clone();
                } else {
                    next.push(after.clone());
                }
            }
        } else {
            let nick = get(&["nickname/昵称", "nickname", "昵称"]);
            let pidtxt = get(&["personid", "固定编号", "id"]);
            let canonical = get(&["canonicalname/姓名", "canonicalname", "姓名"]);
            let del = get(&["delete/删除", "delete", "删除"]);
            let id = pidtxt.parse::<i64>().ok();
            let keyname = attend_engine::normalize_name(&nick);
            let found = next.iter().position(|x| {
                x["zoomName"]
                    .as_str()
                    .or_else(|| x["alias"].as_str())
                    .is_some_and(|n| attend_engine::normalize_name(n) == keyname)
            });
            let delete = matches!(
                del.to_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "是"
            );
            if !del.is_empty()
                && !matches!(
                    del.to_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "是" | "0" | "false" | "no" | "n" | "否"
                )
            {
                note = "invalid delete boolean".into();
            } else if nick.is_empty() {
                note = "nickname is required".into();
            } else if !import_alias_names.insert(keyname.clone()) {
                note = "nickname appears more than once in the import".into();
            } else if delete {
                if let Some(pos) = found {
                    before = next.remove(pos);
                    action = "Delete";
                    pid = before["personId"].as_i64().unwrap_or(0);
                } else {
                    note = "nickname not found".into();
                }
            } else {
                let resolved = id.or_else(|| {
                    roster
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|p| {
                            p["canonicalName"].as_str().is_some_and(|n| {
                                attend_engine::normalize_name(n)
                                    == attend_engine::normalize_name(&canonical)
                            })
                        })
                        .and_then(|p| p["personId"].as_i64().or_else(|| p["id"].as_i64()))
                });
                let target =
                    resolved.and_then(|rid| {
                        roster.as_array().into_iter().flatten().find(|p| {
                            p["personId"].as_i64().or_else(|| p["id"].as_i64()) == Some(rid)
                        })
                    });
                if let Some(rid) = resolved.filter(|_| target.is_some()) {
                    let canonical_conflict = roster.as_array().into_iter().flatten().any(|p| {
                        p["canonicalName"]
                            .as_str()
                            .is_some_and(|n| attend_engine::normalize_name(n) == keyname)
                    });
                    if canonical_conflict {
                        note = "nickname conflicts with canonical name".into();
                    } else if let Some(person) = target {
                        if !canonical.is_empty()
                            && !person["canonicalName"].as_str().is_some_and(|n| {
                                attend_engine::normalize_name(n)
                                    == attend_engine::normalize_name(&canonical)
                            })
                        {
                            note = "ID and canonical name do not match".into();
                        } else {
                            pid = rid;
                            let last =
                                get(&["lastused/最后使用时间", "lastusedat", "最后使用时间"]);
                            if let Some(pos) = found {
                                before = next[pos].clone();
                                let oldlast = next[pos]["lastUsedAt"].clone();
                                after = json!({"zoomName":nick,"personId":pid,"notes":next[pos]["notes"].as_str().unwrap_or(""),"lastUsedAt":if last.is_empty(){oldlast}else{json!(last)}});
                                next[pos] = after.clone();
                                action = "Update";
                            } else {
                                after = json!({"zoomName":nick,"personId":pid,"notes":"","lastUsedAt":if last.is_empty(){Value::Null}else{json!(last)}});
                                next.push(after.clone());
                                action = "Add";
                            }
                        }
                    } else {
                        note = format!("roster person {rid} not found");
                    }
                } else {
                    note = "ID or canonical name must identify a roster person".into();
                }
            }
        }
        changes.push(json!({"row":i+1,"action":action,"id":pid,"before":before,"after":after,"notes":note,"aliasImpact":if key=="roster"&&action=="Delete" { a.store.get_doc("aliases").ok().and_then(|(_,v)|v.as_array().cloned()).unwrap_or_default().iter().filter(|x|x["personId"].as_i64()==Some(pid)).count() } else {0}}));
    }
    let result = json!({"revision":existing.0,"changes":changes,"value":next});
    Json(result).into_response()
}
async fn api_config_import_commit(
    State(a): State<App>,
    axum::extract::Path(key): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if !["roster", "aliases"].contains(&key.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(revision) = body["revision"].as_u64() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(value) = body.get("value") else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if body["changes"]
        .as_array()
        .is_some_and(|cs| cs.iter().any(|c| c["action"] == "Reject"))
    {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"validation_error","message":"Import contains rejected rows"})),
        )
            .into_response();
    }
    if let Some(message) = validate_config(&a, &key, value) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error":"validation_error","message":message})),
        )
            .into_response();
    }
    let mut docs = vec![(key.clone(), revision, value.clone())];
    if key == "roster" {
        let ids: std::collections::HashSet<i64> = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| x["personId"].as_i64().or_else(|| x["id"].as_i64()))
            .collect();
        match a.store.get_doc("aliases") {
            Ok((ar, aliases)) => {
                let filtered: Vec<Value> = aliases
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|x| ids.contains(&x["personId"].as_i64().unwrap_or(-1)))
                    .cloned()
                    .collect();
                docs.push(("aliases".into(), ar, json!(filtered)));
            }
            Err(e) => return api_error(e),
        }
    }
    match a.store.put_docs_atomic(&docs, now()) {
        Ok(Ok(revisions)) => Json(json!({"revision":revisions[0],"value":value})).into_response(),
        Ok(Err((_, actual))) => Json(json!({"error":"stale_revision","revision":actual}))
            .into_response_with_status(StatusCode::CONFLICT),
        Err(e) => api_error(e),
    }
}
async fn api_meetings(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    match a.store.meetings(){Ok(rows)=>Json(rows.iter().map(|m|{let meta=a.store.meeting(&m.instance_id).ok().flatten().unwrap_or(Value::Null);let (_,state)=a.store.get_state(&m.instance_id).unwrap_or((0,Value::Null));json!({"instanceId":m.instance_id,"meetingNumber":m.meeting_number,"topic":m.topic,"startedAt":m.started_at,"endedAt":m.ended_at,"endReason":m.end_reason,"isHost":m.is_host,"lastSeq":m.last_seq,"source":meta["source"],"exportedAt":state["exportedAt"],"needsExport":state["needsExport"]})}).collect::<Vec<_>>()).into_response(),Err(e)=>api_error(e)}
}
async fn api_meeting(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    match a.store.meeting(&id) {
        Ok(Some(v)) => Json(v).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_events(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    match a.store.events(&id) {
        Ok(v) => Json(v.iter().map(|e| stored_event(&id, e)).collect::<Vec<_>>()).into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_record(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    let Ok(Some(meeting)) = a.store.meeting(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if meeting["source"] == "imported" { return Json(meeting["importedRecord"].clone()).into_response(); }
    let events = match a.store.events(&id) {
        Ok(e) => e,
        Err(e) => return api_error(e),
    };
    let vals = events
        .iter()
        .map(|e| {
            let mut v = stored_event(&id, e);
            v["instanceId"] = Value::Null;
            v
        })
        .collect::<Vec<_>>();
    let docs = json!({"roster":a.store.get_doc("roster").map(|x|x.1).unwrap_or(Value::Null),"aliases":a.store.get_doc("aliases").map(|x|x.1).unwrap_or(Value::Null),"profiles":a.store.get_doc("profiles").map(|x|x.1).unwrap_or(Value::Null),"settings":a.store.get_doc("settings").map(|x|x.1).unwrap_or(Value::Null)});
    let state = a
        .store
        .get_state(&id)
        .map(|x| x.1)
        .unwrap_or(json!({"mode":"generic"}));
    match a.provider.try_build_record(&meeting, &vals, &docs, &state, now()) {
        Ok(mut record) => {
            // The UI reads the top-level `rows`; the nested copy doubled the payload.
            if let Some(nested) = record.get_mut("record").and_then(Value::as_object_mut) {
                nested.remove("rows");
            }
            Json(record).into_response()
        }
        Err(error) => {
            log::error!("record {id}: attendance engine failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"engine_failed","message":error})),
            )
                .into_response()
        }
    }
}
async fn api_record_export(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) { return r; }
    let kind=query.get("kind").map(String::as_str).unwrap_or("json");
    if !["json","summary","manual"].contains(&kind) { return StatusCode::BAD_REQUEST.into_response(); }
    let Some(meeting) = a.store.meeting(&id).ok().flatten() else { return StatusCode::NOT_FOUND.into_response(); };
    let events = match a.store.events(&id) { Ok(v) => v, Err(e) => return api_error(e) };
    let values = events.iter().map(|e| stored_event(&id,e)).collect::<Vec<_>>();
    let config=json!({"roster":a.store.get_doc("roster").map(|x|x.1).unwrap_or(Value::Null),"aliases":a.store.get_doc("aliases").map(|x|x.1).unwrap_or(Value::Null),"profiles":a.store.get_doc("profiles").map(|x|x.1).unwrap_or(Value::Null)});
    let (revision, mut state)=match a.store.get_state(&id){Ok(v)=>v,Err(e)=>return api_error(e)};
    if state["raiseHandEntries"].as_array().is_none() { state["raiseHandEntries"]=json!(crate::record_export::hand_entries(&values)); }
    let record=if meeting["source"]=="imported" { meeting["importedRecord"].clone() } else { match a.provider.try_build_record(&meeting,&values,&config,&state,now()) {Ok(v)=>v,Err(e)=>return (StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"message":e}))).into_response()} };
    let exported_at=chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true);
    state["exportedAt"]=json!(exported_at); state["needsExport"]=json!(false);
    match a.store.put_state(&id,revision,&state,now()) {
        Ok(Ok(_)) => {},
        Ok(Err(actual)) => return (StatusCode::CONFLICT,Json(json!({"error":"stale_revision","revision":actual}))).into_response(),
        Err(e) => return api_error(e),
    }
    emit_record_updated(&a,&id);
    let language=query.get("language").map(String::as_str).unwrap_or("en");
    let Some(file)=crate::record_export::artifacts(&meeting,&record,&state,&values,&exported_at,language,kind).into_iter().next() else { return StatusCode::INTERNAL_SERVER_ERROR.into_response(); };
    let mut response=(StatusCode::OK,[(axum::http::header::CONTENT_TYPE,file.content_type.to_string()),(axum::http::header::CONTENT_DISPOSITION,format!("attachment; filename=\"{}\"",file.filename))],file.bytes).into_response();
    response.headers_mut().insert("x-exported-at",axum::http::HeaderValue::from_str(&exported_at).unwrap());
    response
}
fn normalize_imported_record(item:&Value,index:usize)->Result<Value,String>{
    let session=item.get("session").unwrap_or(item);
    let is_session=item.get("session").is_some() || session.get("derivedRows").is_some();
    let source_meeting=if is_session { session } else { item.get("meeting").unwrap_or(item) };
    let id=source_meeting["instanceId"].as_str().or_else(||source_meeting["sessionId"].as_str()).or_else(||session["sessionId"].as_str()).unwrap_or("");
    let number=source_meeting["meetingNumber"].as_str().or_else(||source_meeting["meetingId"].as_str()).or_else(||session["metadata"]["meetingId"].as_str()).unwrap_or("");
    let topic=source_meeting["topic"].as_str().or_else(||source_meeting["meetingTopic"].as_str()).or_else(||session["metadata"]["meetingTopic"].as_str()).unwrap_or("Imported meeting");
    let started=parse_meeting_timestamp(&source_meeting["startedAt"]).unwrap_or_else(now);
    let mut record=if is_session {json!({"rows":session["derivedRows"],"summary":session["summary"],"mode":"roster"})} else {item["record"].clone()};
    if record.is_null(){return Err("record rows are missing".into())}
    if record["rows"].is_null(){if let Some(rows)=session["derivedRows"].as_array(){record["rows"]=json!(rows)}else{return Err("derived rows are missing".into())}}
    let instance_id=if id.is_empty(){format!("imported-{}-{index}",now())}else{format!("imported-{id}")};
    Ok(json!({"meeting":{"instanceId":instance_id,"meetingNumber":number,"topic":topic,"meetingType":0,"status":0,"statusName":"Imported","startedAt":started,"endedAt":parse_meeting_timestamp(&source_meeting["endedAt"]).or(Some(started)),"endReason":"imported","isHost":false,"source":"imported","importedRecord":record},"record":record,"source":"imported"}))
}
fn parse_meeting_timestamp(value:&Value)->Option<u64>{value.as_u64().or_else(||value.as_str().and_then(|s|s.parse().ok())).or_else(||value.as_str().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d|d.timestamp_millis().max(0) as u64))}
async fn api_records_import_preview(State(a):State<App>,headers:HeaderMap,Json(body):Json<Value>)->Response{
    if let Some(r)=api_origin(&a,&headers){return r}
    let items=if let Some(v)=body.as_array(){v.clone()}else if let Some(v)=body["records"].as_array(){v.clone()}else if let Some(v)=body["sessions"].as_array(){v.clone()}else if body["version"].as_u64()==Some(3)&&body["session"].is_object(){vec![body.clone()]}else{return StatusCode::BAD_REQUEST.into_response()};
    let active=a.tracker.lock().unwrap().meeting.as_ref().map(|m|m.meeting_number.clone());
    let mut plans=Vec::new();
    for (i,item) in items.iter().enumerate(){match normalize_imported_record(item,i){Ok(v)=>{let number=v["meeting"]["meetingNumber"].as_str().unwrap_or("");let instance=v["meeting"]["instanceId"].as_str().unwrap_or("");let reject=active.as_deref()==Some(number);let update=a.store.meeting(instance).ok().flatten().is_some();plans.push(json!({"action":if reject{"reject"}else if update{"update"}else{"add"},"reason":if reject{"would overwrite live meeting"}else{""},"value":v}));},Err(e)=>plans.push(json!({"action":"reject","reason":e}))}}
    Json(json!({"plans":plans})).into_response()
}
async fn api_records_import_commit(State(a):State<App>,headers:HeaderMap,Json(body):Json<Value>)->Response{
    if let Some(r)=api_origin(&a,&headers){return r}
    let Some(plans)=body["plans"].as_array() else{return StatusCode::BAD_REQUEST.into_response()};
    if plans.iter().any(|p|p["action"]=="reject"){return (StatusCode::UNPROCESSABLE_ENTITY,Json(json!({"message":"Import preview contains rejected records"}))).into_response()}
    let active=a.tracker.lock().unwrap().meeting.as_ref().map(|m|m.meeting_number.clone());
    if plans.iter().any(|p|active.as_deref()==p["value"]["meeting"]["meetingNumber"].as_str()){return (StatusCode::CONFLICT,Json(json!({"message":"Cannot import over the live meeting"}))).into_response()}
    let mut imported=0;
    for p in plans {let m=&p["value"]["meeting"];if let Err(e)=a.store.upsert_meeting(m,Some((m["endedAt"].as_u64().unwrap_or_else(now),"imported".into())),0){return api_error(e)} imported+=1;}
    Json(json!({"imported":imported})).into_response()
}
fn stored_event(id: &str, e: &attend_store::EventRow) -> Value {
    let mut v = json!({"seq":e.seq,"at":e.at,"instanceId":id,"kind":e.kind});
    if let (Some(dst), Some(src)) = (v.as_object_mut(), e.payload.as_object()) {
        dst.extend(src.clone())
    }
    v
}
async fn api_get_state(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    match a.store.get_state(&id) {
        Ok((revision, value)) => Json(json!({"revision":revision,"value":value})).into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_put_state(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    let Some(rev) = body["revision"].as_u64() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let val = body.get("value").cloned().unwrap_or(json!({}));
    match a.store.put_state(&id, rev, &val, now()) {
        Ok(Ok(revision)) => {
            emit_record_updated(&a, &id);
            Json(json!({"revision":revision,"value":val})).into_response()
        }
        Ok(Err(u64::MAX)) => StatusCode::NOT_FOUND.into_response(),
        Ok(Err(actual)) => Json(json!({"error":"stale_revision","revision":actual}))
            .into_response_with_status(StatusCode::CONFLICT),
        Err(e) => api_error(e),
    }
}
async fn api_reconciliation_decision(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) { return r; }
    let (Some(state_revision), Some(alias_revision), Some(mut state)) =
        (body["revision"].as_u64(), body["aliasesRevision"].as_u64(), body.get("value").cloned())
    else { return StatusCode::BAD_REQUEST.into_response(); };
    let alias_result = a.store.get_doc("aliases");
    let (actual_alias_revision, aliases) = match alias_result { Ok(v) => v, Err(e) => return api_error(e) };
    if actual_alias_revision != alias_revision {
        return (StatusCode::CONFLICT, Json(json!({"error":"stale_revision","key":"aliases","revision":actual_alias_revision}))).into_response();
    }
    let Some(rows) = aliases.as_array() else { return StatusCode::INTERNAL_SERVER_ERROR.into_response(); };
    if state["reconciliationDecisions"].is_null() && state["decisions"].is_array() { state["reconciliationDecisions"] = state["decisions"].clone(); }
    if state["reconciliationDecisions"].is_null() { state["reconciliationDecisions"] = json!([]); }
    let decisions = state["reconciliationDecisions"].as_array_mut().unwrap();
    if body["revertId"].is_string() {
        let revert_id = body["revertId"].as_str().unwrap();
        decisions.retain(|d| d["id"].as_str() != Some(revert_id));
    } else {
        let Some(decision) = body.get("decision") else { return StatusCode::BAD_REQUEST.into_response(); };
        let parsed: attend_engine::ReconciliationDecision = match serde_json::from_value(decision.clone()) {
            Ok(v) => v, Err(e) => return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message":format!("Invalid decision: {e}")}))).into_response()
        };
        let zoom_name = if parsed.zoom_name_at_decision.is_empty() { &parsed.zoom_name } else { &parsed.zoom_name_at_decision };
        let norm = norm_config_name(zoom_name);
        let roster = a.store.get_doc("roster").map(|x| x.1).unwrap_or(json!([]));
        let mut next_aliases = rows.clone();
        match &parsed.action {
            attend_engine::DecisionAction::Match | attend_engine::DecisionAction::NameChangeKeepMatch => {
                let Some(person_id) = parsed.person_id else { return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message":"A roster person is required for a match."}))).into_response(); };
                let Some(person) = roster.as_array().into_iter().flatten().find(|p| p["personId"].as_u64() == Some(person_id)) else { return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message":"The selected roster person no longer exists."}))).into_response(); };
                let canonical = person["canonicalName"].as_str().unwrap_or("");
                if norm_config_name(canonical) == norm {
                    if parsed.action == attend_engine::DecisionAction::Match { return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message":"A nickname must differ from the canonical name."}))).into_response(); }
                }
                if let Some(other) = roster.as_array().into_iter().flatten().find(|p| p["personId"].as_u64() != Some(person_id) && norm_config_name(p["canonicalName"].as_str().unwrap_or("")) == norm) {
                    return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message":format!("Nickname '{}' is already a canonical roster name.", other["canonicalName"].as_str().unwrap_or(zoom_name))}))).into_response();
                }
                if let Some(existing) = rows.iter().find(|a| norm_config_name(a["zoomName"].as_str().unwrap_or("")) == norm && a["personId"].as_u64() != Some(person_id)) {
                    return (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({"message":format!("Nickname '{}' already belongs to another roster person.", existing["zoomName"].as_str().unwrap_or(zoom_name))}))).into_response();
                }
                next_aliases.retain(|a| norm_config_name(a["zoomName"].as_str().unwrap_or("")) != norm);
                if norm_config_name(canonical) != norm { next_aliases.push(json!({"zoomName":zoom_name,"personId":person_id,"notes":""})); }
            }
            attend_engine::DecisionAction::Guest | attend_engine::DecisionAction::Split => {
                next_aliases.retain(|a| norm_config_name(a["zoomName"].as_str().unwrap_or("")) != norm);
            }
            _ => {}
        }
        decisions.retain(|d| d["sessionParticipantId"] != parsed.session_participant_id);
        decisions.push(json!(parsed));
        let put = a.store.put_state_and_doc_atomic(&id, state_revision, &state, "aliases", alias_revision, &json!(next_aliases), now());
        return match put {
            Ok(Ok((revision, aliases_revision))) => { emit_record_updated(&a, &id); Json(json!({"revision":revision,"aliasesRevision":aliases_revision,"value":state})).into_response() },
            Ok(Err((_, u64::MAX))) => StatusCode::NOT_FOUND.into_response(),
            Ok(Err((key, revision))) => (StatusCode::CONFLICT, Json(json!({"error":"stale_revision","key":key,"revision":revision}))).into_response(),
            Err(e) => api_error(e),
        };
    }
    match a.store.put_state(&id, state_revision, &state, now()) {
        Ok(Ok(revision)) => { emit_record_updated(&a, &id); Json(json!({"revision":revision,"aliasesRevision":alias_revision,"value":state})).into_response() },
        Ok(Err(u64::MAX)) => StatusCode::NOT_FOUND.into_response(),
        Ok(Err(actual)) => (StatusCode::CONFLICT, Json(json!({"error":"stale_revision","key":"state","revision":actual}))).into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_delete_meeting(
    State(a): State<App>,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if a.tracker
        .lock()
        .unwrap()
        .meeting
        .as_ref()
        .is_some_and(|m| m.instance_id == id)
        || a.store
            .meeting(&id)
            .ok()
            .flatten()
            .is_some_and(|m| m["endedAt"].is_null())
    {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"active_meeting"})),
        )
            .into_response();
    }
    match a.store.delete_meeting(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_export_records(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    let meetings = match a.store.meetings() {
        Ok(v) => v,
        Err(e) => return api_error(e),
    };
    let mut records = Vec::new();
    for m in meetings {
        let Some(meeting) = a.store.meeting(&m.instance_id).ok().flatten() else {
            continue;
        };
        let events = match a.store.events(&m.instance_id) {
            Ok(v) => v,
            Err(e) => return api_error(e),
        };
        let events: Vec<Value> = events
            .iter()
            .map(|e| stored_event(&m.instance_id, e))
            .collect();
        let config = json!({"roster":a.store.get_doc("roster").map(|x|x.1).unwrap_or(Value::Null),"aliases":a.store.get_doc("aliases").map(|x|x.1).unwrap_or(Value::Null),"profiles":a.store.get_doc("profiles").map(|x|x.1).unwrap_or(Value::Null),"settings":a.store.get_doc("settings").map(|x|x.1).unwrap_or(Value::Null)});
        let state = a
            .store
            .get_state(&m.instance_id)
            .map(|x| x.1)
            .unwrap_or(json!({"mode":"generic"}));
        let record = if meeting["source"]=="imported" { meeting["importedRecord"].clone() } else { a
            .provider
            .try_build_record(&meeting, &events, &config, &state, now())
            .unwrap_or_else(|error| json!({"error":"engine_failed","message":error})) };
        records.push(json!({"meeting":meeting,"events":events,"record":record,"state":state}));
    }
    Json(json!({"version":3,"exportedAt":now(),"records":records})).into_response()
}
async fn api_delete_records(
    State(a): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    let Some(ids) = body["instanceIds"].as_array() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let active = a
        .tracker
        .lock()
        .unwrap()
        .meeting
        .as_ref()
        .map(|m| m.instance_id.clone());
    if ids.iter().filter_map(Value::as_str).any(|id| {
        Some(id) == active.as_deref()
            || a.store
                .meeting(id)
                .ok()
                .flatten()
                .is_some_and(|m| m["endedAt"].is_null())
    }) {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"active_meeting"})),
        )
            .into_response();
    }
    let mut deleted = 0;
    for id in ids.iter().filter_map(Value::as_str) {
        match a.store.delete_meeting(id) {
            Ok(true) => deleted += 1,
            Ok(false) => {}
            Err(e) => return api_error(e),
        }
    }
    Json(json!({"deleted":deleted})).into_response()
}
async fn api_export_backup(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    let docs = |k: &str| {
        a.store
            .get_doc(k)
            .map(|x| x.1)
            .unwrap_or(Value::Array(vec![]))
    };
    let meetings = match a.store.meetings() {
        Ok(v) => v,
        Err(e) => return api_error(e),
    };
    let mut records = Vec::new();
    for m in meetings {
        let Some(meeting) = a.store.meeting(&m.instance_id).ok().flatten() else {
            continue;
        };
        let events = match a.store.events(&m.instance_id) {
            Ok(v) => v,
            Err(e) => return api_error(e),
        };
        records.push(json!({"meeting":meeting,"events":events.iter().map(|e|stored_event(&m.instance_id,e)).collect::<Vec<_>>()}));
    }
    Json(json!({"version":3,"exportedAt":now(),"rosterPeople":docs("roster"),"aliasMappings":docs("aliases"),"attendanceProfiles":docs("profiles"),"settings":docs("settings"),"records":records})).into_response()
}
async fn api_restore_backup(
    State(a): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if body["version"].as_u64() != Some(3) && body["schemaVersion"].as_u64() != Some(3) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"expected AttendFlow backup v3"})),
        )
            .into_response();
    }
    if a.tracker.lock().unwrap().meeting.is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"active_meeting"})),
        )
            .into_response();
    }
    api_import_backup(State(a), headers, Json(body)).await
}
async fn api_clear_local_data(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    if a.tracker.lock().unwrap().meeting.is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error":"active_meeting"})),
        )
            .into_response();
    }
    match a.store.clear_records() {
        Ok(()) => Json(json!({"cleared":true})).into_response(),
        Err(e) => api_error(e),
    }
}
async fn api_import_backup(
    State(a): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    let version = body["version"]
        .as_u64()
        .or_else(|| body["schemaVersion"].as_u64());
    if version != Some(3) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"expected AttendFlow backup v3"})),
        )
            .into_response();
    }
    let entries = [
        ("roster", "rosterPeople"),
        ("aliases", "aliasMappings"),
        ("profiles", "attendanceProfiles"),
        ("settings", "settings"),
    ];
    // Validate every section before writing any, so a bad backup changes nothing.
    for (key, field) in entries {
        let val = body.get(field).cloned().unwrap_or(Value::Array(vec![]));
        if let Some(message) = config_schema_error(key, &val) {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"error":"validation_error","message":message})),
            )
                .into_response();
        }
    }
    let mut counts = serde_json::Map::new();
    for (key, field) in entries {
        let val = body.get(field).cloned().unwrap_or(Value::Array(vec![]));
        let revision = a.store.get_doc(key).map(|x| x.0).unwrap_or(0);
        if let Err(e) = a.store.put_doc(key, revision, &val, now()) {
            return api_error(e);
        }
        counts.insert(field.into(), json!(val.as_array().map_or(0, |x| x.len())));
    }
    let sessions = body
        .get("sessions")
        .and_then(Value::as_array)
        .map_or(0, |x| x.len());
    Json(json!({"imported":counts,"sessionsIgnored":sessions})).into_response()
}
#[tokio::test]
async fn roster_csv_preview_api_decodes_and_returns_commit_plan() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let store = Arc::new(
        Store::open(std::env::temp_dir().join(format!("zc-roster-preview-{}.db", now()))).unwrap(),
    );
    let app = App {
        tx: broadcast::channel(8).0,
        tracker: Arc::new(Mutex::new(Tracker::default())),
        session: Arc::new(Mutex::new(None)),
        shutdown: Arc::new(AtomicBool::new(false)),
        port: 47812,
        allowed: vec![],
        ui: None,
        log_dir: std::env::temp_dir(),
        store: store.clone(),
        provider: Arc::new(StubRecordProvider),
        record_updates: Arc::new(Mutex::new(Default::default())),
        link: Default::default(),
    };
    let api = Router::new()
        .route(
            "/api/config/:key/import/preview",
            post(api_config_import_preview),
        )
        .route(
            "/api/config/:key/import/commit",
            post(api_config_import_commit),
        )
        .with_state(app);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/config/roster/import/preview")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({"csv":"id,姓名,分组\n,张三,一组"}).to_string(),
        ))
        .unwrap();
    let response = api.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["changes"][0]["action"], "Add");
    assert_eq!(body["value"][0]["canonicalName"], "张三");
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/config/roster/import/commit")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({"revision":body["revision"],"value":body["value"],"changes":body["changes"]})
                .to_string(),
        ))
        .unwrap();
    assert_eq!(api.oneshot(req).await.unwrap().status(), StatusCode::OK);
    assert_eq!(
        store.get_doc("roster").unwrap().1[0]["canonicalName"],
        "张三"
    );
}
#[tokio::test]
async fn alias_csv_preview_resolves_canonical_name_and_commits_atomically() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let store = Arc::new(
        Store::open(std::env::temp_dir().join(format!("zc-alias-preview-{}.db", now()))).unwrap(),
    );
    store.put_doc("roster",0,&json!([{"personId":7,"canonicalName":"毛若水","pinyin":"maoruoshui","group":"","active":true}]),now()).unwrap().unwrap();
    let app = App {
        tx: broadcast::channel(8).0,
        tracker: Arc::new(Mutex::new(Tracker::default())),
        session: Arc::new(Mutex::new(None)),
        shutdown: Arc::new(AtomicBool::new(false)),
        port: 47812,
        allowed: vec![],
        ui: None,
        log_dir: std::env::temp_dir(),
        store: store.clone(),
        provider: Arc::new(StubRecordProvider),
        record_updates: Arc::new(Mutex::new(Default::default())),
        link: Default::default(),
    };
    let api = Router::new()
        .route(
            "/api/config/:key/import/preview",
            post(api_config_import_preview),
        )
        .route(
            "/api/config/:key/import/commit",
            post(api_config_import_commit),
        )
        .with_state(app);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/config/aliases/import/preview")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({"csv":"姓名,昵称\n毛若水,小毛"}).to_string(),
        ))
        .unwrap();
    let response = api.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["changes"][0]["action"], "Add");
    assert_eq!(body["value"][0]["personId"], 7);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/config/aliases/import/commit")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            json!({"revision":body["revision"],"value":body["value"],"changes":body["changes"]})
                .to_string(),
        ))
        .unwrap();
    assert_eq!(api.oneshot(req).await.unwrap().status(), StatusCode::OK);
    assert_eq!(store.get_doc("aliases").unwrap().1[0]["zoomName"], "小毛");
}
fn api_origin(a: &App, h: &HeaderMap) -> Option<Response> {
    let o = h.get("origin").and_then(|v| v.to_str().ok());
    if o.is_some() && !origin_allowed(o, a.port, &a.allowed) {
        Some(StatusCode::FORBIDDEN.into_response())
    } else {
        None
    }
}
fn api_error(e: impl std::fmt::Display) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error":e.to_string()})),
    )
        .into_response()
}
trait JsonStatus {
    fn into_response_with_status(self, status: StatusCode) -> Response;
}
impl JsonStatus for Json<Value> {
    fn into_response_with_status(self, status: StatusCode) -> Response {
        (status, self).into_response()
    }
}
fn emit_record_updated(a: &App, id: &str) {
    let id = id.to_owned();
    let generation = {
        let mut g = a.record_updates.lock().unwrap();
        let n = g.entry(id.clone()).or_insert(0);
        *n += 1;
        *n
    };
    let updates = a.record_updates.clone();
    let tx = a.tx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        if updates.lock().unwrap().get(&id) == Some(&generation) {
            let _ = tx.send(json!({"type":"record_updated","instanceId":id}));
        }
    });
}
fn import_logs(a: &App, dir: &std::path::Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|x| x == "jsonl") {
                if let Err(e) = a.store.import_jsonl(&path) {
                    log::warn!("could not import {}: {e}", path.display());
                }
            }
        }
    }
}
fn seed_fake_meeting(a: &App) {
    let mut tracker = a.tracker.lock().unwrap();
    tracker.connected = true;
    tracker.users = vec![
        User {
            user_id: 101,
            name: "Ada Lovelace".into(),
            host: false,
            myself: false,
            waiting_room: false,
            audio_muted: false,
            video_on: true,
            raised_hand: false,
            talking: false,
            spotlighted: false,
        },
        User {
            user_id: 102,
            name: "毛若水".into(),
            host: false,
            myself: false,
            waiting_room: false,
            audio_muted: true,
            video_on: false,
            raised_hand: false,
            talking: false,
            spotlighted: false,
        },
    ];
    let events = tracker.start_with_snapshot(
        "fake-47812".into(),
        "Fake attendance demo".into(),
        1,
        status::MEETING_STATUS_INMEETING,
        None,
    );
    drop(tracker);
    publish(a, events);
}

#[tokio::test]
async fn fake_record_api_returns_attendflow_row_shape() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let store = Arc::new(
        Store::open(std::env::temp_dir().join(format!("zc-record-api-{}.db", now()))).unwrap(),
    );
    let meeting = json!({"instanceId":"fake-1","meetingNumber":"1","topic":"Fake","startedAt":1000,"isHost":false});
    store.upsert_meeting(&meeting, None, 1).unwrap();
    store
        .append_event(
            "fake-1",
            1,
            1000,
            "instance_started",
            &json!({"meeting":meeting}),
        )
        .unwrap();
    store
        .append_event(
            "fake-1",
            2,
            2000,
            "users",
            &json!({"action":"add","users":[{"userId":7,"name":"Ada"}]}),
        )
        .unwrap();
    let app = App {
        tx: broadcast::channel(8).0,
        tracker: Arc::new(Mutex::new(Tracker::default())),
        session: Arc::new(Mutex::new(None)),
        shutdown: Arc::new(AtomicBool::new(false)),
        port: 47812,
        allowed: vec![],
        ui: None,
        log_dir: std::env::temp_dir(),
        store,
        provider: Arc::new(StubRecordProvider),
        record_updates: Arc::new(Mutex::new(Default::default())),
        link: Default::default(),
    };
    let api = Router::new()
        .route("/api/meetings/:id/record", get(api_record))
        .with_state(app);
    let response = api
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/meetings/fake-1/record")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["rows"][0]["zoomName"], "Ada");
    assert_eq!(value["rows"][0]["currentStatus"], "present");
    assert_eq!(value["summary"]["seen"], 1);
}
#[tokio::test]
async fn record_state_captures_persist_and_export_marks_downloaded() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let store=Arc::new(Store::open(std::env::temp_dir().join(format!("zc-export-state-{}.db",now()))).unwrap());
    let meeting=json!({"instanceId":"export-meeting","meetingNumber":"42","topic":"Export test","startedAt":1000,"isHost":false});
    store.upsert_meeting(&meeting,None,1).unwrap();
    let app=App{tx:broadcast::channel(8).0,tracker:Arc::new(Mutex::new(Tracker::default())),session:Arc::new(Mutex::new(None)),shutdown:Arc::new(AtomicBool::new(false)),port:47812,allowed:vec![],ui:None,log_dir:std::env::temp_dir(),store:store.clone(),provider:Arc::new(StubRecordProvider),record_updates:Arc::new(Mutex::new(Default::default())),link:Default::default()};
    let api=Router::new().route("/api/meetings/:id/state",get(api_get_state).put(api_put_state)).route("/api/meetings/:id/export",get(api_record_export)).with_state(app);
    let captured=json!({"mode":"generic","manualSnapshots":[{"id":"snap-1","timestamp":"2026-11-17T00:00:00Z","participants":[{"screenName":"Ada","location":"meeting","matchStatus":"matched"}]}],"raiseHandCaptures":[{"id":"hands-1","timestamp":"2026-11-17T00:01:00Z","screenNames":["Ada"],"totalCount":1}],"raiseHandEntries":[{"participantUUID":"7","screenName":"Ada","action":"raise"}],"needsExport":true});
    let put=api.clone().oneshot(axum::http::Request::builder().method("PUT").uri("/api/meetings/export-meeting/state").header("content-type","application/json").body(axum::body::Body::from(json!({"revision":0,"value":captured}).to_string())).unwrap()).await.unwrap();
    assert_eq!(put.status(),StatusCode::OK);
    let export=api.oneshot(axum::http::Request::builder().uri("/api/meetings/export-meeting/export?kind=json&language=en").body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(export.status(),StatusCode::OK);
    let bundle:Value=serde_json::from_slice(&export.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(bundle["version"],3); assert_eq!(bundle["session"]["manualSnapshots"][0]["participants"][0]["screenName"],"Ada"); assert_eq!(bundle["session"]["raiseHandCaptures"][0]["totalCount"],1);
    let (_,saved)=store.get_state("export-meeting").unwrap(); assert!(!saved["exportedAt"].is_null()); assert_eq!(saved["needsExport"],false);
}
#[tokio::test]
async fn backup_import_revision_conflict_and_state_notification() {
    use tower::ServiceExt;
    let store = Arc::new(
        Store::open(std::env::temp_dir().join(format!("zc-api-import-{}.db", now()))).unwrap(),
    );
    store.upsert_meeting(&json!({"instanceId":"m1","meetingNumber":"1","topic":"Demo","startedAt":1,"isHost":false}),None,1).unwrap();
    let (tx, _) = broadcast::channel(8);
    let mut updates = tx.subscribe();
    let app = App {
        tx,
        tracker: Arc::new(Mutex::new(Tracker::default())),
        session: Arc::new(Mutex::new(None)),
        shutdown: Arc::new(AtomicBool::new(false)),
        port: 47812,
        allowed: vec![],
        ui: None,
        log_dir: std::env::temp_dir(),
        store: store.clone(),
        provider: Arc::new(StubRecordProvider),
        record_updates: Arc::new(Mutex::new(Default::default())),
        link: Default::default(),
    };
    let api = Router::new()
        .route("/api/import/attendflow-backup", post(api_import_backup))
        .route(
            "/api/backup",
            get(api_export_backup).post(api_restore_backup),
        )
        .route("/api/records/export", get(api_export_records))
        .route("/api/records/delete", post(api_delete_records))
        .route("/api/config/:key", get(api_get_config).put(api_put_config))
        .route(
            "/api/meetings/:id",
            get(api_meeting).delete(api_delete_meeting),
        )
        .route(
            "/api/meetings/:id/state",
            get(api_get_state).put(api_put_state),
        )
        .with_state(app);
    let backup = json!({"version":3,
        "rosterPeople":[{"personId":1,"canonicalName":"Ada Lovelace","pinyin":"ada lovelace","group":"","active":true}],
        "aliasMappings":[{"zoomName":"Ada","personId":1,"notes":""}],
        "attendanceProfiles":[{"id":"default","name":"Default","includedSubgroups":[],"lateAfterMinutes":10,"returnWindowMinutes":8,"meetingIds":[],"topicKeywords":[],"groupSortOrder":[],"active":true}],
        "sessions":[{},{}]});
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/import/attendflow-backup")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(backup.to_string()))
        .unwrap();
    assert_eq!(
        api.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(store.get_doc("roster").unwrap().0, 1);
    assert_eq!(
        store
            .get_doc("profiles")
            .unwrap()
            .1
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let req = axum::http::Request::builder()
        .uri("/api/backup")
        .body(axum::body::Body::empty())
        .unwrap();
    let exported = api.clone().oneshot(req).await.unwrap();
    assert_eq!(exported.status(), StatusCode::OK);
    let req = axum::http::Request::builder()
        .uri("/api/records/export")
        .body(axum::body::Body::empty())
        .unwrap();
    let exported = api.clone().oneshot(req).await.unwrap();
    assert_eq!(exported.status(), StatusCode::OK);
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/records/delete")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"instanceIds":["m1"]}"#))
        .unwrap();
    assert_eq!(
        api.clone().oneshot(req).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let stale = json!({"revision":0,"value":[]});
    let req = axum::http::Request::builder()
        .method("PUT")
        .uri("/api/config/roster")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(stale.to_string()))
        .unwrap();
    assert_eq!(
        api.clone().oneshot(req).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri("/api/meetings/m1")
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(
        api.clone().oneshot(req).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let state = json!({"revision":0,"value":{"mode":"roster"}});
    let req = axum::http::Request::builder()
        .method("PUT")
        .uri("/api/meetings/m1/state")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(state.to_string()))
        .unwrap();
    assert_eq!(api.oneshot(req).await.unwrap().status(), StatusCode::OK);
    let update = tokio::time::timeout(Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(update["type"], "record_updated");
    assert_eq!(update["instanceId"], "m1");
}

#[tokio::test]
async fn reconciliation_decisions_commit_aliases_atomically_and_revert_keeps_aliases() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let store=Arc::new(Store::open(std::env::temp_dir().join(format!("zc-reconcile-{}.db",now()))).unwrap());
    store.upsert_meeting(&json!({"instanceId":"rec-1","meetingNumber":"1","topic":"Demo","startedAt":1,"isHost":false}),None,1).unwrap();
    store.put_doc("roster",0,&json!([
        {"personId":1,"canonicalName":"Ada Lovelace","pinyin":"ada lovelace","active":true},
        {"personId":2,"canonicalName":"Grace Hopper","pinyin":"grace hopper","active":true}
    ]),now()).unwrap().unwrap();
    store.put_doc("aliases",0,&json!([{"zoomName":"Known Nick","personId":2,"notes":""},{"zoomName":"Guest Nick","personId":1,"notes":""},{"zoomName":"Shared Nick","personId":1,"notes":""}]),now()).unwrap().unwrap();
    let app=App{tx:broadcast::channel(8).0,tracker:Arc::new(Mutex::new(Tracker::default())),session:Arc::new(Mutex::new(None)),shutdown:Arc::new(AtomicBool::new(false)),port:47812,allowed:vec![],ui:None,log_dir:std::env::temp_dir(),store:store.clone(),provider:Arc::new(StubRecordProvider),record_updates:Arc::new(Mutex::new(Default::default())),link:Default::default()};
    let api=Router::new().route("/api/meetings/:id/reconciliation",post(api_reconciliation_decision)).route("/api/meetings/:id/state",get(api_get_state)).with_state(app);
    async fn post_decision(api:&Router,body:Value)->axum::http::Response<axum::body::Body>{let req=axum::http::Request::builder().method("POST").uri("/api/meetings/rec-1/reconciliation").header("content-type","application/json").body(axum::body::Body::from(body.to_string())).unwrap();api.clone().oneshot(req).await.unwrap()}
    let decision=|id:&str,name:&str,action:&str,person:Option<u64>|{let mut d=json!({"id":id,"sessionParticipantId":id,"zoomName":name,"zoomNameAtDecision":name,"action":action,"createdAt":"2026-11-17T00:00:00Z"});if let Some(p)=person{d["personId"]=json!(p)}d};
    let state=json!({"mode":"roster","decisions":[]});
    let conflict=post_decision(&api,json!({"revision":0,"aliasesRevision":1,"value":state,"decision":decision("d0","Known Nick","match",Some(1))})).await;
    assert_eq!(conflict.status(),StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(store.get_state("rec-1").unwrap().0,0,"alias conflict must not write the decision");
    assert_eq!(store.get_doc("aliases").unwrap().1[0]["personId"],2);
    let matched=post_decision(&api,json!({"revision":0,"aliasesRevision":1,"value":state,"decision":decision("d1","New Nick","match",Some(1))})).await;
    assert_eq!(matched.status(),StatusCode::OK);
    assert_eq!(store.get_state("rec-1").unwrap().1["reconciliationDecisions"][0]["action"],"match");
    assert!(store.get_doc("aliases").unwrap().1.as_array().unwrap().iter().any(|a|a["zoomName"]=="New Nick"&&a["personId"]==1));
    let guest=post_decision(&api,json!({"revision":1,"aliasesRevision":2,"value":store.get_state("rec-1").unwrap().1,"decision":decision("d2","Guest Nick","guest",None)})).await;
    assert_eq!(guest.status(),StatusCode::OK);assert!(!store.get_doc("aliases").unwrap().1.as_array().unwrap().iter().any(|a|a["zoomName"]=="Guest Nick"));
    let split=post_decision(&api,json!({"revision":2,"aliasesRevision":3,"value":store.get_state("rec-1").unwrap().1,"decision":decision("d3","Shared Nick","split",None)})).await;
    assert_eq!(split.status(),StatusCode::OK);assert!(!store.get_doc("aliases").unwrap().1.as_array().unwrap().iter().any(|a|a["zoomName"]=="Shared Nick"));
    let revert=post_decision(&api,json!({"revision":3,"aliasesRevision":4,"value":store.get_state("rec-1").unwrap().1,"revertId":"d1"})).await;
    assert_eq!(revert.status(),StatusCode::OK);let stored=store.get_state("rec-1").unwrap().1;assert!(!stored["reconciliationDecisions"].as_array().unwrap().iter().any(|d|d["id"]=="d1"));assert!(store.get_doc("aliases").unwrap().1.as_array().unwrap().iter().any(|a|a["zoomName"]=="New Nick"),"revert leaves aliases in place");
    let stale=post_decision(&api,json!({"revision":0,"aliasesRevision":4,"value":state,"decision":decision("d4","Other Nick","guest",None)})).await;assert_eq!(stale.status(),StatusCode::CONFLICT);
    let _=matched.into_body().collect().await.unwrap();
}

#[test]
fn imported_attendflow_session_keeps_derived_rows_and_marks_read_only_source() {
    let bundle=json!({"version":3,"session":{"sessionId":"af-1","metadata":{"meetingId":"123456","meetingTopic":"Example","startedAt":1700000000000_u64},"derivedRows":[{"zoomName":"Ada","currentStatus":"present"}]}});
    let imported=normalize_imported_record(&bundle,0).unwrap();
    assert_eq!(imported["source"],"imported");
    assert_eq!(imported["meeting"]["source"],"imported");
    assert_eq!(imported["record"]["rows"][0]["zoomName"],"Ada");
}

#[test]
fn exported_iso_session_start_and_end_times_import_as_milliseconds() {
    let bundle=json!({"version":3,"exportedAt":"2026-11-17T00:00:00Z","session":{"sessionId":"exported-1","meetingId":"123","meetingTopic":"Example","startedAt":"2026-11-17T00:00:01.000Z","endedAt":"2026-11-17T00:01:01.000Z","derivedRows":[]}});
    let imported=normalize_imported_record(&bundle,0).unwrap();
    assert_eq!(imported["meeting"]["startedAt"],1_794_873_601_000_u64);
    assert_eq!(imported["meeting"]["endedAt"],1_794_873_661_000_u64);
}

#[test]
fn profile_validation_rejects_bad_wildcards_thresholds_and_default_removal() {
    let store=Arc::new(Store::open(std::env::temp_dir().join(format!("zc-profile-validation-{}.db",now()))).unwrap());
    let factory=json!([{"id":"default-all","name":"All participants","includedSubgroups":[],"lateAfterMinutes":10,"returnWindowMinutes":8,"meetingIds":[],"topicKeywords":[],"groupSortOrder":[],"active":true}]);
    store.put_doc("profiles",0,&factory,now()).unwrap().unwrap();
    let app=App{tx:broadcast::channel(2).0,tracker:Arc::new(Mutex::new(Tracker::default())),session:Arc::new(Mutex::new(None)),shutdown:Arc::new(AtomicBool::new(false)),port:47812,allowed:vec![],ui:None,log_dir:std::env::temp_dir(),store,provider:Arc::new(StubRecordProvider),record_updates:Arc::new(Mutex::new(Default::default())),link:Default::default()};
    assert!(validate_config(&app,"profiles",&json!([])).unwrap().contains("required"));
    let mut renamed=factory.clone();renamed[0]["name"]=json!("Renamed");
    assert!(validate_config(&app,"profiles",&renamed).unwrap().contains("name is locked"));
    let mut wildcard=factory.clone();wildcard[0]["includedSubgroups"]=json!(["team-**"]);
    assert!(validate_config(&app,"profiles",&wildcard).unwrap().contains("wildcard"));
    let mut threshold=factory;threshold[0]["lateAfterMinutes"]=json!(0);
    assert!(validate_config(&app,"profiles",&threshold).unwrap().contains("positive integer"));
}

#[tokio::test]
async fn example_data_conflict_aborts_all_three_config_documents() {
    let store=Arc::new(Store::open(std::env::temp_dir().join(format!("zc-example-conflict-{}.db",now()))).unwrap());
    let original=json!([{"personId":1,"canonicalName":"Existing"}]);store.put_doc("roster",0,&original,now()).unwrap().unwrap();
    let app=App{tx:broadcast::channel(2).0,tracker:Arc::new(Mutex::new(Tracker::default())),session:Arc::new(Mutex::new(None)),shutdown:Arc::new(AtomicBool::new(false)),port:47812,allowed:vec![],ui:None,log_dir:std::env::temp_dir(),store:store.clone(),provider:Arc::new(StubRecordProvider),record_updates:Arc::new(Mutex::new(Default::default())),link:Default::default()};
    let body=json!({"roster":[{"personId":2,"canonicalName":"Existing"}],"aliases":[{"zoomName":"Alias","personId":2}],"profiles":[{"id":"example-one","name":"Example"}]});
    let response=api_example_data_commit(State(app),HeaderMap::new(),Json(body)).await;
    assert_eq!(response.status(),StatusCode::CONFLICT);
    assert_eq!(store.get_doc("roster").unwrap().1,original);
    assert_eq!(store.get_doc("aliases").unwrap().1,json!([]));
    assert_eq!(store.get_doc("profiles").unwrap().1,json!([]));
}

#[tokio::test]
async fn profile_json_import_preview_and_commit_upsert_by_id() {
    use http_body_util::BodyExt; use tower::ServiceExt;
    let store=Arc::new(Store::open(std::env::temp_dir().join(format!("zc-profile-import-{}.db",now()))).unwrap());
    let factory=json!([{"id":"default-all","name":"All participants","includedSubgroups":[],"lateAfterMinutes":10,"returnWindowMinutes":8,"meetingIds":[],"topicKeywords":[],"groupSortOrder":[],"active":true}]);store.put_doc("profiles",0,&factory,now()).unwrap().unwrap();
    let app=App{tx:broadcast::channel(2).0,tracker:Arc::new(Mutex::new(Tracker::default())),session:Arc::new(Mutex::new(None)),shutdown:Arc::new(AtomicBool::new(false)),port:47812,allowed:vec![],ui:None,log_dir:std::env::temp_dir(),store:store.clone(),provider:Arc::new(StubRecordProvider),record_updates:Arc::new(Mutex::new(Default::default())),link:Default::default()};
    let api=Router::new().route("/api/profiles/import/preview",post(api_profiles_import_preview)).route("/api/profiles/import/commit",post(api_profiles_import_commit)).with_state(app);
    let incoming=json!({"profiles":[{"id":"p1","name":"Profile one","description":"x","includedSubgroups":["Team-*"],"lateAfterMinutes":5,"returnWindowMinutes":6,"meetingIds":[],"topicKeywords":[],"groupSortOrder":[],"active":true,"blacklist":["ignored"]}]});
    let req=axum::http::Request::builder().method("POST").uri("/api/profiles/import/preview").header("content-type","application/json").body(axum::body::Body::from(incoming.to_string())).unwrap();
    let response=api.clone().oneshot(req).await.unwrap();assert_eq!(response.status(),StatusCode::OK);let plan:Value=serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(plan["changes"][0]["action"],"add");assert!(plan["value"][1]["blacklist"].is_null());
    let req=axum::http::Request::builder().method("POST").uri("/api/profiles/import/commit").header("content-type","application/json").body(axum::body::Body::from(plan.to_string())).unwrap();
    assert_eq!(api.oneshot(req).await.unwrap().status(),StatusCode::OK);assert_eq!(store.get_doc("profiles").unwrap().1.as_array().unwrap().len(),2);
}

#[tokio::test]
async fn attendflow_record_import_preview_then_commit_stores_derived_rows() {
    use http_body_util::BodyExt; use tower::ServiceExt;
    let store=Arc::new(Store::open(std::env::temp_dir().join(format!("zc-record-import-{}.db",now()))).unwrap());
    let app=App{tx:broadcast::channel(2).0,tracker:Arc::new(Mutex::new(Tracker::default())),session:Arc::new(Mutex::new(None)),shutdown:Arc::new(AtomicBool::new(false)),port:47812,allowed:vec![],ui:None,log_dir:std::env::temp_dir(),store:store.clone(),provider:Arc::new(StubRecordProvider),record_updates:Arc::new(Mutex::new(Default::default())),link:Default::default()};
    let api=Router::new().route("/api/records/import/preview",post(api_records_import_preview)).route("/api/records/import/commit",post(api_records_import_commit)).route("/api/meetings/:id/record",get(api_record)).with_state(app);
    let input=json!({"version":3,"session":{"sessionId":"af-session","metadata":{"meetingId":"4321","meetingTopic":"Imported session","startedAt":1700000000000_u64},"derivedRows":[{"zoomName":"Ada","currentStatus":"present"}]}});
    let req=axum::http::Request::builder().method("POST").uri("/api/records/import/preview").header("content-type","application/json").body(axum::body::Body::from(input.to_string())).unwrap();
    let response=api.clone().oneshot(req).await.unwrap();let plan:Value=serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();assert_eq!(plan["plans"][0]["action"],"add");
    let req=axum::http::Request::builder().method("POST").uri("/api/records/import/commit").header("content-type","application/json").body(axum::body::Body::from(plan.to_string())).unwrap();assert_eq!(api.clone().oneshot(req).await.unwrap().status(),StatusCode::OK);
    let id=plan["plans"][0]["value"]["meeting"]["instanceId"].as_str().unwrap();assert!(store.meeting(id).unwrap().is_some());let req=axum::http::Request::builder().uri(format!("/api/meetings/{id}/record")).body(axum::body::Body::empty()).unwrap();let response=api.oneshot(req).await.unwrap();assert_eq!(response.status(),StatusCode::OK);let record:Value=serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();assert_eq!(record["rows"][0]["zoomName"],"Ada");
}
#[tokio::test]
async fn zoom_endpoints_report_status_and_reject_foreign_origins() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let store = Arc::new(Store::open(std::env::temp_dir().join(format!("zc-zoom-status-{}.db", now()))).unwrap());
    let app = App {
        tx: broadcast::channel(8).0,
        tracker: Arc::new(Mutex::new(Tracker::default())),
        session: Arc::new(Mutex::new(None)),
        shutdown: Arc::new(AtomicBool::new(false)),
        port: 47812,
        allowed: vec![],
        ui: None,
        log_dir: std::env::temp_dir(),
        store,
        provider: Arc::new(StubRecordProvider),
        record_updates: Arc::new(Mutex::new(Default::default())),
        link: Default::default(),
    };
    set_zoom(&app, ZoomStatus::new(ZoomState::SignedOut, "").with_reason("session_expired"));
    let api = Router::new()
        .route("/api/zoom", get(api_zoom_status))
        .route("/api/zoom/sign-in", post(api_zoom_sign_in))
        .route("/api/zoom/sign-out", post(api_zoom_sign_out))
        .with_state(app.clone());
    let req = |method: &str, uri: &str, origin: Option<&str>| {
        let mut b = axum::http::Request::builder().method(method).uri(uri);
        if let Some(o) = origin {
            b = b.header("origin", o);
        }
        b.body(axum::body::Body::empty()).unwrap()
    };
    let r = api.clone().oneshot(req("GET", "/api/zoom", None)).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let v: Value = serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["state"], "signed_out");
    assert_eq!(v["reason"], "session_expired");
    // POSTs need this app's Origin: missing or foreign origins never reach the token store.
    for uri in ["/api/zoom/sign-in", "/api/zoom/sign-out"] {
        for origin in [None, Some("https://evil.test")] {
            let r = api.clone().oneshot(req("POST", uri, origin)).await.unwrap();
            assert_eq!(r.status(), StatusCode::FORBIDDEN, "{uri} {origin:?}");
        }
    }
    assert_eq!(app.link.status.lock().unwrap().state, ZoomState::SignedOut);
    assert!(!app.link.sign_out.load(Ordering::SeqCst));
}
