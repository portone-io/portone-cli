use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::*;
use crate::cmd::setup::steps::testing::MockRunner;

fn bundle() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("skills/portone-cli")).unwrap();
    fs::write(
        dir.path().join("skills/portone-cli/SKILL.md"),
        "---\nname: portone-cli\ndescription: Use the CLI\n---\n",
    )
    .unwrap();
    fs::write(
        dir.path().join(".mcp.json"),
        json!({"mcpServers": {"portone": {
            "type": "stdio", "command": "npx",
            "args": ["-y", "@portone/mcp-server@latest"], "env": {}
        }}})
        .to_string(),
    )
    .unwrap();
    fs::create_dir(dir.path().join(".codex-plugin")).unwrap();
    fs::write(
        dir.path().join(".codex-plugin/plugin.json"),
        json!({
            "name": "portone-codex", "version": "1.1.0",
            "skills": "./skills/", "mcpServers": "./.mcp.json"
        })
        .to_string(),
    )
    .unwrap();
    dir
}

fn claude_marketplace(repo: &str) -> String {
    json!([{
        "name": "portone", "source": "github", "repo": repo
    }])
    .to_string()
}

fn codex_marketplace(repo: &str) -> String {
    json!({"marketplaces": [{
        "name": "portone",
        "marketplaceSource": {
            "sourceType": "git", "source": format!("https://github.com/{repo}.git")
        }
    }]})
    .to_string()
}

fn claude_installed(path: &Path, enabled: bool) -> String {
    json!([{
        "id": "portone-integration@portone", "scope": "user",
        "enabled": enabled, "installPath": path
    }])
    .to_string()
}

fn codex_installed(enabled: bool) -> String {
    json!({"installed": [{
        "name": "portone-codex", "marketplaceName": "portone",
        "installed": true, "enabled": enabled
    }]})
    .to_string()
}

fn capable(mut runner: MockRunner, assistants: &[Assistant]) -> MockRunner {
    for assistant in assistants {
        let definition = assistant.definition();
        let version = match assistant {
            Assistant::Claude => "2.1.193 (Claude Code)",
            Assistant::Codex => "codex-cli 0.135.0",
        };
        runner = runner.with_output(definition.version_command, version);
        for &(command, flags) in definition.capabilities {
            runner = runner.with_output(command, &flags.join(" "));
        }
    }
    runner
}

fn mutation_calls(runner: &MockRunner) -> Vec<String> {
    runner
        .calls
        .borrow()
        .iter()
        .map(|call| call.strip_prefix("capture:").unwrap())
        .filter(|command| {
            !command.ends_with("--help")
                && !command.ends_with("--version")
                && !command.contains(" list ")
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn claude_first_install_uses_user_scope_and_verifies_bundle() {
    let installed = bundle();
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(CLAUDE_MARKETPLACES, "[]")
        .with_output(
            CLAUDE_MARKETPLACES,
            &claude_marketplace("portone-io/portone-cli"),
        )
        .with_output(CLAUDE_PLUGINS, "[]")
        .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), true));

    let installation = inspect(&runner, Assistant::Claude, cwd.path()).unwrap();
    assert_eq!(installation.assistant, Assistant::Claude);
    assert!(!installation.marketplace_exists);
    assert!(!installation.plugin_installed);
    installation.apply(&runner, cwd.path()).unwrap();

    assert_eq!(
        mutation_calls(&runner),
        [
            "claude plugin marketplace add portone-io/portone-cli --scope user",
            "claude plugin install portone-integration@portone --scope user",
        ]
    );
}

