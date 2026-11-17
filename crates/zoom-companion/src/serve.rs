use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::record::{RecordProvider, StubRecordProvider};
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
}
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
        }
        self.history.push_back(e.clone());
        while self.history.len() > 10000 {
            self.history.pop_front();
        }
        Some(e.wire())
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
            r.meeting_number == number && now().saturating_sub(r.last_seen_at) <= 15 * 60 * 1000
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
                if let Some(v) = self.emit(
                    "hand",
                    json!({"userId":value as u32,"raised":name=="OnUserRaisedHand"}),
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
            provider: Arc::new(StubRecordProvider),
            record_updates: Arc::new(Mutex::new(std::collections::HashMap::new())),
            log_dir,
        };
        import_logs(&app, &app.log_dir);
        if fake {
            seed_fake_meeting(&app);
        }
        let worker = app.clone();
        let stop = app.shutdown.clone();
        let worker_task = if fake {
            None
        } else {
            Some(tokio::task::spawn_blocking(move || sdk_loop(worker)))
        };
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
        let url = format!("http://127.0.0.1:{port}/");
        if open {
            open_browser(&url)?;
        }
        let router = Router::new()
            .route("/bridge", get(ws))
            .route("/api/config/:key", get(api_get_config).put(api_put_config))
            .route("/api/meetings", get(api_meetings))
            .route(
                "/api/meetings/:id",
                get(api_meeting).delete(api_delete_meeting),
            )
            .route("/api/meetings/:id/events", get(api_events))
            .route("/api/meetings/:id/record", get(api_record))
            .route(
                "/api/meetings/:id/state",
                get(api_get_state).put(api_put_state),
            )
            .route("/api/import/attendflow-backup", post(api_import_backup))
            .fallback(get(root))
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
        event_loop::{ControlFlow, EventLoop},
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
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("AttendFlow Desktop")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 820.0))
        .build(&event_loop)?;
    let _webview = wry::WebViewBuilder::new().with_url(&url).build(&window)?;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
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
        match start_session() {
            Ok((session, rx)) => {
                let rx = ChannelEventSource(rx);
                *a.session.lock().unwrap() = Some(session);
                let until = std::time::Instant::now() + Duration::from_secs(30);
                let (mut auth, mut ipc, mut retry) = (false, false, false);
                while std::time::Instant::now() < until
                    && !(auth && ipc)
                    && !retry
                    && !a.shutdown.load(Ordering::SeqCst)
                {
                    if let Ok(e) = rx.recv_event(Duration::from_millis(200)) {
                        match &e {
                            Event::AuthResult { code, .. } => {
                                auth = *code == status::AUTH_RESULT_SUCCESS;
                                if !auth {
                                    retry = true
                                }
                            }
                            Event::IpcStatus { code, .. } => {
                                ipc = *code == status::IPC_STATUS_CONNECTED;
                                if *code == 1 || *code == 3 {
                                    retry = true
                                }
                            }
                            _ => {}
                        }
                        let out = a.tracker.lock().unwrap().apply(e, None);
                        publish(&a, out);
                    }
                }
                if auth && ipc && !retry {
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
                        if a.shutdown.load(Ordering::SeqCst) {
                            break;
                        }
                        let no_meeting = a.tracker.lock().unwrap().meeting.is_none();
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
                log::warn!("serve session unavailable: {e}");
                let event = Event::IpcStatus {
                    code: 1,
                    message: e.to_string(),
                    thread_id: 0,
                };
                let out = a.tracker.lock().unwrap().apply(event, None);
                publish(&a, out);
            }
        }
        for _ in 0..50 {
            if a.shutdown.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    if let Some(s) = a.session.lock().unwrap().take() {
        let _ = s.shutdown();
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
    fn origin_whitelist() {
        assert!(origin_allowed(Some("http://localhost:47812"), 47812, &[]));
        assert!(!origin_allowed(Some("https://evil.test"), 47812, &[]));
        assert!(origin_allowed(
            Some("http://localhost:5173"),
            47812,
            &["http://localhost:5173".into()]
        ));
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
    match a.store.put_doc(&key, rev, &val, now()) {
        Ok(Ok(revision)) => Json(json!({"revision":revision,"value":val})).into_response(),
        Ok(Err(actual)) => Json(json!({"error":"stale_revision","revision":actual}))
            .into_response_with_status(StatusCode::CONFLICT),
        Err(e) => api_error(e),
    }
}
async fn api_meetings(State(a): State<App>, headers: HeaderMap) -> Response {
    if let Some(r) = api_origin(&a, &headers) {
        return r;
    }
    match a.store.meetings(){Ok(rows)=>Json(rows.iter().map(|m|json!({"instanceId":m.instance_id,"meetingNumber":m.meeting_number,"topic":m.topic,"startedAt":m.started_at,"endedAt":m.ended_at,"endReason":m.end_reason,"isHost":m.is_host,"lastSeq":m.last_seq})).collect::<Vec<_>>()).into_response(),Err(e)=>api_error(e)}
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
    Json(
        a.provider
            .build_record(&meeting, &vals, &docs, &state, now()),
    )
    .into_response()
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
    ];
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
    };
    let api = Router::new()
        .route("/api/import/attendflow-backup", post(api_import_backup))
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
    let backup = json!({"version":3,"rosterPeople":[{"personId":1}],"aliasMappings":[{"zoomName":"Ada"}],"attendanceProfiles":[{"id":"default"}],"sessions":[{},{}]});
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
