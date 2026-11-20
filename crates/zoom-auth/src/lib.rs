//! Native OAuth for Zoom: Authorization Code + PKCE with a loopback redirect,
//! tokens stored with Windows DPAPI. No backend, no embedded client secret.
//!
//! Flow ([`login_interactive`]):
//! 1. generate PKCE verifier/challenge and `state`
//! 2. bind the loopback listener on the registered redirect port
//! 3. open the system browser at `https://zoom.us/oauth/authorize?...`
//! 4. receive `code` on `http://localhost:<port>/<path>`, verify `state`
//! 5. POST `https://zoom.us/oauth/token` with `code_verifier`
//! 6. persist the token set encrypted (DPAPI)

pub mod loopback;
pub mod pkce;
mod secret;
pub mod store;
pub mod token;

pub use secret::Secret;
pub use store::TokenStore;
pub use token::TokenSet;

use std::time::Duration;

/// Scope required by the Plugin SDK handshake.
pub const PLUGIN_SDK_SCOPE: &str = "plugin_sdk:read:connection_meta";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("loopback listener error: {0}")]
    Loopback(String),
    #[error("authorization denied or failed: {0}")]
    Denied(String),
    #[error("OAuth state mismatch (possible CSRF); aborting")]
    StateMismatch,
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("token endpoint returned HTTP {status}: {detail}")]
    TokenEndpoint { status: u16, detail: String },
    #[error("token store error: {0}")]
    Store(String),
    #[error("not signed in (run `zoom-companion auth login`)")]
    NotSignedIn,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Marketplace **public client ID** (App Credentials -> Use Public Client
    /// OAuth). Not the confidential Client ID. Not a secret.
    pub client_id: String,
    /// PKCE redirect registered on the Marketplace app; the port is replaced by
    /// a fresh ephemeral port per attempt (Zoom ignores loopback ports for
    /// public clients).
    pub redirect_uri: String,
}

/// Builds the authorize URL. Zoom's General apps request the scopes that are
/// configured on the app; no `scope` parameter is required.
pub fn authorize_url(cfg: &OAuthConfig, challenge: &str, state: &str) -> String {
    let mut u = url::Url::parse(token::ZOOM_AUTHORIZE_URL).expect("static URL");
    u.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &cfg.client_id)
        .append_pair("redirect_uri", &cfg.redirect_uri)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state);
    u.into()
}

/// Interactive login. `open_browser` is injected so the CLI decides how to
/// launch (ShellExecute) and tests can intercept.
pub fn login_interactive(
    cfg: &OAuthConfig,
    store: &TokenStore,
    open_browser: impl FnOnce(&str) -> Result<(), AuthError>,
    timeout: Duration,
) -> Result<TokenSet, AuthError> {
    let pending = begin_login(cfg)?;
    log::info!("opening browser for Zoom authorization (client_id={}, redirect {})", cfg.client_id, pending.redirect_uri);
    open_browser(pending.authorize_url())?;
    pending.finish(store, timeout)
}

/// A sign-in waiting for the browser redirect: the loopback listener is bound and
/// the authorize URL is fixed, so the same URL can be opened again (the user closed
/// the tab) instead of starting a second sign-in.
pub struct PendingLogin {
    server: loopback::LoopbackServer,
    client_id: String,
    redirect_uri: String,
    pkce: pkce::Pkce,
    state: String,
    url: String,
}

/// Binds the loopback listener on a fresh ephemeral port and builds the authorize
/// URL with that port. Zoom matches loopback redirects of public (PKCE) clients
/// ignoring only the port, so the registered `http://127.0.0.1:47811/callback`
/// accepts any port. (The "Invalid redirect" errors of 2026-11-18 came from using
/// the confidential Client ID, which has no such rule.)
pub fn begin_login(cfg: &OAuthConfig) -> Result<PendingLogin, AuthError> {
    if cfg.client_id.trim().is_empty() {
        return Err(AuthError::Config("client_id is empty".into()));
    }
    let mut target = loopback::parse_redirect(&cfg.redirect_uri)?;
    target.port = 0;
    let server = loopback::LoopbackServer::bind(target)?;
    let redirect_uri = loopback::with_port(&cfg.redirect_uri, server.port())?;
    let effective = OAuthConfig { client_id: cfg.client_id.clone(), redirect_uri: redirect_uri.clone() };
    let pkce = pkce::Pkce::generate();
    let state = pkce::random_state();
    let url = authorize_url(&effective, &pkce.challenge, &state);
    Ok(PendingLogin { server, client_id: cfg.client_id.clone(), redirect_uri, pkce, state, url })
}

