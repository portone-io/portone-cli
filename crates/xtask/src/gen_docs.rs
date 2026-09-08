use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};

pub fn render_all() -> BTreeMap<String, String> {
    let mut root = portone_cli::cmd::help::command(&portone_cli::i18n::Localizer::english());
    root.build();
    let mut pages = BTreeMap::new();
    walk(&mut root, vec!["portone".to_string()], &mut pages);
    let index = render_index(&pages);
    pages.insert("index.md".to_string(), index);
    pages
}

fn walk(cmd: &mut Command, path: Vec<String>, pages: &mut BTreeMap<String, String>) {
    let page = render_page(cmd, &path);
    pages.insert(format!("{}.md", path.join("_")), page);
    for sub in cmd.get_subcommands_mut() {
        if sub.get_name() == "help" || sub.is_hide_set() {
            continue;
        }
        let mut child_path = path.clone();
        child_path.push(sub.get_name().to_string());
        walk(sub, child_path, pages);
    }
}

fn render_page(cmd: &mut Command, path: &[String]) -> String {
    let full_name = path.join(" ");
    let mut page = String::new();
    let _ = writeln!(page, "# {full_name}");

    let about = cmd
        .get_long_about()
        .or_else(|| cmd.get_about())
        .map(|text| text.to_string());
    if let Some(about) = about {
        let _ = writeln!(page, "\n{about}");
    }

    let usage = cmd.render_usage().to_string();
    let usage = usage.strip_prefix("Usage: ").unwrap_or(&usage).to_string();
    let _ = writeln!(page, "\n```\n{usage}\n```");

    let subcommands: Vec<(String, String)> = cmd
        .get_subcommands()
        .filter(|sub| sub.get_name() != "help" && !sub.is_hide_set())
        .map(|sub| {
            (
                sub.get_name().to_string(),
                sub.get_about().map(|s| s.to_string()).unwrap_or_default(),
            )
        })
        .collect();
    if !subcommands.is_empty() {
        let _ = writeln!(
            page,
            "\n## Commands\n\n| Command | Description |\n| --- | --- |"
        );
        for (name, about) in subcommands {
            let _ = writeln!(
                page,
                "| [{full_name} {name}]({}_{name}.md) | {} |",
                path.join("_"),
                escape_cell(&about)
            );
        }
    }

    let positionals: Vec<&Arg> = cmd
        .get_positionals()
        .filter(|arg| !arg.is_hide_set())
        .collect();
    if !positionals.is_empty() {
        let _ = writeln!(
            page,
            "\n## Arguments\n\n| Argument | Description |\n| --- | --- |"
        );
        for arg in positionals {
            let _ = writeln!(
                page,
                "| `<{}>` | {} |",
                value_name(arg),
                escape_cell(&help_text(arg))
            );
        }
    }

    let options: Vec<&Arg> = cmd
        .get_arguments()
        .filter(|arg| !arg.is_positional() && !arg.is_hide_set() && !is_builtin(arg))
        .collect();
    if !options.is_empty() {
        let _ = writeln!(
            page,
            "\n## Options\n\n| Option | Description |\n| --- | --- |"
        );
        for arg in options {
            let _ = writeln!(
                page,
                "| `{}` | {} |",
                option_syntax(arg),
                escape_cell(&help_text(arg))
            );
        }
    }

    if let Some(examples) = cmd.get_after_long_help() {
        let examples = examples.to_string();
        let _ = writeln!(page, "\n## Examples\n\n```sh\n{}\n```", examples.trim_end());
    }

    if path.len() > 1 {
        let parent = &path[..path.len() - 1];
        let _ = writeln!(
            page,
            "\n## See also\n\n- [{}]({}.md)",
            parent.join(" "),
            parent.join("_")
        );
    }

    page
}

fn render_index(pages: &BTreeMap<String, String>) -> String {
    let mut index = String::new();
    let _ = writeln!(index, "# PortOne CLI reference");
    let _ = writeln!(index);
    for file in pages.keys() {
        let name = file.trim_end_matches(".md").replace('_', " ");
        let _ = writeln!(index, "- [{name}]({file})");
    }
    index
}

pub fn write(dir: &Path, pages: &BTreeMap<String, String>) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if is_unknown_md(&path, pages) {
            fs::remove_file(&path)?;
        }
    }
    for (file, content) in pages {
        fs::write(dir.join(file), content)?;
    }
    Ok(())
}

pub fn check(dir: &Path, pages: &BTreeMap<String, String>) -> io::Result<Vec<PathBuf>> {
    let mut stale = Vec::new();
    for (file, content) in pages {
        let path = dir.join(file);
        match fs::read_to_string(&path) {
            Ok(existing) => {
                if existing.replace("\r\n", "\n") != *content {
                    stale.push(path);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => stale.push(path),
            Err(err) => return Err(err),
        }
    }
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if is_unknown_md(&path, pages) {
                stale.push(path);
            }
        }
    }
    stale.sort();
    Ok(stale)
}

