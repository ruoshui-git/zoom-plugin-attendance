//! Token set model and Zoom token endpoint calls.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{AuthError, Secret};

pub const ZOOM_AUTHORIZE_URL: &str = "https://zoom.us/oauth/authorize";
pub const ZOOM_TOKEN_URL: &str = "https://zoom.us/oauth/token";
pub const ZOOM_REVOKE_URL: &str = "https://zoom.us/oauth/revoke";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    /// Unix seconds.
    pub expires_at: u64,
    pub scope: String,
    /// The client id the tokens were issued to (not secret).
    pub client_id: String,
}

impl TokenSet {
    pub fn is_expired(&self, skew: Duration) -> bool {
        now_secs() + skew.as_secs() >= self.expires_at
    }
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scope.split([' ', ',']).any(|s| s == scope)
    }
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<String>,
    reason: Option<String>,
    error_description: Option<String>,
}

/// Optional dev-only client secret. Read from the environment at runtime ONLY,
/// never stored, never committed. Needed only if the Marketplace app is not
/// configured as a public PKCE client.
pub(crate) fn dev_client_secret() -> Option<Secret> {
    std::env::var("ZOOM_COMPANION_CLIENT_SECRET").ok().filter(|s| !s.is_empty()).map(Secret::new)
}

/// POSTs a form to a Zoom OAuth endpoint as this client (public: `client_id` in the
/// body; dev-only confidential: HTTP Basic). Returns the body of a 2xx response.
fn post_oauth_form(url: &str, client_id: &str, form: &[(&str, &str)]) -> Result<String, AuthError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();

    let mut fields: Vec<(&str, &str)> = form.to_vec();
    let secret = dev_client_secret();
    let mut req = agent.post(url);
    let basic;
    if let Some(sec) = &secret {
        use base64::Engine;
        basic = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{}", sec.expose()))
        );
        req = req.header("Authorization", &basic);
    } else {
        // Public PKCE client: identify by client_id in the body, no secret.
        fields.push(("client_id", client_id));
    }

    let mut resp = req.send_form(fields).map_err(|e| AuthError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().map_err(|e| AuthError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        // Error bodies never contain tokens; still, only surface known fields.
        let detail = serde_json::from_str::<ErrorResponse>(&body)
            .map(|e| {
                [e.error, e.reason, e.error_description].into_iter().flatten().collect::<Vec<_>>().join(": ")
            })
            .unwrap_or_else(|_| "unparseable error body".into());
        return Err(AuthError::TokenEndpoint { status, detail });
    }
    Ok(body)
}

/// Revokes a token (`POST /oauth/revoke`). Revoking either token ends the grant,
/// which also removes the app from the user's Zoom "Added apps".
pub fn revoke(client_id: &str, token: &Secret) -> Result<(), AuthError> {
    post_oauth_form(ZOOM_REVOKE_URL, client_id, &[("token", token.expose())]).map(|_| ())
}

pub(crate) fn post_token(client_id: &str, form: &[(&str, &str)]) -> Result<TokenSet, AuthError> {
    let body = post_oauth_form(ZOOM_TOKEN_URL, client_id, form)?;
    let t: TokenResponse =
        serde_json::from_str(&body).map_err(|e| AuthError::Http(format!("bad token JSON: {e}")))?;
    Ok(TokenSet {
        access_token: Secret::new(t.access_token),
        refresh_token: t.refresh_token.map(Secret::new),
        expires_at: now_secs() + t.expires_in.unwrap_or(3600),
        scope: t.scope.unwrap_or_default(),
        client_id: client_id.to_string(),
    })
}

/// Authorization-code exchange (RFC 6749 §4.1.3 + RFC 7636 §4.5).
pub fn exchange_code(
    client_id: &str,
    code: &Secret,
    redirect_uri: &str,
    verifier: &Secret,
) -> Result<TokenSet, AuthError> {
    post_token(
        client_id,
        &[
            ("grant_type", "authorization_code"),
            ("code", code.expose()),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier.expose()),
        ],
    )
}

/// Refresh-token grant. Zoom rotates refresh tokens: persist the result.
pub fn refresh(client_id: &str, refresh_token: &Secret) -> Result<TokenSet, AuthError> {
    post_token(client_id, &[("grant_type", "refresh_token"), ("refresh_token", refresh_token.expose())])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_and_expiry() {
        let t = TokenSet {
            access_token: Secret::new("a"),
            refresh_token: None,
            expires_at: now_secs() + 100,
            scope: "plugin_sdk:read:connection_meta user:read:user".into(),
            client_id: "cid".into(),
        };
        assert!(t.has_scope("plugin_sdk:read:connection_meta"));
        assert!(!t.has_scope("plugin_sdk"));
        assert!(!t.is_expired(Duration::from_secs(10)));
        assert!(t.is_expired(Duration::from_secs(200)));
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("access_token: Secret(\"a\")"));
    }
}
