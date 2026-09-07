pub mod setup;

use clap::{Parser, Subcommand};

use crate::error::CliError;
use crate::factory::Factory;

#[derive(Debug, Parser)]
#[command(name = "portone", version, about = "PortOne CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Install PortOne plugins for AI coding assistants")]
    Setup(setup::SetupArgs),
}

pub fn run(f: &mut Factory, command: Command) -> Result<(), CliError> {
    match command {
        Command::Setup(args) => setup::run(f, args),
    }
}
