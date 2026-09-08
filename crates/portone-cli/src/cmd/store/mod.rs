use std::io::Write;

use anyhow::anyhow;
use clap::{Args, Subcommand};

use crate::auth::{self, store, store_discovery};
use crate::config::Config;
use crate::error::CliError;
use crate::factory::Factory;
use crate::output::resource::cell;

#[derive(Debug, Args)]
pub struct StoreArgs {
    #[command(subcommand)]
    pub command: StoreCommand,
}

#[derive(Debug, Subcommand)]
pub enum StoreCommand {
    #[command(about = "Set the default store for a configuration profile")]
    SetDefault(SetDefaultArgs),
}

#[derive(Debug, Args)]
pub struct SetDefaultArgs {
    #[arg(value_name = "ID", help = "Store ID to save as the default")]
    pub id: Option<String>,

    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,

    #[arg(long, conflicts_with_all = ["id", "unset"], help = "Print the saved default store ID")]
    pub view: bool,

    #[arg(long, conflicts_with_all = ["id", "view"], help = "Remove the saved default store")]
    pub unset: bool,
}

pub fn run(f: &mut Factory, args: StoreArgs) -> Result<(), CliError> {
    match args.command {
        StoreCommand::SetDefault(args) => set_default(f, args),
    }
}

fn set_default(f: &mut Factory, args: SetDefaultArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    let mut config = f.config()?.clone();
    let profile = auth::profile_name(args.profile.as_deref(), &config);
    if args.view {
        let id = config
            .profiles
            .get(&profile)
            .and_then(|profile| profile.store_id.as_deref())
            .ok_or_else(|| {
                anyhow!(crate::message!(
                    "store-default-missing",
                    profile = cell(&profile)
                ))
            })?;
        validate_id(id)?;
        writeln!(f.io.out, "{id}")?;
        return Ok(());
    }
    if args.unset {
        save_default(&profile, None)?;
        writeln!(
            f.io.err,
            "{}",
            crate::tr!(localizer, "store-default-unset", profile = cell(&profile))
        )?;
        return Ok(());
    }

    let id = if let Some(id) = args.id {
        validate_id(&id)?;
        let id = id.trim();
        id.to_string()
    } else {
        if !f.io.can_prompt() {
            return Err(CliError::Other(anyhow!(crate::message!(
                "store-selection-requires-tty"
            ))));
        }
        let agent = f.agent();
        let secret_store = f.secret_store();
        let resolved = auth::resolve_fresh_localized(
            &agent,
            secret_store.as_ref(),
            &mut config,
            Some(&profile),
            &mut *f.io.err,
            &localizer,
        )?
        .ok_or_else(|| anyhow!(crate::message!("auth-no-credentials")))?;
        let base_url = auth::resolve_base_url(None, Some(&profile), &config);
        let stores = store_discovery::discover(&agent, &base_url, &resolved.access_token)?;
        let previous = config
            .profiles
            .get(&profile)
            .and_then(|profile| profile.store_id.as_deref());
        let Some(selected) = store_discovery::pick_store(&stores, previous, false, &localizer)?
        else {
            return Ok(());
        };
        selected.plain_id
    };
    save_default(&profile, Some(&id))?;
    writeln!(
        f.io.err,
        "{}",
        crate::tr!(
            localizer,
            "store-default-set",
            store = cell(&id),
            profile = cell(&profile)
        )
    )?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), CliError> {
    if !store_discovery::valid_store_id(id) {
        return Err(CliError::Other(anyhow!(crate::message!(
            "store-invalid-id"
        ))));
    }
    Ok(())
}

