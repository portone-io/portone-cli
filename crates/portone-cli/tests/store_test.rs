use assert_cmd::Command;
use predicates::prelude::*;

fn portone(config_dir: &std::path::Path) -> Command {
    let mut command = Command::cargo_bin("portone").unwrap();
    command
        .env("PORTONE_CONFIG_DIR", config_dir)
        .env("PORTONE_LANG", "en")
        .env("NO_COLOR", "1")
        .env("PORTONE_ACCESS_TOKEN", "unused-token")
        .env("PORTONE_API_BASE", "http://127.0.0.1:1")
        .env("PORTONE_STORE_ID", "store-environment")
        .env_remove("CLICOLOR_FORCE");
    command
}

#[test]
fn default_store_can_be_set_viewed_and_removed_offline() {
    let dir = tempfile::tempdir().unwrap();
    portone(dir.path())
        .args(["store", "set-default", "store-local"])
        .assert()
        .success()
        .stdout("")
        .stderr(predicate::str::contains("store-local"));
    portone(dir.path())
        .args(["store", "set-default", "--view"])
        .assert()
        .success()
        .stdout("store-local\n");
    portone(dir.path())
        .args(["store", "set-default", "--unset"])
        .assert()
        .success();
    portone(dir.path())
        .args(["store", "set-default", "--view"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("has no default store"));
}

#[test]
fn explicit_profile_changes_only_its_store_selection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "default_profile = 'work'\nlanguage = 'ko'\n[profiles.work]\nstore_id = 'store-work'\n[profiles.other]\nbase_url = 'https://other.example'\n").unwrap();
    portone(dir.path())
        .args(["store", "set-default", "store-other", "--profile", "other"])
        .assert()
        .success();
    portone(dir.path())
        .args(["store", "set-default", "--view"])
        .assert()
        .success()
        .stdout("store-work\n");
    portone(dir.path())
        .args(["store", "set-default", "--view", "--profile", "other"])
        .assert()
        .success()
        .stdout("store-other\n");
    let config: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(config["default_profile"].as_str(), Some("work"));
    assert_eq!(config["language"].as_str(), Some("ko"));
    assert_eq!(
        config["profiles"]["other"]["base_url"].as_str(),
        Some("https://other.example")
    );
}

#[test]
fn conflicting_options_and_noninteractive_picker_do_not_write_config() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec!["store", "set-default", "store-one", "--unset"],
        vec!["store", "set-default", "store-one", "--view"],
        vec!["store", "set-default", "--unset", "--view"],
    ] {
        portone(dir.path()).args(args).assert().code(2);
    }
    portone(dir.path())
        .args(["store", "set-default"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("store ID is required"));
    assert!(!dir.path().join("config.toml").exists());
}
