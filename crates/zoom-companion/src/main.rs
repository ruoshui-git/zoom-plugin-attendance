//! zoom-companion — development CLI for the Zoom Plugin SDK baseline.
//!
//! Commands:
//!   doctor              environment / SDK / DLL / Zoom Workplace / auth checks
//!   selftest            Rust<->C++ interop self-tests (no Zoom needed)
//!   config init|show    non-secret config (%APPDATA%\zoom-companion\config.toml)
//!   auth login|status|logout   OAuth PKCE via loopback, DPAPI token store
//!   connect             init SDK, authenticate over IPC, print events
//!   participants        (in a meeting) GetParticipantsList / GetMySelf / meeting info

mod config;
mod serve;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};
use std::io::{BufRead, Write};
use std::sync::mpsc;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use zoom_auth::{AuthError, TokenStore};
use zoom_bridge::status::{self, MeetingInstance};
use zoom_bridge::{Event, Participant, Session, SessionConfig};

use crate::config::Config;

#[derive(Parser)]
#[command(name = "zoom-companion", version, about = "Zoom Plugin SDK development companion")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check toolchain artifacts, SDK DLLs, Zoom Workplace and auth state.
    Doctor,
    /// Run Rust<->C++ interop self-tests (UTF-8/UTF-16, callbacks, exceptions).
    Selftest,
    /// Manage the non-secret configuration file.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// OAuth (Authorization Code + PKCE, loopback redirect).
    Auth {
        #[command(subcommand)]
        cmd: AuthCmd,
    },
    /// Initialise the SDK, authenticate with Zoom Workplace over IPC, print events.
    Connect {
        /// How long to stay connected and print events.
        #[arg(long, default_value_t = 20)]
        seconds: u64,
    },
    /// Read the participant list of the CURRENT meeting (only use in test meetings).
    Participants {
        /// Seconds to wait for IPC before giving up.
        #[arg(long, default_value_t = 20)]
        wait: u64,
        /// Keep printing user-status events for this many seconds afterwards.
        #[arg(long, default_value_t = 0)]
        watch: u64,
    },
    /// Interactive meeting experiment console; use only in an owner controlled test meeting.
    Lab {
        #[arg(long)] log: Option<std::path::PathBuf>,
        #[arg(long)] script: Option<std::path::PathBuf>,
        #[arg(long)] exec: Option<String>,
    },
    /// Run the local companion HTTP and WebSocket server.
    Serve {
        #[arg(long, default_value_t = 47812)] port: u16,
        #[arg(long)] ui: Option<std::path::PathBuf>,
        #[arg(long)] open: bool,
        #[arg(long = "allow-origin")] allow_origin: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Write config with the Marketplace PUBLIC client ID (PKCE).
    Init {
        #[arg(long)]
        client_id: String,
        #[arg(long, default_value = config::DEFAULT_REDIRECT_URI)]
        redirect_uri: String,
    },
    /// Print the effective configuration.
    Show,
}

#[derive(Subcommand)]
enum AuthCmd {
    /// Open the browser, sign in, store tokens with DPAPI.
    Login {
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    /// Show whether tokens exist (never prints them).
    Status,
    /// Delete stored tokens.
    Logout,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if let Err(e) = run(Cli::parse()) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.cmd {
        Cmd::Doctor => doctor(),
        Cmd::Selftest => selftest(),
        Cmd::Config { cmd } => match cmd {
            ConfigCmd::Init { client_id, redirect_uri } => {
                zoom_auth::loopback::parse_redirect(&redirect_uri)?;
                let cfg = Config { client_id: client_id.trim().into(), redirect_uri, ..Config::default() };
                let p = cfg.save()?;
                println!("wrote {}", p.display());
                Ok(())
            }
            ConfigCmd::Show => {
                let cfg = Config::load()?;
                println!("config file : {}", config::config_path()?.display());
                println!("client_id   : {}", if cfg.client_id.is_empty() { "<unset>" } else { &cfg.client_id });
                println!("redirect_uri: {}", cfg.redirect_uri);
                println!("domain      : {}", cfg.domain);
                Ok(())
            }
        },
        Cmd::Auth { cmd } => auth(cmd),
        Cmd::Connect { seconds } => connect(seconds),
        Cmd::Participants { wait, watch } => participants(wait, watch),
        Cmd::Lab { log, script, exec } => lab(log, script, exec),
        Cmd::Serve { port, ui, open, allow_origin } => serve::run(port, ui, open, allow_origin),
    }
}

fn open_browser(url: &str) -> Result<(), AuthError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let w = |s: &str| OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let (op, target) = (w("open"), w(url));
    // SAFETY: NUL-terminated wide strings; null HWND/params/dir are allowed.
    let h = unsafe {
        ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL)
    };
    if (h as isize) <= 32 {
        return Err(AuthError::Config(format!("could not open browser; open this URL manually:\n{url}")));
    }
    Ok(())
}

