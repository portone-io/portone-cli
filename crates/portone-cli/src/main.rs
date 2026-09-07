use std::io::Write;

use clap::Parser;

use portone_cli::cmd;
use portone_cli::error::CliError;
use portone_cli::factory::Factory;

fn main() {
    let cli = cmd::Cli::parse();
    let mut f = Factory::detect();

    let code = match cmd::run(&mut f, cli.command) {
        Ok(()) => 0,
        Err(CliError::Silent) => 1,
        Err(CliError::Flag(message)) => {
            let _ = writeln!(f.io.err, "portone: {message}");
            1
        }
        Err(CliError::Other(err)) => {
            if is_broken_pipe(&err) {
                0
            } else {
                let _ = writeln!(f.io.err, "portone: {err:#}");
                1
            }
        }
    };

    let _ = f.io.out.flush();
    let _ = f.io.err.flush();
    if code != 0 {
        std::process::exit(code);
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
    })
}
