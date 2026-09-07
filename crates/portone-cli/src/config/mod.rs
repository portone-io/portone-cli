pub mod paths;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context;

pub const DEFAULT_BASE_URL: &str = "https://api.portone.io";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    Keyring,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OAuthProfile {
    pub storage: Storage,
    pub client_id: String,
    pub token_url: String,
    pub console_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<OAuthTokens>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub expires_at: u64,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default = "default_token_type")]
    pub token_type: String,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl Config {
    pub fn path() -> PathBuf {
        paths::config_dir().join("config.toml")
    }

    pub fn load() -> anyhow::Result<Config> {
        let path = Self::path();
        let contents = match std::fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Config::default());
            }
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to read config file: {}", path.display()));
            }
        };
        toml::from_str(&contents).map_err(|err| {
            let position = err
                .to_string()
                .lines()
                .next()
                .unwrap_or("TOML parse error")
                .to_string();
            anyhow::anyhow!("invalid config file: {} ({position})", path.display())
        })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory: {}", parent.display())
            })?;
        }
        let contents = toml::to_string_pretty(self).context("failed to serialize config")?;
        write_private(&path, &contents)
            .with_context(|| format!("failed to save config file: {}", path.display()))
    }
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    let tmp = dir.join(format!(".config.toml.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    std::fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_config_dir(f: impl FnOnce(&std::path::Path)) {
        let dir = tempfile::tempdir().expect("failed to create temporary directory");
        let value = dir.path().to_str().expect("temporary path is not UTF-8");
        paths::with_env(&[("PORTONE_CONFIG_DIR", Some(value))], || f(dir.path()));
    }

    fn sample_config() -> Config {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "default".to_string(),
            Profile {
                base_url: None,
                oauth: None,
            },
        );
        profiles.insert(
            "staging".to_string(),
            Profile {
                base_url: Some("https://api.example.test".to_string()),
                oauth: None,
            },
        );
        Config {
            default_profile: Some("default".to_string()),
            profiles,
        }
    }

    #[test]
    fn path_is_under_config_dir() {
        with_config_dir(|dir| {
            assert_eq!(Config::path(), dir.join("config.toml"));
        });
    }

    #[test]
    fn load_returns_default_when_missing() {
        with_config_dir(|_| {
            let config = Config::load().expect("load failed");
            assert!(config.default_profile.is_none());
            assert!(config.profiles.is_empty());
        });
    }

    #[test]
    fn save_and_load_round_trip() {
        with_config_dir(|_| {
            let config = sample_config();
            config.save().expect("save failed");
            let loaded = Config::load().expect("load failed");
            assert_eq!(loaded.default_profile, config.default_profile);
            assert_eq!(loaded.profiles, config.profiles);
        });
    }

    #[test]
    fn save_omits_none_fields() {
        with_config_dir(|_| {
            Config::default().save().expect("save failed");
            let contents = std::fs::read_to_string(Config::path()).expect("failed to read file");
            assert!(!contents.contains("default_profile"));
        });
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        with_config_dir(|_| {
            sample_config().save().expect("save failed");
            let mode = std::fs::metadata(Config::path())
                .expect("failed to read metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        });
    }

    #[cfg(unix)]
    #[test]
    fn save_tightens_permissions_of_existing_loose_file() {
        use std::os::unix::fs::PermissionsExt;

        with_config_dir(|_| {
            let path = Config::path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

            sample_config().save().expect("save failed");
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        });
    }

    fn oauth_profile(storage: Storage, tokens: Option<OAuthTokens>) -> OAuthProfile {
        OAuthProfile {
            storage,
            client_id: "CLI".to_string(),
            token_url: "https://merchant.example/oauth/token".to_string(),
            console_url: "https://console.example".to_string(),
            credential_id: Some("cred-1".to_string()),
            tokens,
        }
    }

    #[test]
    fn oauth_keyring_profile_omits_tokens() {
        with_config_dir(|_| {
            let mut config = Config::default();
            config.profiles.insert(
                "console".to_string(),
                Profile {
                    base_url: Some("https://api.example".to_string()),
                    oauth: Some(oauth_profile(Storage::Keyring, None)),
                },
            );
            config.save().expect("save failed");
            let contents = std::fs::read_to_string(Config::path()).unwrap();
            assert!(contents.contains("[profiles.console.oauth]"), "{contents}");
            assert!(contents.contains(r#"storage = "keyring""#), "{contents}");
            assert!(
                contents.contains(r#"credential_id = "cred-1""#),
                "{contents}"
            );
            assert!(!contents.contains("access_token"), "{contents}");
            assert_eq!(Config::load().unwrap().profiles, config.profiles);
        });
    }

    #[test]
    fn oauth_file_profile_round_trips_tokens() {
        with_config_dir(|_| {
            let tokens = OAuthTokens {
                access_token: "access".to_string(),
                refresh_token: Some("refresh".to_string()),
                expires_at: 1_800_000_000,
                scope: vec!["STORE_READ".to_string()],
                token_type: "Bearer".to_string(),
            };
            let mut config = Config::default();
            config.profiles.insert(
                "console".to_string(),
                Profile {
                    base_url: None,
                    oauth: Some(oauth_profile(Storage::File, Some(tokens))),
                },
            );
            config.save().expect("save failed");
            let contents = std::fs::read_to_string(Config::path()).unwrap();
            assert!(
                contents.contains("[profiles.console.oauth.tokens]"),
                "{contents}"
            );
            assert_eq!(Config::load().unwrap().profiles, config.profiles);
        });
    }

    #[test]
    fn load_error_does_not_leak_file_contents() {
        with_config_dir(|_| {
            let path = Config::path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "default_profile = \"TOPSECRET\" oops\n").unwrap();

            let err = Config::load().expect_err("parsing should fail");
            let message = format!("{err:#}");
            assert!(!message.contains("TOPSECRET"), "leaked: {message}");
            assert!(message.contains("invalid config file"));
        });
    }
}