fn auth(cmd: AuthCmd) -> Result<()> {
    let store = TokenStore::open_default()?;
    match cmd {
        AuthCmd::Login { timeout } => {
            let cfg = Config::load()?;
            if cfg.client_id.is_empty() {
                bail!("client_id not configured: run `zoom-companion config init --client-id <ID>`");
            }
            println!("Opening the browser for Zoom sign-in (redirect {}).", cfg.redirect_uri);
            println!("Sign in as the SAME user that is signed in to Zoom Workplace.");
            let t = zoom_auth::login_interactive(&cfg.oauth(), &store, open_browser, Duration::from_secs(timeout))?;
            println!("Signed in. Scopes granted: {}", t.scope);
            if !t.has_scope(zoom_auth::PLUGIN_SDK_SCOPE) {
                println!("WARNING: token lacks {} — add it to the Marketplace app scopes.", zoom_auth::PLUGIN_SDK_SCOPE);
            }
            println!("Tokens stored (DPAPI-encrypted) at {}", store.path().display());
            Ok(())
        }
        AuthCmd::Status => {
            match store.load()? {
                None => println!("not signed in ({})", store.path().display()),
                Some(t) => {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
                    println!("signed in (client_id {})", t.client_id);
                    println!("scopes   : {}", t.scope);
                    if t.expires_at > now {
                        println!("access   : valid for {} min", (t.expires_at - now) / 60);
                    } else {
                        println!("access   : expired (refresh token {})", if t.refresh_token.is_some() { "present" } else { "missing" });
                    }
                }
            }
            Ok(())
        }
        AuthCmd::Logout => {
            println!("{}", if store.clear()? { "tokens deleted" } else { "no tokens stored" });
            Ok(())
        }
    }
}

fn zoom_workplace_running() -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq Zoom.exe", "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Zoom.exe"))
        .unwrap_or(false)
}

fn doctor() -> Result<()> {
    let mut problems = 0;
    let mut check = |ok: bool, what: &str, detail: String| {
        println!("[{}] {what}: {detail}", if ok { " ok " } else { "FAIL" });
        if !ok {
            problems += 1;
        }
    };
    check(
        zoom_bridge::adapter_abi_version() == zoom_bridge::RUST_ABI_VERSION,
        "adapter ABI",
        format!("C++ {} / Rust {}", zoom_bridge::adapter_abi_version(), zoom_bridge::RUST_ABI_VERSION),
    );
    let linked = zoom_bridge::sdk_linked();
    check(
        linked,
        "SDK linked",
        match zoom_bridge::build_time_sdk_root() {
            Some(r) => r.display().to_string(),
            None => "stub build (run scripts\\extract-sdk.ps1 and rebuild)".into(),
        },
    );
    if linked {
        check(
            zoom_bridge::sdk_runtime_beside_exe().is_some(),
            "SDK runtime beside exe",
            match zoom_bridge::sdk_runtime_beside_exe() {
                Some(d) => d.display().to_string(),
                None => "missing: run scripts\\build.ps1 (copies SDK bin\\ next to the exe)".into(),
            },
        );
        match zoom_bridge::ensure_sdk_dll_loaded(zoom_bridge::build_time_sdk_bin_dir().as_deref()) {
            Ok(p) => check(true, "SDK DLL load", p.display().to_string()),
            Err(e) => check(false, "SDK DLL load", e.to_string()),
        }
    }
    check(selftest_inner().is_ok(), "interop selftest", "UTF-8/UTF-16, callbacks, exceptions".into());
    check(zoom_workplace_running(), "Zoom Workplace running", "Zoom.exe process".into());
    let cfg = Config::load()?;
    check(!cfg.client_id.is_empty(), "client_id configured", config::config_path()?.display().to_string());
    match TokenStore::open_default()?.load() {
        Ok(Some(t)) => check(
            t.has_scope(zoom_auth::PLUGIN_SDK_SCOPE),
            "OAuth tokens",
            format!("stored; scope {} {}", zoom_auth::PLUGIN_SDK_SCOPE, if t.has_scope(zoom_auth::PLUGIN_SDK_SCOPE) { "present" } else { "MISSING" }),
        ),
        Ok(None) => check(false, "OAuth tokens", "not signed in (zoom-companion auth login)".into()),
        Err(e) => check(false, "OAuth tokens", e.to_string()),
    }
    if problems > 0 {
        bail!("{problems} check(s) failed");
    }
    Ok(())
}