#[test]
fn codex_first_install_uses_official_add_and_checks_enabled_state() {
    let installed = bundle();
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(CODEX_MARKETPLACES, r#"{"marketplaces":[]}"#)
        .with_output(
            CODEX_MARKETPLACES,
            &codex_marketplace("portone-io/portone-cli"),
        )
        .with_output(CODEX_PLUGINS, r#"{"installed":[]}"#)
        .with_output(CODEX_PLUGINS, &codex_installed(true))
        .with_output(
            CODEX_ADD,
            &json!({"installedPath": installed.path()}).to_string(),
        );

    let installation = inspect(&runner, Assistant::Codex, cwd.path()).unwrap();
    assert!(!installation.marketplace_exists);
    assert!(!installation.plugin_installed);
    installation.apply(&runner, cwd.path()).unwrap();

    assert_eq!(
        mutation_calls(&runner),
        [
            "codex plugin marketplace add portone-io/portone-cli --json",
            CODEX_ADD,
        ]
    );
}

#[test]
fn reruns_upgrade_plugins_without_removing_marketplaces_or_updating_assistants() {
    let installed = bundle();
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(
            CLAUDE_MARKETPLACES,
            &claude_marketplace("portone-io/portone-cli"),
        )
        .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), true))
        .with_output(
            CODEX_MARKETPLACES,
            &codex_marketplace("portone-io/portone-cli"),
        )
        .with_output(CODEX_PLUGINS, &codex_installed(true))
        .with_output(
            CODEX_ADD,
            &json!({"installedPath": installed.path()}).to_string(),
        );

    for assistant in [Assistant::Claude, Assistant::Codex] {
        let installation = inspect(&runner, assistant, cwd.path()).unwrap();
        assert!(installation.marketplace_exists);
        assert!(installation.plugin_installed);
        installation.apply(&runner, cwd.path()).unwrap();
    }

    assert_eq!(
        mutation_calls(&runner),
        [
            "claude plugin marketplace update portone",
            "claude plugin update portone-integration@portone --scope user",
            "codex plugin marketplace upgrade portone --json",
            CODEX_ADD,
        ]
    );
}

#[test]
fn foreign_marketplace_sources_fail_before_mutation() {
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(
            CLAUDE_MARKETPLACES,
            &claude_marketplace("someone/another-plugin"),
        )
        .with_output(
            CODEX_MARKETPLACES,
            &codex_marketplace("someone/another-plugin"),
        );

    for assistant in [Assistant::Claude, Assistant::Codex] {
        assert!(inspect(&runner, assistant, cwd.path()).is_err());
    }
    assert!(mutation_calls(&runner).is_empty());
}

#[test]
fn claude_marketplace_reads_repo_for_github_and_url_for_git() {
    let cwd = tempfile::tempdir().unwrap();
    for entry in [
        json!({"name": "portone", "source": "github", "repo": "portone-io/portone-cli"}),
        json!({
            "name": "portone", "source": "git",
            "url": "https://github.com/portone-io/portone-cli.git"
        }),
        json!({
            "name": "portone", "source": "git",
            "url": "ssh://git@github.com/portone-io/portone-cli.git"
        }),
        json!({
            "name": "portone", "source": "git",
            "url": "git@github.com:portone-io/portone-cli.git"
        }),
    ] {
        let runner = MockRunner::new()
            .with_output(CLAUDE_MARKETPLACES, &json!([entry]).to_string())
            .with_output(CLAUDE_PLUGINS, "[]");

        let installation = inspect(&runner, Assistant::Claude, cwd.path()).unwrap();

        assert!(installation.marketplace_exists);
        assert!(mutation_calls(&runner).is_empty());
    }

    for entry in [
        json!({"name": "portone", "source": "git", "repo": "portone-io/portone-cli"}),
        json!({
            "name": "portone", "source": "git", "repo": "portone-io/portone-cli",
            "url": "https://github.com/someone/another-plugin.git"
        }),
    ] {
        let runner = MockRunner::new()
            .with_output(CLAUDE_MARKETPLACES, &json!([entry]).to_string())
            .with_output(CLAUDE_PLUGINS, "[]");

        assert!(inspect(&runner, Assistant::Claude, cwd.path()).is_err());
        assert!(mutation_calls(&runner).is_empty());
    }
}

#[test]
fn repository_matching_rejects_lookalike_hosts_and_repository_suffixes() {
    for repository in [
        "portone-io/portone-cli",
        "https://github.com/portone-io/portone-cli.git",
        "git@github.com:portone-io/portone-cli.git",
    ] {
        assert!(expected_repository(repository), "{repository}");
    }
    for repository in [
        "https://evil.example/portone-io/portone-cli.git",
        "https://github.com/portone-io/portone-cli-malicious",
        "portone-io/portone-cli-malicious",
        "someone/portone-cli",
    ] {
        assert!(!expected_repository(repository), "{repository}");
    }
}

#[test]
fn invalid_marketplace_and_plugin_lists_are_errors_not_absent_installs() {
    let cwd = tempfile::tempdir().unwrap();
    for assistant in [Assistant::Claude, Assistant::Codex] {
        let (marketplaces, plugins, registered) = match assistant {
            Assistant::Claude => (
                CLAUDE_MARKETPLACES,
                CLAUDE_PLUGINS,
                claude_marketplace("portone-io/portone-cli"),
            ),
            Assistant::Codex => (
                CODEX_MARKETPLACES,
                CODEX_PLUGINS,
                codex_marketplace("portone-io/portone-cli"),
            ),
        };
        for invalid in ["not JSON", "null", r#"{"unexpected":[]}"#] {
            let runner = MockRunner::new().with_output(marketplaces, invalid);
            assert!(inspect(&runner, assistant, cwd.path()).is_err());
            assert!(mutation_calls(&runner).is_empty());

            let runner = MockRunner::new()
                .with_output(marketplaces, &registered)
                .with_output(plugins, invalid);
            assert!(inspect(&runner, assistant, cwd.path()).is_err());
            assert!(mutation_calls(&runner).is_empty());
        }
    }
}

