use std::io::Write;
use std::time::Duration;

use anyhow::anyhow;
use clap::Args;

use crate::auth::callback::{Callback, CallbackServer};
use crate::auth::oauth::{self, OAuthConfig};
use crate::auth::store::{self, KEYRING_SERVICE, SecretStore, StoreError};
use crate::auth::{self, browser};
use crate::config::{Config, OAuthProfile, OAuthTokens, Storage};
use crate::error::CliError;
use crate::factory::Factory;

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to store")]
    pub profile: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Base URL for API requests (default: https://api.portone.io)"
    )]
    pub base_url: Option<String>,

    #[arg(
        long,
        value_name = "SCOPES",
        value_delimiter = ',',
        help = "Comma-separated console scopes to request (default: HOME_AND_REPORT,TX_READ,CHANNEL_READ,STORE_READ,MERCHANT_READ)"
    )]
    pub scopes: Option<Vec<String>>,

    #[arg(
        long,
        help = "Store tokens in the config file instead of the OS keyring"
    )]
    pub insecure_storage: bool,

    #[arg(long, help = "Print the login URL without opening a browser")]
    pub no_browser: bool,
}

pub fn run(f: &mut Factory, args: LoginArgs) -> Result<(), CliError> {
    if let Some(name) = auth::active_env_credential() {
        let _ = writeln!(
            f.io.err,
            "portone: the {name} environment variable is being used for authentication; unset it before storing login credentials"
        );
        return Err(CliError::Silent);
    }
    let mut config = f.config()?.clone();
    let profile_name = args
        .profile
        .clone()
        .unwrap_or_else(|| "default".to_string());
    login_oauth(f, &mut config, &profile_name, &args)
}

fn login_oauth(
    f: &mut Factory,
    config: &mut Config,
    profile_name: &str,
    args: &LoginArgs,
) -> Result<(), CliError> {
    let cfg = OAuthConfig::from_env(args.scopes.clone())?;
    let server = CallbackServer::bind(&cfg.redirect_uri)?;
    let pkce = oauth::generate_pkce()?;
    let state = oauth::generate_state()?;
    let url = oauth::authorize_url(&cfg, &pkce, &state);

    let _ = writeln!(
        f.io.err,
        "Complete console login in your browser. If the browser did not open, visit this URL:\n  {url}"
    );
    if !args.no_browser
        && let Err(err) = browser::open(url.as_str())
    {
        let _ = writeln!(f.io.err, "portone: failed to open browser: {err}");
    }

    let code = match server.wait(&state, CALLBACK_TIMEOUT, &mut *f.io.err)? {
        Callback::Code(code) => code,
        Callback::Denied { error, description } => {
            let detail = description.map(|d| format!(" ({d})")).unwrap_or_default();
            return Err(CliError::Other(anyhow!(
                "console login was denied: {error}{detail}"
            )));
        }
    };
    drop(server);

    let agent = f.agent();
    let tokens = oauth::exchange_code(&agent, &cfg, &code, &pkce.verifier)
        .map_err(|err| CliError::Other(anyhow!("failed to obtain tokens: {err}")))?;
    let missing = oauth::missing_scopes(&cfg.scopes, &tokens.scope);
    if !missing.is_empty() {
        let _ = writeln!(
            f.io.err,
            "portone: some requested scopes were not granted: {}",
            missing.join(", ")
        );
    }

    let base_url = auth::resolve_base_url(args.base_url.as_deref(), Some(profile_name), config);
    let plain_id = auth::verify_bearer(&agent, &base_url, &tokens.access_token)?.ok_or_else(|| {
        CliError::Other(anyhow!(
            "the issued token could not access {base_url}; verify that the console and API environments match"
        ))
    })?;

    let store = f.secret_store();
    let stored = store_web_login(
        store.as_ref(),
        config,
        profile_name,
        &cfg,
        &tokens,
        base_url,
        args.insecure_storage,
        &mut *f.io.err,
    )?;

    let _ = writeln!(f.io.err, "Console login complete (merchant {plain_id})");
    let _ = writeln!(
        f.io.err,
        "Stored console login credentials in profile '{profile_name}'."
    );
    let location = match (stored.storage, stored.credential_id.as_deref()) {
        (Storage::Keyring, Some(id)) => format!("keyring ({KEYRING_SERVICE}/{id})"),
        _ => "config file (plain text)".to_string(),
    };
    let _ = writeln!(f.io.err, "Storage: {location}");
    Ok(())
}

