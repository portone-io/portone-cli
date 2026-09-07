use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow};
use base64::Engine;
use base64::engine::general_purpose::{URL_SAFE_NO_PAD, URL_SAFE_NO_PAD_INDIFFERENT};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;

use crate::config::OAuthTokens;

pub const CONSOLE_URL: &str = "https://admin.portone.io";
pub const MERCHANT_SERVICE_URL: &str = "https://merchant-service.prod.iamport.co";
pub const CLIENT_ID: &str = "CLI";
pub const REDIRECT_URI: &str = "http://127.0.0.1:1271/oauth/cli";
pub const DEFAULT_SCOPES: &[&str] = &[
    "HOME_AND_REPORT",
    "TX_READ",
    "CHANNEL_READ",
    "STORE_READ",
    "MERCHANT_READ",
];
pub const REFRESH_MARGIN_SECS: u64 = 60;
const TOKEN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthConfig {
    pub console_url: String,
    pub merchant_service_url: String,
    pub client_id: String,
    pub redirect_uri: Url,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthIssuer {
    pub client_id: String,
    pub token_url: String,
    pub console_url: String,
}

impl OAuthConfig {
    pub fn from_env(scopes: Option<Vec<String>>) -> anyhow::Result<Self> {
        let console_url = base_url_from_env("PORTONE_CONSOLE_URL", CONSOLE_URL)?;
        let merchant_service_url =
            base_url_from_env("PORTONE_MERCHANT_SERVICE_URL", MERCHANT_SERVICE_URL)?;
        let client_id = env_or("PORTONE_OAUTH_CLIENT_ID", CLIENT_ID);
        let redirect = env_or("PORTONE_OAUTH_REDIRECT_URI", REDIRECT_URI);
        let redirect_uri =
            Url::parse(&redirect).with_context(|| format!("invalid redirect URI: {redirect}"))?;
        let scopes = match scopes {
            Some(scopes) if !scopes.is_empty() => scopes,
            _ => DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        };
        Ok(Self {
            console_url,
            merchant_service_url,
            client_id,
            redirect_uri,
            scopes,
        })
    }

    pub fn token_url(&self) -> String {
        format!("{}/oauth/token", self.merchant_service_url)
    }

    pub fn issuer(&self) -> OAuthIssuer {
        OAuthIssuer {
            client_id: self.client_id.clone(),
            token_url: self.token_url(),
            console_url: self.console_url.clone(),
        }
    }
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn base_url_from_env(name: &str, default: &str) -> anyhow::Result<String> {
    let value = env_or(name, default);
    let parsed = Url::parse(&value).with_context(|| format!("{name} is not a URL: {value}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!("{name} must be an HTTP or HTTPS URL: {value}");
    }
    Ok(value.trim_end_matches('/').to_string())
}

#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

pub fn generate_pkce() -> anyhow::Result<Pkce> {
    let verifier = random_base64url(32)?;
    let challenge = code_challenge(&verifier);
    Ok(Pkce {
        verifier,
        challenge,
    })
}

pub fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn generate_state() -> anyhow::Result<String> {
    random_base64url(16)
}

pub fn random_base64url(len: usize) -> anyhow::Result<String> {
    let mut buf = vec![0u8; len];
    getrandom::getrandom(&mut buf)
        .map_err(|err| anyhow!("failed to generate random bytes: {err}"))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

pub fn authorize_url(cfg: &OAuthConfig, pkce: &Pkce, state: &str) -> Url {
    let mut url = Url::parse(&format!("{}/oauth/authorize", cfg.console_url))
        .expect("console_url was validated by from_env");
    url.query_pairs_mut()
        .append_pair("client_id", &cfg.client_id)
        .append_pair("redirect_uri", cfg.redirect_uri.as_str())
        .append_pair("response_type", "code")
        .append_pair("scope", &cfg.scopes.join(" "))
        .append_pair("state", state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256");
    url
}

#[derive(Debug)]
pub enum TokenError {
    InvalidGrant(String),
    Rejected { error: String, detail: String },
    Transient(anyhow::Error),
    Malformed(anyhow::Error),
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::InvalidGrant(detail) if detail.is_empty() => write!(f, "invalid_grant"),
            TokenError::InvalidGrant(detail) => write!(f, "invalid_grant: {detail}"),
            TokenError::Rejected { error, detail } if detail.is_empty() => write!(f, "{error}"),
            TokenError::Rejected { error, detail } => write!(f, "{error}: {detail}"),
            TokenError::Transient(err) | TokenError::Malformed(err) => write!(f, "{err:#}"),
        }
    }
}

impl std::error::Error for TokenError {}

pub fn exchange_code(
    agent: &ureq::Agent,
    cfg: &OAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<OAuthTokens, TokenError> {
    token_request(
        agent,
        &cfg.token_url(),
        json!({
            "client_id": cfg.client_id,
            "grant_type": "authorization_code",
            "code": code,
            "code_verifier": verifier,
        }),
        None,
    )
}

pub fn refresh(
    agent: &ureq::Agent,
    issuer: &OAuthIssuer,
    refresh_token: &str,
) -> Result<OAuthTokens, TokenError> {
    token_request(
        agent,
        &issuer.token_url,
        json!({
            "client_id": issuer.client_id,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }),
        Some(refresh_token),
    )
}

fn token_request(
    agent: &ureq::Agent,
    url: &str,
    body: Value,
    previous_refresh: Option<&str>,
) -> Result<OAuthTokens, TokenError> {
    let mut response = agent
        .post(url)
        .config()
        .timeout_global(Some(TOKEN_TIMEOUT))
        .build()
        .send_json(body)
        .map_err(|err| TokenError::Transient(anyhow!("token request failed: {err}")))?;
    let status = response.status().as_u16();
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|err| TokenError::Transient(anyhow!("failed to read token response: {err}")))?;
    classify_response(status, &bytes, now(), previous_refresh)
}

pub(crate) fn classify_response(
    status: u16,
    body: &[u8],
    now: u64,
    previous_refresh: Option<&str>,
) -> Result<OAuthTokens, TokenError> {
    if (200..300).contains(&status) {
        return parse_token_response(body, now, previous_refresh);
    }
    if status >= 500 {
        return Err(TokenError::Transient(anyhow!("HTTP {status}")));
    }
    let (error, detail) =
        parse_error_body(body).unwrap_or_else(|| (format!("HTTP {status}"), String::new()));
    if error == "invalid_grant" {
        Err(TokenError::InvalidGrant(detail))
    } else {
        Err(TokenError::Rejected { error, detail })
    }
}

fn parse_error_body(body: &[u8]) -> Option<(String, String)> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let error = value.get("error")?.as_str()?.to_string();
    let detail = value
        .get("detail")
        .or_else(|| value.get("error_description"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((error, detail))
}

fn parse_token_response(
    body: &[u8],
    now: u64,
    previous_refresh: Option<&str>,
) -> Result<OAuthTokens, TokenError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|err| TokenError::Malformed(anyhow!("failed to parse token response: {err}")))?;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| TokenError::Malformed(anyhow!("token response is missing access_token")))?
        .to_string();
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .ok_or_else(|| TokenError::Malformed(anyhow!("token response is missing expires_in")))?;
    let scope = match value.get("scope") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(text)) => text.split_whitespace().map(str::to_string).collect(),
        _ => Vec::new(),
    };
    let token_type = value
        .get("token_type")
        .and_then(Value::as_str)
        .unwrap_or("Bearer")
        .to_string();
    let refresh_token = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| previous_refresh.map(str::to_string));
    Ok(OAuthTokens {
        access_token,
        refresh_token,
        expires_at: now.saturating_add(expires_in),
        scope,
        token_type,
    })
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn needs_refresh(tokens: &OAuthTokens, now: u64) -> bool {
    tokens.expires_at <= now.saturating_add(REFRESH_MARGIN_SECS)
}

