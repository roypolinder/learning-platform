//! Request validation for the OAuth 2.1 authorization-code flow with PKCE, following
//! the MCP authorization spec. Kept free of I/O so the rules are unit-testable.

use axum::http::HeaderMap;
use axum::http::header::HOST;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::AppConfig;

pub const SCOPE: &str = "mcp";
const MAX_REDIRECT_URIS: usize = 10;
const MAX_URI_LENGTH: usize = 2000;
const BLOCKED_SCHEMES: &[&str] = &[
    "javascript",
    "data",
    "file",
    "vbscript",
    "blob",
    "about",
    "ws",
    "wss",
    "ftp",
];

/// Query parameters of an authorization request, shared by the public endpoint and
/// the consent API so both apply identical validation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthorizationParams {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub state: Option<String>,
    pub scope: Option<String>,
    pub resource: Option<String>,
}

/// An error that is reported back to the client through its redirect URI.
#[derive(Debug, PartialEq, Eq)]
pub struct RedirectableError {
    pub error: &'static str,
    pub description: String,
}

impl RedirectableError {
    fn new(error: &'static str, description: impl Into<String>) -> Self {
        Self {
            error,
            description: description.into(),
        }
    }
}

/// Checks everything except the client and redirect URI, which must be verified first.
/// Returns the granted scope.
pub fn validate_request(params: &AuthorizationParams) -> Result<String, RedirectableError> {
    if params.response_type.as_deref() != Some("code") {
        return Err(RedirectableError::new(
            "unsupported_response_type",
            "response_type must be `code`",
        ));
    }
    if params.code_challenge_method.as_deref() != Some("S256") {
        return Err(RedirectableError::new(
            "invalid_request",
            "PKCE with code_challenge_method `S256` is required",
        ));
    }
    let challenge = params.code_challenge.as_deref().unwrap_or_default();
    if challenge.len() != 43 || !challenge.bytes().all(is_base64url) {
        return Err(RedirectableError::new(
            "invalid_request",
            "code_challenge must be a base64url-encoded SHA-256 digest",
        ));
    }
    if let Some(resource) = params.resource.as_deref()
        && !is_mcp_resource(resource)
    {
        return Err(RedirectableError::new(
            "invalid_target",
            "resource must be this platform's MCP endpoint",
        ));
    }
    if params
        .state
        .as_deref()
        .is_some_and(|state| state.len() > 1024)
    {
        return Err(RedirectableError::new(
            "invalid_request",
            "state is too long",
        ));
    }
    granted_scope(params.scope.as_deref()).ok_or_else(|| {
        RedirectableError::new("invalid_scope", format!("only `{SCOPE}` is supported"))
    })
}

/// Absent scope means the default. Anything beyond `mcp` is rejected rather than
/// silently narrowed, so clients don't believe they hold permissions they don't.
pub fn granted_scope(requested: Option<&str>) -> Option<String> {
    requested
        .unwrap_or(SCOPE)
        .split_ascii_whitespace()
        .all(|scope| scope == SCOPE)
        .then(|| SCOPE.to_owned())
}

/// Tokens are only ever valid on `/mcp`, so the host part is not compared: agents may
/// reach the server as `localhost` or `127.0.0.1`.
pub fn is_mcp_resource(resource: &str) -> bool {
    Url::parse(resource).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https") && url.path().trim_end_matches('/') == "/mcp"
    })
}

pub fn verify_pkce(verifier: &str, challenge: &str) -> bool {
    let valid_verifier = (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte));
    valid_verifier && URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == challenge
}

