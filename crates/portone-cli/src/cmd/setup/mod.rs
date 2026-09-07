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
use crate::i18n::{LocalizedErrorContext, Localizer};

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

pub fn run(f: &mut Factory, args: SetupArgs) -> Result<(), CliError> {
    let localizer = f.localizer.clone();
    let runner = ShellRunner;
    let cwd = std::env::current_dir()?;

    anstream::println!(
        "{}",
        paint(
            BOLD,
            &format!("\n{}\n", crate::tr!(localizer, "setup-starting"))
        )
    );

    if !args.allow_dirty {
        let spinner = start_spinner(&crate::tr!(localizer, "setup-check-git"));
        let clean = steps::is_git_clean(&runner, &cwd);
        if !clean {
            finish_fail(&spinner, &crate::tr!(localizer, "setup-git-dirty"));
            anstream::println!(
                "{}",
                paint(
                    YELLOW,
                    &format!("\n{}", crate::tr!(localizer, "setup-allow-dirty-hint"))
                )
            );
            return Err(CliError::Silent);
        }
        finish_succeed(&spinner, &crate::tr!(localizer, "setup-git-checked"));
    }

    let selection = resolve_assistant_selection(
        &localizer,
        args.assistant.as_deref(),
        std::io::stdin().is_terminal(),
    )?;
    let targets = resolve_targets(selection);

    for &assistant in &targets {
        let definition = assistant.definition();
        let display = definition.display_name;

        let spinner = start_spinner(&crate::tr!(
            localizer,
            "setup-check-installation",
            assistant = display
        ));
        let installed = steps::check_assistant_installed(&runner, assistant, &cwd);

        if !installed {
            finish_warn(
                &spinner,
                &crate::tr!(localizer, "setup-not-installed", assistant = display),
            );

            let should_install = confirm_installation(&localizer, display)?;

            if should_install {
                let spinner = start_spinner(&crate::tr!(
                    localizer,
                    "setup-installing",
                    assistant = display
                ));
                match steps::install_assistant(&runner, assistant, &cwd) {
                    Ok(()) => finish_succeed(
                        &spinner,
                        &crate::tr!(localizer, "setup-installed", assistant = display),
                    ),
                    Err(_) => {
                        finish_fail(
                            &spinner,
                            &crate::tr!(localizer, "setup-install-failed", assistant = display),
                        );
                        anstream::println!(
                            "{}",
                            paint(
                                YELLOW,
                                &format!(
                                    "\n{}",
                                    crate::tr!(
                                        localizer,
                                        "setup-install-manually",
                                        assistant = display,
                                        command = definition.install_hint
                                    )
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
                        &format!(
                            "\n{}",
                            crate::tr!(
                                localizer,
                                "setup-install-manually",
                                assistant = display,
                                command = definition.install_hint
                            )
                        )
                    )
                );
                return Err(CliError::Silent);
            }
        } else {
            finish_succeed(
                &spinner,
                &crate::tr!(localizer, "setup-installation-found", assistant = display),
            );
        }

        if definition.update_command.is_some() {
            let spinner = start_spinner(&crate::tr!(
                localizer,
                "setup-updating",
                assistant = display
            ));
            match steps::update_assistant(&runner, assistant, &cwd) {
                Ok(()) => finish_succeed(
                    &spinner,
                    &crate::tr!(localizer, "setup-updated", assistant = display),
                ),
                Err(_) => finish_warn(
                    &spinner,
                    &crate::tr!(localizer, "setup-update-failed", assistant = display),
                ),
            }
        }

        let spinner = start_spinner(&crate::tr!(
            localizer,
            "setup-configuring-plugin",
            assistant = display
        ));
        match plugin::configure(&runner, assistant, &cwd) {
            Ok(()) => finish_succeed(
                &spinner,
                &crate::tr!(localizer, "setup-plugin-configured", assistant = display),
            ),
            Err(err) => {
                finish_fail(
                    &spinner,
                    &crate::tr!(localizer, "setup-plugin-failed", assistant = display),
                );
                anstream::eprintln!("{}", paint(RED, &localizer.format_error(&err)));
                return Err(CliError::Silent);
            }
        }
    }

    anstream::println!(
        "{}",
        paint(
            GREEN,
            &format!("\n{}", crate::tr!(localizer, "setup-complete"))
        )
    );
    show_integration_guide(&localizer, &targets);
    Ok(())
}

fn resolve_assistant_selection(
    localizer: &Localizer,
    input: Option<&str>,
    interactive: bool,
) -> Result<AssistantSelection, CliError> {
    if let Some(input) = input {
        return input.parse().map_err(|()| {
            anstream::println!(
                "{}",
                paint(
                    RED,
                    &crate::tr!(localizer, "setup-unsupported-assistant", assistant = input)
                )
            );
            CliError::Silent
        });
    }

    if !interactive {
        return Err(CliError::Flag(crate::tr!(
            localizer,
            "setup-assistant-required"
        )));
    }

    let text = SelectionPromptText::new(localizer);
    let choice = text.prompt().prompt().map_err(prompt_error)?;

    Ok(match choice {
        "Claude Code" => AssistantSelection::Claude,
        "Codex" => AssistantSelection::Codex,
        _ => AssistantSelection::Both,
    })
}

struct SelectionPromptText {
    question: String,
    hint: String,
    canceled: String,
}

impl SelectionPromptText {
    fn new(localizer: &Localizer) -> Self {
        Self {
            question: crate::tr!(localizer, "setup-assistant-question"),
            hint: crate::tr!(localizer, "setup-selection-hint"),
            canceled: crate::tr!(localizer, "setup-prompt-canceled-indicator"),
        }
    }

    fn prompt(&self) -> inquire::Select<'_, &str> {
        let mut prompt = inquire::Select::new(
            &self.question,
            vec!["Claude Code + Codex", "Claude Code", "Codex"],
        )
        .with_help_message(&self.hint);
        // Preserve inquire's terminal and NO_COLOR configuration while changing its text.
        prompt.render_config.canceled_prompt_indicator.content = &self.canceled;
        prompt
    }
}

fn confirm_installation(localizer: &Localizer, assistant: &str) -> Result<bool, CliError> {
    let question = crate::tr!(localizer, "setup-install-question", assistant = assistant);
    let invalid_answer = crate::tr!(localizer, "setup-confirm-invalid-answer");
    let canceled = crate::tr!(localizer, "setup-prompt-canceled-indicator");
    let formatter = |answer| {
        if answer {
            crate::tr!(localizer, "setup-confirm-yes")
        } else {
            crate::tr!(localizer, "setup-confirm-no")
        }
    };
    let mut prompt = inquire::Confirm::new(&question)
        .with_default(true)
        .with_error_message(&invalid_answer)
        .with_formatter(&formatter);
    prompt.render_config.canceled_prompt_indicator.content = &canceled;
    prompt.prompt().map_err(prompt_error)
}

fn prompt_error(error: inquire::InquireError) -> CliError {
    use inquire::InquireError;

    let message = match error {
        InquireError::NotTTY => crate::message!("setup-prompt-not-tty"),
        InquireError::OperationCanceled => crate::message!("setup-prompt-canceled"),
        InquireError::OperationInterrupted => crate::message!("setup-prompt-interrupted"),
        // Preserve external error details, localizing only the surrounding prompt message.
        InquireError::InvalidConfiguration(detail) => {
            crate::message!("setup-prompt-invalid-config", detail = detail)
        }
        InquireError::IO(detail) => {
            return CliError::Other(
                anyhow::Error::new(detail).lcontext(crate::message!("setup-prompt-io-error")),
            );
        }
        InquireError::Custom(detail) => {
            return CliError::Other(
                anyhow::Error::from_boxed(detail)
                    .lcontext(crate::message!("setup-prompt-custom-error")),
            );
        }
    };
    CliError::Other(anyhow::anyhow!(message))
}

fn show_integration_guide(localizer: &Localizer, assistants: &[Assistant]) {
    anstream::println!(
        "{}",
        paint(
            CYAN,
            &format!("\n{}", crate::tr!(localizer, "setup-next-steps"))
        )
    );
    anstream::println!("{}", paint(WHITE, &"─".repeat(40)));

    if assistants.contains(&Assistant::Claude) {
        anstream::println!("{}", paint(WHITE, "\n[Claude Code]"));
        anstream::println!(
            "{}",
            paint(
                WHITE,
                &crate::tr!(
                    localizer,
                    "setup-start-assistant",
                    assistant = "Claude Code"
                )
            )
        );
        anstream::println!("{}", paint(YELLOW, "   $ claude\n"));
        anstream::println!(
            "{}",
            paint(WHITE, &crate::tr!(localizer, "setup-run-slash-command"))
        );
        anstream::println!("{}", paint(GREEN, "   /portone-integration:start\n"));
    }

    if assistants.contains(&Assistant::Codex) {
        anstream::println!("{}", paint(WHITE, "[Codex]"));
        anstream::println!(
            "{}",
            paint(
                WHITE,
                &crate::tr!(localizer, "setup-start-assistant", assistant = "Codex")
            )
        );
        anstream::println!("{}", paint(YELLOW, "   $ codex\n"));
        anstream::println!(
            "{}",
            paint(WHITE, &crate::tr!(localizer, "setup-codex-prompts"))
        );
        anstream::println!(
            "{}",
            paint(
                GREEN,
                &format!("   {}", crate::tr!(localizer, "setup-example-implement"))
            )
        );
        anstream::println!(
            "{}",
            paint(
                GREEN,
                &format!("   {}\n", crate::tr!(localizer, "setup-example-review"))
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_selection_keeps_assistant_identifiers_and_default_choice() {
        let english = SelectionPromptText::new(&Localizer::english());
        let korean = SelectionPromptText::new(&Localizer::korean());
        let english_prompt = english.prompt();
        let korean_prompt = korean.prompt();

        assert_eq!(
            english_prompt.message,
            "Which assistant would you like to configure?"
        );
        assert_eq!(korean_prompt.message, "어떤 어시스턴트를 설정할까요?");
        assert_eq!(english_prompt.options, korean_prompt.options);
        assert_eq!(korean_prompt.options[0], "Claude Code + Codex");
        assert_eq!(
            english_prompt.help_message,
            Some("↑↓ to move, enter to select, type to filter")
        );
        assert_eq!(
            korean_prompt.help_message,
            Some("↑↓로 이동, Enter로 선택, 입력하여 필터링")
        );
        assert_eq!(
            korean_prompt
                .render_config
                .canceled_prompt_indicator
                .content,
            "<취소됨>"
        );
    }

    #[test]
    fn noninteractive_selection_requires_flag_in_selected_language() {
        for (localizer, expected) in [
            (
                Localizer::english(),
                "--assistant is required in non-interactive environments (claude | codex | both)",
            ),
            (
                Localizer::korean(),
                "비대화형 환경에서는 --assistant가 필요합니다 (claude | codex | both)",
            ),
        ] {
            let error = resolve_assistant_selection(&localizer, None, false).unwrap_err();
            assert!(matches!(error, CliError::Flag(message) if message == expected));
            assert_eq!(
                resolve_assistant_selection(&localizer, Some("codex"), false).unwrap(),
                AssistantSelection::Codex
            );
        }
    }

    #[test]
    fn prompt_cancel_and_interrupt_are_localized_when_rendered() {
        for (error, expected_english, expected_korean) in [
            (
                inquire::InquireError::OperationCanceled,
                "Operation was canceled by the user",
                "사용자가 작업을 취소했습니다",
            ),
            (
                inquire::InquireError::OperationInterrupted,
                "Operation was interrupted by the user",
                "사용자가 작업을 중단했습니다",
            ),
        ] {
            let CliError::Other(error) = prompt_error(error) else {
                panic!("prompt errors must retain their diagnostic");
            };
            assert_eq!(Localizer::english().format_error(&error), expected_english);
            assert_eq!(Localizer::korean().format_error(&error), expected_korean);
        }
    }

    #[test]
    fn prompt_io_error_preserves_source_for_exit_handling() {
        let source = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "external detail");
        let CliError::Other(error) = prompt_error(inquire::InquireError::IO(source)) else {
            panic!("prompt errors must retain their diagnostic");
        };
        assert!(error.chain().any(|cause| {
            cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::BrokenPipe)
        }));
        assert_eq!(
            Localizer::english().format_error(&error),
            "IO error: external detail"
        );
        assert_eq!(
            Localizer::korean().format_error(&error),
            "입출력 오류: external detail"
        );
    }

    #[test]
    fn prompt_custom_error_renders_external_detail_once() {
        let source = std::io::Error::other("external detail");
        let CliError::Other(error) = prompt_error(inquire::InquireError::Custom(Box::new(source)))
        else {
            panic!("prompt errors must retain their diagnostic");
        };
        assert_eq!(
            Localizer::english().format_error(&error),
            "User-provided error: external detail"
        );
        assert_eq!(
            Localizer::korean().format_error(&error),
            "사용자 정의 오류: external detail"
        );
        assert_eq!(error.root_cause().to_string(), "external detail");
    }
}
