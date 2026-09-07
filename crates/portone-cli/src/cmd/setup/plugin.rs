use std::path::Path;

use anyhow::Context;
use serde_json::{Map, Value, json};

use super::assets;
use super::assistants::Assistant;
use super::steps::CommandRunner;

const CLAUDE_PLUGIN_NAME: &str = "portone-integration";
const CODEX_PLUGIN_NAME: &str = "portone-codex";
const REPO_MARKETPLACE_NAME: &str = "portone";
const REPO_MARKETPLACE_DISPLAY_NAME: &str = "PortOne Plugins";

pub fn configure(
    runner: &dyn CommandRunner,
    assistant: Assistant,
    project_dir: &Path,
) -> anyhow::Result<()> {
    match assistant {
        Assistant::Claude => configure_claude(runner, project_dir),
        Assistant::Codex => configure_codex(project_dir),
    }
}

pub fn configure_claude(runner: &dyn CommandRunner, cwd: &Path) -> anyhow::Result<()> {
    let _ = runner.run_capture("claude plugin marketplace remove portone", cwd);
    runner.run_capture("claude plugin marketplace add portone-io/portone-cli", cwd)?;
    runner.run_capture(&format!("claude plugin install {CLAUDE_PLUGIN_NAME}"), cwd)?;
    Ok(())
}

pub fn configure_codex(project_dir: &Path) -> anyhow::Result<()> {
    let target_plugin_dir = project_dir.join("plugins").join(CODEX_PLUGIN_NAME);
    assets::extract(&target_plugin_dir)?;

    let marketplace_path = project_dir
        .join(".agents")
        .join("plugins")
        .join("marketplace.json");
    update_codex_marketplace(&marketplace_path)
}

fn update_codex_marketplace(marketplace_path: &Path) -> anyhow::Result<()> {
    let existing = match std::fs::read_to_string(marketplace_path) {
        Ok(raw) => Some(raw),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to read marketplace.json: {}",
                    marketplace_path.display()
                )
            });
        }
    };

    let content = merge_marketplace(existing.as_deref())?;

    if let Some(parent) = marketplace_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(marketplace_path, content).with_context(|| {
        format!(
            "failed to write marketplace.json: {}",
            marketplace_path.display()
        )
    })?;
    Ok(())
}

fn merge_marketplace(existing: Option<&str>) -> anyhow::Result<String> {
    let mut name = Value::String(REPO_MARKETPLACE_NAME.to_string());
    let mut display_name = Value::String(REPO_MARKETPLACE_DISPLAY_NAME.to_string());
    let mut plugins: Vec<Value> = Vec::new();

    if let Some(raw) = existing {
        let parsed: Value =
            serde_json::from_str(raw).context("failed to parse marketplace.json")?;
        if let Some(existing_name) = parsed.get("name").filter(|v| v.is_string()) {
            name = existing_name.clone();
        }
        if let Some(existing_display) = parsed
            .get("interface")
            .filter(|v| v.is_object())
            .and_then(|interface| interface.get("displayName"))
            .filter(|v| !v.is_null())
        {
            display_name = existing_display.clone();
        }
        if let Some(existing_plugins) = parsed.get("plugins").and_then(Value::as_array) {
            plugins = existing_plugins.clone();
        }
    }

    plugins.retain(|plugin| {
        !plugin
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| n == CODEX_PLUGIN_NAME)
    });
    plugins.push(marketplace_entry());

    let mut interface = Map::new();
    interface.insert("displayName".to_string(), display_name);

    let mut marketplace = Map::new();
    marketplace.insert("name".to_string(), name);
    marketplace.insert("interface".to_string(), Value::Object(interface));
    marketplace.insert("plugins".to_string(), Value::Array(plugins));

    let mut content = serde_json::to_string_pretty(&Value::Object(marketplace))?;
    content.push('\n');
    Ok(content)
}

