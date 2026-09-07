//! Localized presentation of the derive-based command grammar.

use clap::{Arg, Command, CommandFactory};

use crate::i18n::Localizer;

/// Build the command tree for one invocation's fixed display language.
///
/// Building before localization includes clap's generated help commands and flags.
/// Their actions and the argument grammar remain unchanged, so clap's diagnostics
/// continue to use its built-in English text.
pub fn command(localizer: &Localizer) -> Command {
    let mut command = super::Cli::command();
    command.build();
    localize_command(command, localizer)
}

fn localize_command(mut command: Command, localizer: &Localizer) -> Command {
    let name = command.get_name().to_string();
    if command.get_about().is_some()
        && let Some(about) = about(&name, localizer)
    {
        command = command.about(about);
    }
    if name == "api" && command.get_long_about().is_some() {
        command = command.long_about(crate::tr!(localizer, "help-api-long-about"));
    }
    if name == "api" && command.get_after_long_help().is_some() {
        command = command.after_long_help(crate::tr!(localizer, "help-api-examples"));
    }
    if localizer.lang() == "ko" {
        command = command
            .subcommand_help_heading(crate::tr!(localizer, "help-heading-commands"))
            .help_template(format!(
                "{{before-help}}{{about-with-newline}}\n{} {{usage}}\n\n{{all-args}}{{after-help}}",
                crate::tr!(localizer, "help-heading-usage")
            ));
    }
    command
        .mut_args(|arg| localize_arg(arg, &name, localizer))
        .mut_subcommands(|subcommand| localize_command(subcommand, localizer))
}

fn about(name: &str, localizer: &Localizer) -> Option<String> {
    Some(match name {
        "portone" => crate::tr!(localizer, "help-about-portone"),
        "api" => crate::tr!(localizer, "help-about-api"),
        "auth" => crate::tr!(localizer, "help-about-auth"),
        "login" => crate::tr!(localizer, "help-about-login"),
        "logout" => crate::tr!(localizer, "help-about-logout"),
        "status" => crate::tr!(localizer, "help-about-status"),
        "token" => crate::tr!(localizer, "help-about-token"),
        "setup" => crate::tr!(localizer, "help-about-setup"),
        "completion" => crate::tr!(localizer, "help-about-completion"),
        "help" => crate::tr!(localizer, "help-about-help"),
        _ => return None,
    })
}

fn arg_help(arg: &Arg, owner: &str, localizer: &Localizer) -> Option<String> {
    Some(match arg.get_id().as_str() {
        "help" if arg.get_long_help().is_some() => {
            crate::tr!(localizer, "help-flag-help-short")
        }
        "help" => crate::tr!(localizer, "help-flag-help"),
        "version" => crate::tr!(localizer, "help-flag-version"),
        "profile" if owner == "login" => crate::tr!(localizer, "help-profile-store"),
        "profile" if owner == "logout" => crate::tr!(localizer, "help-profile-remove"),
        "profile" => crate::tr!(localizer, "help-profile-use"),
        "base_url" => crate::tr!(localizer, "help-base-url"),
        "endpoint" => crate::tr!(localizer, "help-endpoint"),
        "method" => crate::tr!(localizer, "help-method"),
        "fields" => crate::tr!(localizer, "help-fields"),
        "raw_fields" => crate::tr!(localizer, "help-raw-fields"),
        "headers" => crate::tr!(localizer, "help-headers"),
        "input" => crate::tr!(localizer, "help-input"),
        "include" => crate::tr!(localizer, "help-include"),
        "paginate" => crate::tr!(localizer, "help-paginate"),
        "slurp" => crate::tr!(localizer, "help-slurp"),
        "jq" => crate::tr!(localizer, "help-jq"),
        "cache" => crate::tr!(localizer, "help-cache"),
        "silent" => crate::tr!(localizer, "help-silent"),
        "verbose" => crate::tr!(localizer, "help-verbose"),
        "allow_escape_sequences" => crate::tr!(localizer, "help-allow-escape-sequences"),
        "scopes" => crate::tr!(localizer, "help-scopes"),
        "insecure_storage" => crate::tr!(localizer, "help-insecure-storage"),
        "no_browser" => crate::tr!(localizer, "help-no-browser"),
        "show_secret" => crate::tr!(localizer, "help-show-secret"),
        "allow_dirty" => crate::tr!(localizer, "help-allow-dirty"),
        "assistant" => crate::tr!(localizer, "help-assistant"),
        "shell" => crate::tr!(localizer, "help-shell"),
        "subcommand" if owner == "help" => crate::tr!(localizer, "help-subcommand"),
        _ => return None,
    })
}

