mod gen_docs;
mod sync_plugin_skills;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Development tasks for portone-cli")]
struct Xtask {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Generate the command reference in docs/reference")]
    GenDocs(GenDocsArgs),
    #[command(about = "Copy the canonical CLI skill into both plugin bundles")]
    SyncPluginSkills(SyncPluginSkillsArgs),
}

#[derive(Args)]
struct SyncPluginSkillsArgs {
    #[arg(long, help = "Check generated output against the canonical skill")]
    check: bool,
}

#[derive(Args)]
struct GenDocsArgs {
    #[arg(long, help = "Check generated output against committed files")]
    check: bool,

    #[arg(
        long,
        value_name = "DIR",
        help = "Output directory (default: docs/reference)"
    )]
    out_dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    let xtask = Xtask::parse();
    match xtask.command {
        Command::GenDocs(args) => run_gen_docs(args),
        Command::SyncPluginSkills(args) => run_sync_plugin_skills(args),
    }
}

fn run_sync_plugin_skills(args: SyncPluginSkillsArgs) -> ExitCode {
    match sync_plugin_skills::run(&workspace_root(), args.check) {
        Ok(differences) if args.check && !differences.is_empty() => {
            for difference in differences {
                eprintln!("{difference}");
            }
            eprintln!("run `cargo xtask sync-plugin-skills` to update them");
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_gen_docs(args: GenDocsArgs) -> ExitCode {
    let dir = args
        .out_dir
        .unwrap_or_else(|| workspace_root().join("docs/reference"));
    let pages = gen_docs::render_all();
    if args.check {
        match gen_docs::check(&dir, &pages) {
            Ok(stale) if stale.is_empty() => ExitCode::SUCCESS,
            Ok(stale) => {
                for path in stale {
                    eprintln!("stale: {}", path.display());
                }
                eprintln!("run `cargo xtask gen-docs` to update them");
                ExitCode::FAILURE
            }
            Err(err) => {
                eprintln!("xtask: {err}");
                ExitCode::FAILURE
            }
        }
    } else {
        match gen_docs::write(&dir, &pages) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask: {err}");
                ExitCode::FAILURE
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root not found")
        .to_path_buf()
}
