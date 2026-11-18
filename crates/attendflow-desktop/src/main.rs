//! AttendFlow Desktop launcher: the Start-menu / MSIX entry point.
//!
//! `zoom-companion.exe` is a console program (it is also the developer CLI), so
//! starting it directly from a shortcut would flash a console window. This GUI
//! subsystem launcher starts `zoom-companion.exe app` beside it with no console
//! and sends its log output to `%LOCALAPPDATA%\zoom-companion\logs\app.log`
//! (the file users attach to support requests).
#![windows_subsystem = "windows"]

use std::fs::OpenOptions;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

fn main() {
    if let Err(e) = launch() {
        message_box(&format!("AttendFlow Desktop could not start.\n\n{e}"));
    }
}

fn launch() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let companion = exe
        .parent()
        .ok_or("launcher has no parent folder")?
        .join("zoom-companion.exe");
    if !companion.is_file() {
        return Err(format!("Missing {}. Please reinstall the app.", companion.display()));
    }
    let mut cmd = Command::new(&companion);
    cmd.arg("app").stdin(Stdio::null()).creation_flags(CREATE_NO_WINDOW);
    match open_log() {
        Some(log) => {
            let err = log.try_clone().map_err(|e| e.to_string())?;
            cmd.stdout(log).stderr(err);
        }
        None => {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("{}: {e}", companion.display()))
}

fn open_log() -> Option<std::fs::File> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
        .join("zoom-companion")
        .join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let log = dir.join("app.log");
    if std::fs::metadata(&log).map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let _ = std::fs::rename(&log, dir.join("app.log.1"));
    }
    OpenOptions::new().create(true).append(true).open(log).ok()
}

fn message_box(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (text, caption) = (wide(text), wide("AttendFlow Desktop"));
    // SAFETY: NUL-terminated UTF-16 buffers that outlive the call; null owner window.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), MB_OK | MB_ICONERROR);
    }
}
