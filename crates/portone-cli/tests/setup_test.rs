use assert_cmd::Command;
use predicates::prelude::*;

fn portone() -> Command {
    let mut command = Command::cargo_bin("portone").expect("portone binary not found");
    command.env("PORTONE_LANG", "en").env("NO_COLOR", "1");
    command
}

#[test]
fn setup_requires_an_explicit_assistant_without_a_terminal_and_hides_legacy_flag() {
    portone()
        .args(["setup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--assistant"))
        .stdout(predicate::str::contains("--allow-dirty").not());
    portone()
        .args(["setup", "--allow-dirty"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "--assistant is required in non-interactive environments",
        ));
}

#[cfg(unix)]
mod native_hosts {
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    // All state lives beside the project. The fake host rejects unknown commands,
    // so invoking an old host installer or a Git worktree check fails the test.
    const HOST: &str = r#"#!/bin/sh
name=${0##*/}
printf '%s %s\n' "$name" "$*" >> "$SETUP_FIXTURE/commands"
case "$*" in
  --version)
    case "$name" in
      claude) printf '%s\n' '2.0.0 (Claude Code)' ;;
      codex) printf '%s\n' 'codex-cli 1.0.0' ;;
      *) printf '%s\n' '1.0.0' ;;
    esac
    exit 0 ;;
  *--help)
    if [ "$SETUP_MISSING_CAPABILITY" = "$name" ]; then
      printf '%s\n' 'plugin management is unavailable'
    else
      printf '%s\n' '--scope --json --marketplace'
    fi
    exit 0 ;;
esac
case "$name $*" in
  'claude plugin marketplace list --json')
    if [ -f "$SETUP_FIXTURE/claude.marketplace" ]; then
      printf '%s\n' '[{"name":"portone","source":"github","repo":"portone-io/portone-cli"}]'
    else printf '%s\n' '[]'; fi ;;
  'codex plugin marketplace list --json')
    if [ -f "$SETUP_FIXTURE/codex.marketplace" ]; then
      printf '%s\n' '{"marketplaces":[{"name":"portone","marketplaceSource":{"sourceType":"git","source":"https://github.com/portone-io/portone-cli.git"}}]}'
    else printf '%s\n' '{"marketplaces":[]}'; fi ;;
  'claude plugin list --json'|'codex plugin list --marketplace portone --json')
    if [ -f "$SETUP_FIXTURE/$name.installed" ]; then
      /bin/cat "$SETUP_FIXTURE/$name.json"
    elif [ "$name" = claude ]; then printf '%s\n' '[]'
    else printf '%s\n' '{"installed":[]}'; fi ;;
  'claude plugin marketplace add portone-io/portone-cli --scope user'|'codex plugin marketplace add portone-io/portone-cli --json')
    : > "$SETUP_FIXTURE/$name.marketplace"
    printf '%s\n' '{}' ;;
  'claude plugin marketplace update portone'|'codex plugin marketplace upgrade portone --json')
    printf '%s\n' '{"errors":[]}' ;;
  'claude plugin install portone-integration@portone --scope user'|'claude plugin update portone-integration@portone --scope user'|'codex plugin add portone-codex@portone --json')
    if [ "$SETUP_FAIL" = "$name" ]; then
      printf '%s\n' "$name fixture install failed" >&2
      exit 1
    fi
    : > "$SETUP_FIXTURE/$name.installed"
    /bin/cat "$SETUP_FIXTURE/install.json" ;;
  *) printf '%s\n' "unexpected fixture command: $name $*" >&2; exit 2 ;;