impl PendingLogin {
    pub fn authorize_url(&self) -> &str {
        &self.url
    }

    /// The redirect actually sent to Zoom (registered URL with this attempt's port).
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Waits for the redirect, exchanges the code and persists the tokens.
    pub fn finish(self, store: &TokenStore, timeout: Duration) -> Result<TokenSet, AuthError> {
        let cb = self.server.wait(&self.state, timeout)?;
        // redirect_uri must be identical to the one in the authorize request.
        let tokens = token::exchange_code(&self.client_id, &cb.code, &self.redirect_uri, &self.pkce.verifier)?;
        store.save(&tokens)?;
        log::info!("signed in; scopes: {}", tokens.scope);
        Ok(tokens)
    }
}

/// Signs out: revokes the grant at Zoom (best effort, so the app also leaves the
/// user's Zoom "Added apps") and always deletes the local token store.
/// Returns whether Zoom confirmed the revocation.
pub fn sign_out(store: &TokenStore) -> Result<bool, AuthError> {
    let revoked = match store.load() {
        Ok(Some(t)) => {
            let mut ok = token::revoke(&t.client_id, &t.access_token);
            if ok.is_err() {
                if let Some(rt) = &t.refresh_token {
                    ok = token::revoke(&t.client_id, rt);
                }
            }
            if let Err(e) = &ok {
                log::warn!("token revocation failed (local tokens are still deleted): {e}");
            }
            ok.is_ok()
        }
        Ok(None) => false,
        Err(e) => {
            log::warn!("token store unreadable during sign-out; deleting it: {e}");
            false
        }
    };
    store.clear()?;
    Ok(revoked)
}

/// Refreshes now, even if the access token has not expired (re-persisting the
/// rotated tokens). A 4xx from Zoom means the grant is gone: the user removed the
/// app from their Zoom account, or the refresh token expired.
pub fn refresh_now(store: &TokenStore) -> Result<TokenSet, AuthError> {
    let t = store.load()?.ok_or(AuthError::NotSignedIn)?;
    let rt = t.refresh_token.clone().ok_or(AuthError::NotSignedIn)?;
    let fresh = token::refresh(&t.client_id, &rt)?;
    store.save(&fresh)?;
    Ok(fresh)
}

/// Returns a valid access token, refreshing (and re-persisting) if needed.
pub fn valid_tokens(store: &TokenStore, client_id: &str) -> Result<TokenSet, AuthError> {
    let t = store.load()?.ok_or(AuthError::NotSignedIn)?;
    if t.client_id != client_id {
        return Err(AuthError::Config(format!(
            "stored tokens belong to client_id {}, configured {}; run `auth login`",
            t.client_id, client_id
        )));
    }
    if !t.is_expired(Duration::from_secs(120)) {
        return Ok(t);
    }
    let rt = t.refresh_token.clone().ok_or(AuthError::NotSignedIn)?;
    log::info!("access token expired; refreshing");
    let fresh = token::refresh(client_id, &rt)?;
    store.save(&fresh)?;
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn begin_login_uses_a_fresh_loopback_port() {
        let cfg = OAuthConfig { client_id: "CID".into(), redirect_uri: "http://127.0.0.1:47811/callback".into() };
        let a = begin_login(&cfg).unwrap();
        let b = begin_login(&cfg).unwrap();
        for p in [&a, &b] {
            let u = url::Url::parse(p.authorize_url()).unwrap();
            let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
            assert_eq!(q["redirect_uri"], p.redirect_uri());
            let r = url::Url::parse(p.redirect_uri()).unwrap();
            assert_eq!((r.host_str(), r.path()), (Some("127.0.0.1"), "/callback"));
        }
        assert_ne!(a.redirect_uri(), b.redirect_uri(), "each attempt has its own port");
        let none = OAuthConfig { client_id: " ".into(), redirect_uri: cfg.redirect_uri.clone() };
        assert!(matches!(begin_login(&none), Err(AuthError::Config(_))));
    }

    #[test]
    fn authorize_url_shape() {
        let cfg = OAuthConfig { client_id: "CID".into(), redirect_uri: "http://127.0.0.1:47811/callback".into() };
        let u = url::Url::parse(&authorize_url(&cfg, "CH", "ST")).unwrap();
        assert_eq!(u.host_str(), Some("zoom.us"));
        assert_eq!(u.path(), "/oauth/authorize");
        let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["client_id"], "CID");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:47811/callback");
        assert_eq!(q["code_challenge"], "CH");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["state"], "ST");
        assert!(!q.contains_key("client_secret"));
    }
}