pub fn is_valid(tokens: &OAuthTokens, now: u64) -> bool {
    tokens.expires_at > now
}

pub fn jwt_exp(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD_INDIFFERENT.decode(payload).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value.get("exp")?.as_u64()
}

pub fn missing_scopes(requested: &[String], granted: &[String]) -> Vec<String> {
    requested
        .iter()
        .filter(|scope| !granted.contains(scope))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::with_env;

    fn oauth_env_cleared() -> [(&'static str, Option<&'static str>); 4] {
        [
            ("PORTONE_CONSOLE_URL", None),
            ("PORTONE_MERCHANT_SERVICE_URL", None),
            ("PORTONE_OAUTH_CLIENT_ID", None),
            ("PORTONE_OAUTH_REDIRECT_URI", None),
        ]
    }

    #[test]
    fn code_challenge_matches_rfc7636_vector() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_pkce_and_state_are_url_safe() {
        let pkce = generate_pkce().unwrap();
        assert_eq!(pkce.verifier.len(), 43);
        assert_eq!(pkce.challenge, code_challenge(&pkce.verifier));
        let state = generate_state().unwrap();
        assert_eq!(state.len(), 22);
        assert!(
            state
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn from_env_uses_defaults_and_overrides() {
        with_env(&oauth_env_cleared(), || {
            let cfg = OAuthConfig::from_env(None).unwrap();
            assert_eq!(cfg.console_url, CONSOLE_URL);
            assert_eq!(cfg.client_id, CLIENT_ID);
            assert_eq!(cfg.redirect_uri.as_str(), REDIRECT_URI);
            assert_eq!(
                cfg.token_url(),
                format!("{MERCHANT_SERVICE_URL}/oauth/token")
            );
            assert_eq!(cfg.scopes, DEFAULT_SCOPES);
        });
        with_env(
            &[
                ("PORTONE_CONSOLE_URL", Some("https://console.example/")),
                ("PORTONE_MERCHANT_SERVICE_URL", Some("https://ms.example")),
                ("PORTONE_OAUTH_CLIENT_ID", Some("MCP")),
                (
                    "PORTONE_OAUTH_REDIRECT_URI",
                    Some("http://127.0.0.1:1270/oauth/mcp"),
                ),
            ],
            || {
                let cfg = OAuthConfig::from_env(Some(vec!["TX_READ".to_string()])).unwrap();
                assert_eq!(cfg.console_url, "https://console.example");
                assert_eq!(cfg.token_url(), "https://ms.example/oauth/token");
                assert_eq!(cfg.client_id, "MCP");
                assert_eq!(cfg.redirect_uri.port(), Some(1270));
                assert_eq!(cfg.scopes, vec!["TX_READ".to_string()]);
                let issuer = cfg.issuer();
                assert_eq!(issuer.client_id, "MCP");
                assert_eq!(issuer.console_url, "https://console.example");
            },
        );
    }

    #[test]
    fn from_env_rejects_non_http_console_url() {
        with_env(&[("PORTONE_CONSOLE_URL", Some("ftp://x"))], || {
            assert!(OAuthConfig::from_env(None).is_err());
        });
    }

    #[test]
    fn authorize_url_contains_all_parameters() {
        with_env(&oauth_env_cleared(), || {
            let cfg = OAuthConfig::from_env(None).unwrap();
            let pkce = Pkce {
                verifier: "v".to_string(),
                challenge: "challenge-value".to_string(),
            };
            let url = authorize_url(&cfg, &pkce, "state-value");
            assert_eq!(url.path(), "/oauth/authorize");
            let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(pairs["client_id"], "CLI");
            assert_eq!(pairs["redirect_uri"], REDIRECT_URI);
            assert_eq!(pairs["response_type"], "code");
            assert_eq!(
                pairs["scope"],
                "HOME_AND_REPORT TX_READ CHANNEL_READ STORE_READ MERCHANT_READ"
            );
            assert_eq!(pairs["state"], "state-value");
            assert_eq!(pairs["code_challenge"], "challenge-value");
            assert_eq!(pairs["code_challenge_method"], "S256");
            assert_eq!(pairs.len(), 7);
        });
    }

    #[test]
    fn success_response_is_parsed_with_array_scope() {
        let body = br#"{"access_token":"a","token_type":"Bearer","expires_in":1800,"scope":["TX_READ","STORE_READ"],"refresh_token":"r"}"#;
        let tokens = classify_response(200, body, 1_000, None).unwrap();
        assert_eq!(tokens.access_token, "a");
        assert_eq!(tokens.refresh_token.as_deref(), Some("r"));
        assert_eq!(tokens.expires_at, 2_800);
        assert_eq!(tokens.scope, vec!["TX_READ", "STORE_READ"]);
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[test]
    fn missing_refresh_token_keeps_previous() {
        let body =
            br#"{"access_token":"a","token_type":"Bearer","expires_in":10,"scope":"TX_READ"}"#;
        let tokens = classify_response(200, body, 5, Some("old-refresh")).unwrap();
        assert_eq!(tokens.refresh_token.as_deref(), Some("old-refresh"));
        assert_eq!(tokens.scope, vec!["TX_READ"]);
        assert_eq!(tokens.expires_at, 15);
    }

    #[test]
    fn error_responses_are_classified() {
        let detail = br#"{"error":"invalid_grant","detail":"Invalid refresh_token"}"#;
        match classify_response(400, detail, 0, None) {
            Err(TokenError::InvalidGrant(d)) => assert_eq!(d, "Invalid refresh_token"),
            other => panic!("unexpected: {other:?}"),
        }
        let description = br#"{"error":"invalid_grant","error_description":"expired"}"#;
        match classify_response(400, description, 0, None) {
            Err(TokenError::InvalidGrant(d)) => assert_eq!(d, "expired"),
            other => panic!("unexpected: {other:?}"),
        }
        let client = br#"{"error":"invalid_client","detail":"Invalid client_id"}"#;
        match classify_response(400, client, 0, None) {
            Err(TokenError::Rejected { error, detail }) => {
                assert_eq!(error, "invalid_client");
                assert_eq!(detail, "Invalid client_id");
            }
            other => panic!("unexpected: {other:?}"),
        }
        assert!(matches!(
            classify_response(503, b"unavailable", 0, None),
            Err(TokenError::Transient(_))
        ));
        assert!(matches!(
            classify_response(200, b"not json", 0, None),
            Err(TokenError::Malformed(_))
        ));
        assert!(matches!(
            classify_response(200, br#"{"token_type":"Bearer"}"#, 0, None),
            Err(TokenError::Malformed(_))
        ));
        match classify_response(404, b"", 0, None) {
            Err(TokenError::Rejected { error, .. }) => assert_eq!(error, "HTTP 404"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn refresh_decision_has_no_underflow() {
        let tokens = |expires_at| OAuthTokens {
            access_token: "a".to_string(),
            refresh_token: None,
            expires_at,
            scope: vec![],
            token_type: "Bearer".to_string(),
        };
        assert!(needs_refresh(&tokens(0), 1_000));
        assert!(needs_refresh(&tokens(1_000 + 60), 1_000));
        assert!(!needs_refresh(&tokens(1_000 + 61), 1_000));
        assert!(needs_refresh(&tokens(u64::MAX), u64::MAX));
        assert!(is_valid(&tokens(1_001), 1_000));
        assert!(!is_valid(&tokens(1_000), 1_000));
    }

    #[test]
    fn jwt_exp_is_decoded_without_verification() {
        let payload = URL_SAFE_NO_PAD.encode(br#"{"exp":1788494662,"user_id":"u"}"#);
        let token = format!("eyJhbGciOiJFUzI1NiJ9.{payload}.signature");
        assert_eq!(jwt_exp(&token), Some(1788494662));
        assert_eq!(jwt_exp("not-a-jwt"), None);
        assert_eq!(jwt_exp("a.!!!.c"), None);
    }

    #[test]
    fn missing_scopes_reports_dropped_ones() {
        let requested = vec!["A".to_string(), "B".to_string()];
        let granted = vec!["B".to_string()];
        assert_eq!(missing_scopes(&requested, &granted), vec!["A".to_string()]);
        assert!(missing_scopes(&requested, &requested).is_empty());
    }
}
