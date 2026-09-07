use std::io::Write;

use clap::Args;

use crate::auth;
use crate::error::CliError;
use crate::factory::Factory;

#[derive(Debug, Args)]
pub struct TokenArgs {
    #[arg(long, value_name = "NAME", help = "Configuration profile to use")]
    pub profile: Option<String>,
}

pub fn run(f: &mut Factory, args: TokenArgs) -> Result<(), CliError> {
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
    match resolved {
        Some(resolved) => {
            writeln!(f.io.out, "{}", resolved.access_token)?;
            Ok(())
        }
        None => {
            let _ = writeln!(f.io.err, "{}", crate::tr!(localizer, "auth-no-credentials"));
            Err(CliError::Silent)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, paths::with_env};
    use crate::i18n::Localizer;
    use crate::ui::IoStreams;
    use std::sync::Arc;

    #[test]
    fn token_stdout_is_identical_in_both_languages() {
        with_env(
            &[(
                "PORTONE_ACCESS_TOKEN",
                Some("console-token.payload.signature"),
            )],
            || {
                for localizer in [Localizer::english(), Localizer::korean()] {
                    let (io, buffers) = IoStreams::test();
                    let mut factory = Factory::with_config(io, Config::default());
                    factory.localizer = Arc::new(localizer);
                    run(&mut factory, TokenArgs { profile: None }).unwrap();
                    assert_eq!(buffers.out(), "console-token.payload.signature\n");
                    assert!(buffers.err().is_empty());
                }
            },
        );
    }
}
