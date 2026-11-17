//! Owns the (process-global) Zoom Plugin SDK session on a dedicated thread.
//!
//! Why a dedicated thread with a Win32 message pump: the SDK is a COM/IPC
//! style client library whose official sample is an MFC dialog app, i.e. it
//! runs on a thread with a message loop. Until verified otherwise against the
//! real SDK, we assume callbacks may be dispatched through that thread's
//! message queue, so the thread that calls `InitZMToolSuite` keeps pumping.
//! Callers interact via channels only; no SDK call happens on other threads.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    QS_ALLINPUT,
};

use crate::events::{meeting_info_sink, trampoline, users_sink, Event, MeetingInfo, Participant};
use crate::status::MeetingInstance;
use crate::{check, dll, ffi, BridgeError};

static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Parameters for [`Session::start`].
pub struct SessionConfig {
    /// OAuth access token with `plugin_sdk:read:connection_meta`.
    /// Consumed and wiped by the SDK thread after `StartToolSuiteAuth`.
    pub access_token: String,
    /// Usually `https://zoom.us`.
    pub domain: String,
    /// Directory to load the SDK DLL from if it isn't beside the executable.
    pub dll_fallback_dir: Option<PathBuf>,
}

type Reply<T> = Sender<Result<T, BridgeError>>;

enum Command {
    Shutdown,
    MeetingStatus(MeetingInstance, Reply<i32>),
    Participants(MeetingInstance, Reply<Vec<Participant>>),
    Myself(MeetingInstance, Reply<Participant>),
    MeetingInfo(MeetingInstance, Reply<MeetingInfo>),
}

/// A running SDK session. Dropping it uninitialises the SDK.
pub struct Session {
    cmd_tx: Sender<Command>,
    thread: Option<JoinHandle<Result<(), BridgeError>>>,
}

impl Session {
    /// Starts the SDK thread: load DLL, `InitZMToolSuite`, register listener,
    /// `StartToolSuiteAuth`. Returns once those calls were *submitted*;
    /// results arrive as [`Event`]s on the returned receiver.
    pub fn start(mut cfg: SessionConfig) -> Result<(Session, Receiver<Event>), BridgeError> {
        if !crate::sdk_linked() {
            wipe(&mut cfg.access_token);
            return Err(BridgeError::SdkNotLinked);
        }
        if SESSION_ACTIVE.swap(true, Ordering::SeqCst) {
            wipe(&mut cfg.access_token);
            return Err(BridgeError::AlreadyActive);
        }
        let (ev_tx, ev_rx) = mpsc::channel::<Event>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), BridgeError>>();

        let thread = std::thread::Builder::new()
            .name("zoom-sdk".into())
            .spawn(move || sdk_thread(cfg, ev_tx, cmd_rx, ready_tx))
            .map_err(|e| {
                SESSION_ACTIVE.store(false, Ordering::SeqCst);
                BridgeError::Thread(e.to_string())
            })?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok((Session { cmd_tx, thread: Some(thread) }, ev_rx)),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                let _ = thread.join();
                Err(BridgeError::Thread("SDK thread exited during startup".into()))
            }
        }
    }

    fn request<T>(&self, make: impl FnOnce(Reply<T>) -> Command) -> Result<T, BridgeError> {
        let (tx, rx) = mpsc::channel();
        self.cmd_tx
            .send(make(tx))
            .map_err(|_| BridgeError::Thread("SDK thread not running".into()))?;
        rx.recv_timeout(Duration::from_secs(30))
            .map_err(|_| BridgeError::Thread("SDK thread did not answer".into()))?
    }

    /// `IMeetingToolkit::GetMeetingStatus()` (raw `ZMToolSuiteMeetingStatus`).
    pub fn meeting_status(&self, inst: MeetingInstance) -> Result<i32, BridgeError> {
        self.request(|r| Command::MeetingStatus(inst, r))
    }

    /// `IMeetingToolkit::GetParticipantsList()`, copied.
    pub fn participants(&self, inst: MeetingInstance) -> Result<Vec<Participant>, BridgeError> {
        self.request(|r| Command::Participants(inst, r))
    }

    /// `IParticipantsToolkit::GetMySelf()`, copied.
    pub fn myself(&self, inst: MeetingInstance) -> Result<Participant, BridgeError> {
        self.request(|r| Command::Myself(inst, r))
    }

    /// `IMeetingToolkit::GetMeetingProperty()` (without password / join URL).
    pub fn meeting_info(&self, inst: MeetingInstance) -> Result<MeetingInfo, BridgeError> {
        self.request(|r| Command::MeetingInfo(inst, r))
    }

    /// Orderly shutdown (also performed on drop).
    pub fn shutdown(mut self) -> Result<(), BridgeError> {
        self.stop()
    }

    fn stop(&mut self) -> Result<(), BridgeError> {
        let Some(t) = self.thread.take() else { return Ok(()) };
        let _ = self.cmd_tx.send(Command::Shutdown);
        t.join().map_err(|_| BridgeError::Thread("SDK thread panicked".into()))?
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            log::warn!("zoom SDK shutdown error: {e}");
        }
    }
}

