pub mod browser;
pub mod callback;
pub mod oauth;
pub mod store;
pub mod store_discovery;

use std::io::Write;

use anyhow::anyhow;
use serde_json::Value;

use crate::config::{Config, DEFAULT_BASE_URL, OAuthProfile, OAuthTokens, Storage};
use crate::error::CliError;
use crate::i18n::{LocalizedContext, LocalizedErrorContext, Localizer};
use oauth::{OAuthIssuer, TokenError};
use store::{KEYRING_SERVICE, SecretStore};

pub const ACCESS_TOKEN_ENV: &str = "PORTONE_ACCESS_TOKEN";
pub const SESSION_EXPIRED: &str =
    "console login session has expired; run `portone auth login` to authenticate again";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthSource {
    Env(&'static str),
    ConfigProfile(String),
    Keyring(String),
}

#[derive(Debug, Clone)]
pub struct OAuthSession {
    pub profile: String,
    pub oauth: OAuthProfile,
    pub tokens: OAuthTokens,
}

impl OAuthSession {
    pub fn issuer(&self) -> OAuthIssuer {
        OAuthIssuer {
            client_id: self.oauth.client_id.clone(),
            token_url: self.oauth.token_url.clone(),
            console_url: self.oauth.console_url.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub access_token: String,
    pub source: AuthSource,
    pub oauth: Option<OAuthSession>,
}

impl ResolvedAuth {
    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }
}

pub fn active_env_credential() -> Option<&'static str> {
    env_value(ACCESS_TOKEN_ENV).map(|_| ACCESS_TOKEN_ENV)
}

pub fn resolve_static() -> Option<ResolvedAuth> {
    if let Some(token) = env_value(ACCESS_TOKEN_ENV) {
        return Some(ResolvedAuth {
            access_token: token,
            source: AuthSource::Env(ACCESS_TOKEN_ENV),
            oauth: None,
        });
    }
    None
}

pub fn resolve(
    store: &dyn SecretStore,
    profile: Option<&str>,
    config: &Config,
) -> anyhow::Result<Option<ResolvedAuth>> {
    if let Some(resolved) = resolve_static() {
        return Ok(Some(resolved));
    }
    let name = profile_name(profile, config);
    let Some(oauth) = config.profiles.get(&name).and_then(|p| p.oauth.as_ref()) else {
        return Ok(None);
    };
    let Some(tokens) = load_tokens(store, &name, oauth)? else {
        return Ok(None);
    };
    let source = match oauth.storage {
        Storage::Keyring => AuthSource::Keyring(oauth.credential_id.clone().unwrap_or_default()),
        Storage::File => AuthSource::ConfigProfile(name.clone()),
    };
    Ok(Some(ResolvedAuth {
        access_token: tokens.access_token.clone(),
        source,
        oauth: Some(OAuthSession {
            profile: name,
            oauth: oauth.clone(),
            tokens,
        }),
    }))
}

pub fn load_tokens(
    store: &dyn SecretStore,
    profile: &str,
    oauth: &OAuthProfile,
) -> anyhow::Result<Option<OAuthTokens>> {
    match oauth.storage {
        Storage::File => Ok(oauth.tokens.clone()),
        Storage::Keyring => {
            let Some(id) = normalize(oauth.credential_id.as_deref()) else {
                return Ok(None);
            };
            store.load(&id).map_err(|err| {
                err.into_anyhow().lcontext(crate::message!(
                    "auth-keyring-load-failed",
                    profile = profile,
                    service = KEYRING_SERVICE,
                    id = id
                ))
            })
        }
    }
}

pub fn resolve_fresh(
    agent: &ureq::Agent,
    store: &dyn SecretStore,
    config: &mut Config,
    profile: Option<&str>,
    err: &mut dyn Write,
) -> Result<Option<ResolvedAuth>, CliError> {
    resolve_fresh_localized(agent, store, config, profile, err, &Localizer::english())
}

pub fn resolve_fresh_localized(
    agent: &ureq::Agent,
    store: &dyn SecretStore,
    config: &mut Config,
    profile: Option<&str>,
    err: &mut dyn Write,
    localizer: &Localizer,
) -> Result<Option<ResolvedAuth>, CliError> {
    let Some(mut resolved) = resolve(store, profile, config)? else {
        return Ok(None);
    };
    let Some(mut session) = resolved.oauth.take() else {
        return Ok(Some(resolved));
    };
    let now = oauth::now();
    if !oauth::needs_refresh(&session.tokens, now) {
        return Ok(Some(finish(resolved, session)));
    }

    let lock_key = normalize(session.oauth.credential_id.as_deref())
        .unwrap_or_else(|| format!("profile-{}", session.profile));
    let _lock = store::lock_refresh(&lock_key)?;

    // A previous lock holder may have switched from keyring to file storage.
    *config = Config::load()?;
    let Some(oauth) = config
        .profiles
        .get(&session.profile)
        .and_then(|profile| profile.oauth.as_ref())
    else {
        return Ok(None);
    };
    session.oauth = oauth.clone();
    let Some(latest) = load_tokens(store, &session.profile, &session.oauth)? else {
        return Ok(None);
    };
    let changed = latest != session.tokens;
    session.tokens = latest;
    let now = oauth::now();
    if changed && oauth::is_valid(&session.tokens, now) {
        return Ok(Some(finish(resolved, session)));
    }

    let Some(refresh_token) = normalize(session.tokens.refresh_token.as_deref()) else {
        return Err(CliError::Flag(crate::tr!(
            localizer,
            "auth-session-expired"
        )));
    };
    match oauth::refresh(agent, &session.issuer(), &refresh_token) {
        Ok(tokens) => {
            persist_refreshed(store, config, &mut session, &tokens, err, localizer)?;
            session.tokens = tokens;
            Ok(Some(finish(resolved, session)))
        }
        Err(TokenError::InvalidGrant(_)) => Err(CliError::Flag(crate::tr!(
            localizer,
            "auth-session-expired"
        ))),
        Err(TokenError::Rejected { error, detail }) => Err(CliError::Other(anyhow!(
            crate::message!("auth-refresh-rejected", error = error, detail = detail)
        ))),
        Err(error) => {
            if oauth::is_valid(&session.tokens, now) {
                let _ = writeln!(
                    err,
                    "{}",
                    crate::tr!(
                        localizer,
                        "auth-refresh-failed-continuing",
                        error = error.localized(localizer)
                    )
                );
                Ok(Some(finish(resolved, session)))
            } else {
                Err(CliError::Other(
                    error
                        .into_anyhow()
                        .lcontext(crate::message!("auth-refresh-failed")),
                ))
            }
        }
    }
}

fn finish(mut resolved: ResolvedAuth, session: OAuthSession) -> ResolvedAuth {
    resolved.access_token = session.tokens.access_token.clone();
    resolved.source = match session.oauth.storage {
        Storage::Keyring => {
            AuthSource::Keyring(session.oauth.credential_id.clone().unwrap_or_default())
        }
        Storage::File => AuthSource::ConfigProfile(session.profile.clone()),
    };
    resolved.oauth = Some(session);
    resolved
}

fn persist_refreshed(
    store: &dyn SecretStore,
    config: &mut Config,
    session: &mut OAuthSession,
    tokens: &OAuthTokens,
    err: &mut dyn Write,
    localizer: &Localizer,
) -> Result<(), CliError> {
    if session.oauth.storage == Storage::Keyring
        && let Some(id) = normalize(session.oauth.credential_id.as_deref())
    {
        match store.save(&id, tokens) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let _ = writeln!(
                    err,
                    "{}",
                    crate::tr!(
                        localizer,
                        "auth-refreshed-keyring-fallback",
                        error = error.localized(localizer)
                    )
                );
            }
        }
    }
    // Credential locks do not serialize updates to different profiles in this file.
    let _config_lock = store::lock_config().map_err(save_error)?;
    let mut fresh = Config::load().map_err(save_error)?;
    let profile = fresh.profiles.entry(session.profile.clone()).or_default();
    let mut oauth = profile
        .oauth
        .clone()
        .unwrap_or_else(|| session.oauth.clone());
    oauth.storage = Storage::File;
    oauth.tokens = Some(tokens.clone());
    profile.oauth = Some(oauth.clone());
    fresh.save().map_err(save_error)?;
    session.oauth = oauth;
    *config = fresh;
    Ok(())
}

