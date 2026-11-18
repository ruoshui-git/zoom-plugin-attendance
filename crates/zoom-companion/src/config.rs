//! Non-secret local configuration: `%APPDATA%\zoom-companion\config.toml`.
//! Never put tokens or client secrets here.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:47811/callback";
pub const DEFAULT_DOMAIN: &str = "https://zoom.us";

/// Public client ID compiled into release builds (`scripts\package.ps1 -ClientId`,
/// via the `ZOOM_COMPANION_BUILTIN_CLIENT_ID` build env var) so end users never
/// configure anything. A public PKCE client ID is not a secret. A non-empty
/// `client_id` in config.toml or `ZOOM_COMPANION_CLIENT_ID` still overrides it (dev).
pub const BUILTIN_CLIENT_ID: &str = match option_env!("ZOOM_COMPANION_BUILTIN_CLIENT_ID") {
    Some(id) => id,
    None => "",
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Marketplace PUBLIC client ID (App Credentials -> Use Public Client OAuth), not the confidential Client ID.
    pub client_id: String,
    /// Must exactly match the redirect URL registered on the Marketplace app.
    pub redirect_uri: String,
    /// Domain passed to StartToolSuiteAuth.
    pub domain: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            client_id: BUILTIN_CLIENT_ID.trim().into(),
            redirect_uri: DEFAULT_REDIRECT_URI.into(),
            domain: DEFAULT_DOMAIN.into(),
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base = std::env::var_os("APPDATA").context("APPDATA not set")?;
    Ok(PathBuf::from(base).join("zoom-companion").join("config.toml"))
}

impl Config {
    /// File values, then env overrides (ZOOM_COMPANION_CLIENT_ID / _REDIRECT_URI).
    pub fn load() -> Result<Config> {
        let path = config_path()?;
        let mut cfg = match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).with_context(|| format!("parsing {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        if let Ok(v) = std::env::var("ZOOM_COMPANION_CLIENT_ID") {
            cfg.client_id = v;
        }
        if cfg.client_id.trim().is_empty() {
            cfg.client_id = BUILTIN_CLIENT_ID.trim().into();
        }
        if let Ok(v) = std::env::var("ZOOM_COMPANION_REDIRECT_URI") {
            cfg.redirect_uri = v;
        }
        Ok(cfg)
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = config_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(path)
    }

    pub fn oauth(&self) -> zoom_auth::OAuthConfig {
        zoom_auth::OAuthConfig { client_id: self.client_id.clone(), redirect_uri: self.redirect_uri.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toml_roundtrip_and_defaults() {
        let c: Config = toml::from_str("client_id = \"abc\"").unwrap();
        assert_eq!(c.client_id, "abc");
        assert_eq!(c.redirect_uri, DEFAULT_REDIRECT_URI);
        assert_eq!(c.domain, DEFAULT_DOMAIN);
        let s = toml::to_string_pretty(&c).unwrap();
        assert!(!s.to_lowercase().contains("secret"));
    }
}
