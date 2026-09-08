pub mod api;
pub mod auth;
pub mod completion;
pub mod help;
pub mod payment;
pub mod setup;
pub mod store;

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
    #[command(about = "Make an authenticated PortOne V2 API request")]
    Api(api::ApiArgs),
    #[command(about = "Authenticate with PortOne")]
    Auth(auth::AuthArgs),
    #[command(about = "Inspect and manage payments")]
    Payment(payment::PaymentArgs),
    #[command(about = "Configure the default store")]
    Store(store::StoreArgs),
    #[command(about = "Install PortOne plugins for AI coding assistants")]
    Setup(setup::SetupArgs),
    #[command(about = "Generate shell completion scripts")]
    Completion(completion::CompletionArgs),
}

pub fn run(f: &mut Factory, command: Command) -> Result<(), CliError> {
    match command {
        Command::Api(args) => api::run(f, args),
        Command::Auth(args) => auth::run(f, args),
        Command::Payment(args) => payment::run(f, args),
        Command::Store(args) => store::run(f, args),
        Command::Setup(args) => setup::run(f, args),
        Command::Completion(args) => completion::run(f, args),
    }
}