fn localize_arg(mut arg: Arg, owner: &str, localizer: &Localizer) -> Arg {
    if let Some(help) = arg_help(&arg, owner, localizer) {
        arg = arg.help(help);
    }
    if arg.get_id() == "help" && arg.get_long_help().is_some() {
        arg = arg.long_help(crate::tr!(localizer, "help-flag-help-long"));
    }
    if localizer.lang() == "ko" {
        let metadata = metadata(&arg, localizer);
        if !metadata.is_empty() {
            let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
            let long_help = arg.get_long_help().map(ToString::to_string);
            arg = arg.help(format!("{help} {}", metadata.join(" ")));
            if let Some(long_help) = long_help {
                arg = arg.long_help(format!("{long_help}\n{}", metadata.join("\n")));
            }
        }
        let heading = if arg.is_positional() {
            crate::tr!(localizer, "help-heading-arguments")
        } else {
            crate::tr!(localizer, "help-heading-options")
        };
        arg = arg
            .help_heading(heading)
            .hide_default_value(true)
            .hide_env(true)
            .hide_possible_values(true);
    }
    arg
}

fn metadata(arg: &Arg, localizer: &Localizer) -> Vec<String> {
    let mut metadata = Vec::new();
    if !arg.is_hide_env_set()
        && let Some(name) = arg.get_env()
    {
        let mut value = name.to_string_lossy().into_owned();
        if !arg.is_hide_env_values_set() {
            value.push('=');
            if let Some(env_value) = std::env::var_os(name) {
                value.push_str(&env_value.to_string_lossy());
            }
        }
        metadata.push(crate::tr!(localizer, "help-metadata-env", value = value));
    }
    if arg.get_action().takes_values()
        && !arg.is_hide_default_value_set()
        && !arg.get_default_values().is_empty()
    {
        let value = arg
            .get_default_values()
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        metadata.push(crate::tr!(
            localizer,
            "help-metadata-default",
            value = value
        ));
    }
    if !arg.is_hide_possible_values_set() {
        let values = arg
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_string())
            .collect::<Vec<_>>();
        if !values.is_empty() {
            metadata.push(crate::tr!(
                localizer,
                "help-metadata-possible-values",
                values = values.join(", ")
            ));
        }
    }
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_metadata_matches(source: &Command, translated: &Command, localizer: &Localizer) {
        assert_eq!(source.get_name(), translated.get_name());
        if source.get_about().is_some() {
            assert!(about(source.get_name(), localizer).is_some());
        }
        assert_eq!(source.get_about(), translated.get_about());
        assert_eq!(source.get_long_about(), translated.get_long_about());
        assert_eq!(
            source.get_after_long_help(),
            translated.get_after_long_help()
        );
        for arg in source.get_arguments() {
            assert!(
                arg_help(arg, source.get_name(), localizer).is_some(),
                "missing translation for {} {}",
                source.get_name(),
                arg.get_id()
            );
            let translated_arg = translated
                .get_arguments()
                .find(|candidate| candidate.get_id() == arg.get_id())
                .unwrap();
            assert_eq!(arg.get_help(), translated_arg.get_help());
            assert_eq!(arg.get_long_help(), translated_arg.get_long_help());
        }
        for subcommand in source.get_subcommands() {
            assert_metadata_matches(
                subcommand,
                translated.find_subcommand(subcommand.get_name()).unwrap(),
                localizer,
            );
        }
    }

    #[test]
    fn english_catalog_covers_and_matches_the_derive_grammar() {
        let localizer = Localizer::english();
        let mut original = super::super::Cli::command();
        original.build();
        assert_metadata_matches(&original, &command(&localizer), &localizer);
    }

    #[test]
    fn examples_translate_comments_and_preserve_all_shell_code() {
        fn shell_code(command: &Command) -> String {
            command
                .find_subcommand("api")
                .unwrap()
                .get_after_long_help()
                .unwrap()
                .to_string()
                .lines()
                .filter(|line| !line.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
        }
        assert_eq!(
            shell_code(&command(&Localizer::english())),
            shell_code(&command(&Localizer::korean()))
        );
    }

    #[test]
    fn korean_metadata_preserves_visible_values_and_hides_hidden_values() {
        let localizer = Localizer::korean();
        let arg = Arg::new("example")
            .long("example")
            .help("example")
            .env("PORTONE_HELP_TEST_ENV")
            .hide_env_values(true)
            .default_value("first")
            .value_parser(["first", "second"]);
        let translated = localize_arg(arg.clone(), "example", &localizer);
        let help = translated.get_help().unwrap().to_string();
        assert!(
            help.contains("[환경 변수: PORTONE_HELP_TEST_ENV]"),
            "{help}"
        );
        assert!(help.contains("[기본값: first]"), "{help}");
        assert!(help.contains("[가능한 값: first, second]"), "{help}");
        let hidden = localize_arg(
            arg.hide_env(true)
                .hide_default_value(true)
                .hide_possible_values(true),
            "example",
            &localizer,
        );
        assert_eq!(hidden.get_help().unwrap().to_string(), "example");
    }
}
