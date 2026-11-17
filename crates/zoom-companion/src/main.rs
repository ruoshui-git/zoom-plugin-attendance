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

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

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
            "  id={:<10} host={} me={} attendee={} muted={} video={} waiting={}  {}",
            u.user_id, f.host() as u8, f.myself() as u8, f.attendee() as u8, f.audio_muted() as u8, f.video_on() as u8,
            f.in_waiting_room() as u8, u.name
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