fn selftest_inner() -> Result<()> {
    use zoom_bridge::selftest as st;
    for s in ["张三", "José", "👩‍💻", ""] {
        if st::utf8_roundtrip(s)? != s {
            bail!("utf8 roundtrip mismatch for {s:?}");
        }
    }
    if st::utf8_roundtrip_bytes(&[0xff, 0xfe]).is_ok() {
        bail!("invalid UTF-8 was accepted");
    }
    match st::callback_roundtrip("回调 ✓")? {
        Event::SelfTest { text, .. } if text == "回调 ✓" => {}
        other => bail!("unexpected callback event {other:?}"),
    }
    match st::users_roundtrip(&["李雷", "韩梅梅"])? {
        Event::UserStatus { users, .. } if users.len() == 2 && users[1].name == "韩梅梅" => {}
        other => bail!("unexpected users event {other:?}"),
    }
    if !matches!(st::exception_boundary(), Err(zoom_bridge::BridgeError::Exception)) {
        bail!("C++ exception not converted");
    }
    Ok(())
}

fn selftest() -> Result<()> {
    selftest_inner()?;
    println!("interop selftest passed (sdk_linked={})", zoom_bridge::sdk_linked());
    Ok(())
}

fn describe(ev: &Event) -> String {
    match ev {
        Event::AuthResult { code, thread_id } => format!("OnAuthResult {} [tid {thread_id}]", status::auth_result_name(*code)),
        Event::IpcStatus { code, message, thread_id } => {
            format!("OnIPCConnectStatusChanged {} {message:?} [tid {thread_id}]", status::ipc_status_name(*code))
        }
        Event::MeetingStatus { code, thread_id } => {
            format!("OnMeetingStatusChanged {} [tid {thread_id}]", status::meeting_status_name(*code))
        }
        Event::UserStatus { action, error_code, instance, users, thread_id } => format!(
            "OnUserStatusChanged {} {} instance={instance} users={} [tid {thread_id}]",
            status::user_action_name(*action),
            status::proxy_error_name(*error_code),
            users.len()
        ),
        Event::Other { name, code, value, instance, thread_id } => {
            format!("{name} code={code} value={value} instance={instance} [tid {thread_id}]")
        }
        other => format!("{other:?}"),
    }
}

fn start_session() -> Result<(Session, std::sync::mpsc::Receiver<Event>)> {
    if !zoom_bridge::sdk_linked() {
        bail!("built without the Zoom Plugin SDK; run scripts\\extract-sdk.ps1 and rebuild");
    }
    let cfg = Config::load()?;
    if cfg.client_id.is_empty() {
        bail!("client_id not configured");
    }
    if zoom_bridge::sdk_runtime_beside_exe().is_none() {
        bail!(
            "the Zoom SDK runtime (bin\\*) must be next to {} - run scripts\\build.ps1 or scripts\\package.ps1",
            std::env::current_exe()?.display()
        );
    }
    let store = TokenStore::open_default()?;
    let tokens = zoom_auth::valid_tokens(&store, &cfg.client_id)?;
    if !zoom_workplace_running() {
        log::warn!("Zoom.exe does not appear to be running; IPC will likely time out");
    }
    log::info!("SDK thread will run on a dedicated message-pump thread; main tid {}", current_tid());
    let (session, rx) = Session::start(SessionConfig {
        access_token: tokens.access_token.expose().to_string(),
        domain: cfg.domain.clone(),
        dll_fallback_dir: None,
    })
    .context("starting Zoom SDK session")?;
    Ok((session, rx))
}

