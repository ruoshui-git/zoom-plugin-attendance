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
    /// a fresh ephemeral port per attempt (Zoom ignores loopback ports).
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
    if cfg.client_id.trim().is_empty() {
        return Err(AuthError::Config("client_id is empty".into()));
    }
    let mut target = loopback::parse_redirect(&cfg.redirect_uri)?;
    // Fresh ephemeral port per attempt (Zoom ignores the port for loopback).
    target.port = 0;
    let server = loopback::LoopbackServer::bind(target)?;
    let redirect_uri = loopback::with_port(&cfg.redirect_uri, server.port())?;
    let effective = OAuthConfig { client_id: cfg.client_id.clone(), redirect_uri: redirect_uri.clone() };
    let pkce = pkce::Pkce::generate();
    let state = pkce::random_state();
    let url = authorize_url(&effective, &pkce.challenge, &state);
    log::info!("opening browser for Zoom authorization (client_id={}, redirect {redirect_uri})", cfg.client_id);
    open_browser(&url)?;
    let cb = server.wait(&state, timeout)?;
    // redirect_uri must be identical to the one in the authorize request.
    let tokens = token::exchange_code(&cfg.client_id, &cb.code, &redirect_uri, &pkce.verifier)?;
    store.save(&tokens)?;
    log::info!("signed in; scopes: {}", tokens.scope);
    Ok(tokens)
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
