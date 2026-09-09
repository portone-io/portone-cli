use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::i18n::LocalizedContext;

use super::assistants::Assistant;
use super::steps::{self, CommandRunner};

const REPOSITORY: &str = "portone-io/portone-cli";
const CLAUDE_MARKETPLACES: &str = "claude plugin marketplace list --json";
const CLAUDE_PLUGINS: &str = "claude plugin list --json";
const CODEX_MARKETPLACES: &str = "codex plugin marketplace list --json";
const CODEX_PLUGINS: &str = "codex plugin list --marketplace portone --json";
const CODEX_ADD: &str = "codex plugin add portone-codex@portone --json";

pub struct Installation {
    pub assistant: Assistant,
    marketplace_exists: bool,
    plugin_installed: bool,
}

/// Inspect every selected host before making any changes to either host.
pub fn preflight(
    runner: &dyn CommandRunner,
    targets: &[Assistant],
    cwd: &Path,
) -> anyhow::Result<Vec<Installation>> {
    steps::check_runtime(runner, cwd)?;
    targets
        .iter()
        .map(|&assistant| {
            steps::check_assistant(runner, assistant, cwd)?;
            inspect(runner, assistant, cwd)
        })
        .collect()
}

fn read_json<T: DeserializeOwned>(
    runner: &dyn CommandRunner,
    command: &str,
    cwd: &Path,
) -> anyhow::Result<T> {
    let output = runner.run_capture_stdout(command, cwd)?;
    serde_json::from_str(&output)
        .with_lcontext(|| crate::message!("setup-invalid-command-json", command = command))
}

