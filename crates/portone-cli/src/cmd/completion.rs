use clap::Args;
use clap_complete::aot::{Generator, Shell};

use crate::error::CliError;
use crate::factory::Factory;
use crate::i18n::Localizer;

#[derive(Debug, Args)]
pub struct CompletionArgs {
    #[arg(
        value_enum,
        value_name = "SHELL",
        help = "Shell for which to generate a completion script"
    )]
    pub shell: Shell,
}

pub fn run(f: &mut Factory, args: CompletionArgs) -> Result<(), CliError> {
    let mut cmd = crate::cmd::help::command(&Localizer::english());
    cmd.set_bin_name("portone");
    cmd.build();
    if args.shell == Shell::Fish {
        fish::generate(&cmd, &mut f.io.out)?;
    } else {
        args.shell.try_generate(&cmd, &mut f.io.out)?;
    }
    Ok(())
}
mod fish;
