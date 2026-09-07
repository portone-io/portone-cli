use std::io::Write;

use anyhow::anyhow;
use clap::Args;

use crate::auth::{self, store::KEYRING_SERVICE};
use crate::error::CliError;
use crate::factory::Factory;

#[derive(Debug, Args)]
pub struct LogoutArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to remove")]
    pub profile: Option<String>,
}

pub fn run(f: &mut Factory, args: LogoutArgs) -> Result<(), CliError> {
    if let Some(name) = auth::active_env_credential() {
        let _ = writeln!(
            f.io.err,
            "portone: the {name} environment variable is being used for authentication; unset it before removing stored credentials"
        );
        return Err(CliError::Silent);
    }
    let mut config = f.config()?.clone();
    let name = args
        .profile
        .or_else(|| config.default_profile.clone())
        .unwrap_or_else(|| "default".to_string());
    let Some(profile) = config.profiles.get(&name) else {
        return Err(CliError::Other(anyhow!("profile '{name}' does not exist")));
    };
    if let Some(oauth) = &profile.oauth
        && let Some(id) = auth::normalize(oauth.credential_id.as_deref())
    {
        f.secret_store().delete(&id).map_err(|err| {
            CliError::Other(anyhow!(
                "failed to delete tokens from the keyring ({KEYRING_SERVICE}/{id}): {err}"
            ))
        })?;
    }
    config.profiles.remove(&name);
    if config.default_profile.as_deref() == Some(name.as_str()) {
        config.default_profile = None;
    }
    config.save()?;
    let _ = writeln!(f.io.err, "Removed profile '{name}'.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::auth::store::StoreError;
    use crate::auth::store::testing::MemoryStore;
    use crate::config::paths::with_env;
    use crate::config::{Config, OAuthProfile, OAuthTokens, Profile, Storage};
    use crate::ui::IoStreams;

    fn keyring_config() -> Config {
        let mut config = Config::default();
        config.profiles.insert(
            "default".to_string(),
            Profile {
                base_url: None,
                oauth: Some(OAuthProfile {
                    storage: Storage::Keyring,
                    client_id: "CLI".to_string(),
                    token_url: "https://ms.example/oauth/token".to_string(),
                    console_url: "https://console.example".to_string(),
                    credential_id: Some("cred-1".to_string()),
                    tokens: None,
                }),
            },
        );
        config
    }

    fn config_for_storage(storage: Storage) -> Config {
        let mut config = keyring_config();
        let oauth = config
            .profiles
            .get_mut("default")
            .unwrap()
            .oauth
            .as_mut()
            .unwrap();
        oauth.storage = storage;
        if storage == Storage::File {
            oauth.tokens = Some(tokens());
        }
        config
    }

    fn tokens() -> OAuthTokens {
        OAuthTokens {
            access_token: "a".to_string(),
            refresh_token: None,
            expires_at: 1,
            scope: vec![],
            token_type: "Bearer".to_string(),
        }
    }

    fn with_config_dir(f: impl FnOnce()) {
        let dir = tempfile::tempdir().unwrap();
        with_env(
            &[
                ("PORTONE_CONFIG_DIR", Some(dir.path().to_str().unwrap())),
                ("PORTONE_ACCESS_TOKEN", None),
            ],
            f,
        );
    }

    #[test]
    fn logout_deletes_keyring_entry_then_profile() {
        with_config_dir(|| {
            for storage in [Storage::Keyring, Storage::File] {
                let config = config_for_storage(storage);
                config.save().unwrap();
                let store = MemoryStore::with("cred-1", tokens());
                let (io, bufs) = IoStreams::test();
                let mut f = Factory::with_store(io, config, Rc::new(store.clone()));
                run(&mut f, LogoutArgs { profile: None }).unwrap();
                assert_eq!(store.deleted.borrow().as_slice(), ["cred-1".to_string()]);
                assert!(store.ids().is_empty());
                assert!(Config::load().unwrap().profiles.is_empty());
                assert!(bufs.err().contains("Removed profile"));
            }
        });
    }

    #[test]
    fn logout_keeps_profile_when_keyring_delete_fails() {
        with_config_dir(|| {
            for storage in [Storage::Keyring, Storage::File] {
                let config = config_for_storage(storage);
                config.save().unwrap();
                let store = MemoryStore::with("cred-1", tokens());
                *store.fail_delete.borrow_mut() =
                    Some(StoreError::Unavailable("locked".to_string()));
                let (io, _) = IoStreams::test();
                let mut f = Factory::with_store(io, config.clone(), Rc::new(store));
                let err = run(&mut f, LogoutArgs { profile: None }).unwrap_err();
                match err {
                    CliError::Other(error) => {
                        let text = format!("{error:#}");
                        assert!(text.contains("portone-cli/cred-1"), "{text}");
                        assert!(text.contains("locked"), "{text}");
                    }
                    other => panic!("unexpected: {other:?}"),
                }
                assert_eq!(Config::load().unwrap().profiles, config.profiles);
            }
        });
    }

    #[test]
    fn logout_refuses_when_env_credential_is_active() {
        with_env(&[("PORTONE_ACCESS_TOKEN", Some("t"))], || {
            let (io, bufs) = IoStreams::test();
            let mut f = Factory::with_config(io, keyring_config());
            assert!(matches!(
                run(&mut f, LogoutArgs { profile: None }),
                Err(CliError::Silent)
            ));
            assert!(bufs.err().contains("PORTONE_ACCESS_TOKEN"));
        });
    }
}
