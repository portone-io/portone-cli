use std::io::Write;
use std::time::{Duration, UNIX_EPOCH};

use clap::Args;

use crate::auth::store::KEYRING_SERVICE;
use crate::auth::{self, AuthSource, oauth};
use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::Localizer;

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,

    #[arg(long, help = "Display the access token without masking it")]
    pub show_secret: bool,
}

pub fn run(f: &mut Factory, args: StatusArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    let mut config = f.config()?.clone();
    let agent = f.agent();
    let store = f.secret_store();
    let resolved = auth::resolve_fresh_localized(
        &agent,
        store.as_ref(),
        &mut config,
        args.profile.as_deref(),
        &mut *f.io.err,
        &localizer,
    )?;
    let Some(resolved) = resolved else {
        let _ = writeln!(f.io.err, "{}", crate::tr!(localizer, "auth-no-credentials"));
        return Err(CliError::Silent);
    };
    let base = auth::resolve_base_url(None, args.profile.as_deref(), &config);

    let source = match &resolved.source {
        AuthSource::Env(name) => crate::tr!(localizer, "auth-source-environment", name = *name),
        AuthSource::ConfigProfile(name) => {
            crate::tr!(localizer, "auth-source-config", profile = name)
        }
        AuthSource::Keyring(id) => crate::tr!(
            localizer,
            "auth-source-keyring",
            service = KEYRING_SERVICE,
            id = id
        ),
    };

    let token = &resolved.access_token;
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(localizer, "auth-status-authentication")
    )?;
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(localizer, "auth-status-source", source = source)
    )?;
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(
            localizer,
            "auth-status-access-token",
            token = mask_secret(token, args.show_secret)
        )
    )?;
    if let Some(session) = &resolved.oauth {
        let now = oauth::now();
        writeln!(
            f.io.out,
            "{}",
            crate::tr!(
                localizer,
                "auth-status-expires",
                timestamp = rfc3339(session.tokens.expires_at),
                remaining = remaining(session.tokens.expires_at, now, &localizer)
            )
        )?;
        if let Some(exp) = session
            .tokens
            .refresh_token
            .as_deref()
            .and_then(oauth::jwt_exp)
        {
            writeln!(
                f.io.out,
                "{}",
                crate::tr!(
                    localizer,
                    "auth-status-session-expires",
                    timestamp = rfc3339(exp)
                )
            )?;
        }
        if !session.tokens.scope.is_empty() {
            writeln!(
                f.io.out,
                "{}",
                crate::tr!(
                    localizer,
                    "auth-status-scopes",
                    scopes = session.tokens.scope.join(", ")
                )
            )?;
        }
        writeln!(
            f.io.out,
            "{}",
            crate::tr!(
                localizer,
                "auth-status-issued-by",
                client_id = &session.oauth.client_id,
                url = &session.oauth.console_url
            )
        )?;
    }
    writeln!(
        f.io.out,
        "{}",
        crate::tr!(localizer, "auth-status-api-base-url", url = &base)
    )?;

    match auth::verify_bearer(&agent, &base, token)? {
        Some(plain_id) => {
            writeln!(
                f.io.out,
                "{}",
                crate::tr!(localizer, "auth-status-valid", merchant = plain_id)
            )?;
            Ok(())
        }
        None => {
            writeln!(f.io.out, "{}", crate::tr!(localizer, "auth-status-invalid"))?;
            let _ = writeln!(
                f.io.err,
                "{}",
                crate::tr!(localizer, "auth-status-invalid-token")
            );
            Err(CliError::Silent)
        }
    }
}

fn mask_secret(secret: &str, show: bool) -> String {
    if show {
        return secret.to_string();
    }
    let prefix: String = secret.chars().take(4).collect();
    format!("{prefix}****")
}

fn rfc3339(secs: u64) -> String {
    humantime::format_rfc3339_seconds(UNIX_EPOCH + Duration::from_secs(secs)).to_string()
}

