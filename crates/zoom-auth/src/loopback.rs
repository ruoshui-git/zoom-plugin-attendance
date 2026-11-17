//! One-shot loopback HTTP listener for the OAuth redirect (RFC 8252 §7.3).
//! Accepts exactly one matching `GET <path>?code=..&state=..` and replies with
//! a tiny static page. No web framework, no external exposure: binds only to
//! 127.0.0.1 / ::1.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use crate::{AuthError, Secret};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectTarget {
    pub port: u16,
    pub path: String,
}

/// Validates that `redirect_uri` is a loopback http URL with explicit port.
pub fn parse_redirect(redirect_uri: &str) -> Result<RedirectTarget, AuthError> {
    let u = url::Url::parse(redirect_uri).map_err(|e| AuthError::Config(format!("redirect_uri: {e}")))?;
    if u.scheme() != "http" {
        return Err(AuthError::Config("loopback redirect must use http://".into()));
    }
    match u.host_str() {
        Some("localhost") | Some("127.0.0.1") | Some("[::1]") => {}
        _ => return Err(AuthError::Config("redirect host must be localhost/127.0.0.1/[::1]".into())),
    }
    // Zoom matches loopback redirects ignoring only the port, and recommends a
    // fresh ephemeral port per attempt; a missing port means "ephemeral" (0).
    Ok(RedirectTarget { port: u.port().unwrap_or(0), path: u.path().to_string() })
}

pub struct Callback {
    pub code: Secret,
}

pub struct LoopbackServer {
    listeners: Vec<TcpListener>,
    target: RedirectTarget,
}

impl LoopbackServer {
    /// Bind before opening the browser so the redirect can't race us.
    pub fn bind(target: RedirectTarget) -> Result<Self, AuthError> {
        let mut listeners = Vec::new();
        let v4 = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, target.port)))
            .map_err(|e| AuthError::Loopback(format!("bind 127.0.0.1:{}: {e}", target.port)))?;
        let port = v4.local_addr().map_err(|e| AuthError::Loopback(e.to_string()))?.port();
        listeners.push(v4);
        // Browsers may resolve localhost to ::1 first; best-effort, same port.
        if let Ok(v6) = TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, port))) {
            listeners.push(v6);
        }
        for l in &listeners {
            l.set_nonblocking(true).map_err(|e| AuthError::Loopback(e.to_string()))?;
        }
        Ok(LoopbackServer { listeners, target })
    }

    /// The port actually bound (differs from the requested one when it was 0).
    pub fn port(&self) -> u16 {
        self.listeners[0].local_addr().map(|a| a.port()).unwrap_or(0)
    }

    /// Waits for the redirect. Requests to other paths get 404 and are ignored.
    pub fn wait(&self, expected_state: &str, timeout: Duration) -> Result<Callback, AuthError> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            for l in &self.listeners {
                match l.accept() {
                    Ok((stream, _)) => {
                        match self.handle(stream, expected_state) {
                            // A stale tab from an earlier attempt: its code is
                            // discarded (never exchanged); keep waiting.
                            Some(Err(AuthError::StateMismatch)) => {
                                log::warn!("ignored a redirect with a non-matching state (stale browser tab?)");
                            }
                            Some(r) => return r,
                            None => {}
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(AuthError::Loopback(e.to_string())),
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(AuthError::Loopback("timed out waiting for the browser redirect".into()))
    }

    fn handle(&self, mut stream: TcpStream, expected_state: &str) -> Option<Result<Callback, AuthError>> {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            return None;
        }
        // "GET /callback?code=...&state=... HTTP/1.1"
        let target = line.split_whitespace().nth(1).unwrap_or("");
        let url = match url::Url::parse(&format!("http://localhost{target}")) {
            Ok(u) => u,
            Err(_) => return None,
        };
        if url.path() != self.target.path {
            respond(&mut stream, "404 Not Found", "Not found.");
            return None;
        }
        let mut code = None;
        let mut state = None;
        let mut error = None;
        for (k, v) in url.query_pairs() {
            match &*k {
                "code" => code = Some(Secret::new(v.into_owned())),
                "state" => state = Some(v.into_owned()),
                "error" => error = Some(v.into_owned()),
                _ => {}
            }
        }
        let result = classify(code, state.as_deref(), error, expected_state);
        match &result {
            Ok(_) => respond(&mut stream, "200 OK", "Zoom sign-in complete. You can close this tab and return to zoom-companion."),
            Err(_) => respond(&mut stream, "400 Bad Request", "Zoom sign-in failed. Return to zoom-companion for details."),
        }
        Some(result)
    }
}

fn classify(
    code: Option<Secret>,
    state: Option<&str>,
    error: Option<String>,
    expected_state: &str,
) -> Result<Callback, AuthError> {
    if let Some(e) = error {
        return Err(AuthError::Denied(e));
    }
    if state != Some(expected_state) {
        return Err(AuthError::StateMismatch);
    }
    code.filter(|c| !c.is_empty()).map(|code| Callback { code }).ok_or(AuthError::Denied("missing code".into()))
}

fn respond(stream: &mut TcpStream, status: &str, msg: &str) {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>zoom-companion</title><p style=\"font:16px system-ui;margin:2em\">{msg}</p>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_validation() {
        assert_eq!(
            parse_redirect("http://127.0.0.1:47811/callback").unwrap(),
            RedirectTarget { port: 47811, path: "/callback".into() }
        );
        assert!(parse_redirect("https://example.com/cb").is_err());
        assert_eq!(parse_redirect("http://127.0.0.1/cb").unwrap().port, 0);
        assert!(parse_redirect("http://192.168.1.2:80/cb").is_err());
    }

    #[test]
    fn end_to_end_loopback() {
        let srv = LoopbackServer::bind(RedirectTarget { port: 0, path: "/callback".into() });
        // Port 0 binds an ephemeral v4 port; discover it.
        let srv = srv.unwrap();
        let port = srv.port();
        assert_ne!(port, 0);
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(s, "GET /favicon.ico HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
            drop(s);
            let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(s, "GET /callback?code=abc%20123&state=S1 HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
            let mut resp = String::new();
            std::io::Read::read_to_string(&mut s, &mut resp).unwrap();
            resp
        });
        let cb = srv.wait("S1", Duration::from_secs(5)).unwrap();
        assert_eq!(cb.code.expose(), "abc 123");
        assert!(t.join().unwrap().starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn state_mismatch_and_denial() {
        assert!(matches!(classify(Some(Secret::new("c")), Some("X"), None, "Y"), Err(AuthError::StateMismatch)));
        assert!(matches!(classify(None, Some("Y"), Some("access_denied".into()), "Y"), Err(AuthError::Denied(_))));
        assert!(matches!(classify(None, Some("Y"), None, "Y"), Err(AuthError::Denied(_))));
    }
}

/// Returns `redirect_uri` with its port replaced by `port`.
pub fn with_port(redirect_uri: &str, port: u16) -> Result<String, AuthError> {
    let mut u = url::Url::parse(redirect_uri).map_err(|e| AuthError::Config(format!("redirect_uri: {e}")))?;
    u.set_port(Some(port)).map_err(|_| AuthError::Config("cannot set port".into()))?;
    Ok(u.into())
}

#[cfg(test)]
mod port_tests {
    #[test]
    fn with_port_rewrites_only_port() {
        assert_eq!(super::with_port("http://127.0.0.1:47811/callback", 50123).unwrap(), "http://127.0.0.1:50123/callback");
        assert_eq!(super::with_port("http://127.0.0.1/callback", 50123).unwrap(), "http://127.0.0.1:50123/callback");
    }
}