#[derive(Deserialize)]
struct ClaudeMarketplace {
    name: String,
    source: String,
    repo: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudePlugin {
    id: String,
    scope: String,
    enabled: bool,
    install_path: PathBuf,
}

#[derive(Deserialize)]
struct CodexMarketplaces {
    marketplaces: Vec<CodexMarketplace>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexMarketplace {
    name: String,
    marketplace_source: Option<CodexSource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexSource {
    source_type: String,
    source: String,
}

#[derive(Deserialize)]
struct CodexPlugins {
    installed: Vec<CodexPlugin>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexPlugin {
    name: String,
    marketplace_name: String,
    installed: bool,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexInstall {
    installed_path: PathBuf,
}

fn expected_repository(source: &str) -> bool {
    let source = source.trim_end_matches('/');
    let repository = source
        .strip_prefix("https://github.com/")
        .or_else(|| source.strip_prefix("ssh://git@github.com/"))
        .or_else(|| source.strip_prefix("git@github.com:"))
        .unwrap_or(source);
    repository
        .strip_suffix(".git")
        .unwrap_or(repository)
        .eq_ignore_ascii_case(REPOSITORY)
}

fn source_conflict(assistant: Assistant) -> anyhow::Error {
    let command = match assistant {
        Assistant::Claude => CLAUDE_MARKETPLACES,
        Assistant::Codex => CODEX_MARKETPLACES,
    };
    anyhow::anyhow!(crate::message!(
        "setup-marketplace-conflict",
        assistant = assistant.definition().display_name,
        command = command
    ))
}

fn inspect(
    runner: &dyn CommandRunner,
    assistant: Assistant,
    cwd: &Path,
) -> anyhow::Result<Installation> {
    let (marketplace_exists, plugin_installed) = match assistant {
        Assistant::Claude => {
            let marketplaces: Vec<ClaudeMarketplace> = read_json(runner, CLAUDE_MARKETPLACES, cwd)?;
            let entries: Vec<_> = marketplaces
                .iter()
                .filter(|entry| entry.name == "portone")
                .collect();
            if entries.len() > 1
                || entries.iter().any(|entry| {
                    let source = match entry.source.as_str() {
                        "github" => entry.repo.as_deref(),
                        "git" => entry.url.as_deref(),
                        _ => None,
                    };
                    !source.is_some_and(expected_repository)
                })
            {
                return Err(source_conflict(assistant));
            }
            let plugins: Vec<ClaudePlugin> = read_json(runner, CLAUDE_PLUGINS, cwd)?;
            (!entries.is_empty(), plugins.iter().any(is_claude_plugin))
        }
        Assistant::Codex => {
            let marketplaces: CodexMarketplaces = read_json(runner, CODEX_MARKETPLACES, cwd)?;
            let entries: Vec<_> = marketplaces
                .marketplaces
                .iter()
                .filter(|entry| entry.name == "portone")
                .collect();
            if entries.len() > 1
                || entries.iter().any(|entry| {
                    !entry.marketplace_source.as_ref().is_some_and(|source| {
                        source.source_type == "git" && expected_repository(&source.source)
                    })
                })
            {
                return Err(source_conflict(assistant));
            }
            let plugins: CodexPlugins = read_json(runner, CODEX_PLUGINS, cwd)?;
            (
                !entries.is_empty(),
                plugins
                    .installed
                    .iter()
                    .any(|plugin| is_codex_plugin(plugin) && plugin.installed),
            )
        }
    };
    Ok(Installation {
        assistant,
        marketplace_exists,
        plugin_installed,
    })
}

fn is_claude_plugin(plugin: &ClaudePlugin) -> bool {
    plugin.id == "portone-integration@portone" && plugin.scope == "user"
}

fn is_codex_plugin(plugin: &CodexPlugin) -> bool {
    plugin.name == "portone-codex" && plugin.marketplace_name == "portone"
}

impl Installation {
    pub fn apply(&self, runner: &dyn CommandRunner, cwd: &Path) -> anyhow::Result<()> {
        let installed_path = match self.assistant {
            Assistant::Claude => {
                runner.run_capture(
                    if self.marketplace_exists {
                        "claude plugin marketplace update portone"
                    } else {
                        "claude plugin marketplace add portone-io/portone-cli --scope user"
                    },
                    cwd,
                )?;
                runner.run_capture(
                    if self.plugin_installed {
                        "claude plugin update portone-integration@portone --scope user"
                    } else {
                        "claude plugin install portone-integration@portone --scope user"
                    },
                    cwd,
                )?;
                let mut plugin = claude_installed(runner, cwd)?;
                if !plugin.enabled {
                    runner.run_capture(
                        "claude plugin enable portone-integration@portone --scope user",
                        cwd,
                    )?;
                    plugin = claude_installed(runner, cwd)?;
                }
                if !plugin.enabled {
                    return Err(not_ready(self.assistant));
                }
                plugin.install_path
            }
            Assistant::Codex => {
                // Native upgrade exits nonzero if any marketplace refresh fails.
                runner.run_capture_stdout(
                    if self.marketplace_exists {
                        "codex plugin marketplace upgrade portone --json"
                    } else {
                        "codex plugin marketplace add portone-io/portone-cli --json"
                    },
                    cwd,
                )?;
                // Native add also reinstalls from the refreshed marketplace snapshot.
                let result: CodexInstall = read_json(runner, CODEX_ADD, cwd)?;
                let plugins: CodexPlugins = read_json(runner, CODEX_PLUGINS, cwd)?;
                if !plugins
                    .installed
                    .iter()
                    .any(|plugin| is_codex_plugin(plugin) && plugin.installed && plugin.enabled)
                {
                    return Err(not_ready(self.assistant));
                }
                result.installed_path
            }
        };
        verify_bundle(self.assistant, &installed_path)
    }
}

fn claude_installed(runner: &dyn CommandRunner, cwd: &Path) -> anyhow::Result<ClaudePlugin> {
    let plugins: Vec<ClaudePlugin> = read_json(runner, CLAUDE_PLUGINS, cwd)?;
    plugins
        .into_iter()
        .find(is_claude_plugin)
        .ok_or_else(|| not_ready(Assistant::Claude))
}

fn not_ready(assistant: Assistant) -> anyhow::Error {
    let command = match assistant {
        Assistant::Claude => "/plugin",
        Assistant::Codex => "/plugins",
    };
    anyhow::anyhow!(crate::message!(
        "setup-plugin-not-ready",
        assistant = assistant.definition().display_name,
        command = command
    ))
}

fn verify_bundle(assistant: Assistant, root: &Path) -> anyhow::Result<()> {
    let verify = || -> anyhow::Result<()> {
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(".mcp.json"))?)?;
        let server = &config["mcpServers"]["portone"];
        anyhow::ensure!(
            server["command"] == "npx"
                && server["args"] == serde_json::json!(["-y", "@portone/mcp-server@latest"]),
            "unexpected PortOne MCP command"
        );
        anyhow::ensure!(
            root.join("skills/portone-cli/SKILL.md").is_file(),
            "missing portone-cli skill"
        );
        if assistant == Assistant::Codex {
            let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
                root.join(".codex-plugin/plugin.json"),
            )?)?;
            anyhow::ensure!(
                manifest["skills"] == "./skills/" && manifest["mcpServers"] == "./.mcp.json",
                "missing skill or MCP manifest reference"
            );
        }
        Ok(())
    };
    verify().with_lcontext(|| {
        crate::message!(
            "setup-invalid-bundle",
            assistant = assistant.definition().display_name,
            path = root.display()
        )
    })
}

#[cfg(test)]
mod tests;