fn is_unknown_md(path: &Path, pages: &BTreeMap<String, String>) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
        && !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| pages.contains_key(name))
}

fn is_builtin(arg: &Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

fn option_syntax(arg: &Arg) -> String {
    let mut syntax = String::new();
    if let Some(short) = arg.get_short() {
        let _ = write!(syntax, "-{short}");
    }
    if let Some(long) = arg.get_long() {
        if !syntax.is_empty() {
            syntax.push_str(", ");
        }
        let _ = write!(syntax, "--{long}");
    }
    if arg.get_action().takes_values() {
        if arg
            .get_num_args()
            .is_some_and(|range| range.min_values() == 0)
        {
            let _ = write!(syntax, " [<{}>]", value_name(arg));
        } else {
            let _ = write!(syntax, " <{}>", value_name(arg));
        }
    }
    syntax
}

fn value_name(arg: &Arg) -> String {
    arg.get_value_names()
        .and_then(|names| names.first())
        .map(|name| name.to_string())
        .unwrap_or_else(|| arg.get_id().to_string().to_uppercase())
}

fn help_text(arg: &Arg) -> String {
    let mut text = arg
        .get_help()
        .map(|help| help.to_string())
        .unwrap_or_default();
    if arg.get_action().takes_values() && !arg.is_hide_possible_values_set() {
        let values: Vec<String> = arg
            .get_possible_values()
            .iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_string())
            .collect();
        if !values.is_empty() {
            if !text.is_empty() {
                text.push(' ');
            }
            let _ = write!(text, "[possible values: {}]", values.join(", "));
        }
    }
    text
}

fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_all_contains_expected_pages() {
        let pages = render_all();
        let files: Vec<&str> = pages.keys().map(String::as_str).collect();
        assert_eq!(
            files,
            [
                "index.md",
                "portone.md",
                "portone_api.md",
                "portone_auth.md",
                "portone_auth_login.md",
                "portone_auth_logout.md",
                "portone_auth_status.md",
                "portone_auth_token.md",
                "portone_completion.md",
                "portone_payment.md",
                "portone_payment_cancel.md",
                "portone_payment_list.md",
                "portone_payment_transactions.md",
                "portone_payment_view.md",
                "portone_payment_webhook.md",
                "portone_payment_webhook_list.md",
                "portone_payment_webhook_resend.md",
                "portone_setup.md",
                "portone_store.md",
                "portone_store_set-default.md",
            ]
        );
    }

    #[test]
    fn subcommand_usage_includes_full_path() {
        let pages = render_all();
        let login = &pages["portone_auth_login.md"];
        assert!(login.starts_with("# portone auth login\n"));
        assert!(login.contains("portone auth login [OPTIONS]"), "{login}");
        assert!(!login.contains('\u{1b}'));
    }

    #[test]
    fn api_page_lists_flattened_auth_options() {
        let pages = render_all();
        let api = &pages["portone_api.md"];
        assert!(api.contains("`-X, --method <METHOD>`"));
        assert!(api.contains("`--profile <NAME>`"));
        assert!(api.contains("`<ENDPOINT>`"));
        assert!(!api.contains("--help"));
    }

    #[test]
    fn nested_payment_pages_include_full_paths_and_inherited_options() {
        let pages = render_all();
        let webhook = &pages["portone_payment_webhook_resend.md"];
        assert!(
            webhook.contains("portone payment webhook resend [OPTIONS] <PAYMENT_ID>"),
            "{webhook}"
        );
        assert!(webhook.contains("`--webhook-id <WEBHOOK_ID>`"));
        assert!(webhook.contains("`--store <STORE_ID>`"));
        assert!(webhook.contains("`--profile <NAME>`"));
        assert!(webhook.contains("`--json [<FIELDS>]`"));
        assert!(webhook.contains("[portone payment webhook](portone_payment_webhook.md)"));
        let list = &pages["portone_payment_list.md"];
        assert!(list.contains("`--version <VERSION>`"));
        assert!(list.contains("`-L, --limit <LIMIT>`"));
    }

    #[test]
    fn api_page_wraps_examples_in_code_fence() {
        let pages = render_all();
        let api = &pages["portone_api.md"];
        assert!(api.contains("\n## Examples\n\n```sh\n# "), "{api}");
        assert!(api.contains("$ portone api graphql"));
        assert!(api.contains("```\n\n## See also"), "{api}");
    }

    #[test]
    fn cells_escape_pipes() {
        let pages = render_all();
        let setup = &pages["portone_setup.md"];
        assert!(setup.contains("(claude \\| codex \\| both)"));
    }

    #[test]
    fn completion_page_lists_possible_shells() {
        let pages = render_all();
        let completion = &pages["portone_completion.md"];
        assert!(
            completion.contains("[possible values: bash, elvish, fish, powershell, zsh]"),
            "{completion}"
        );
    }

    #[test]
    fn render_is_deterministic() {
        assert_eq!(render_all(), render_all());
    }
}