fn save_error(err: anyhow::Error) -> CliError {
    CliError::Other(err.lcontext(crate::message!("auth-refreshed-save-failed")))
}

pub fn resolve_base_url(flag: Option<&str>, profile: Option<&str>, config: &Config) -> String {
    if let Some(url) = normalize(flag) {
        return url;
    }
    if let Some(url) = env_value("PORTONE_API_BASE") {
        return url;
    }
    let name = profile_name(profile, config);
    if let Some(url) = config
        .profiles
        .get(&name)
        .and_then(|profile| normalize(profile.base_url.as_deref()))
    {
        return url;
    }
    DEFAULT_BASE_URL.to_string()
}

pub fn profile_name(profile: Option<&str>, config: &Config) -> String {
    profile
        .or(config.default_profile.as_deref())
        .unwrap_or("default")
        .to_string()
}

pub(crate) fn normalize(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn env_value(name: &str) -> Option<String> {
    normalize(std::env::var(name).ok().as_deref())
}

pub fn verify_bearer(
    agent: &ureq::Agent,
    base_url: &str,
    access_token: &str,
) -> anyhow::Result<Option<String>> {
    let url = format!("{}/graphql", base_url.trim_end_matches('/'));
    let mut response = agent
        .post(&url)
        .header("Authorization", &format!("Bearer {access_token}"))
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .send_json(serde_json::json!({
            "query": "query { merchant { __typename ... on Merchant { plainId } } }"
        }))
        .lcontext(crate::message!("auth-validation-request-failed"))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Ok(None);
    }
    let value: Value = response
        .body_mut()
        .read_json()
        .lcontext(crate::message!("auth-validation-parse-failed"))?;
    let merchant = value.pointer("/data/merchant");
    let is_merchant = merchant
        .and_then(|m| m.get("__typename"))
        .and_then(Value::as_str)
        == Some("Merchant");
    if !is_merchant {
        return Ok(None);
    }
    Ok(Some(
        merchant
            .and_then(|m| m.get("plainId"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;
    use crate::config::paths::with_env;
    use store::StoreError;
    use store::testing::MemoryStore;

    fn without_auth_env(f: impl FnOnce()) {
        with_env(&[(ACCESS_TOKEN_ENV, None), ("PORTONE_API_BASE", None)], f);
    }

    fn config_with_base(base_url: &str) -> Config {
        let mut config = Config {
            default_profile: Some("default".to_string()),
            ..Config::default()
        };
        config.profiles.insert(
            "default".to_string(),
            Profile {
                base_url: Some(base_url.to_string()),
                store_id: None,
                oauth: None,
            },
        );
        config
    }

    fn tokens(access: &str, expires_at: u64) -> OAuthTokens {
        OAuthTokens {
            access_token: access.to_string(),
            refresh_token: Some(format!("refresh-{access}")),
            expires_at,
            scope: vec!["TX_READ".to_string()],
            token_type: "Bearer".to_string(),
        }
    }

    fn oauth_profile(
        storage: Storage,
        token_url: &str,
        tokens: Option<OAuthTokens>,
    ) -> OAuthProfile {
        OAuthProfile {
            storage,
            client_id: "CLI".to_string(),
            token_url: token_url.to_string(),
            console_url: "https://console.example".to_string(),
            credential_id: Some("cred-1".to_string()),
            tokens,
        }
    }

    fn oauth_config(storage: Storage, token_url: &str, tokens: Option<OAuthTokens>) -> Config {
        let mut config = Config::default();
        config.profiles.insert(
            "default".to_string(),
            Profile {
                base_url: None,
                store_id: None,
                oauth: Some(oauth_profile(storage, token_url, tokens)),
            },
        );
        config
    }

    fn with_config_dir(f: impl FnOnce()) {
        let dir = tempfile::tempdir().unwrap();
        with_env(
            &[
                ("PORTONE_CONFIG_DIR", Some(dir.path().to_str().unwrap())),
                (ACCESS_TOKEN_ENV, None),
                ("PORTONE_API_BASE", None),
            ],
            f,
        );
    }

    fn refresh_mock<'a>(
        server: &'a httpmock::MockServer,
        status: u16,
        body: &str,
    ) -> httpmock::Mock<'a> {
        let body = body.to_string();
        server.mock(move |when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/token")
                .json_body_includes(r#"{"client_id":"CLI","grant_type":"refresh_token","refresh_token":"refresh-old"}"#);
            then.status(status)
                .header("content-type", "application/json")
                .body(body.clone());
        })
    }

    const NEW_TOKENS: &str = r#"{"access_token":"new","token_type":"Bearer","expires_in":1800,"scope":["TX_READ"],"refresh_token":"refresh-new"}"#;

    #[test]
    fn resolve_static_uses_access_token_env() {
        with_env(&[(ACCESS_TOKEN_ENV, Some("  console-token  "))], || {
            let resolved = resolve_static().unwrap();
            assert_eq!(resolved.access_token, "console-token");
            assert_eq!(resolved.source, AuthSource::Env(ACCESS_TOKEN_ENV));
            assert_eq!(resolved.authorization_header(), "Bearer console-token");
        });
    }

    #[test]
    fn resolve_loads_file_and_keyring_tokens() {
        without_auth_env(|| {
            let store = MemoryStore::with("cred-1", tokens("from-keyring", 10));
            let file = oauth_config(Storage::File, "u", Some(tokens("from-file", 10)));
            let resolved = resolve(&store, None, &file).unwrap().unwrap();
            assert_eq!(resolved.access_token, "from-file");
            assert_eq!(
                resolved.source,
                AuthSource::ConfigProfile("default".to_string())
            );
            assert_eq!(resolved.oauth.unwrap().tokens.access_token, "from-file");

            let keyring = oauth_config(Storage::Keyring, "u", None);
            let resolved = resolve(&store, None, &keyring).unwrap().unwrap();
            assert_eq!(resolved.access_token, "from-keyring");
            assert_eq!(resolved.source, AuthSource::Keyring("cred-1".to_string()));
        });
    }

    #[test]
    fn resolve_distinguishes_missing_entry_from_keyring_errors() {
        without_auth_env(|| {
            let keyring = oauth_config(Storage::Keyring, "u", None);
            let empty = MemoryStore::default();
            assert!(resolve(&empty, None, &keyring).unwrap().is_none());

            let broken = MemoryStore::default();
            *broken.fail_load.borrow_mut() = Some(StoreError::Timeout);
            let err = format!("{:#}", resolve(&broken, None, &keyring).unwrap_err());
            assert!(err.contains("portone-cli/cred-1"), "{err}");
            assert!(err.contains("30 seconds"), "{err}");
        });
    }

    #[test]
    fn resolve_fresh_skips_refresh_for_valid_tokens() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = refresh_mock(&server, 200, NEW_TOKENS);
            let store = MemoryStore::default();
            let mut config = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", oauth::now() + 3600)),
            );
            let mut err = Vec::new();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "old");
            mock.assert_calls(0);
        });
    }

    #[test]
    fn resolve_fresh_refreshes_and_persists_file_tokens() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = refresh_mock(&server, 200, NEW_TOKENS);
            let store = MemoryStore::default();
            let mut config = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", 1)),
            );
            config.save().unwrap();
            let mut err = Vec::new();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "new");
            mock.assert_calls(1);
            let saved = Config::load().unwrap();
            let saved_tokens = saved.profiles["default"]
                .oauth
                .as_ref()
                .unwrap()
                .tokens
                .clone()
                .unwrap();
            assert_eq!(saved_tokens.access_token, "new");
            assert_eq!(saved_tokens.refresh_token.as_deref(), Some("refresh-new"));
            assert_eq!(
                config.profiles["default"]
                    .oauth
                    .as_ref()
                    .unwrap()
                    .tokens
                    .as_ref()
                    .unwrap()
                    .access_token,
                "new"
            );
            assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
        });
    }

    #[test]
    fn resolve_fresh_refreshes_keyring_tokens_in_place() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = refresh_mock(&server, 200, NEW_TOKENS);
            let store = MemoryStore::with("cred-1", tokens("old", 1));
            let mut config = oauth_config(Storage::Keyring, &server.url("/oauth/token"), None);
            config.save().unwrap();
            let mut err = Vec::new();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "new");
            mock.assert_calls(1);
            assert_eq!(store.entries.borrow()["cred-1"].access_token, "new");
            assert!(
                Config::load().unwrap().profiles["default"]
                    .oauth
                    .as_ref()
                    .unwrap()
                    .tokens
                    .is_none()
            );
        });
    }

    #[test]
    fn resolve_fresh_downgrades_to_file_when_keyring_save_fails() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = refresh_mock(&server, 200, NEW_TOKENS);
            let store = MemoryStore::with("cred-1", tokens("old", 1));
            *store.fail_save.borrow_mut() = Some(StoreError::Unavailable("locked".to_string()));
            let mut config = oauth_config(Storage::Keyring, &server.url("/oauth/token"), None);
            config.save().unwrap();
            let mut waiting_config = config.clone();
            let mut err = Vec::new();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "new");
            let saved = Config::load().unwrap();
            let oauth = saved.profiles["default"].oauth.as_ref().unwrap();
            assert_eq!(oauth.storage, Storage::File);
            assert_eq!(oauth.tokens.as_ref().unwrap().access_token, "new");
            assert_eq!(oauth.credential_id.as_deref(), Some("cred-1"));
            let text = String::from_utf8_lossy(&err);
            assert!(text.contains("storing them in the config file"), "{text}");

            // The waiting process still has keyring metadata and rotated-out tokens.
            let waiting = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut waiting_config,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(waiting.access_token, "new");
            assert_eq!(
                waiting.source,
                AuthSource::ConfigProfile("default".to_string())
            );
            assert_eq!(waiting.oauth.unwrap().oauth.storage, Storage::File);
            assert_eq!(
                waiting_config.profiles["default"].oauth.as_ref().unwrap(),
                oauth
            );
            mock.assert_calls(1);
        });
    }

    #[test]
    fn persist_refreshed_serializes_config_updates_across_profiles() {
        with_config_dir(|| {
            use std::sync::mpsc;
            use std::time::Duration;

            let mut config = Config::default();
            for name in ["first", "second"] {
                let mut oauth = oauth_profile(Storage::File, "u", Some(tokens("old", 1)));
                oauth.credential_id = None;
                config.profiles.insert(
                    name.to_string(),
                    Profile {
                        oauth: Some(oauth),
                        ..Profile::default()
                    },
                );
            }
            config.save().unwrap();

            let config_lock = store::lock_config().unwrap();
            let (started_tx, started_rx) = mpsc::channel();
            let (done_tx, done_rx) = mpsc::channel();
            let mut workers = Vec::new();
            for name in ["first", "second"] {
                let mut snapshot = config.clone();
                let started = started_tx.clone();
                let done = done_tx.clone();
                workers.push(std::thread::spawn(move || {
                    let _refresh_lock = store::lock_refresh(&format!("profile-{name}")).unwrap();
                    let mut session = OAuthSession {
                        profile: name.to_string(),
                        oauth: snapshot.profiles[name].oauth.clone().unwrap(),
                        tokens: tokens("old", 1),
                    };
                    started.send(()).unwrap();
                    let result = persist_refreshed(
                        &MemoryStore::default(),
                        &mut snapshot,
                        &mut session,
                        &tokens(name, oauth::now() + 3600),
                        &mut Vec::new(),
                        &Localizer::english(),
                    );
                    done.send(()).unwrap();
                    result.unwrap();
                }));
            }
            for _ in 0..2 {
                started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            let blocked = matches!(
                done_rx.recv_timeout(Duration::from_millis(200)),
                Err(mpsc::RecvTimeoutError::Timeout)
            );

            // Both writers must load this update after acquiring the shared lock.
            config
                .profiles
                .insert("unrelated".to_string(), Profile::default());
            config.save().unwrap();
            drop(config_lock);
            for worker in workers {
                worker.join().unwrap();
            }
            assert!(blocked, "token persistence bypassed the config lock");
            let saved = Config::load().unwrap();
            assert!(saved.profiles.contains_key("unrelated"));
            for name in ["first", "second"] {
                let saved_tokens = saved.profiles[name]
                    .oauth
                    .as_ref()
                    .unwrap()
                    .tokens
                    .as_ref()
                    .unwrap();
                assert_eq!(saved_tokens.access_token, name);
                assert_eq!(saved_tokens.refresh_token, Some(format!("refresh-{name}")));
            }
        });
    }

    #[test]
    fn resolve_fresh_uses_latest_refresh_token_even_when_access_token_expired() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = server.mock(|when, then| {
                when.method(httpmock::Method::POST)
                    .path("/oauth/token")
                    .json_body_includes(r#"{"refresh_token":"refresh-latest"}"#);
                then.status(200)
                    .header("content-type", "application/json")
                    .body(NEW_TOKENS);
            });
            let mut stale = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", 1)),
            );
            oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("latest", 1)),
            )
            .save()
            .unwrap();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &MemoryStore::default(),
                &mut stale,
                None,
                &mut Vec::new(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "new");
            mock.assert_calls(1);
        });
    }

    #[test]
    fn resolve_fresh_uses_tokens_refreshed_by_another_process() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = refresh_mock(&server, 200, NEW_TOKENS);
            let store = MemoryStore::default();
            let on_disk = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("fresh", oauth::now() + 3600)),
            );
            on_disk.save().unwrap();
            let mut stale = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", 1)),
            );
            let mut err = Vec::new();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut stale,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "fresh");
            mock.assert_calls(0);
        });
    }

    #[test]
    fn resolve_fresh_invalid_grant_requires_relogin() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            let mock = refresh_mock(
                &server,
                400,
                r#"{"error":"invalid_grant","detail":"Invalid refresh_token"}"#,
            );
            let store = MemoryStore::default();
            let mut config = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", 1)),
            );
            config.save().unwrap();
            let mut err = Vec::new();
            let result = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            );
            match result {
                Err(CliError::Flag(message)) => assert_eq!(message, SESSION_EXPIRED),
                other => panic!("unexpected: {other:?}"),
            }
            mock.assert_calls(1);
        });
    }

    #[test]
    fn resolve_fresh_rejected_reports_without_relogin_hint() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            refresh_mock(
                &server,
                400,
                r#"{"error":"invalid_client","detail":"Invalid client_id"}"#,
            );
            let store = MemoryStore::default();
            let mut config = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", 1)),
            );
            config.save().unwrap();
            let mut err = Vec::new();
            let result = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            );
            match result {
                Err(CliError::Other(error)) => {
                    let text = format!("{error:#}");
                    assert!(text.contains("invalid_client"), "{text}");
                    assert!(!text.contains("authenticate again"), "{text}");
                }
                other => panic!("unexpected: {other:?}"),
            }
        });
    }

    #[test]
    fn resolve_fresh_transient_failure_keeps_valid_token_but_fails_expired() {
        with_config_dir(|| {
            let server = httpmock::MockServer::start();
            refresh_mock(&server, 503, "down");
            let store = MemoryStore::default();
            let mut config = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", oauth::now() + 30)),
            );
            config.save().unwrap();
            let mut err = Vec::new();
            let resolved = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut config,
                None,
                &mut err,
            )
            .unwrap()
            .unwrap();
            assert_eq!(resolved.access_token, "old");
            assert!(String::from_utf8_lossy(&err).contains("continuing with the current token"));

            let mut expired = oauth_config(
                Storage::File,
                &server.url("/oauth/token"),
                Some(tokens("old", 1)),
            );
            expired.save().unwrap();
            let mut err = Vec::new();
            let result = resolve_fresh(
                &crate::http::build_agent(),
                &store,
                &mut expired,
                None,
                &mut err,
            );
            match result {
                Err(CliError::Other(error)) => {
                    assert!(format!("{error:#}").contains("token refresh failed"))
                }
                other => panic!("unexpected: {other:?}"),
            }
        });
    }

    #[test]
    fn resolve_base_url_priority() {
        without_auth_env(|| {
            let config = config_with_base("https://profile.example");
            assert_eq!(
                resolve_base_url(Some("https://flag.example"), None, &config),
                "https://flag.example"
            );
            assert_eq!(
                resolve_base_url(None, None, &config),
                "https://profile.example"
            );
            assert_eq!(
                resolve_base_url(None, None, &Config::default()),
                DEFAULT_BASE_URL
            );
        });
        with_env(&[("PORTONE_API_BASE", Some("https://env.example"))], || {
            let config = config_with_base("https://profile.example");
            assert_eq!(
                resolve_base_url(Some("https://flag.example"), None, &config),
                "https://flag.example"
            );
            assert_eq!(resolve_base_url(None, None, &config), "https://env.example");
        });
    }

    #[test]
    fn active_env_credential_reports_access_token_only() {
        with_env(&[(ACCESS_TOKEN_ENV, None)], || {
            assert_eq!(active_env_credential(), None);
        });
        with_env(&[(ACCESS_TOKEN_ENV, Some("t"))], || {
            assert_eq!(active_env_credential(), Some(ACCESS_TOKEN_ENV));
        });
    }

    #[test]
    fn verify_bearer_checks_merchant_typename() {
        let server = httpmock::MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/graphql")
                .header("authorization", "Bearer good");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":{"merchant":{"__typename":"Merchant","plainId":"merchant-1"}}}"#);
        });
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/graphql")
                .header("authorization", "Bearer forbidden");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":{"merchant":{"__typename":"UnauthorizedError"}}}"#);
        });
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/graphql")
                .header("authorization", "Bearer rejected");
            then.status(401);
        });
        let agent = crate::http::build_agent();
        assert_eq!(
            verify_bearer(&agent, &server.base_url(), "good").unwrap(),
            Some("merchant-1".to_string())
        );
        assert_eq!(
            verify_bearer(&agent, &server.base_url(), "forbidden").unwrap(),
            None
        );
        assert_eq!(
            verify_bearer(&agent, &server.base_url(), "rejected").unwrap(),
            None
        );
    }

    #[test]
    fn nested_keyring_errors_are_localized_at_the_output_boundary() {
        without_auth_env(|| {
            let config = oauth_config(Storage::Keyring, "u", None);
            let store = MemoryStore::default();
            *store.fail_load.borrow_mut() = Some(StoreError::Timeout);
            let error = resolve(&store, None, &config).unwrap_err();
            let english = Localizer::english().format_error(&error);
            let korean = Localizer::korean().format_error(&error);
            assert!(english.contains("failed to read tokens"), "{english}");
            assert!(
                english.contains("keyring did not respond within 30 seconds"),
                "{english}"
            );
            assert!(korean.contains("토큰을 읽지 못했습니다"), "{korean}");
            assert!(
                korean.contains("키링이 30초 이내에 응답하지 않았습니다"),
                "{korean}"
            );
            assert!(korean.contains("portone-cli/cred-1"), "{korean}");
        });
    }
}