#[test]
fn preflight_checks_every_selected_assistant_before_installation() {
    let cwd = tempfile::tempdir().unwrap();
    let assistants = [Assistant::Claude, Assistant::Codex];
    let runner = capable(MockRunner::new(), &assistants)
        .with_output(CLAUDE_MARKETPLACES, "[]")
        .with_output(CLAUDE_PLUGINS, "[]")
        .fail_on("codex plugin add --help");

    assert!(preflight(&runner, &assistants, cwd.path()).is_err());

    assert!(mutation_calls(&runner).is_empty());
    assert!(
        runner
            .calls
            .borrow()
            .iter()
            .any(|call| call == "capture:codex plugin add --help")
    );
}

#[test]
fn missing_runtime_or_assistant_stops_preflight_without_changes() {
    let cwd = tempfile::tempdir().unwrap();
    for command in [
        "git --version",
        "node --version",
        "npx --version",
        "claude --version",
    ] {
        let runner = capable(MockRunner::new(), &[Assistant::Claude]).fail_on(command);

        assert!(preflight(&runner, &[Assistant::Claude], cwd.path()).is_err());
        assert!(mutation_calls(&runner).is_empty());
    }
}

#[test]
fn claude_enables_disabled_plugin_and_requires_confirmed_activation() {
    let installed = bundle();
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(
            CLAUDE_MARKETPLACES,
            &claude_marketplace("portone-io/portone-cli"),
        )
        .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), false))
        .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), false))
        .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), true));

    inspect(&runner, Assistant::Claude, cwd.path())
        .unwrap()
        .apply(&runner, cwd.path())
        .unwrap();

    assert_eq!(
        mutation_calls(&runner).last().unwrap(),
        "claude plugin enable portone-integration@portone --scope user"
    );

    let runner = MockRunner::new()
        .with_output(
            CLAUDE_MARKETPLACES,
            &claude_marketplace("portone-io/portone-cli"),
        )
        .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), false));
    assert!(
        inspect(&runner, Assistant::Claude, cwd.path())
            .unwrap()
            .apply(&runner, cwd.path())
            .is_err()
    );
}

#[test]
fn disabled_codex_plugin_is_not_reported_as_completed() {
    let installed = bundle();
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(
            CODEX_MARKETPLACES,
            &codex_marketplace("portone-io/portone-cli"),
        )
        .with_output(CODEX_PLUGINS, &codex_installed(false))
        .with_output(
            CODEX_ADD,
            &json!({"installedPath": installed.path()}).to_string(),
        );

    let result = inspect(&runner, Assistant::Codex, cwd.path())
        .unwrap()
        .apply(&runner, cwd.path());

    let error = result.unwrap_err();
    assert!(
        crate::i18n::Localizer::english()
            .format_error(&error)
            .contains("/plugins")
    );
    assert!(
        !mutation_calls(&runner)
            .iter()
            .any(|command| command.contains(" enable "))
    );
}