fn wipe(s: &mut String) {
    // SAFETY: zeroing bytes keeps the String valid UTF-8 (NULs).
    unsafe { s.as_bytes_mut().iter_mut().for_each(|b| std::ptr::write_volatile(b, 0)) };
    s.clear();
}

fn sdk_thread(
    mut cfg: SessionConfig,
    ev_tx: Sender<Event>,
    cmd_rx: Receiver<Command>,
    ready_tx: Sender<Result<(), BridgeError>>,
) -> Result<(), BridgeError> {
    struct ActiveGuard;
    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            SESSION_ACTIVE.store(false, Ordering::SeqCst);
        }
    }
    let _guard = ActiveGuard;

    // The sender lives in a Box whose address is handed to C++ as `user`.
    // It is freed only after zc_uninit() has returned (no more callbacks).
    let user: Box<Sender<Event>> = Box::new(ev_tx);
    let user_ptr = &*user as *const Sender<Event> as *mut c_void;

    // SAFETY: trivially safe.
    let tid = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
    log::info!("zoom-sdk thread started (tid {tid}); InitZMToolSuite + message pump run here");

    let startup = (|| {
        let loaded = dll::ensure_sdk_dll_loaded(cfg.dll_fallback_dir.as_deref())?;
        log::info!("loaded SDK DLL: {}", loaded.display());
        // SAFETY: trampoline matches zc_event_fn; user_ptr outlives zc_uninit.
        check(unsafe { ffi::zc_init(trampoline, user_ptr) })?;
        // SAFETY: pointers/lengths describe live UTF-8 buffers.
        let r = unsafe {
            ffi::zc_start_auth(
                cfg.access_token.as_ptr().cast(),
                cfg.access_token.len(),
                cfg.domain.as_ptr().cast(),
                cfg.domain.len(),
            )
        };
        wipe(&mut cfg.access_token);
        if let Err(e) = check(r) {
            // SAFETY: init succeeded above; undo it.
            unsafe { ffi::zc_uninit() };
            return Err(e);
        }
        Ok(())
    })();
    wipe(&mut cfg.access_token);

    let started = startup.is_ok();
    let _ = ready_tx.send(startup);
    if !started {
        drop(user);
        return Ok(());
    }

    // Message pump until shutdown is requested.
    'outer: loop {
        loop {
            match cmd_rx.try_recv() {
                Ok(Command::Shutdown) | Err(mpsc::TryRecvError::Disconnected) => break 'outer,
                Err(mpsc::TryRecvError::Empty) => break,
                Ok(cmd) => handle(cmd),
            }
        }
        pump_messages(Duration::from_millis(20));
    }

    // SAFETY: SDK was initialised on this thread.
    let r = check(unsafe { ffi::zc_uninit() });
    // Give in-flight callbacks a chance to drain through the queue.
    pump_messages(Duration::from_millis(10));
    drop(user);
    r
}

/// Executes a query on the SDK thread.
fn handle(cmd: Command) {
    match cmd {
        Command::Shutdown => {}
        Command::MeetingStatus(inst, reply) => {
            let mut status = -1i32;
            // SAFETY: out-pointer valid; called on the SDK thread.
            let r = check(unsafe { ffi::zc_get_meeting_status(inst as i32, &mut status) });
            let _ = reply.send(r.map(|_| status));
        }
        Command::Participants(inst, reply) => {
            let mut out: Vec<Participant> = Vec::new();
            // SAFETY: `out` outlives the synchronous call; sink matches zc_users_fn.
            let r = check(unsafe {
                ffi::zc_get_participants(inst as i32, users_sink, &mut out as *mut _ as *mut c_void)
            });
            let _ = reply.send(r.map(|_| out));
        }
        Command::Myself(inst, reply) => {
            let mut out: Vec<Participant> = Vec::new();
            // SAFETY: as above.
            let r = check(unsafe {
                ffi::zc_get_myself(inst as i32, users_sink, &mut out as *mut _ as *mut c_void)
            });
            let _ = reply.send(r.and_then(|_| out.into_iter().next().ok_or(BridgeError::SdkRejected)));
        }
        Command::MeetingInfo(inst, reply) => {
            let mut out: Option<MeetingInfo> = None;
            // SAFETY: `out` outlives the synchronous call.
            let r = check(unsafe {
                ffi::zc_get_meeting_info(inst as i32, meeting_info_sink, &mut out as *mut _ as *mut c_void)
            });
            let _ = reply.send(r.and_then(|_| out.ok_or(BridgeError::SdkRejected)));
        }
    }
}

fn pump_messages(wait: Duration) {
    // SAFETY: standard Win32 message loop on the current thread.
    unsafe {
        MsgWaitForMultipleObjects(0, std::ptr::null(), 0, wait.as_millis() as u32, QS_ALLINPUT);
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