fn is_base64url(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

/// Accepts https, http on loopback (native apps, RFC 8252 §7.3) and private-use
/// schemes such as `vscode://` (RFC 8252 §7.1).
pub fn validate_redirect_uris(uris: &[String]) -> Result<(), String> {
    if uris.is_empty() || uris.len() > MAX_REDIRECT_URIS {
        return Err(format!(
            "redirect_uris must contain between 1 and {MAX_REDIRECT_URIS} URIs"
        ));
    }
    for raw in uris {
        if raw.len() > MAX_URI_LENGTH {
            return Err("redirect URI is too long".into());
        }
        let url = Url::parse(raw).map_err(|_| format!("`{raw}` is not an absolute URI"))?;
        if url.fragment().is_some() {
            return Err(format!("`{raw}` must not contain a fragment"));
        }
        match url.scheme() {
            "https" => {}
            "http" if is_loopback(&url) => {}
            "http" => return Err(format!("`{raw}` must use https or a loopback host")),
            scheme if BLOCKED_SCHEMES.contains(&scheme) => {
                return Err(format!("`{raw}` uses a disallowed scheme"));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Exact match, except that loopback redirects may use any port (RFC 8252 §7.3).
pub fn redirect_uri_matches(registered: &str, requested: &str) -> bool {
    if registered == requested {
        return true;
    }
    let (Ok(registered), Ok(requested)) = (Url::parse(registered), Url::parse(requested)) else {
        return false;
    };
    registered.scheme() == "http"
        && requested.scheme() == "http"
        && is_loopback(&registered)
        && registered.host_str() == requested.host_str()
        && registered.path() == requested.path()
        && registered.query() == requested.query()
        && requested.fragment().is_none()
        && requested.username().is_empty()
        && requested.password().is_none()
}

fn is_loopback(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
}

pub fn redirect_with(redirect_uri: &str, params: &[(&str, &str)]) -> String {
    match Url::parse(redirect_uri) {
        Ok(mut url) => {
            url.query_pairs_mut().extend_pairs(params);
            url.into()
        }
        Err(_) => redirect_uri.to_owned(),
    }
}

/// Origin the client used to reach us. OAuth clients compare the issuer and resource
/// in metadata with the URL they requested, so `127.0.0.1` and `localhost` must each
/// see themselves. Only hosts on the MCP allow-list are reflected.
pub fn request_origin(config: &AppConfig, headers: &HeaderMap) -> String {
    let scheme = config
        .public_base_url
        .split_once("://")
        .map_or("http", |(scheme, _)| scheme);
    headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|host| {
            let name = host_name(host);
            config
                .mcp_allowed_hosts
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(name))
        })
        .map_or_else(
            || config.public_base_url.clone(),
            |host| format!("{scheme}://{host}"),
        )
}

fn host_name(host: &str) -> &str {
    if let Some(rest) = host.strip_prefix('[') {
        return rest.split(']').next().unwrap_or_default();
    }
    host.split(':').next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> AuthorizationParams {
        AuthorizationParams {
            response_type: Some("code".into()),
            client_id: Some("client".into()),
            redirect_uri: Some("http://127.0.0.1:33418/".into()),
            code_challenge: Some(URL_SAFE_NO_PAD.encode(Sha256::digest(b"verifier"))),
            code_challenge_method: Some("S256".into()),
            ..Default::default()
        }
    }

    #[test]
    fn requires_pkce_s256_and_known_scope() {
        assert_eq!(validate_request(&params()).unwrap(), SCOPE);
        let plain = AuthorizationParams {
            code_challenge_method: Some("plain".into()),
            ..params()
        };
        assert_eq!(
            validate_request(&plain).unwrap_err().error,
            "invalid_request"
        );
        let admin = AuthorizationParams {
            scope: Some("mcp admin".into()),
            ..params()
        };
        assert_eq!(validate_request(&admin).unwrap_err().error, "invalid_scope");
        let other = AuthorizationParams {
            resource: Some("https://evil.example/api".into()),
            ..params()
        };
        assert_eq!(
            validate_request(&other).unwrap_err().error,
            "invalid_target"
        );
        let ours = AuthorizationParams {
            resource: Some("http://localhost:8080/mcp".into()),
            ..params()
        };
        assert!(validate_request(&ours).is_ok());
    }

    #[test]
    fn verifies_pkce() {
        let verifier = "a".repeat(43);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert!(verify_pkce(&verifier, &challenge));
        assert!(!verify_pkce(&"b".repeat(43), &challenge));
        assert!(!verify_pkce("short", &challenge));
    }

    #[test]
    fn restricts_redirect_uris() {
        let ok = |uri: &str| validate_redirect_uris(&[uri.to_owned()]).is_ok();
        assert!(ok("https://vscode.dev/redirect"));
        assert!(ok("http://127.0.0.1:33418/"));
        assert!(ok("vscode://vscode.github-authentication/did-authenticate"));
        assert!(!ok("http://example.com/callback"));
        assert!(!ok("javascript:alert(1)"));
        assert!(!ok("https://example.com/cb#fragment"));
        assert!(validate_redirect_uris(&[]).is_err());
    }

    #[test]
    fn loopback_redirects_ignore_the_port_only() {
        assert!(redirect_uri_matches(
            "http://127.0.0.1:33418/",
            "http://127.0.0.1:50000/"
        ));
        assert!(!redirect_uri_matches(
            "http://127.0.0.1/",
            "http://localhost/"
        ));
        assert!(!redirect_uri_matches(
            "http://127.0.0.1/a",
            "http://127.0.0.1/b"
        ));
        assert!(!redirect_uri_matches(
            "https://vscode.dev/redirect",
            "https://vscode.dev/other"
        ));
    }

    #[test]
    fn redirect_adds_query_parameters() {
        assert_eq!(
            redirect_with(
                "http://127.0.0.1:1/cb?x=1",
                &[("code", "a b"), ("state", "s")]
            ),
            "http://127.0.0.1:1/cb?x=1&code=a+b&state=s"
        );
    }
}
