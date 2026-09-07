pub mod login;
pub mod logout;
pub mod status;
pub mod token;

use clap::{Args, Subcommand};

use crate::error::CliError;
use crate::factory::Factory;

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    #[command(about = "Authenticate with PortOne Console")]
    Login(login::LoginArgs),
    #[command(about = "View authentication status")]
    Status(status::StatusArgs),
    #[command(about = "Print the current console access token")]
    Token(token::TokenArgs),
    #[command(about = "Remove local credentials without revoking the server-side token")]
    Logout(logout::LogoutArgs),
}

pub fn run(f: &mut Factory, args: AuthArgs) -> Result<(), CliError> {
    match args.command {
        AuthCommand::Login(args) => login::run(f, args),
        AuthCommand::Status(args) => status::run(f, args),
        AuthCommand::Token(args) => token::run(f, args),
        AuthCommand::Logout(args) => logout::run(f, args),
    }
}
