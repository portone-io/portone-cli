use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assistant {
    Claude,
    Codex,
}

pub struct AssistantDefinition {
    pub display_name: &'static str,
    pub version_command: &'static str,
    pub setup_url: &'static str,
    pub capabilities: &'static [(&'static str, &'static [&'static str])],
    validate: fn(&str) -> bool,
}

impl AssistantDefinition {
    pub fn validate_version_output(&self, output: &str) -> bool {
        (self.validate)(output)
    }
}

static CLAUDE: AssistantDefinition = AssistantDefinition {
    display_name: "Claude Code",
    version_command: "claude --version",
    setup_url: "https://code.claude.com/docs/en/setup",
    capabilities: &[
        ("claude plugin marketplace list --help", &["--json"]),
        ("claude plugin marketplace add --help", &["--scope"]),
        ("claude plugin marketplace update --help", &[]),
        ("claude plugin list --help", &["--json"]),
        ("claude plugin install --help", &["--scope"]),
        ("claude plugin update --help", &["--scope"]),
        ("claude plugin enable --help", &["--scope"]),
    ],
    validate: |output| output.contains("Claude Code"),
};

static CODEX: AssistantDefinition = AssistantDefinition {
    display_name: "Codex",
    version_command: "codex --version",
    setup_url: "https://developers.openai.com/codex/cli",
    capabilities: &[
        ("codex plugin marketplace list --help", &["--json"]),
        ("codex plugin marketplace add --help", &["--json"]),
        ("codex plugin marketplace upgrade --help", &["--json"]),
        ("codex plugin list --help", &["--json", "--marketplace"]),
        ("codex plugin add --help", &["--json"]),
    ],
    validate: |output| output.to_lowercase().contains("codex"),
};

impl Assistant {
    pub fn definition(self) -> &'static AssistantDefinition {
        match self {
            Assistant::Claude => &CLAUDE,
            Assistant::Codex => &CODEX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantSelection {
    Claude,
    Codex,
    Both,
}

impl FromStr for AssistantSelection {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "claude" => Ok(AssistantSelection::Claude),
            "codex" => Ok(AssistantSelection::Codex),
            "both" => Ok(AssistantSelection::Both),
            _ => Err(()),
        }
    }
}

pub fn resolve_targets(selection: AssistantSelection) -> Vec<Assistant> {
    match selection {
        AssistantSelection::Claude => vec![Assistant::Claude],
        AssistantSelection::Codex => vec![Assistant::Codex],
        AssistantSelection::Both => vec![Assistant::Claude, Assistant::Codex],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_parses_supported_values() {
        assert_eq!("claude".parse(), Ok(AssistantSelection::Claude));
        assert_eq!("codex".parse(), Ok(AssistantSelection::Codex));
        assert_eq!("both".parse(), Ok(AssistantSelection::Both));
    }

    #[test]
    fn selection_rejects_unknown_values() {
        assert_eq!("gemini".parse::<AssistantSelection>(), Err(()));
        assert_eq!("Claude".parse::<AssistantSelection>(), Err(()));
        assert_eq!("".parse::<AssistantSelection>(), Err(()));
    }

    #[test]
    fn resolve_targets_expands_both() {
        assert_eq!(
            resolve_targets(AssistantSelection::Both),
            vec![Assistant::Claude, Assistant::Codex]
        );
        assert_eq!(
            resolve_targets(AssistantSelection::Claude),
            vec![Assistant::Claude]
        );
        assert_eq!(
            resolve_targets(AssistantSelection::Codex),
            vec![Assistant::Codex]
        );
    }

    #[test]
    fn version_output_validation() {
        let claude = Assistant::Claude.definition();
        assert!(claude.validate_version_output("1.0.0 (Claude Code)"));
        assert!(!claude.validate_version_output("claude 1.0.0"));

        let codex = Assistant::Codex.definition();
        assert!(codex.validate_version_output("Codex CLI 0.1.0"));
        assert!(codex.validate_version_output("codex-cli 0.1.0"));
        assert!(!codex.validate_version_output("command not found"));
    }
}