pub(crate) struct StoredLogin {
    pub storage: Storage,
    pub credential_id: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn store_web_login(
    store: &dyn SecretStore,
    config: &mut Config,
    profile_name: &str,
    cfg: &OAuthConfig,
    tokens: &OAuthTokens,
    base_url: String,
    insecure_storage: bool,
    err: &mut dyn Write,
) -> Result<StoredLogin, CliError> {
    let previous = config
        .profiles
        .get(profile_name)
        .and_then(|p| p.oauth.clone());
    let mut oauth = OAuthProfile {
        storage: Storage::Keyring,
        client_id: cfg.client_id.clone(),
        token_url: cfg.token_url(),
        console_url: cfg.console_url.clone(),
        credential_id: None,
        tokens: None,
    };
    if insecure_storage {
        oauth.storage = Storage::File;
        oauth.tokens = Some(tokens.clone());
    } else {
        let id = store::new_credential_id()?;
        match store.save(&id, tokens) {
            Ok(()) => oauth.credential_id = Some(id),
            Err(StoreError::Timeout) => {
                return Err(CliError::Other(anyhow!(
                    "the keyring did not respond within 30 seconds ({KEYRING_SERVICE}/{id}); check the keyring and try again"
                )));
            }
            Err(error) => {
                let _ = writeln!(
                    err,
                    "portone: keyring is unavailable; storing tokens in the config file: {error}"
                );
                oauth.storage = Storage::File;
                oauth.tokens = Some(tokens.clone());
            }
        }
    }

    let entry = config.profiles.entry(profile_name.to_string()).or_default();
    entry.base_url = Some(base_url);
    entry.oauth = Some(oauth.clone());
    if config.default_profile.is_none() {
        config.default_profile = Some(profile_name.to_string());
    }
    if let Err(error) = config.save() {
        if let Some(id) = &oauth.credential_id {
            let _ = store.delete(id);
        }
        return Err(error.into());
    }
    if let Some(previous) = previous {
        delete_previous_entry(store, &previous, oauth.credential_id.as_deref(), err);
    }
    Ok(StoredLogin {
        storage: oauth.storage,
        credential_id: oauth.credential_id,
    })
}

fn delete_previous_entry(
    store: &dyn SecretStore,
    previous: &OAuthProfile,
    keep: Option<&str>,
    err: &mut dyn Write,
) {
    let Some(id) = auth::normalize(previous.credential_id.as_deref()) else {
        return;
    };
    if keep == Some(id.as_str()) {
        return;
    }
    if let Err(error) = store.delete(&id) {
        let _ = writeln!(
            err,
            "portone: failed to delete previous console login tokens ({KEYRING_SERVICE}/{id}): {error}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::store::testing::MemoryStore;
    use crate::config::Profile;
    use crate::config::paths::with_env;
    use crate::ui::IoStreams;

    fn tokens(access: &str) -> OAuthTokens {
        OAuthTokens {
            access_token: access.to_string(),
            refresh_token: Some("r".to_string()),
            expires_at: 4_000_000_000,
            scope: vec!["TX_READ".to_string()],
            token_type: "Bearer".to_string(),
        }
    }

    fn oauth_config() -> OAuthConfig {
        OAuthConfig {
            console_url: "https://console.example".to_string(),
            merchant_service_url: "https://ms.example".to_string(),
            client_id: "CLI".to_string(),
            redirect_uri: url::Url::parse("http://127.0.0.1:1271/oauth/cli").unwrap(),
            scopes: vec!["TX_READ".to_string()],
        }
    }

    fn config_with_previous(credential_id: &str) -> Config {
        let mut config = Config::default();
        config.profiles.insert(
            "default".to_string(),
            Profile {
                base_url: None,
                oauth: Some(OAuthProfile {
                    storage: Storage::Keyring,
                    client_id: "MCP".to_string(),
                    token_url: "https://old.example/oauth/token".to_string(),
                    console_url: "https://old.example".to_string(),
                    credential_id: Some(credential_id.to_string()),
                    tokens: None,
                }),
            },
        );
        config
    }

    fn with_config_dir(f: impl FnOnce(&std::path::Path)) {
        let dir = tempfile::tempdir().unwrap();
        with_env(
            &[
                ("PORTONE_CONFIG_DIR", Some(dir.path().to_str().unwrap())),
                ("PORTONE_ACCESS_TOKEN", None),
                ("PORTONE_API_BASE", None),
            ],
            || f(dir.path()),
        );
    }

    #[test]
    fn web_login_saves_new_entry_then_config_then_deletes_previous() {
        with_config_dir(|_| {
            let store = MemoryStore::with("cred-old", tokens("old"));
            let mut config = config_with_previous("cred-old");
            let mut err = Vec::new();
            let stored = store_web_login(
                &store,
                &mut config,
                "default",
                &oauth_config(),
                &tokens("new"),
                "https://api.example".to_string(),
                false,
                &mut err,
            )
            .unwrap();
            assert_eq!(stored.storage, Storage::Keyring);
            let new_id = stored.credential_id.unwrap();
            assert_ne!(new_id, "cred-old");
            assert_eq!(store.ids(), vec![new_id.clone()]);
            assert_eq!(store.deleted.borrow().as_slice(), ["cred-old".to_string()]);

            let saved = Config::load().unwrap();
            let profile = &saved.profiles["default"];
            assert_eq!(profile.base_url.as_deref(), Some("https://api.example"));
            let oauth = profile.oauth.as_ref().unwrap();
            assert_eq!(oauth.credential_id.as_deref(), Some(new_id.as_str()));
            assert_eq!(oauth.client_id, "CLI");
            assert_eq!(oauth.token_url, "https://ms.example/oauth/token");
            assert!(oauth.tokens.is_none());
            assert_eq!(saved.default_profile.as_deref(), Some("default"));
            assert!(err.is_empty());
        });
    }

    #[test]
    fn web_login_rolls_back_new_entry_when_config_save_fails() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "").unwrap();
        let bad_dir = blocker.join("config");
        with_env(
            &[
                ("PORTONE_CONFIG_DIR", Some(bad_dir.to_str().unwrap())),
                ("PORTONE_ACCESS_TOKEN", None),
            ],
            || {
                let store = MemoryStore::with("cred-old", tokens("old"));
                let mut config = config_with_previous("cred-old");
                let mut err = Vec::new();
                let result = store_web_login(
                    &store,
                    &mut config,
                    "default",
                    &oauth_config(),
                    &tokens("new"),
                    "https://api.example".to_string(),
                    false,
                    &mut err,
                );
                assert!(result.is_err());
                assert_eq!(store.ids(), vec!["cred-old".to_string()]);
                assert_eq!(store.deleted.borrow().len(), 1);
                assert_ne!(store.deleted.borrow()[0], "cred-old");
            },
        );
    }

    #[test]
    fn web_login_falls_back_to_file_when_keyring_unavailable() {
        with_config_dir(|_| {
            let store = MemoryStore::default();
            *store.fail_save.borrow_mut() =
                Some(StoreError::Unavailable("no secret service".to_string()));
            let mut config = Config::default();
            let mut err = Vec::new();
            let stored = store_web_login(
                &store,
                &mut config,
                "default",
                &oauth_config(),
                &tokens("new"),
                "https://api.example".to_string(),
                false,
                &mut err,
            )
            .unwrap();
            assert_eq!(stored.storage, Storage::File);
            assert!(stored.credential_id.is_none());
            let saved = Config::load().unwrap();
            let oauth = saved.profiles["default"].oauth.as_ref().unwrap();
            assert_eq!(oauth.tokens.as_ref().unwrap().access_token, "new");
            assert!(String::from_utf8_lossy(&err).contains("storing tokens in the config file"));
        });
    }

    #[test]
    fn web_login_aborts_on_keyring_timeout_without_fallback() {
        with_config_dir(|_| {
            let store = MemoryStore::default();
            *store.fail_save.borrow_mut() = Some(StoreError::Timeout);
            let mut config = Config::default();
            let mut err = Vec::new();
            let result = store_web_login(
                &store,
                &mut config,
                "default",
                &oauth_config(),
                &tokens("new"),
                "https://api.example".to_string(),
                false,
                &mut err,
            );
            match result {
                Err(CliError::Other(error)) => {
                    let text = format!("{error:#}");
                    assert!(text.contains("30 seconds"), "{text}");
                    assert!(text.contains("portone-cli/"), "{text}");
                }
                other => panic!("unexpected: {:?}", other.map(|s| s.storage)),
            }
            assert!(!Config::path().exists());
        });
    }

    #[test]
    fn insecure_storage_writes_tokens_to_file_and_deletes_previous_entry() {
        with_config_dir(|_| {
            let store = MemoryStore::with("cred-old", tokens("old"));
            let mut config = config_with_previous("cred-old");
            let mut err = Vec::new();
            let stored = store_web_login(
                &store,
                &mut config,
                "default",
                &oauth_config(),
                &tokens("new"),
                "https://api.example".to_string(),
                true,
                &mut err,
            )
            .unwrap();
            assert_eq!(stored.storage, Storage::File);
            assert!(stored.credential_id.is_none());
            assert!(store.ids().is_empty());
            let saved = Config::load().unwrap();
            let oauth = saved.profiles["default"].oauth.as_ref().unwrap();
            assert_eq!(oauth.storage, Storage::File);
            assert_eq!(oauth.tokens.as_ref().unwrap().access_token, "new");
        });
    }

    #[test]
    fn login_refuses_when_env_credential_is_active() {
        with_env(&[("PORTONE_ACCESS_TOKEN", Some("t"))], || {
            let (io, bufs) = IoStreams::test();
            let mut f = Factory::with_config(io, Config::default());
            let result = run(
                &mut f,
                LoginArgs {
                    profile: None,
                    base_url: None,
                    scopes: None,
                    insecure_storage: false,
                    no_browser: false,
                },
            );
            assert!(matches!(result, Err(CliError::Silent)));
            assert!(bufs.err().contains("PORTONE_ACCESS_TOKEN"));
        });
    }
}