fn current_tid() -> u32 {
    // SAFETY: trivially safe.
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
}

/// Waits for auth + IPC connected. Returns false on failure/timeout.
fn wait_connected(rx: &std::sync::mpsc::Receiver<Event>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let (mut auth_ok, mut ipc_ok) = (false, false);
    while Instant::now() < deadline && !(auth_ok && ipc_ok) {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(ev) => {
                println!("event: {}", describe(&ev));
                match ev {
                    Event::AuthResult { code, .. } => {
                        if code != status::AUTH_RESULT_SUCCESS {
                            return false;
                        }
                        auth_ok = true;
                    }
                    Event::IpcStatus { code, .. } => {
                        if code == status::IPC_STATUS_CONNECTED {
                            ipc_ok = true;
                        } else if code != 0 {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return false,
        }
    }
    auth_ok && ipc_ok
}

fn connect(seconds: u64) -> Result<()> {
    let (session, rx) = start_session()?;
    let connected = wait_connected(&rx, Duration::from_secs(seconds.max(5)));
    if connected {
        println!("CONNECTED: auth succeeded and IPC is connected to Zoom Workplace.");
        match session.meeting_status(MeetingInstance::Default) {
            Ok(s) => println!("GetMeetingStatus: {}", status::meeting_status_name(s)),
            Err(e) => println!(
                "GetMeetingStatus: {e} (observed outside meetings: GetMeetingToolkit returns null until in a meeting)"
            ),
        }
        let until = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < until {
            if let Ok(ev) = rx.recv_timeout(Duration::from_millis(200)) {
                println!("event: {}", describe(&ev));
            }
        }
    }
    session.shutdown()?;
    println!("SDK uninitialised.");
    if !connected {
        bail!("did not reach AUTH_RESULT_SUCCESS + IPC_STATUS_CONNECTED");
    }
    Ok(())
}

fn print_users(label: &str, users: &[Participant]) {
    println!("{label}: {} user(s)", users.len());
    for u in users {
        let f = u.flags;
        println!(
            "  id={:<10} host={} me={} attendee={} muted={} video={} spotlit={} raised={} talking={} waiting={} audio={}  {}",
            u.user_id, f.host() as u8, f.myself() as u8, f.attendee() as u8, f.audio_muted() as u8, f.video_on() as u8,
            f.spotlighted() as u8, f.raised_hand() as u8, f.talking() as u8, f.in_waiting_room() as u8, u.audio_type, u.name
        );
    }
}

fn participants(wait: u64, watch: u64) -> Result<()> {
    let (session, rx) = start_session()?;
    if !wait_connected(&rx, Duration::from_secs(wait)) {
        session.shutdown()?;
        bail!("not connected to Zoom Workplace");
    }
    let inst = MeetingInstance::Default;
    // Observed 2026-11-17: GetMeetingToolkit is null right after IPC_STATUS_CONNECTED;
    // SYNC_ALL + MEETING_STATUS_INMEETING arrive a few hundred ms later. Wait for them.
    let ready_by = Instant::now() + Duration::from_secs(5);
    while Instant::now() < ready_by {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(100)) {
            println!("event: {}", describe(&ev));
            if let Event::UserStatus { users, .. } = &ev {
                print_users("  users", users);
            }
            if matches!(ev, Event::MeetingStatus { .. }) {
                break;
            }
        }
    }
    let st = session.meeting_status(inst)?;
    println!("meeting status: {}", status::meeting_status_name(st));
    match session.meeting_info(inst) {
        Ok(i) => println!(
            "meeting: type={} number={} topic={:?} ({})",
            status::meeting_type_name(i.meeting_type),
            i.meeting_number,
            i.topic,
            status::proxy_error_name(i.error_code)
        ),
        Err(e) => println!("GetMeetingProperty: {e}"),
    }
    match session.myself(inst) {
        Ok(me) => print_users("GetMySelf", &[me]),
        Err(e) => println!("GetMySelf: {e}"),
    }
    match session.participants(inst) {
        Ok(list) => print_users("GetParticipantsList", &list),
        Err(e) => println!("GetParticipantsList: {e}"),
    }
    let until = Instant::now() + Duration::from_secs(watch);
    while Instant::now() < until {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(200)) {
            println!("event: {}", describe(&ev));
            if let Event::UserStatus { users, .. } = &ev {
                print_users("  users", users);
            }
        }
    }
    session.shutdown()?;
    Ok(())
}

fn lab(log_path: Option<std::path::PathBuf>, script: Option<std::path::PathBuf>, exec: Option<String>) -> Result<()> {
    use std::collections::BTreeMap;
    let script_lines = script.map(|p| -> Result<Vec<String>> {
        Ok(std::io::BufReader::new(std::fs::File::open(&p).with_context(|| format!("opening script {}", p.display()))?)
            .lines().collect::<std::io::Result<Vec<_>>>()?)
    }).transpose()?;
    let path = log_path.unwrap_or_else(|| {
        let root = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        root.join("zoom-companion").join("lab").join(format!("lab-{}.jsonl", chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ")))
    });
    if let Some(p) = path.parent() { std::fs::create_dir_all(p).context("creating lab log directory")?; }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).with_context(|| format!("opening {}", path.display()))?;
    println!("Lab log: {}", path.display());
    let start = Instant::now();
    let mut emit = |kind: &str, payload: serde_json::Value| -> Result<()> {
        let row = serde_json::json!({"timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), "monotonic_ms": start.elapsed().as_millis(), "kind": kind, "payload": payload});
        let line = serde_json::to_string(&row)?;
        println!("{line}");
        writeln!(file, "{line}")?;
        file.flush()?;
        Ok(())
    };

    let (session, rx) = start_session()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let (mut auth_ok, mut ipc_ok) = (false, false);
    while Instant::now() < deadline && !(auth_ok && ipc_ok) {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(100)) {
            emit("event", lab_event_payload(&ev)?)?;
            match ev { Event::AuthResult { code, .. } => auth_ok = code == status::AUTH_RESULT_SUCCESS,
                Event::IpcStatus { code, .. } => ipc_ok = code == status::IPC_STATUS_CONNECTED, _ => {} }
        }
    }
    if !(auth_ok && ipc_ok) { session.shutdown()?; bail!("did not reach AUTH_RESULT_SUCCESS + IPC_STATUS_CONNECTED"); }
    emit("connected", serde_json::json!({"message":"SDK authenticated and IPC connected"}))?;
    // Toolkits are null for a few hundred ms after IPC_STATUS_CONNECTED; wait for the
    // meeting status / SYNC_ALL burst (or 5 s when not in a meeting) before running commands.
    let mut early: Vec<Event> = Vec::new();
    let ready_by = Instant::now() + Duration::from_secs(5);
    while Instant::now() < ready_by {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(100)) {
            let is_status = matches!(ev, Event::MeetingStatus { .. });
            early.push(ev);
            if is_status { break; }
        }
    }

    let (line_tx, line_rx) = mpsc::channel::<Option<String>>();
    if let Some(lines) = script_lines {
        for line in lines { let _ = line_tx.send(Some(line)); }
        let _ = line_tx.send(None);
    } else if let Some(commands) = exec {
        for line in commands.split(';') { let _ = line_tx.send(Some(line.trim().to_owned())); }
        let _ = line_tx.send(None);
    } else {
        println!("Type help for commands; quit exits.");
        std::thread::spawn(move || { for line in std::io::stdin().lock().lines().flatten() { if line_tx.send(Some(line)).is_err() { break; } } let _ = line_tx.send(None); });
    }
    let inst = MeetingInstance::Default;
    let mut ledger: BTreeMap<u32, (String, String, Option<String>)> = BTreeMap::new();
    let mut previous: BTreeMap<u32, String> = BTreeMap::new();
    let mut poll_every: Option<Duration> = None;
    let mut next_poll = Instant::now();
    let mut done = false;
    let mut sleep_until: Option<(Instant, u64)> = None;
    while !done {
        for ev in early.drain(..).chain(std::iter::from_fn(|| rx.try_recv().ok())) {
            emit("event", lab_event_payload(&ev)?)?;
            if let Event::UserStatus { action, users, .. } = &ev {
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                for u in users {
                    if *action == 1 || *action == 2 { ledger.entry(u.user_id).and_modify(|v| {v.0=u.name.clone();v.2=None;}).or_insert((u.name.clone(), now.clone(), None)); }
                    if *action == 3 { let e = ledger.entry(u.user_id).or_insert((u.name.clone(), now.clone(), None)); e.0=u.name.clone(); e.2=Some(now.clone()); }
                }
            }
        }
        if poll_every.is_some() && Instant::now() >= next_poll {
            match session.participants(inst) { Ok(users) => { let current: BTreeMap<_,_> = users.iter().map(|u|(u.user_id,u.name.clone())).collect();
                let added: Vec<_> = current.iter().filter(|(id,_)| !previous.contains_key(id)).map(|(id,n)| serde_json::json!({"user_id":id,"name":n})).collect();
                let removed: Vec<_> = previous.iter().filter(|(id,_)| !current.contains_key(id)).map(|(id,n)| serde_json::json!({"user_id":id,"name":n})).collect();
                let renamed: Vec<_> = current.iter().filter_map(|(id,n)| previous.get(id).filter(|old| *old != n).map(|old| serde_json::json!({"user_id":id,"from":old,"to":n}))).collect();
                emit("poll_diff", serde_json::json!({"added":added,"removed":removed,"renamed":renamed,"users":users}))?; previous=current;
            }, Err(e)=>{emit("command_result",serde_json::json!({"command":"poll","error":e.to_string()}))?;} }
            next_poll = Instant::now() + poll_every.unwrap();
        }
        if let Some((until, secs)) = sleep_until {
            if Instant::now() < until { std::thread::sleep(Duration::from_millis(20)); continue; }
            sleep_until = None;
            emit("command_result", serde_json::json!({"command":"sleep","result":{"slept_seconds":secs}}))?;
        }
        match line_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(None) => done = true,
            Ok(Some(line)) => {
                let line=line.trim(); if line.is_empty(){continue}
                let mut parts=line.split_whitespace(); let cmd=parts.next().unwrap_or(""); let args:Vec<_>=parts.collect();
                let safe_line=if cmd=="join" && args.len()>=2 {format!("join {} <redacted-password> {}",args[0],args[2..].join(" "))}else{line.to_owned()};
                emit("command",serde_json::json!({"line":safe_line}))?;
                let res: Result<serde_json::Value> = (|| { Ok(match cmd {
                    "help" => "status info me list attendees spotlit waiting user <id> role <id> roles start <meetingNumber> join <meetingNumber> <password> <displayName...> makehost <id> reclaim cohost <id> uncohost <id> admit <id> admitall wr [on|off] unread leave end poll <seconds> sleep <seconds> note <text> quit".into(),
                    "status" => format!("{}",status::meeting_status_name(session.meeting_status(inst)?)).into(),
                    "info" => serde_json::to_value(session.meeting_info(inst)?)?,
                    "me" => serde_json::to_value(session.myself(inst)?)?,
                    "list" => serde_json::to_value(session.participants(inst)?)?,
                    "attendees" => serde_json::to_value(session.attendees(inst)?)?,
                    "spotlit" => serde_json::to_value(session.spotlighted(inst)?)?,
                    "waiting" => serde_json::to_value(session.waiting_room(inst)?)?,
                    "user" => serde_json::to_value(session.user(inst, resolve_id(&session,inst,&args)?)?)?,
                    "role" => {session.user_role(inst,resolve_id(&session,inst,&args)?)?; serde_json::json!({"message":"role request submitted"})},
                    "roles" => {for u in session.participants(inst)? {session.user_role(inst,u.user_id)?;} serde_json::json!({"message":"role requests submitted"})},
                    "start" => {session.start_meeting(args.first().context("missing meeting number")?.parse()?,true,true)?; serde_json::json!({"message":"start request submitted"})},
                    "join" => {if args.len()<3 {anyhow::bail!("usage: join <meetingNumber> <password> <displayName...>")} session.join_meeting(args[2..].join(" "),args[0].parse()?,args[1].into())?; serde_json::json!({"message":"join request submitted"})},
                    "makehost" => {session.make_host(inst,resolve_id(&session,inst,&args)?)?;serde_json::json!({"message":"request submitted"})}, "reclaim"=>{session.reclaim_host(inst)?;serde_json::json!({"message":"request submitted"})},
                    "cohost"=>{session.assign_cohost(inst,resolve_id(&session,inst,&args)?)?;serde_json::json!({"message":"request submitted"})}, "uncohost"=>{session.revoke_cohost(inst,resolve_id(&session,inst,&args)?)?;serde_json::json!({"message":"request submitted"})},
                    "unread"=>{session.unread_chat_count(inst)?;serde_json::json!({"message":"request submitted"})},
                    "admit"=>{session.admit(inst,resolve_id(&session,inst,&args)?)?;serde_json::json!({"message":"request submitted"})},
                    "admitall"=>{session.admit_all(inst)?;serde_json::json!({"message":"request submitted"})},
                    "wr"=>{match args.first().copied() { Some("on")=>session.enable_waiting_room(inst,true)?, Some("off")=>session.enable_waiting_room(inst,false)?, _=>session.is_waiting_room_on(inst)? } serde_json::json!({"message":"request submitted"})}, "leave"=>{session.leave_meeting(inst)?;serde_json::json!({"message":"request submitted"})}, "end"=>{session.end_meeting(inst)?;serde_json::json!({"message":"request submitted"})},
                    "poll"=>{let n:u64=args.first().context("missing seconds")?.parse()?; poll_every=if n==0{None}else{Some(Duration::from_secs(n))};next_poll=Instant::now();serde_json::json!({"poll_seconds":n})},
                    "note"=>{let text=line.strip_prefix("note").unwrap_or("").trim(); emit("note",serde_json::json!({"text":text}))?; serde_json::json!({"text":text})},
                    "quit"=>{done=true;serde_json::json!({"message":"quitting"})}, "sleep"=>{let secs:u64=args.first().context("missing seconds")?.parse()?; sleep_until=Some((Instant::now()+Duration::from_secs(secs),secs)); serde_json::json!({"sleeping_seconds":secs})},
                    _=>anyhow::bail!("unknown command: {cmd}")
                }) })();
                match res {Ok(output)=>emit("command_result",serde_json::json!({"command":cmd,"result":output}))?,Err(e)=>emit("command_result",serde_json::json!({"command":cmd,"error":e.to_string()}))?}
            }
            Err(mpsc::RecvTimeoutError::Timeout)=>{}, Err(mpsc::RecvTimeoutError::Disconnected)=>done=true,
        }
    }
    let rows:Vec<_>=ledger.iter().map(|(id,(name,first,left))|serde_json::json!({"user_id":id,"name":name,"first_seen":first,"left":left})).collect();
    println!("Join/leave ledger:"); for (id,(name,first,left)) in &ledger {println!("{id:<12} {name:<32} first_seen={first} left={}",left.as_deref().unwrap_or("still present"));}
    emit("ledger_summary",serde_json::json!({"users":rows}))?;
    session.shutdown()?;
    Ok(())
}

fn parse_id(args: &[&str]) -> Result<u32> { Ok(args.first().context("missing user id")?.parse()?) }

/// Resolves `<id>` or `@Display Name` (exact match; participants first, then waiting room).
fn resolve_id(session: &Session, inst: MeetingInstance, args: &[&str]) -> Result<u32> {
    let first = *args.first().context("missing user id or @name")?;
    let Some(rest) = first.strip_prefix('@') else { return parse_id(args) };
    let name = std::iter::once(rest).chain(args[1..].iter().copied()).collect::<Vec<_>>().join(" ");
    let mut pool = session.participants(inst).unwrap_or_default();
    pool.extend(session.waiting_room(inst).unwrap_or_default());
    let hits: Vec<_> = pool.iter().filter(|u| u.name == name).map(|u| u.user_id).collect();
    match hits.as_slice() {
        [id] => Ok(*id),
        [] => bail!("no participant named {name:?}"),
        many => Ok(*many.iter().max().unwrap()), // newest id wins (ids grow monotonically)
    }
}

fn lab_event_payload(ev: &Event) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(ev)?;
    if let Event::AsyncResult { operation, value: role, .. } = ev {
        if operation == "GetUserRoleByUserID" {
            value["role_name"] = serde_json::Value::String(status::user_role_name(*role as i32));
        }
    }
    Ok(value)
}
