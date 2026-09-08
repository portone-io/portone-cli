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
    localize_command(command, "", localizer)
}

fn command_path(parent: &str, name: &str) -> String {
    // Clap's generated help subtree mirrors the real commands without arguments.
    let parent = parent.strip_suffix(" help").unwrap_or(parent);
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent} {name}")
    }
}

fn localize_command(mut command: Command, parent: &str, localizer: &Localizer) -> Command {
    let name = command.get_name().to_string();
    let path = command_path(parent, &name);
    if command.get_about().is_some()
        && let Some(about) = about(&path, localizer)
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
        .mut_args(|arg| localize_arg(arg, &path, localizer))
        .mut_subcommands(|subcommand| localize_command(subcommand, &path, localizer))
}

fn about(path: &str, localizer: &Localizer) -> Option<String> {
    Some(match path {
        "portone" => crate::tr!(localizer, "help-about-portone"),
        "portone api" => crate::tr!(localizer, "help-about-api"),
        "portone auth" => crate::tr!(localizer, "help-about-auth"),
        "portone auth login" => crate::tr!(localizer, "help-about-login"),
        "portone auth logout" => crate::tr!(localizer, "help-about-logout"),
        "portone auth status" => crate::tr!(localizer, "help-about-status"),
        "portone auth token" => crate::tr!(localizer, "help-about-token"),
        "portone payment" => crate::tr!(localizer, "help-about-payment"),
        "portone payment list" => crate::tr!(localizer, "help-about-payment-list"),
        "portone payment view" => crate::tr!(localizer, "help-about-payment-view"),
        "portone payment transactions" => crate::tr!(localizer, "help-about-payment-transactions"),
        "portone payment cancel" => crate::tr!(localizer, "help-about-payment-cancel"),
        "portone payment webhook" => crate::tr!(localizer, "help-about-payment-webhook"),
        "portone payment webhook list" => crate::tr!(localizer, "help-about-payment-webhook-list"),
        "portone payment webhook resend" => {
            crate::tr!(localizer, "help-about-payment-webhook-resend")
        }
        "portone store" => crate::tr!(localizer, "help-about-store"),
        "portone store set-default" => crate::tr!(localizer, "help-about-store-set-default"),
        "portone setup" => crate::tr!(localizer, "help-about-setup"),
        "portone completion" => crate::tr!(localizer, "help-about-completion"),
        path if path.ends_with(" help") => crate::tr!(localizer, "help-about-help"),
        _ => return None,
    })
}

fn arg_help(arg: &Arg, owner: &str, localizer: &Localizer) -> Option<String> {
    if owner.starts_with("portone payment")
        && let Some(help) = payment_arg_help(arg.get_id().as_str(), localizer)
    {
        return Some(help);
    }
    Some(match arg.get_id().as_str() {
        "help" if arg.get_long_help().is_some() => {
            crate::tr!(localizer, "help-flag-help-short")
        }
        "help" => crate::tr!(localizer, "help-flag-help"),
        "version" => crate::tr!(localizer, "help-flag-version"),
        "profile" if owner == "portone auth login" => {
            crate::tr!(localizer, "help-profile-store")
        }
        "profile" if owner == "portone auth logout" => {
            crate::tr!(localizer, "help-profile-remove")
        }
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
        "id" if owner == "portone store set-default" => crate::tr!(localizer, "help-store-id"),
        "view" if owner == "portone store set-default" => crate::tr!(localizer, "help-store-view"),
        "unset" if owner == "portone store set-default" => {
            crate::tr!(localizer, "help-store-unset")
        }
        "subcommand" if owner.ends_with(" help") => crate::tr!(localizer, "help-subcommand"),
        _ => return None,
    })
}

fn payment_arg_help(id: &str, localizer: &Localizer) -> Option<String> {
    Some(match id {
        "store" => crate::tr!(localizer, "help-payment-store"),
        "payment_id" => crate::tr!(localizer, "help-payment-id"),
        "json" => crate::tr!(localizer, "help-resource-json"),
        "jq" => crate::tr!(localizer, "help-resource-jq"),
        "limit" => crate::tr!(localizer, "help-payment-limit"),
        "status" => crate::tr!(localizer, "help-payment-status"),
        "method" => crate::tr!(localizer, "help-payment-method"),
        "pg" => crate::tr!(localizer, "help-payment-pg"),
        "currency" => crate::tr!(localizer, "help-payment-currency"),
        "test" => crate::tr!(localizer, "help-payment-test"),
        "live" => crate::tr!(localizer, "help-payment-live"),
        "version" => crate::tr!(localizer, "help-payment-version"),
        "from" => crate::tr!(localizer, "help-payment-from"),
        "until" => crate::tr!(localizer, "help-payment-until"),
        "time_field" => crate::tr!(localizer, "help-payment-time-field"),
        "sort" => crate::tr!(localizer, "help-payment-sort"),
        "order" => crate::tr!(localizer, "help-payment-order"),
        "search" => crate::tr!(localizer, "help-payment-search"),
        "search_field" => crate::tr!(localizer, "help-payment-search-field"),
        "all_stores" => crate::tr!(localizer, "help-payment-all-stores"),
        "reason" => crate::tr!(localizer, "help-payment-cancel-reason"),
        "amount" => crate::tr!(localizer, "help-payment-cancel-amount"),
        "tax_free_amount" => crate::tr!(localizer, "help-payment-cancel-tax-free"),
        "vat_amount" => crate::tr!(localizer, "help-payment-cancel-vat"),
        "current_cancellable_amount" => crate::tr!(localizer, "help-payment-cancel-current"),
        "input" => crate::tr!(localizer, "help-payment-cancel-input"),
        "yes" => crate::tr!(localizer, "help-payment-cancel-yes"),
        "webhook_id" => crate::tr!(localizer, "help-payment-webhook-id"),
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

    fn assert_metadata_matches(
        source: &Command,
        translated: &Command,
        parent: &str,
        localizer: &Localizer,
    ) {
        assert_eq!(source.get_name(), translated.get_name());
        let path = command_path(parent, source.get_name());
        if source.get_about().is_some() {
            assert!(
                about(&path, localizer).is_some(),
                "missing translation for {path}"
            );
        }
        assert_eq!(source.get_about(), translated.get_about());
        assert_eq!(source.get_long_about(), translated.get_long_about());
        assert_eq!(
            source.get_after_long_help(),
            translated.get_after_long_help()
        );
        for arg in source.get_arguments() {
            assert!(
                arg_help(arg, &path, localizer).is_some(),
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
                &path,
                localizer,
            );
        }
    }

    #[test]
    fn english_catalog_covers_and_matches_the_derive_grammar() {
        let localizer = Localizer::english();
        let mut original = super::super::Cli::command();
        original.build();
        assert_metadata_matches(&original, &command(&localizer), "", &localizer);
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