fn remaining(expires_at: u64, now: u64, localizer: &Localizer) -> String {
    if expires_at <= now {
        return crate::tr!(localizer, "auth-remaining-expired");
    }
    let secs = expires_at - now;
    if secs >= 3600 {
        crate::tr!(
            localizer,
            "auth-remaining-hours",
            hours = secs / 3600,
            minutes = (secs % 3600) / 60
        )
    } else if secs >= 60 {
        crate::tr!(localizer, "auth-remaining-minutes", minutes = secs / 60)
    } else {
        crate::tr!(localizer, "auth-remaining-seconds", seconds = secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_shows_prefix_only() {
        assert_eq!(mask_secret("sk_live_abcdef", false), "sk_l****");
        assert_eq!(mask_secret("ab", false), "ab****");
        assert_eq!(mask_secret("sk_live_abcdef", true), "sk_live_abcdef");
    }

    #[test]
    fn expiry_is_rendered_in_utc_rfc3339() {
        assert_eq!(rfc3339(1_788_494_662), "2026-09-04T04:04:22Z");
        assert_eq!(remaining(1_000, 1_000, &Localizer::english()), "expired");
        assert_eq!(
            remaining(1_030, 1_000, &Localizer::english()),
            "30s remaining"
        );
        assert_eq!(
            remaining(1_000 + 12 * 60 + 5, 1_000, &Localizer::english()),
            "12m remaining"
        );
        assert_eq!(
            remaining(1_000 + 3 * 3600 + 7 * 60, 1_000, &Localizer::english()),
            "3h 7m remaining"
        );
    }

    #[test]
    fn remaining_time_is_localized_without_affecting_timestamps() {
        let korean = Localizer::korean();
        assert_eq!(remaining(1_000, 1_000, &korean), "만료됨");
        assert_eq!(remaining(1_030, 1_000, &korean), "30초 남음");
        assert_eq!(remaining(1_725, 1_000, &korean), "12분 남음");
        assert_eq!(remaining(12_220, 1_000, &korean), "3시간 7분 남음");
        assert_eq!(rfc3339(1_788_494_662), "2026-09-04T04:04:22Z");
    }

    #[test]
    fn status_localizes_labels_and_preserves_auth_values_and_masking() {
        use crate::config::paths::with_env;
        use crate::config::{Config, OAuthProfile, OAuthTokens, Profile, Storage};
        use crate::ui::IoStreams;
        use std::sync::Arc;

        let server = httpmock::MockServer::start();
        let validation = server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/graphql")
                .header("authorization", "Bearer secret-console-token");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":{"merchant":{"__typename":"Merchant","plainId":"merchant-42"}}}"#);
        });
        with_env(
            &[("PORTONE_ACCESS_TOKEN", None), ("PORTONE_API_BASE", None)],
            || {
                let mut config = Config::default();
                config.profiles.insert(
                    "default".to_string(),
                    Profile {
                        base_url: Some(server.base_url()),
                        store_id: None,
                        oauth: Some(OAuthProfile {
                            storage: Storage::File,
                            client_id: "CLI".to_string(),
                            token_url: "https://issuer.example/oauth/token".to_string(),
                            console_url: "https://console.example".to_string(),
                            credential_id: None,
                            tokens: Some(OAuthTokens {
                                access_token: "secret-console-token".to_string(),
                                refresh_token: None,
                                expires_at: 4_000_000_000,
                                scope: vec!["TX_READ".to_string()],
                                token_type: "Bearer".to_string(),
                            }),
                        }),
                    },
                );
                for (localizer, label) in [
                    (Localizer::english(), "Authentication:"),
                    (Localizer::korean(), "인증 방식:"),
                ] {
                    let (io, buffers) = IoStreams::test();
                    let mut factory = Factory::with_config(io, config.clone());
                    factory.localizer = Arc::new(localizer);
                    run(
                        &mut factory,
                        StatusArgs {
                            profile: None,
                            show_secret: false,
                        },
                    )
                    .unwrap();
                    let output = buffers.out();
                    assert!(output.contains(label), "{output}");
                    for preserved in [
                        "secr****",
                        "TX_READ",
                        "merchant-42",
                        "CLI @ https://console.example",
                    ] {
                        assert!(output.contains(preserved), "{output}");
                    }
                    assert!(!output.contains("secret-console-token"), "{output}");
                    assert!(!output.contains('\u{2068}'), "{output}");
                }
            },
        );
        validation.assert_calls(2);
    }
}
