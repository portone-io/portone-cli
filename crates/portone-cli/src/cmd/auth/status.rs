use std::io::Write;
use std::time::{Duration, UNIX_EPOCH};

use clap::Args;

use crate::auth::store::KEYRING_SERVICE;
use crate::auth::{self, AuthSource, oauth};
use crate::error::CliError;
use crate::factory::Factory;

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,

    #[arg(long, help = "Display the access token without masking it")]
    pub show_secret: bool,
}

pub fn run(f: &mut Factory, args: StatusArgs) -> Result<(), CliError> {
    let mut config = f.config()?.clone();
    let agent = f.agent();
    let store = f.secret_store();
    let resolved = auth::resolve_fresh(
        &agent,
        store.as_ref(),
        &mut config,
        args.profile.as_deref(),
        &mut *f.io.err,
    )?;
    let Some(resolved) = resolved else {
        let _ = writeln!(
            f.io.err,
            "portone: no stored credentials found; run `portone auth login` to authenticate"
        );
        return Err(CliError::Silent);
    };
    let base = auth::resolve_base_url(None, args.profile.as_deref(), &config);

    let source = match &resolved.source {
        AuthSource::Env(name) => format!("environment variable {name}"),
        AuthSource::ConfigProfile(name) => format!("config profile '{name}'"),
        AuthSource::Keyring(id) => format!("keyring ({KEYRING_SERVICE}/{id})"),
    };

    let token = &resolved.access_token;
    writeln!(f.io.out, "Authentication: Console OAuth")?;
    writeln!(f.io.out, "Source: {source}")?;
    writeln!(
        f.io.out,
        "Access token: {}",
        mask_secret(token, args.show_secret)
    )?;
    if let Some(session) = &resolved.oauth {
        let now = oauth::now();
        writeln!(
            f.io.out,
            "Expires: {} ({})",
            rfc3339(session.tokens.expires_at),
            remaining(session.tokens.expires_at, now)
        )?;
        if let Some(exp) = session
            .tokens
            .refresh_token
            .as_deref()
            .and_then(oauth::jwt_exp)
        {
            writeln!(f.io.out, "Session expires: {}", rfc3339(exp))?;
        }
        if !session.tokens.scope.is_empty() {
            writeln!(f.io.out, "Scopes: {}", session.tokens.scope.join(", "))?;
        }
        writeln!(
            f.io.out,
            "Issued by: {} @ {}",
            session.oauth.client_id, session.oauth.console_url
        )?;
    }
    writeln!(f.io.out, "API base URL: {base}")?;

    match auth::verify_bearer(&agent, &base, token)? {
        Some(plain_id) => {
            writeln!(f.io.out, "Validation: valid (merchant {plain_id})")?;
            Ok(())
        }
        None => {
            writeln!(f.io.out, "Validation: invalid")?;
            let _ = writeln!(
                f.io.err,
                "portone: console token is invalid; run `portone auth login` to authenticate again"
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

fn remaining(expires_at: u64, now: u64) -> String {
    if expires_at <= now {
        return "expired".to_string();
    }
    let secs = expires_at - now;
    if secs >= 3600 {
        format!("{}h {}m remaining", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m remaining", secs / 60)
    } else {
        format!("{secs}s remaining")
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
        assert_eq!(remaining(1_000, 1_000), "expired");
        assert_eq!(remaining(1_030, 1_000), "30s remaining");
        assert_eq!(remaining(1_000 + 12 * 60 + 5, 1_000), "12m remaining");
        assert_eq!(
            remaining(1_000 + 3 * 3600 + 7 * 60, 1_000),
            "3h 7m remaining"
        );
    }
}
