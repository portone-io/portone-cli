use std::io::{self, Write};

use clap::{Arg, Command};
use clap_complete::aot::{Generator, Shell};

// The upstream Fish generator stops at two subcommands and identifies nested
// commands by any matching word. Resolve full paths, retaining its option output.
pub(super) fn generate(root: &Command, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "function __fish_portone_command_path")?;
    writeln!(out, "    set -l words (commandline -opc)")?;
    writeln!(out, "    set -e words[1]")?;
    writeln!(out, "    set -l path portone")?;
    writeln!(out, "    while true\n        switch \"$path\"")?;
    path_cases(root, "portone", out)?;
    writeln!(out, "            case '*'\n                break")?;
    writeln!(
        out,
        "        end\n    end\n    printf '%s\\n' \"$path\"\nend"
    )?;
    writeln!(out, "\nfunction __fish_portone_command_is")?;
    writeln!(out, "    set -l path (__fish_portone_command_path)")?;
    writeln!(out, "    test \"$path\" = \"$argv[1]\"\nend\n")?;
    commands(root, "portone", out)
}

fn path_cases(command: &Command, path: &str, out: &mut dyn Write) -> io::Result<()> {
    if command.has_subcommands() {
        writeln!(out, "            case {}", quote(path))?;
        write!(out, "                argparse -s")?;
        for arg in command.get_arguments().filter(|arg| !arg.is_positional()) {
            if let Some(spec) = option_spec(arg) {
                write!(out, " {}", quote(&spec))?;
            }
        }
        writeln!(out, " -- $words 2>/dev/null\n                or return 1")?;
        writeln!(out, "                set words $argv")?;
        writeln!(
            out,
            "                if not set -q words[1]\n                    break\n                end"
        )?;
        writeln!(out, "                switch \"$words[1]\"")?;
        for child in command
            .get_subcommands()
            .filter(|child| !child.is_hide_set())
        {
            let names = child
                .get_name_and_visible_aliases()
                .into_iter()
                .map(quote)
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(out, "                    case {names}")?;
            writeln!(
                out,
                "                        set path {}",
                quote(&child_path(path, child.get_name()))
            )?;
        }
        writeln!(
            out,
            "                    case '*'\n                        break\n                end"
        )?;
        writeln!(out, "                set -e words[1]")?;
    }
    for child in command
        .get_subcommands()
        .filter(|child| !child.is_hide_set())
    {
        path_cases(child, &child_path(path, child.get_name()), out)?;
    }
    Ok(())
}

fn option_spec(arg: &Arg) -> Option<String> {
    let mut spec = match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(long)) => format!("{short}/{long}"),
        (Some(short), None) => short.to_string(),
        (None, Some(long)) => long.to_string(),
        (None, None) => return None,
    };
    if arg.get_action().takes_values() {
        spec.push('=');
        if arg
            .get_num_args()
            .is_some_and(|range| range.min_values() == 0)
        {
            spec.push('?');
        }
    }
    Some(spec)
}

fn commands(command: &Command, path: &str, out: &mut dyn Write) -> io::Result<()> {
    let condition = quote(&format!("__fish_portone_command_is {}", quote(path)));
    let prefix = format!("complete -c portone -n {condition}");
    let options = Command::new("portone")
        .bin_name("portone")
        .args(command.get_arguments().cloned());
    let mut generated = Vec::new();
    Shell::Fish.try_generate(&options, &mut generated)?;
    let generated = String::from_utf8(generated).map_err(io::Error::other)?;
    for line in generated.lines() {
        if let Some(options) = line.strip_prefix("complete -c portone") {
            writeln!(out, "{prefix}{options}")?;
        } else {
            writeln!(out, "{line}")?;
        }
    }
    for child in command
        .get_subcommands()
        .filter(|child| !child.is_hide_set())
    {
        for name in child.get_name_and_visible_aliases() {
            write!(out, "{prefix} -f -a {}", quote(name))?;
            if let Some(about) = child.get_about() {
                write!(out, " -d {}", quote(&about.to_string().replace('\n', " ")))?;
            }
            writeln!(out)?;
        }
        commands(child, &child_path(path, child.get_name()), out)?;
    }
    Ok(())
}

fn child_path(parent: &str, name: &str) -> String {
    format!("{parent} {name}")
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}
