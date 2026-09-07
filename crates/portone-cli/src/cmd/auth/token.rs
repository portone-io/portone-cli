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
    match resolved {
        Some(resolved) => {
            writeln!(f.io.out, "{}", resolved.access_token)?;
            Ok(())
        }
        None => {
            let _ = writeln!(
                f.io.err,
                "portone: no stored credentials found; run `portone auth login` to authenticate"
            );
            Err(CliError::Silent)
        }
    }
}
