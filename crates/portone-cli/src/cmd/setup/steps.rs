use std::path::Path;
use std::process::Command;

use crate::i18n::LocalizedContext;

use super::assistants::Assistant;

pub trait CommandRunner {
    fn run_capture(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String>;
    fn run_capture_stdout(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        self.run_capture(cmd, cwd)
    }
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
            .with_lcontext(|| crate::message!("setup-command-run-failed", command = cmd))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            anyhow::bail!(crate::message!(
                "setup-command-output-failed",
                command = cmd,
                output = combined.trim_end()
            ));
        }
        Ok(combined)
    }

    fn run_capture_stdout(&self, cmd: &str, cwd: &Path) -> anyhow::Result<String> {
        let output = shell_command(cmd)
            .current_dir(cwd)
            .output()
            .with_lcontext(|| crate::message!("setup-command-run-failed", command = cmd))?;
        if !output.status.success() {
            let detail = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            anyhow::bail!(crate::message!(
                "setup-command-output-failed",
                command = cmd,
                output = detail.trim_end()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

pub fn check_runtime(runner: &dyn CommandRunner, cwd: &Path) -> anyhow::Result<()> {
    for (command, requirement) in [
        ("git --version", "Git"),
        ("node --version", "Node.js"),
        ("npx --version", "npx (npm)"),
    ] {
        runner.run_capture_stdout(command, cwd).with_lcontext(|| {
            crate::message!("setup-runtime-required", requirement = requirement)
        })?;
    }
    Ok(())
}

pub fn check_assistant(
    runner: &dyn CommandRunner,
    assistant: Assistant,
    cwd: &Path,
) -> anyhow::Result<()> {
    let definition = assistant.definition();
    let hint = || {
        crate::message!(
            "setup-assistant-required-capabilities",
            assistant = definition.display_name,
            url = definition.setup_url
        )
    };
    let version = runner
        .run_capture_stdout(definition.version_command, cwd)
        .with_lcontext(hint)?;
    if !definition.validate_version_output(&version) {
        anyhow::bail!(hint());
    }
    for &(command, flags) in definition.capabilities {
        let help = runner
            .run_capture_stdout(command, cwd)
            .with_lcontext(hint)?;
        if flags.iter().any(|flag| !help.contains(flag)) {
            anyhow::bail!(hint());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;
    use std::collections::{HashMap, VecDeque};
    use std::path::Path;

    use super::CommandRunner;

    pub(crate) struct MockRunner {
        pub(crate) calls: RefCell<Vec<String>>,
        fail_commands: Vec<String>,
        outputs: RefCell<HashMap<String, VecDeque<String>>>,
    }

    impl MockRunner {
        pub(crate) fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail_commands: Vec::new(),
                outputs: RefCell::new(HashMap::new()),
            }
        }

        pub(crate) fn fail_on(mut self, cmd: &str) -> Self {
            self.fail_commands.push(cmd.to_string());
            self
        }

        pub(crate) fn with_output(self, cmd: &str, output: &str) -> Self {
            self.outputs
                .borrow_mut()
                .entry(cmd.to_string())
                .or_default()
                .push_back(output.to_string());
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
            let mut outputs = self.outputs.borrow_mut();
            let Some(queue) = outputs.get_mut(cmd) else {
                return Ok(String::new());
            };
            // Keep the final response for repeated reads of an unchanged state.
            Ok(if queue.len() > 1 {
                queue.pop_front().unwrap()
            } else {
                queue.front().cloned().unwrap_or_default()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::testing::MockRunner;
    use super::*;
    use crate::i18n::Localizer;

    #[test]
    fn failed_shell_commands_localize_diagnostic_and_preserve_external_output() {
        let runner = ShellRunner;
        let command = "echo external diagnostic 1>&2 && exit 1";
        for error in [
            runner.run_capture(command, Path::new(".")).unwrap_err(),
            runner
                .run_capture_stdout(command, Path::new("."))
                .unwrap_err(),
        ] {
            assert_eq!(
                Localizer::english().format_error(&error),
                format!("command failed: {command}\nexternal diagnostic")
            );
            assert_eq!(
                Localizer::korean().format_error(&error),
                format!("명령 실패: {command}\nexternal diagnostic")
            );
        }
    }

    #[test]
    fn assistant_requires_recognized_version_and_plugin_capabilities() {
        let runner = MockRunner::new().with_output("claude --version", "zsh: command not found");
        assert!(check_assistant(&runner, Assistant::Claude, Path::new(".")).is_err());
        let runner = MockRunner::new().with_output("codex --version", "codex-cli 0.1.0");
        assert!(check_assistant(&runner, Assistant::Codex, Path::new(".")).is_err());
        let runner = MockRunner::new().fail_on("codex --version");
        assert!(check_assistant(&runner, Assistant::Codex, Path::new(".")).is_err());
    }

    #[test]
    fn runtime_errors_explain_the_missing_dependency() {
        let runner = MockRunner::new().fail_on("npx --version");
        let error = check_runtime(&runner, Path::new(".")).unwrap_err();
        assert!(
            Localizer::english()
                .format_error(&error)
                .contains("npx (npm)")
        );
    }

    #[test]
    fn json_capture_excludes_stderr_warnings() {
        let output = ShellRunner
            .run_capture_stdout("echo {} && echo warning 1>&2", Path::new("."))
            .unwrap();
        assert_eq!(output.trim(), "{}");
    }

    #[test]
    fn failed_json_command_preserves_stdout_diagnostics() {
        let error = ShellRunner
            .run_capture_stdout("echo external diagnostic && exit 1", Path::new("."))
            .unwrap_err();
        assert!(
            Localizer::english()
                .format_error(&error)
                .contains("external diagnostic")
        );
    }
}