fn marketplace_entry() -> Value {
    json!({
        "name": CODEX_PLUGIN_NAME,
        "source": {
            "source": "local",
            "path": format!("./plugins/{CODEX_PLUGIN_NAME}")
        },
        "policy": {
            "installation": "AVAILABLE",
            "authentication": "ON_INSTALL"
        },
        "category": "Productivity"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::setup::steps::testing::MockRunner;

    #[test]
    fn merge_creates_default_marketplace() {
        let content = merge_marketplace(None).unwrap();
        let expected = r#"{
  "name": "portone",
  "interface": {
    "displayName": "PortOne Plugins"
  },
  "plugins": [
    {
      "name": "portone-codex",
      "source": {
        "source": "local",
        "path": "./plugins/portone-codex"
      },
      "policy": {
        "installation": "AVAILABLE",
        "authentication": "ON_INSTALL"
      },
      "category": "Productivity"
    }
  ]
}
"#;
        assert_eq!(content, expected);
    }

    #[test]
    fn merge_preserves_existing_and_replaces_current_entry() {
        let existing = r#"{
            "name": "custom",
            "interface": { "displayName": "Custom Name" },
            "plugins": [
                { "name": "other-plugin", "category": "Other" },
                { "name": "portone-codex", "stale": true }
            ]
        }"#;
        let merged: Value =
            serde_json::from_str(&merge_marketplace(Some(existing)).unwrap()).unwrap();

        assert_eq!(merged["name"], "custom");
        assert_eq!(merged["interface"]["displayName"], "Custom Name");

        let plugins = merged["plugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0]["name"], "other-plugin");
        assert_eq!(plugins[1]["name"], "portone-codex");
        assert!(plugins[1].get("stale").is_none());
        assert_eq!(plugins[1]["source"]["path"], "./plugins/portone-codex");
    }

    #[test]
    fn merge_applies_defaults_for_missing_fields() {
        let existing = r#"{ "name": 42, "plugins": "invalid" }"#;
        let merged: Value =
            serde_json::from_str(&merge_marketplace(Some(existing)).unwrap()).unwrap();

        assert_eq!(merged["name"], "portone");
        assert_eq!(merged["interface"]["displayName"], "PortOne Plugins");
        assert_eq!(merged["plugins"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn merge_defaults_null_display_name() {
        let existing =
            r#"{ "name": "portone", "interface": { "displayName": null }, "plugins": [] }"#;
        let merged: Value =
            serde_json::from_str(&merge_marketplace(Some(existing)).unwrap()).unwrap();
        assert_eq!(merged["interface"]["displayName"], "PortOne Plugins");
    }

    #[test]
    fn merge_rejects_invalid_json() {
        assert!(merge_marketplace(Some("{ invalid")).is_err());
    }

    #[test]
    fn configure_claude_runs_command_sequence() {
        let runner = MockRunner::new();
        configure_claude(&runner, Path::new(".")).unwrap();
        assert_eq!(
            *runner.calls.borrow(),
            vec![
                "capture:claude plugin marketplace remove portone".to_string(),
                "capture:claude plugin marketplace add portone-io/portone-cli".to_string(),
                "capture:claude plugin install portone-integration".to_string(),
            ]
        );
    }

    #[test]
    fn configure_claude_ignores_remove_failure() {
        let runner = MockRunner::new().fail_on("claude plugin marketplace remove portone");
        configure_claude(&runner, Path::new(".")).unwrap();
        assert_eq!(runner.calls.borrow().len(), 3);
    }

    #[test]
    fn configure_claude_propagates_add_failure() {
        let runner =
            MockRunner::new().fail_on("claude plugin marketplace add portone-io/portone-cli");
        assert!(configure_claude(&runner, Path::new(".")).is_err());
        assert_eq!(runner.calls.borrow().len(), 2);
    }

    #[test]
    fn configure_codex_extracts_assets_and_writes_marketplace() {
        let dir = tempfile::tempdir().unwrap();
        configure_codex(dir.path()).unwrap();

        assert!(
            dir.path()
                .join("plugins/portone-codex/.codex-plugin/plugin.json")
                .is_file()
        );

        let marketplace_path = dir.path().join(".agents/plugins/marketplace.json");
        let raw = std::fs::read_to_string(&marketplace_path).unwrap();
        assert!(raw.ends_with("}\n"));
        let marketplace: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(marketplace["plugins"][0]["name"], "portone-codex");
    }

    #[test]
    fn update_codex_marketplace_merges_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".agents/plugins/marketplace.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{ "name": "keep", "plugins": [{ "name": "other" }] }"#,
        )
        .unwrap();

        update_codex_marketplace(&path).unwrap();

        let merged: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(merged["name"], "keep");
        let names: Vec<&str> = merged["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["other", "portone-codex"]);
    }
}
