pub mod assets;
pub mod assistants;
pub mod plugin;
pub mod steps;

use std::io::IsTerminal;
use std::time::Duration;

use anstyle::{AnsiColor, Style};
use clap::Args;
use indicatif::ProgressBar;

use crate::error::CliError;
use crate::factory::Factory;

use assistants::{Assistant, AssistantSelection, resolve_targets};
use steps::ShellRunner;

const BOLD: Style = Style::new().bold();
const YELLOW: Style = AnsiColor::Yellow.on_default();
const GREEN: Style = AnsiColor::Green.on_default();
const RED: Style = AnsiColor::Red.on_default();
const CYAN: Style = AnsiColor::Cyan.on_default();
const WHITE: Style = AnsiColor::White.on_default();

#[derive(Debug, Args)]
pub struct SetupArgs {
    #[arg(long, help = "Proceed even when the Git working tree is dirty")]
    pub allow_dirty: bool,

    #[arg(
        long,
        value_name = "ASSISTANT",
        help = "Assistant to configure (claude | codex | both)"
    )]
    pub assistant: Option<String>,
}

pub fn run(_f: &mut Factory, args: SetupArgs) -> Result<(), CliError> {
    let runner = ShellRunner;
    let cwd = std::env::current_dir()?;

    anstream::println!(
        "{}",
        paint(BOLD, "\n🚀 Starting PortOne integration setup\n")
    );

    if !args.allow_dirty {
        let spinner = start_spinner("Checking Git status...");
        let clean = steps::is_git_clean(&runner, &cwd);
        if !clean {
            finish_fail(&spinner, "The Git working tree has uncommitted changes");
            anstream::println!(
                "{}",
                paint(
                    YELLOW,
                    "\nCommit your changes or pass --allow-dirty to continue"
                )
            );
            return Err(CliError::Silent);
        }
        finish_succeed(&spinner, "Git status checked");
    }

    let selection = resolve_assistant_selection(args.assistant.as_deref())?;
    let targets = resolve_targets(selection);

    for &assistant in &targets {
        let definition = assistant.definition();
        let display = definition.display_name;

        let spinner = start_spinner(&format!("Checking {display} installation..."));
        let installed = steps::check_assistant_installed(&runner, assistant, &cwd);

        if !installed {
            finish_warn(&spinner, &format!("{display} is not installed"));

            let should_install = inquire::Confirm::new(&format!("Install {display}?"))
                .with_default(true)
                .prompt()
                .map_err(|err| CliError::Other(err.into()))?;

            if should_install {
                let spinner = start_spinner(&format!("Installing {display}..."));
                match steps::install_assistant(&runner, assistant, &cwd) {
                    Ok(()) => finish_succeed(&spinner, &format!("Installed {display}")),
                    Err(_) => {
                        finish_fail(&spinner, &format!("Failed to install {display}"));
                        anstream::println!(
                            "{}",
                            paint(
                                YELLOW,
                                &format!(
                                    "\nInstall {display} manually: {}",
                                    definition.install_hint
                                )
                            )
                        );
                        return Err(CliError::Silent);
                    }
                }
            } else {
                anstream::println!(
                    "{}",
                    paint(
                        YELLOW,
                        &format!("\nInstall {display} manually: {}", definition.install_hint)
                    )
                );
                return Err(CliError::Silent);
            }
        } else {
            finish_succeed(&spinner, &format!("{display} installation found"));
        }

        if definition.update_command.is_some() {
            let spinner = start_spinner(&format!("Updating {display}..."));
            match steps::update_assistant(&runner, assistant, &cwd) {
                Ok(()) => finish_succeed(&spinner, &format!("Updated {display}")),
                Err(_) => finish_warn(
                    &spinner,
                    &format!("Failed to update {display} (continuing)"),
                ),
            }
        }

        let spinner = start_spinner(&format!("Configuring the PortOne plugin for {display}..."));
        match plugin::configure(&runner, assistant, &cwd) {
            Ok(()) => finish_succeed(&spinner, &format!("Configured plugin for {display}")),
            Err(err) => {
                finish_fail(
                    &spinner,
                    &format!("Failed to configure plugin for {display}"),
                );
                anstream::eprintln!("{}", paint(RED, &format!("{err:#}")));
                return Err(CliError::Silent);
            }
        }
    }

    anstream::println!("{}", paint(GREEN, "\n✅ Setup complete!"));
    show_integration_guide(&targets);
    Ok(())
}

fn resolve_assistant_selection(input: Option<&str>) -> Result<AssistantSelection, CliError> {
    if let Some(input) = input {
        return input.parse().map_err(|()| {
            anstream::println!("{}", paint(RED, &format!("Unsupported assistant: {input}")));
            CliError::Silent
        });
    }

    if !std::io::stdin().is_terminal() {
        return Err(CliError::Flag(
            "--assistant is required in non-interactive environments (claude | codex | both)"
                .to_string(),
        ));
    }

    let choice = inquire::Select::new(
        "Which assistant would you like to configure?",
        vec!["Claude Code + Codex", "Claude Code", "Codex"],
    )
    .prompt()
    .map_err(|err| CliError::Other(err.into()))?;

    Ok(match choice {
        "Claude Code" => AssistantSelection::Claude,
        "Codex" => AssistantSelection::Codex,
        _ => AssistantSelection::Both,
    })
}

fn show_integration_guide(assistants: &[Assistant]) {
    anstream::println!("{}", paint(CYAN, "\n📋 Next steps"));
    anstream::println!("{}", paint(WHITE, &"─".repeat(40)));

    if assistants.contains(&Assistant::Claude) {
        anstream::println!("{}", paint(WHITE, "\n[Claude Code]"));
        anstream::println!("{}", paint(WHITE, "1. Start Claude Code:"));
        anstream::println!("{}", paint(YELLOW, "   $ claude\n"));
        anstream::println!("{}", paint(WHITE, "2. Run this slash command:"));
        anstream::println!("{}", paint(GREEN, "   /portone-integration:start\n"));
    }

    if assistants.contains(&Assistant::Codex) {
        anstream::println!("{}", paint(WHITE, "[Codex]"));
        anstream::println!("{}", paint(WHITE, "1. Start Codex:"));
        anstream::println!("{}", paint(YELLOW, "   $ codex\n"));
        anstream::println!(
            "{}",
            paint(
                WHITE,
                "2. With the `portone-codex` plugin installed, try one of these prompts:"
            )
        );
        anstream::println!(
            "{}",
            paint(
                GREEN,
                "   Implement a PortOne V2 one-time payment integration"
            )
        );
        anstream::println!(
            "{}",
            paint(GREEN, "   Review the PortOne integration in this project\n")
        );
    }

    anstream::println!("{}", paint(WHITE, &"─".repeat(40)));
}

fn paint(style: Style, text: &str) -> String {
    format!("{}{}{}", style.render(), text, style.render_reset())
}

fn start_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner().with_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

fn finish_with_symbol(spinner: &ProgressBar, style: Style, symbol: &str, message: &str) {
    spinner.finish_and_clear();
    anstream::eprintln!("{} {message}", paint(style, symbol));
}

fn finish_succeed(spinner: &ProgressBar, message: &str) {
    finish_with_symbol(spinner, GREEN, "✔", message);
}

fn finish_fail(spinner: &ProgressBar, message: &str) {
    finish_with_symbol(spinner, RED, "✖", message);
}

fn finish_warn(spinner: &ProgressBar, message: &str) {
    finish_with_symbol(spinner, YELLOW, "⚠", message);
}
