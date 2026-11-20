//! Zoom connection status shown in the desktop UI (sign-in banner + sidebar).
//!
//! The SDK worker (`serve::sdk_loop`) and the sign-in/sign-out endpoints report
//! here; changes are broadcast to the UI as `{"type":"zoom_status",...}`.

use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;

use serde_json::{json, Value};
use zoom_auth::AuthError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomState {
    /// Build has no public client ID and none is configured (developer builds only).
    NotConfigured,
    /// No usable tokens: the user must click "Sign in with Zoom".
    SignedOut,
    /// Browser sign-in in progress.
    SigningIn,
    /// Signed in, but Zoom Workplace (Zoom.exe) is not running.
    ZoomNotRunning,
    /// Signed in, connecting to Zoom Workplace over IPC.
    Connecting,
    /// Authenticated with Zoom Workplace; attendance is being captured.
    Connected,
    /// Anything else (account mismatch, SDK failure, network); see `detail`.
    Error,
}

impl ZoomState {
    pub fn as_str(self) -> &'static str {
        match self {
            ZoomState::NotConfigured => "not_configured",
            ZoomState::SignedOut => "signed_out",
            ZoomState::SigningIn => "signing_in",
            ZoomState::ZoomNotRunning => "zoom_not_running",
            ZoomState::Connecting => "connecting",
            ZoomState::Connected => "connected",
            ZoomState::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoomStatus {
    pub state: ZoomState,
    /// Technical detail for support (shown small under the localized message).
    pub detail: String,
    /// Machine-readable reason for `Error` (e.g. `account_mismatch`), else empty.
    pub reason: &'static str,
}

impl ZoomStatus {
    pub fn new(state: ZoomState, detail: impl Into<String>) -> Self {
        ZoomStatus { state, detail: detail.into(), reason: "" }
    }
    pub fn with_reason(mut self, reason: &'static str) -> Self {
        self.reason = reason;
        self
    }
    pub fn wire(&self) -> Value {
        json!({"type":"zoom_status","state":self.state.as_str(),"reason":self.reason,"detail":self.detail})
    }
}

/// Shared between the SDK worker and the sign-in/sign-out endpoints.
pub struct ZoomLink {
    pub status: Mutex<ZoomStatus>,
    /// Set to cut the worker's retry pause short (after sign-in / sign-out).
    pub wake: AtomicBool,
    /// Set by sign-out: the worker drops its SDK session.
    pub sign_out: AtomicBool,
    /// Browser sign-ins in flight; while > 0 the worker does not report `SignedOut`.
    pub login_pending: AtomicUsize,
    /// Authorize URL of the sign-in waiting for its redirect (one at a time).
    /// "Sign in" again just reopens this URL.
    pub login_url: Mutex<Option<String>>,
}

impl Default for ZoomLink {
    fn default() -> Self {
        ZoomLink {
            status: Mutex::new(ZoomStatus::new(ZoomState::Connecting, "")),
            wake: AtomicBool::new(false),
            sign_out: AtomicBool::new(false),
            login_pending: AtomicUsize::new(0),
            login_url: Mutex::new(None),
        }
    }
}

/// Typed `start_session` failures that are not auth errors.
#[derive(Debug, PartialEq, Eq)]
pub enum StartError {
    NotConfigured,
    ZoomNotRunning,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::NotConfigured => {
                f.write_str("client_id not configured: run `zoom-companion config init --client-id <ID>`")
            }
            StartError::ZoomNotRunning => f.write_str("Zoom Workplace (Zoom.exe) is not running"),
        }
    }
}

impl std::error::Error for StartError {}

/// What the worker should show for a failed `start_session`, and whether the
/// stored tokens are unusable and should be deleted (forcing a fresh sign-in).
pub fn classify_start_error(e: &anyhow::Error) -> (ZoomStatus, bool) {
    let detail = format!("{e:#}");
    if let Some(s) = e.downcast_ref::<StartError>() {
        return match s {
            StartError::NotConfigured => (ZoomStatus::new(ZoomState::NotConfigured, detail), false),
            StartError::ZoomNotRunning => (ZoomStatus::new(ZoomState::ZoomNotRunning, detail), false),
        };
    }
    if let Some(a) = e.downcast_ref::<AuthError>() {
        return match a {
            AuthError::NotSignedIn => (ZoomStatus::new(ZoomState::SignedOut, ""), false),
            // Tokens issued to another client ID (dev build vs release build).
            AuthError::Config(m) if m.contains("stored tokens belong to client_id") => {
                (ZoomStatus::new(ZoomState::SignedOut, ""), true)
            }
            // Refresh rejected: expired (90 days) or the user removed the app in Zoom.
            AuthError::TokenEndpoint { status, .. } if (400..500).contains(status) => {
                (ZoomStatus::new(ZoomState::SignedOut, detail).with_reason("session_expired"), true)
            }
            // DPAPI blob from another Windows user/machine, or a corrupt file.
            AuthError::Store(_) => (ZoomStatus::new(ZoomState::SignedOut, detail), true),
            AuthError::Http(_) => (ZoomStatus::new(ZoomState::Error, detail).with_reason("network"), false),
            _ => (ZoomStatus::new(ZoomState::Error, detail), false),
        };
    }
    (ZoomStatus::new(ZoomState::Error, detail), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(e: anyhow::Error) -> (ZoomState, bool) {
        let (s, clear) = classify_start_error(&e);
        (s.state, clear)
    }

    #[test]
    fn classifies_start_failures() {
        assert_eq!(state(AuthError::NotSignedIn.into()), (ZoomState::SignedOut, false));
        assert_eq!(
            state(AuthError::Config("stored tokens belong to client_id a, configured b; run `auth login`".into()).into()),
            (ZoomState::SignedOut, true)
        );
        assert_eq!(
            state(AuthError::TokenEndpoint { status: 401, detail: "invalid_grant".into() }.into()),
            (ZoomState::SignedOut, true)
        );
        assert_eq!(
            state(AuthError::TokenEndpoint { status: 503, detail: "down".into() }.into()),
            (ZoomState::Error, false)
        );
        assert_eq!(state(AuthError::Http("offline".into()).into()), (ZoomState::Error, false));
        assert_eq!(state(StartError::NotConfigured.into()), (ZoomState::NotConfigured, false));
        assert_eq!(state(StartError::ZoomNotRunning.into()), (ZoomState::ZoomNotRunning, false));
        assert_eq!(state(anyhow::anyhow!("sdk exploded")), (ZoomState::Error, false));
    }

    #[test]
    fn auth_error_survives_context() {
        let e = anyhow::Error::from(AuthError::NotSignedIn).context("loading tokens");
        assert_eq!(state(e).0, ZoomState::SignedOut);
    }

    #[test]
    fn wire_shape() {
        let v = ZoomStatus::new(ZoomState::Error, "x").with_reason("account_mismatch").wire();
        assert_eq!(v["type"], "zoom_status");
        assert_eq!(v["state"], "error");
        assert_eq!(v["reason"], "account_mismatch");
    }
}