fn save_default(profile: &str, id: Option<&str>) -> anyhow::Result<()> {
    let _lock = store::lock_config()?;
    let mut config = Config::load()?;
    if let Some(id) = id {
        config
            .profiles
            .entry(profile.to_string())
            .or_default()
            .store_id = Some(id.to_string());
    } else if let Some(profile) = config.profiles.get_mut(profile) {
        profile.store_id = None;
    }
    config.save()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::with_env;
    use crate::config::{OAuthProfile, OAuthTokens, Profile, Storage};
    use crate::ui::IoStreams;

    fn args(id: Option<&str>) -> SetDefaultArgs {
        SetDefaultArgs {
            id: id.map(str::to_string),
            profile: None,
            view: false,
            unset: false,
        }
    }

    #[test]
    fn direct_id_saves_offline_and_preserves_fresh_profile_metadata() {
        let dir = tempfile::tempdir().unwrap();
        with_env(
            &[("PORTONE_CONFIG_DIR", Some(dir.path().to_str().unwrap()))],
            || {
                let mut config = Config {
                    default_profile: Some("work".to_string()),
                    ..Config::default()
                };
                let (io, buffers) = IoStreams::test();
                let mut f = Factory::with_config(io, config.clone());
                config.profiles.insert(
                    "work".to_string(),
                    Profile {
                        base_url: Some("https://fresh.example".to_string()),
                        oauth: Some(OAuthProfile {
                            storage: Storage::File,
                            client_id: "CLI".to_string(),
                            token_url: "https://issuer.example/token".to_string(),
                            console_url: "https://console.example".to_string(),
                            credential_id: None,
                            tokens: Some(OAuthTokens {
                                access_token: "fresh".to_string(),
                                refresh_token: None,
                                expires_at: 4_000_000_000,
                                scope: vec![],
                                token_type: "Bearer".to_string(),
                            }),
                        }),
                        ..Profile::default()
                    },
                );
                config.save().unwrap();
                set_default(&mut f, args(Some("store-plain"))).unwrap();
                let saved = Config::load().unwrap();
                assert_eq!(
                    saved.profiles["work"].store_id.as_deref(),
                    Some("store-plain")
                );
                assert_eq!(saved.profiles["work"].oauth, config.profiles["work"].oauth);
                assert_eq!(
                    saved.profiles["work"].base_url,
                    config.profiles["work"].base_url
                );
                assert!(buffers.out().is_empty());
                assert!(!buffers.err().contains("fresh"));
            },
        );
    }

    #[test]
    fn view_and_unset_use_only_local_profile_defaults() {
        let dir = tempfile::tempdir().unwrap();
        with_env(
            &[
                ("PORTONE_CONFIG_DIR", Some(dir.path().to_str().unwrap())),
                ("PORTONE_STORE_ID", Some("store-env")),
            ],
            || {
                let mut config = Config::default();
                config.profiles.insert(
                    "default".to_string(),
                    Profile {
                        store_id: Some("store-local".to_string()),
                        ..Profile::default()
                    },
                );
                config.save().unwrap();
                let (io, buffers) = IoStreams::test();
                let mut f = Factory::with_config(io, config);
                set_default(
                    &mut f,
                    SetDefaultArgs {
                        view: true,
                        ..args(None)
                    },
                )
                .unwrap();
                assert_eq!(buffers.out(), "store-local\n");
                set_default(
                    &mut f,
                    SetDefaultArgs {
                        unset: true,
                        ..args(None)
                    },
                )
                .unwrap();
                assert!(
                    Config::load().unwrap().profiles["default"]
                        .store_id
                        .is_none()
                );
            },
        );
    }

    #[test]
    fn absent_id_requires_terminal_and_empty_ids_are_rejected() {
        let (io, _) = IoStreams::test();
        let mut f = Factory::with_config(io, Config::default());
        assert!(set_default(&mut f, args(None)).is_err());
        assert!(set_default(&mut f, args(Some("  "))).is_err());
    }

    #[test]
    fn control_ids_are_neither_saved_nor_printed() {
        for id in ["store\n", "store\t", "store\r", "store\u{1b}[31m"] {
            let mut config = Config::default();
            config.profiles.insert(
                "default".to_string(),
                Profile {
                    store_id: Some(id.to_string()),
                    ..Profile::default()
                },
            );
            let (io, buffers) = IoStreams::test();
            let mut f = Factory::with_config(io, config);
            assert!(set_default(&mut f, args(Some(id))).is_err());
            assert!(
                set_default(
                    &mut f,
                    SetDefaultArgs {
                        view: true,
                        ..args(None)
                    }
                )
                .is_err()
            );
            assert!(buffers.out().is_empty());
            assert!(buffers.err().is_empty());
        }
    }
}
