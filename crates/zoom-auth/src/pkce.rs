//! RFC 7636 PKCE helpers (S256 only).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

use crate::Secret;

pub struct Pkce {
    pub verifier: Secret,
    pub challenge: String,
}

fn random_b64url(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("OS RNG unavailable");
    URL_SAFE_NO_PAD.encode(&buf)
}

/// `code_challenge = BASE64URL(SHA256(code_verifier))`
pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

impl Pkce {
    /// 32 random bytes -> 43-char verifier (RFC 7636 §4.1 allows 43..=128).
    pub fn generate() -> Self {
        let verifier = random_b64url(32);
        let challenge = challenge_for(&verifier);
        Pkce { verifier: Secret::new(verifier), challenge }
    }
}

/// Opaque anti-CSRF `state` value.
pub fn random_state() -> String {
    random_b64url(24)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc7636_appendix_b_vector() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_verifier_is_valid() {
        let p = Pkce::generate();
        let v = p.verifier.expose();
        assert_eq!(v.len(), 43);
        assert!(v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        assert_eq!(p.challenge, challenge_for(v));
        assert_ne!(Pkce::generate().verifier.expose(), v);
    }
}
