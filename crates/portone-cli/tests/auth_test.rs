use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use serde_json::json;

fn portone(config_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("portone").expect("failed to build portone binary");
    cmd.env_remove("PORTONE_ACCESS_TOKEN")
        .env_remove("PORTONE_API_BASE")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("PORTONE_PAGER")
        .env_remove("PAGER")
        .env("NO_COLOR", "1")
        .env("PORTONE_CONFIG_DIR", config_dir);
    cmd
}

fn write_oauth_config(dir: &std::path::Path, base_url: &str, refresh_token: &str) {
    write_config(
        dir,
        &format!(
            "default_profile = \"default\"\n\n[profiles.default]\nbase_url = \"{base_url}\"\n\n[profiles.default.oauth]\nstorage = \"file\"\nclient_id = \"CLI\"\ntoken_url = \"{base_url}/oauth/token\"\nconsole_url = \"https://console.example\"\n\n[profiles.default.oauth.tokens]\naccess_token = \"access-1\"\nrefresh_token = \"{refresh_token}\"\nexpires_at = 4102444800\nscope = [\"TX_READ\", \"STORE_READ\"]\ntoken_type = \"Bearer\"\n"
        ),
    );
}

fn mock_probe<'a>(server: &'a MockServer, typename: &str) -> httpmock::Mock<'a> {
    let typename = typename.to_string();
    server.mock(move |when, then| {
        when.method(POST)
            .path("/graphql")
            .header("authorization", "Bearer access-1");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "data": { "merchant": { "__typename": typename, "plainId": "merchant-1" } }
            }));
    })
}

fn write_config(dir: &std::path::Path, contents: &str) {
    std::fs::write(dir.join("config.toml"), contents).unwrap();
}

#[test]
fn logout_removes_profile_from_config() {
    let dir = tempfile::tempdir().unwrap();
    write_config(
        dir.path(),
        "default_profile = \"default\"\n\n[profiles.default]\nbase_url = \"https://api.example\"\n",
    );

    portone(dir.path())
        .arg("auth")
        .arg("logout")
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed profile 'default'."));

    let contents = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(!contents.contains("api.example"), "contents: {contents}");
    assert!(
        !contents.contains("default_profile"),
        "default profile setting should also be removed: {contents}"
    );

    portone(dir.path())
        .arg("auth")
        .arg("logout")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("does not exist"));
}

fn refresh_jwt(exp: u64) -> String {
    use base64::Engine;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(format!(r#"{{"exp":{exp},"user_id":"u"}}"#));
    format!("eyJhbGciOiJFUzI1NiJ9.{payload}.sig")
}

#[test]
fn status_shows_oauth_profile() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    let probe = mock_probe(&server, "Merchant");
    write_oauth_config(dir.path(), &server.base_url(), &refresh_jwt(1_900_000_000));

    portone(dir.path())
        .arg("auth")
        .arg("status")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Authentication: Console OAuth")
                .and(predicate::str::contains("Source: config profile 'default'"))
                .and(predicate::str::contains("Access token: acce****"))
                .and(predicate::str::contains("Expires: 2100-01-01T00:00:00Z"))
                .and(predicate::str::contains(
                    "Session expires: 2030-03-17T17:46:40Z",
                ))
                .and(predicate::str::contains("Scopes: TX_READ, STORE_READ"))
                .and(predicate::str::contains(
                    "Issued by: CLI @ https://console.example",
                ))
                .and(predicate::str::contains(
                    "Validation: valid (merchant merchant-1)",
                ))
                .and(predicate::str::contains("access-1").not()),
        );
    probe.assert();
}

#[test]
fn status_reports_invalid_console_token() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    mock_probe(&server, "UnauthorizedError");
    write_oauth_config(dir.path(), &server.base_url(), "refresh-1");

    portone(dir.path())
        .arg("auth")
        .arg("status")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Validation: invalid"))
        .stderr(predicate::str::contains("`portone auth login`"));
}

#[test]
fn token_prints_access_token() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    write_oauth_config(dir.path(), &server.base_url(), "refresh-1");

    portone(dir.path())
        .arg("auth")
        .arg("token")
        .assert()
        .success()
        .stdout(predicate::eq("access-1\n"));
}

#[test]
fn logout_removes_oauth_profile() {
    let dir = tempfile::tempdir().unwrap();
    write_oauth_config(dir.path(), "https://api.example", "refresh-1");

    portone(dir.path())
        .arg("auth")
        .arg("logout")
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed profile 'default'."));

    let contents = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(!contents.contains("access-1"), "contents: {contents}");
    assert!(!contents.contains("oauth"), "contents: {contents}");
}

#[test]
fn login_refuses_when_env_access_token_set() {
    let dir = tempfile::tempdir().unwrap();
    portone(dir.path())
        .env("PORTONE_ACCESS_TOKEN", "env-token")
        .arg("auth")
        .arg("login")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "PORTONE_ACCESS_TOKEN environment variable",
        ));
    assert!(!dir.path().join("config.toml").exists());
}

#[test]
fn logout_refuses_when_env_token_set() {
    let dir = tempfile::tempdir().unwrap();
    write_oauth_config(dir.path(), "https://api.example", "refresh-1");
    portone(dir.path())
        .env("PORTONE_ACCESS_TOKEN", "env-token")
        .arg("auth")
        .arg("logout")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "PORTONE_ACCESS_TOKEN environment variable",
        ));
    let contents = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
    assert!(
        contents.contains("access-1"),
        "profile should be preserved: {contents}"
    );
}