esac
"#;

    struct Fixture {
        root: TempDir,
        project: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let project = root.path().join("project with spaces");
            let bin = root.path().join("bin");
            let bundle = root.path().join("bundle");
            fs::create_dir(&project).unwrap();
            fs::create_dir(&bin).unwrap();
            fs::create_dir_all(bundle.join("skills/portone-cli")).unwrap();
            fs::create_dir(bundle.join(".codex-plugin")).unwrap();
            fs::write(project.join("invoice.txt"), "keep my work\n").unwrap();
            fs::write(bundle.join("skills/portone-cli/SKILL.md"), "CLI skill\n").unwrap();
            fs::write(
                bundle.join(".mcp.json"),
                json!({"mcpServers": {"portone": {
                    "command": "npx", "args": ["-y", "@portone/mcp-server@latest"]
                }}})
                .to_string(),
            )
            .unwrap();
            fs::write(
                bundle.join(".codex-plugin/plugin.json"),
                json!({"skills": "./skills/", "mcpServers": "./.mcp.json"}).to_string(),
            )
            .unwrap();
            for (file, value) in [
                (
                    "claude.json",
                    json!([{"id": "portone-integration@portone", "scope": "user",
                        "enabled": true, "installPath": bundle}]),
                ),
                (
                    "codex.json",
                    json!({"installed": [{"name": "portone-codex", "marketplaceName": "portone",
                        "installed": true, "enabled": true}]}),
                ),
                ("install.json", json!({"installedPath": bundle})),
            ] {
                fs::write(root.path().join(file), value.to_string()).unwrap();
            }
            for name in ["git", "node", "npx", "claude", "codex"] {
                let executable = bin.join(name);
                fs::write(&executable, HOST).unwrap();
                fs::set_permissions(executable, fs::Permissions::from_mode(0o755)).unwrap();
            }
            Self { root, project }
        }

        fn command(&self) -> Command {
            let mut command = portone();
            command
                .current_dir(&self.project)
                .env("PORTONE_CONFIG_DIR", self.root.path().join("config"))
                .env("SETUP_FIXTURE", self.root.path())
                .env_remove("SETUP_FAIL")
                .env_remove("SETUP_MISSING_CAPABILITY")
                .env(
                    "PATH",
                    std::env::join_paths([self.root.path().join("bin"), PathBuf::from("/bin")])
                        .unwrap(),
                )
                .args(["setup", "--assistant", "both"]);
            command
        }

        fn installed(&self, host: &str) -> bool {
            self.root.path().join(format!("{host}.installed")).exists()
        }
    }

    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn visit(root: &Path, current: &Path, files: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in fs::read_dir(current).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    files.insert(relative, None);
                    visit(root, &path, files);
                } else {
                    files.insert(relative, Some(fs::read(path).unwrap()));
                }
            }
        }
        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    #[test]
    fn setup_and_refresh_preserve_nongit_and_dirty_project_files() {
        for dirty_repository in [false, true] {
            let fixture = Fixture::new();
            if dirty_repository {
                assert!(
                    std::process::Command::new("git")
                        .args(["init", "--quiet"])
                        .current_dir(&fixture.project)
                        .status()
                        .unwrap()
                        .success()
                );
            }
            let before = snapshot(&fixture.project);
            fixture
                .command()
                .assert()
                .success()
                .stdout(predicate::str::contains("Plugin setup complete"))
                .stdout(predicate::str::contains("[Claude Code]"))
                .stdout(predicate::str::contains("[Codex]"));
            fixture.command().arg("--allow-dirty").assert().success();
            assert!(fixture.installed("claude"));
            assert!(fixture.installed("codex"));
            assert_eq!(snapshot(&fixture.project), before);
            let calls = fs::read_to_string(fixture.root.path().join("commands")).unwrap();
            assert!(calls.contains("claude plugin marketplace update portone\n"));
            assert!(calls.contains("codex plugin marketplace upgrade portone --json\n"));
        }
    }

    #[test]
    fn setup_continues_after_either_host_fails_and_retains_the_other_installation() {
        for (failed, successful, success_heading, failure_heading) in [
            ("claude", "codex", "[Codex]", "[Claude Code]"),
            ("codex", "claude", "[Claude Code]", "[Codex]"),
        ] {
            let fixture = Fixture::new();
            fixture
                .command()
                .env("SETUP_FAIL", failed)
                .assert()
                .code(1)
                .stderr(predicate::str::contains(format!(
                    "{failed} fixture install failed"
                )))
                .stderr(predicate::str::contains(
                    "Successful installations were kept",
                ))
                .stdout(predicate::str::contains(success_heading))
                .stdout(predicate::str::contains(failure_heading).not())
                .stdout(predicate::str::contains("Plugin setup complete").not());
            assert!(fixture.installed(successful));
            assert!(!fixture.installed(failed));
        }
    }

    #[test]
    fn a_late_preflight_failure_leaves_both_hosts_unchanged() {
        let fixture = Fixture::new();
        fixture
            .command()
            .env("SETUP_MISSING_CAPABILITY", "codex")
            .assert()
            .code(1)
            .stderr(predicate::str::contains("Install or update Codex"))
            .stdout(predicate::str::contains("Plugin setup complete").not());
        for host in ["claude", "codex"] {
            assert!(!fixture.installed(host));
            assert!(
                !fixture
                    .root
                    .path()
                    .join(format!("{host}.marketplace"))
                    .exists()
            );
        }
    }
}
