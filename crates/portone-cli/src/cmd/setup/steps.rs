use std::path::Path;
use std::process::Command;

use anyhow::Context;

use super::assistants::Assistant;

pub trait CommandRunner {
    fn run_capture(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String>;
    fn run_capture_stdout(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        self.run_capture(cmd, cwd)
    }
    fn run_inherit(&self, cmd: &str, cwd: &Path) -> anyhow::Result<()>;
}

pub struct ShellRunner;

fn shell_command(cmd: &str) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(cmd);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("sh");
        command.arg("-c").arg(cmd);
        command
    }
}

impl CommandRunner for ShellRunner {
    fn run_capture(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        let output = shell_command(cmd)
            .current_dir(cwd)
            .output()
            .with_context(|| format!("failed to run command: {cmd}"))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            anyhow::bail!("command failed: {cmd}\n{}", combined.trim_end());
        }
        Ok(combined)
    }

    fn run_capture_stdout(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        let output = shell_command(cmd)
            .current_dir(cwd)
            .output()
            .with_context(|| format!("failed to run command: {cmd}"))?;
        if !output.status.success() {
            anyhow::bail!(
                "command failed: {cmd}\n{}",
                String::from_utf8_lossy(&output.stderr).trim_end()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn run_inherit(&self, cmd: &str, cwd: &Path) -> anyhow::Result<()> {
        let status = shell_command(cmd)
            .current_dir(cwd)
            .status()
            .with_context(|| format!("failed to run command: {cmd}"))?;
        if !status.success() {
            anyhow::bail!("command failed: {cmd}");
        }
        Ok(())
    }
}

pub fn is_git_clean(runner: &dyn CommandRunner, cwd: &Path) -> bool {
    match runner.run_capture_stdout("git status --porcelain", cwd) {
        Ok(output) => output.trim().is_empty(),
        Err(_) => true,
    }
}

pub fn check_assistant_installed(
    runner: &dyn CommandRunner,
    assistant: Assistant,
    cwd: &Path,
) -> bool {
    let definition = assistant.definition();
    match runner.run_capture(definition.version_command, cwd) {
        Ok(output) => definition.validate_version_output(&output),
        Err(_) => false,
    }
}

pub fn install_assistant(
    runner: &dyn CommandRunner,
    assistant: Assistant,
    cwd: &Path,
) -> anyhow::Result<()> {
    runner.run_inherit(assistant.definition().install_command, cwd)
}

pub fn update_assistant(
    runner: &dyn CommandRunner,
    assistant: Assistant,
    cwd: &Path,
) -> anyhow::Result<()> {
    match assistant.definition().update_command {
        Some(command) => runner.run_inherit(command, cwd),
        None => Ok(()),
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;

    use super::CommandRunner;

    pub(crate) struct MockRunner {
        pub(crate) calls: RefCell<Vec<String>>,
        fail_commands: Vec<String>,
        outputs: HashMap<String, String>,
    }

    impl MockRunner {
        pub(crate) fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail_commands: Vec::new(),
                outputs: HashMap::new(),
            }
        }

        pub(crate) fn fail_on(mut self, cmd: &str) -> Self {
            self.fail_commands.push(cmd.to_string());
            self
        }

        pub(crate) fn with_output(mut self, cmd: &str, output: &str) -> Self {
            self.outputs.insert(cmd.to_string(), output.to_string());
            self
        }

        fn record(&self, kind: &str, cmd: &str) -> anyhow::Result<()> {
            self.calls.borrow_mut().push(format!("{kind}:{cmd}"));
            if self.fail_commands.iter().any(|c| c == cmd) {
                anyhow::bail!("mock failed: {cmd}");
            }
            Ok(())
        }
    }

    impl CommandRunner for MockRunner {
        fn run_capture(&self, cmd: &str, _cwd: &Path) -> anyhow::Result<String> {
            self.record("capture", cmd)?;
            Ok(self.outputs.get(cmd).cloned().unwrap_or_default())
        }

        fn run_inherit(&self, cmd: &str, _cwd: &Path) -> anyhow::Result<()> {
            self.record("inherit", cmd)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::testing::MockRunner;
    use super::*;

    #[test]
    fn git_clean_when_no_output() {
        let runner = MockRunner::new().with_output("git status --porcelain", "  \n");
        assert!(is_git_clean(&runner, Path::new(".")));
    }

    #[test]
    fn git_dirty_when_changes_listed() {
        let runner = MockRunner::new().with_output("git status --porcelain", " M src/main.rs\n");
        assert!(!is_git_clean(&runner, Path::new(".")));
    }

    #[test]
    fn git_clean_when_command_fails() {
        let runner = MockRunner::new().fail_on("git status --porcelain");
        assert!(is_git_clean(&runner, Path::new(".")));
    }

    #[test]
    fn assistant_installed_requires_valid_output() {
        let runner = MockRunner::new().with_output("claude --version", "1.0.0 (Claude Code)");
        assert!(check_assistant_installed(
            &runner,
            Assistant::Claude,
            Path::new(".")
        ));

        let runner = MockRunner::new().with_output("claude --version", "zsh: command not found");
        assert!(!check_assistant_installed(
            &runner,
            Assistant::Claude,
            Path::new(".")
        ));

        let runner = MockRunner::new().fail_on("codex --version");
        assert!(!check_assistant_installed(
            &runner,
            Assistant::Codex,
            Path::new(".")
        ));
    }

    #[test]
    fn install_and_update_run_registry_commands() {
        let runner = MockRunner::new();
        install_assistant(&runner, Assistant::Claude, Path::new(".")).unwrap();
        update_assistant(&runner, Assistant::Claude, Path::new(".")).unwrap();
        update_assistant(&runner, Assistant::Codex, Path::new(".")).unwrap();
        assert_eq!(
            *runner.calls.borrow(),
            vec![
                "inherit:npm install -g @anthropic-ai/claude-code".to_string(),
                "inherit:claude update".to_string(),
            ]
        );
    }
}
