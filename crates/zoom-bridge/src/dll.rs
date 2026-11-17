//! Explicit SDK DLL probing.
//!
//! The SDK import library is linked with `/DELAYLOAD`, so the real DLL is only
//! loaded on the first SDK call. A missing DLL at that moment would raise an
//! SEH exception, so callers MUST call [`ensure_sdk_dll_loaded`] first: it
//! loads the DLL with a well-defined search order and reports a clean error.

use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{GetLastError, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, LoadLibraryExW, LOAD_LIBRARY_SEARCH_APPLICATION_DIR,
    LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
};

pub const SDK_DLL_NAME: &str = "zToolSuiteIPCProxy.dll";

#[derive(Debug, thiserror::Error)]
pub enum DllError {
    #[error("{dll} could not be loaded (Win32 error {code}). Copy the complete SDK bin/ folder next to the executable.")]
    LoadFailed { dll: String, code: u32 },
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn module_path(h: HMODULE) -> PathBuf {
    let mut buf = vec![0u16; 32 * 1024];
    // SAFETY: buffer is valid for buf.len() u16s.
    let n = unsafe { GetModuleFileNameW(h, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(std::ffi::OsString::from_wide(&buf[..n]))
}

/// Load `zToolSuiteIPCProxy.dll`, preferring the executable's own directory.
/// If `fallback_dir` is given and the DLL isn't beside the exe, load it from
/// there (its dependencies are resolved from the same directory).
/// Returns the full path of the module actually loaded. The module stays
/// loaded for the rest of the process (intentional: the SDK is process-global).
pub fn ensure_sdk_dll_loaded(fallback_dir: Option<&Path>) -> Result<PathBuf, DllError> {
    let name = wide(OsStr::new(SDK_DLL_NAME));
    // SAFETY: `name` is a NUL-terminated UTF-16 string.
    let h = unsafe {
        LoadLibraryExW(
            name.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        )
    };
    if !h.is_null() {
        return Ok(module_path(h));
    }
    // SAFETY: trivially safe.
    let mut code = unsafe { GetLastError() };
    if let Some(dir) = fallback_dir.filter(|d| d.join(SDK_DLL_NAME).is_file()) {
        let full = wide(dir.join(SDK_DLL_NAME).as_os_str());
        // SAFETY: NUL-terminated absolute path.
        let h = unsafe {
            LoadLibraryExW(
                full.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        };
        if !h.is_null() {
            return Ok(module_path(h));
        }
        // SAFETY: trivially safe.
        code = unsafe { GetLastError() };
    }
    Err(DllError::LoadFailed { dll: SDK_DLL_NAME.into(), code })
}

/// True if the SDK runtime DLL sits beside the current executable. Verified
/// requirement: with the DLL loaded from elsewhere (absolute path) the SDK
/// initialises but never delivers auth/IPC callbacks, so the complete SDK
/// `bin\` folder must be copied next to the exe (scripts\build.ps1 does this).
pub fn sdk_runtime_beside_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    dir.join(SDK_DLL_NAME).is_file().then_some(dir)
}