#[test]
fn successful_command_with_missing_post_install_state_is_an_error() {
    let installed = bundle();
    let cwd = tempfile::tempdir().unwrap();
    for assistant in [Assistant::Claude, Assistant::Codex] {
        let runner = MockRunner::new()
            .with_output(CLAUDE_MARKETPLACES, "[]")
            .with_output(CLAUDE_PLUGINS, "[]")
            .with_output(CODEX_MARKETPLACES, r#"{"marketplaces":[]}"#)
            .with_output(CODEX_PLUGINS, r#"{"installed":[]}"#)
            .with_output(
                CODEX_ADD,
                &json!({"installedPath": installed.path()}).to_string(),
            );
        assert!(
            inspect(&runner, assistant, cwd.path())
                .unwrap()
                .apply(&runner, cwd.path())
                .is_err()
        );
    }
}

#[test]
fn external_install_failure_is_propagated_without_post_install_success() {
    let cwd = tempfile::tempdir().unwrap();
    let runner = MockRunner::new()
        .with_output(CLAUDE_MARKETPLACES, "[]")
        .with_output(CLAUDE_PLUGINS, "[]")
        .fail_on("claude plugin install portone-integration@portone --scope user");

    let result = inspect(&runner, Assistant::Claude, cwd.path())
        .unwrap()
        .apply(&runner, cwd.path());

    assert!(result.is_err());
    assert_eq!(
        runner.calls.borrow().last().unwrap(),
        "capture:claude plugin install portone-integration@portone --scope user"
    );
}

#[test]
fn bundle_requires_cli_skill_and_expected_mcp_configuration() {
    for assistant in [Assistant::Claude, Assistant::Codex] {
        for missing in [".mcp.json", "skills/portone-cli/SKILL.md"] {
            let installed = bundle();
            verify_bundle(assistant, installed.path()).unwrap();
            fs::remove_file(installed.path().join(missing)).unwrap();
            assert!(verify_bundle(assistant, installed.path()).is_err());
        }
        let installed = bundle();
        fs::write(
            installed.path().join(".mcp.json"),
            r#"{"mcpServers":{"portone":{"command":"different","args":[]}}}"#,
        )
        .unwrap();
        assert!(verify_bundle(assistant, installed.path()).is_err());
    }
}

#[test]
fn codex_bundle_manifest_must_reference_bundled_skills_and_mcp() {
    for (skills, mcp) in [
        ("./missing-skills/", "./.mcp.json"),
        ("./skills/", "./missing.json"),
    ] {
        let installed = bundle();
        fs::write(
            installed.path().join(".codex-plugin/plugin.json"),
            json!({"name":"portone-codex", "skills": skills, "mcpServers": mcp}).to_string(),
        )
        .unwrap();
        assert!(verify_bundle(Assistant::Codex, installed.path()).is_err());
    }
}

#[test]
fn published_bundles_satisfy_setup_and_claude_agents_can_read_mcp_docs() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (assistant, directory) in [
        (Assistant::Claude, "portone-integration"),
        (Assistant::Codex, "portone-codex"),
    ] {
        verify_bundle(assistant, &repository.join("plugins").join(directory)).unwrap();
    }
    for agent in ["payment-code-generator", "integration-validator"] {
        let content = fs::read_to_string(
            repository.join(format!("plugins/portone-integration/agents/{agent}.md")),
        )
        .unwrap();
        let tools: Vec<String> = serde_json::from_str(
            content
                .lines()
                .find_map(|line| line.strip_prefix("tools: "))
                .unwrap(),
        )
        .unwrap();
        for tool in [
            "listPortoneDocs",
            "readPortoneDoc",
            "readPortoneOpenapiSchema",
        ] {
            assert!(
                tools.contains(&format!("mcp__plugin_portone-integration_portone__{tool}")),
                "{agent} cannot use {tool} from the bundled server"
            );
        }
        assert!(!tools.iter().any(|tool| tool.starts_with("mcp__portone__")));
    }
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

#[test]
fn setup_preserves_project_files_without_git_cleanliness_checks() {
    let installed = bundle();
    for git_project in [false, true] {
        let cwd = tempfile::tempdir().unwrap();
        fs::write(cwd.path().join("changed.txt"), "uncommitted user changes").unwrap();
        if git_project {
            fs::create_dir(cwd.path().join(".git")).unwrap();
            fs::write(cwd.path().join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        }
        let before = files(cwd.path());
        let assistants = [Assistant::Claude, Assistant::Codex];
        let runner = capable(MockRunner::new(), &assistants)
            .with_output(
                CLAUDE_MARKETPLACES,
                &claude_marketplace("portone-io/portone-cli"),
            )
            .with_output(CLAUDE_PLUGINS, &claude_installed(installed.path(), true))
            .with_output(
                CODEX_MARKETPLACES,
                &codex_marketplace("portone-io/portone-cli"),
            )
            .with_output(CODEX_PLUGINS, &codex_installed(true))
            .with_output(
                CODEX_ADD,
                &json!({"installedPath": installed.path()}).to_string(),
            );

        for installation in preflight(&runner, &assistants, cwd.path()).unwrap() {
            installation.apply(&runner, cwd.path()).unwrap();
        }

        assert_eq!(before, files(cwd.path()));
        assert!(!cwd.path().join("plugins").exists());
        assert!(!cwd.path().join(".agents").exists());
        assert!(
            runner
                .calls
                .borrow()
                .iter()
                .filter(|call| call.starts_with("capture:git "))
                .all(|call| call == "capture:git --version")
        );
    }
}
