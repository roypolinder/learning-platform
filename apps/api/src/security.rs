//! Secrets are random 256-bit values encoded as base64url. Only their SHA-256
//! digest is persisted, so a database leak does not leak usable tokens.

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

pub const SESSION_TOKEN_PREFIX: &str = "lp_web_";
pub const MCP_TOKEN_PREFIX: &str = "lp_mcp_";

pub fn generate_token(prefix: &str) -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(format!("{prefix}{}", URL_SAFE_NO_PAD.encode(bytes)))
}

pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Short, non-secret fragment for identifying a token in listings.
pub fn token_hint(token: &str) -> String {
    let visible: String = token.chars().take(MCP_TOKEN_PREFIX.len() + 4).collect();
    format!("{visible}…")
}

pub fn hash_password(password: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), hash)
        .is_ok()
}

/// Runs a full verification against a throwaway hash so unknown emails take as
/// long as wrong passwords, which avoids account enumeration through timing.
pub fn burn_password_check(password: &str) {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let dummy = DUMMY.get_or_init(|| hash_password("timing-equalizer").unwrap_or_default());
    let _ = verify_password(password, dummy);
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_prefixed() {
        let first = generate_token(MCP_TOKEN_PREFIX).unwrap();
        let second = generate_token(MCP_TOKEN_PREFIX).unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with(MCP_TOKEN_PREFIX));
        assert_eq!(hash_token(&first).len(), 32);
        assert!(token_hint(&first).ends_with('…'));
    }

    #[test]
    fn passwords_round_trip() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}
