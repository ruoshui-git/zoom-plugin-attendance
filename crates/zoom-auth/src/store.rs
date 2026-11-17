//! Per-user encrypted token storage using Windows DPAPI (CryptProtectData).
//!
//! File: `%LOCALAPPDATA%\zoom-companion\tokens.dpapi`. Only the same Windows
//! user on the same machine can decrypt it. Nothing is stored in plaintext and
//! nothing is stored inside the repository.

use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
use windows_sys::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

use crate::{AuthError, TokenSet};

/// Application-specific entropy (not a secret; scopes the blob to this app).
const ENTROPY: &[u8] = b"zoom-companion/tokens/v1";

pub fn default_store_path() -> Result<PathBuf, AuthError> {
    let base = std::env::var_os("LOCALAPPDATA").ok_or(AuthError::Store("LOCALAPPDATA not set".into()))?;
    Ok(PathBuf::from(base).join("zoom-companion").join("tokens.dpapi"))
}

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
}

pub fn protect(plain: &[u8]) -> Result<Vec<u8>, AuthError> {
    let input = blob(plain);
    let entropy = blob(ENTROPY);
    let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    // SAFETY: input/entropy point to live buffers; out is written by the API.
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            &entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    };
    if ok == 0 {
        return Err(AuthError::Store("CryptProtectData failed".into()));
    }
    // SAFETY: out.pbData is valid for cbData bytes until LocalFree.
    let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec() };
    // SAFETY: memory was allocated by the API with LocalAlloc.
    unsafe { LocalFree(out.pbData as HLOCAL) };
    Ok(v)
}

pub fn unprotect(cipher: &[u8]) -> Result<Vec<u8>, AuthError> {
    let input = blob(cipher);
    let entropy = blob(ENTROPY);
    let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    // SAFETY: as in `protect`.
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            &entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
    };
    if ok == 0 {
        return Err(AuthError::Store("CryptUnprotectData failed (other user/machine or corrupt)".into()));
    }
    // SAFETY: out.pbData valid for cbData bytes.
    let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec() };
    // SAFETY: zero then free API-allocated plaintext.
    unsafe {
        std::ptr::write_bytes(out.pbData, 0, out.cbData as usize);
        LocalFree(out.pbData as HLOCAL);
    }
    Ok(v)
}

pub struct TokenStore {
    path: PathBuf,
}

impl TokenStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        TokenStore { path: path.into() }
    }
    pub fn open_default() -> Result<Self, AuthError> {
        Ok(Self::new(default_store_path()?))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self, t: &TokenSet) -> Result<(), AuthError> {
        let mut json = serde_json::to_vec(t).map_err(|e| AuthError::Store(e.to_string()))?;
        let enc = protect(&json);
        json.iter_mut().for_each(|b| *b = 0);
        let enc = enc?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| AuthError::Store(e.to_string()))?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, enc).map_err(|e| AuthError::Store(e.to_string()))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| AuthError::Store(e.to_string()))?;
        Ok(())
    }

    pub fn load(&self) -> Result<Option<TokenSet>, AuthError> {
        let enc = match std::fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(AuthError::Store(e.to_string())),
        };
        let mut plain = unprotect(&enc)?;
        let t = serde_json::from_slice::<TokenSet>(&plain).map_err(|e| AuthError::Store(e.to_string()));
        plain.iter_mut().for_each(|b| *b = 0);
        t.map(Some)
    }

    pub fn clear(&self) -> Result<bool, AuthError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(AuthError::Store(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Secret;

    #[test]
    fn dpapi_roundtrip() {
        let c = protect(b"hello \xe4\xb8\xad").unwrap();
        assert!(!c.windows(5).any(|w| w == b"hello"));
        assert_eq!(unprotect(&c).unwrap(), b"hello \xe4\xb8\xad");
    }

    #[test]
    fn store_roundtrip_is_encrypted() {
        let dir = std::env::temp_dir().join(format!("zc-store-test-{}", std::process::id()));
        let store = TokenStore::new(dir.join("t.dpapi"));
        let t = TokenSet {
            access_token: Secret::new("ACCESS-abc123"),
            refresh_token: Some(Secret::new("REFRESH-xyz")),
            expires_at: 42,
            scope: "plugin_sdk:read:connection_meta".into(),
            client_id: "cid".into(),
        };
        store.save(&t).unwrap();
        let raw = std::fs::read(store.path()).unwrap();
        assert!(!raw.windows(6).any(|w| w == b"ACCESS"));
        let back = store.load().unwrap().unwrap();
        assert_eq!(back.access_token.expose(), "ACCESS-abc123");
        assert_eq!(back.refresh_token.unwrap().expose(), "REFRESH-xyz");
        assert!(store.clear().unwrap());
        assert!(store.load().unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
