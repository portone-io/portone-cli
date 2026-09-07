use std::path::Path;

use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use serde_json::json;

fn portone(config: &Path, cache: &Path) -> Command {
    let mut command = Command::cargo_bin("portone").expect("portone binary not found");
    for name in [
        "PORTONE_LANG",
        "PORTONE_ACCESS_TOKEN",
        "PORTONE_API_BASE",
        "PORTONE_PAGER",
        "PAGER",
        "CLICOLOR_FORCE",
        "LANGUAGE",
        "LC_MESSAGES",
    ] {
        command.env_remove(name);
    }
    command
        .env("LC_ALL", "en_US.UTF-8")
        .env("LANG", "en_US.UTF-8")
        .env("NO_COLOR", "1")
        .env("PORTONE_CONFIG_DIR", config)
        .env("PORTONE_CACHE_DIR", cache);
    command
}

struct Harness {
    config: tempfile::TempDir,
    cache: tempfile::TempDir,
    server: MockServer,
}

impl Harness {
    fn new() -> Self {
        Self {
            config: tempfile::tempdir().unwrap(),
            cache: tempfile::tempdir().unwrap(),
            server: MockServer::start(),
        }
    }

    fn api(&self, language: &str, endpoint: &str) -> Command {
        let mut command = portone(self.config.path(), self.cache.path());
        command
            .env("PORTONE_LANG", language)
            .env("PORTONE_ACCESS_TOKEN", "unchanged-token.payload.signature")
            .args(["api", endpoint, "--base-url", &self.server.base_url()]);
        command
    }
}

#[test]
fn process_override_and_config_select_runtime_language() {
    let config = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for (configured, override_language, locale, korean) in [
        ("ko", None, "en_US.UTF-8", true),
        ("ko", Some("en"), "ko_KR.UTF-8", false),
        ("en", Some("ko"), "en_US.UTF-8", true),
        ("ko", Some(""), "en_US.UTF-8", true),
        ("ko", Some("fr"), "ko_KR.UTF-8", false),
    ] {
        std::fs::write(
            config.path().join("config.toml"),
            format!("language = {configured:?}\n"),
        )
        .unwrap();
        let mut command = portone(config.path(), cache.path());
        command.env("LC_ALL", locale);
        if let Some(language) = override_language {
            command.env("PORTONE_LANG", language);
        }
        // Missing --assistant fails before any assistant discovery or installation.
        let error = if korean {
            "비대화형 환경에서는 --assistant가 필요합니다"
        } else {
            "--assistant is required in non-interactive environments"
        };
        command
            .args(["setup", "--allow-dirty"])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(error));
    }
}

#[test]
fn automatic_language_uses_platform_preferences() {
    let config = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    // Native UI preferences cannot be replaced with POSIX environment variables
    // on macOS/Windows. Read them without changing the user's system settings.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let native_korean = i18n_embed::DesktopLanguageRequester::requested_languages()
        .iter()
        .find_map(|locale| match locale.language.as_str() {
            "ko" => Some(true),
            "en" => Some(false),
            _ => None,
        })
        .unwrap_or(false);

    for (lc_all, lang, language_list, _unix_korean) in [
        ("C.UTF-8", "C.UTF-8", "", false),
        ("en_US.UTF-8", "en_US.UTF-8", "ja:ko:en", true),
        ("ko_KR.UTF-8", "ko_KR.UTF-8", "fr:en:ko", false),
        ("C.UTF-8", "ko_KR.UTF-8", "", true),
        ("fr_FR.UTF-8", "ko_KR.UTF-8", "ja", true),
    ] {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let korean = native_korean;
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let korean = _unix_korean;

        for (configured, override_language) in [
            (None, None),
            (Some("auto"), None),
            (Some(if korean { "en" } else { "ko" }), Some("auto")),
        ] {
            let content = configured
                .map(|language| format!("language = {language:?}\n"))
                .unwrap_or_default();
            std::fs::write(config.path().join("config.toml"), content).unwrap();

            let mut command = portone(config.path(), cache.path());
            command
                .env("LC_ALL", lc_all)
                .env("LANG", lang)
                .env("LANGUAGE", language_list);
            if let Some(language) = override_language {
                command.env("PORTONE_LANG", language);
            }
            command
                .args(["auth", "login", "--help"])
                .assert()
                .success()
                .stdout(predicate::str::contains(if korean {
                    "사용법:"
                } else {
                    "Usage:"
                }));
        }
    }
}

#[test]
fn invalid_config_allows_help_and_fails_when_auth_uses_it() {
    let config = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for content in [
        "access_token = \"private-test-value\"\nbroken = [",
        "language = \"ko\"\nprofiles = 42\n",
    ] {
        std::fs::write(config.path().join("config.toml"), content).unwrap();
        for (language, diagnostic) in [
            ("en", "invalid config file:"),
            ("ko", "잘못된 설정 파일입니다:"),
        ] {
            portone(config.path(), cache.path())
                .env("PORTONE_LANG", language)
                .args(["auth", "token", "--help"])
                .assert()
                .success();
            portone(config.path(), cache.path())
                .env("PORTONE_LANG", language)
                .args(["auth", "token"])
                .assert()
                .code(1)
                .stdout(predicate::str::is_empty())
                .stderr(
                    predicate::str::contains(diagnostic)
                        .and(predicate::str::contains("config.toml"))
                        .and(predicate::str::contains("private-test-value").not()),
                );
        }
    }
}

#[test]
fn api_validation_header_and_field_errors_follow_language_with_same_exit_code() {
    let config = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    for (arguments, english, korean) in [
        (
            vec!["--slurp"],
            "`--paginate` required when passing `--slurp`",
            "`--slurp`를 사용하려면 `--paginate`가 필요합니다",
        ),
        (
            vec!["-H", "X-Test"],
            "header \"X-Test\" requires a value separated by ':'",
            "헤더 \"X-Test\"에는 ':' 뒤에 값이 필요합니다",
        ),
        (
            vec!["-F", "amount"],
            "invalid key: \"amount\"",
            "잘못된 키입니다: \"amount\"",
        ),
    ] {
        for (language, message) in [("en", english), ("ko", korean)] {
            portone(config.path(), cache.path())
                .env("PORTONE_LANG", language)
                .args(["api", "/payments"])
                .args(&arguments)
                .assert()
                .code(1)
                .stdout(predicate::str::is_empty())
                .stderr(format!("portone: {message}\n"));
        }
    }
}

#[test]
fn json_raw_jq_and_token_outputs_are_identical_in_both_languages() {
    let harness = Harness::new();
    let json_body = "{\"message\": \"External message 한국어\",\n  \"status\": \"PAID\"}";
    let json_response = harness.server.mock(|when, then| {
        when.method(GET)
            .path("/json")
            .header("authorization", "Bearer unchanged-token.payload.signature");
        then.status(200)
            .header("content-type", "application/json")
            .body(json_body);
    });
    let raw_body = "Raw external text 한국어\n\0\x1b[31m";
    let raw_response = harness.server.mock(|when, then| {
        when.method(GET).path("/raw");
        then.status(200)
            .header("content-type", "application/octet-stream")
            .body(raw_body);
    });
    for language in ["en", "ko"] {
        for (endpoint, arguments, expected) in [
            ("/json", vec![], json_body),
            (
                "/json",
                vec!["--jq", ".message"],
                "External message 한국어\n",
            ),
            ("/raw", vec![], raw_body),
        ] {
            harness
                .api(language, endpoint)
                .args(arguments)
                .assert()
                .success()
                .stdout(expected)
                .stderr(predicate::str::is_empty());
        }
        portone(harness.config.path(), harness.cache.path())
            .env("PORTONE_LANG", language)
            .env("PORTONE_ACCESS_TOKEN", "unchanged-token.payload.signature")
            .args(["auth", "token"])
            .assert()
            .success()
            .stdout("unchanged-token.payload.signature\n")
            .stderr(predicate::str::is_empty());
    }
    json_response.assert_calls(4);
    raw_response.assert_calls(2);
}

#[test]
fn rest_fields_and_stdin_payloads_are_identical_in_both_languages() {
    let harness = Harness::new();
    let fields = harness.server.mock(|when, then| {
        when.method(POST)
            .path("/typed")
            .header("x-note", "External value")
            .header("authorization", "Bearer unchanged-token.payload.signature")
            .json_body(json!({
                "reason": "Customer request 고객 요청",
                "amount": 42,
                "isTest": true,
                "filter": {"statuses": ["PAID"]}
            }));
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });
    let raw_input = "{\"reason\": \"Customer request 고객 요청\", \"amount\": 42}";
    let stdin = harness.server.mock(|when, then| {
        when.method(POST).path("/input").body(raw_input);
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });
    for language in ["en", "ko"] {
        harness
            .api(language, "/typed")
            .args([
                "-H",
                "X-Note: External value",
                "-f",
                "reason=Customer request 고객 요청",
                "-F",
                "amount=42",
                "-F",
                "isTest=true",
                "-f",
                "filter[statuses][]=PAID",
            ])
            .assert()
            .success()
            .stdout("{}")
            .stderr(predicate::str::is_empty());
        harness
            .api(language, "/input")
            .args(["--input", "-"])
            .write_stdin(raw_input)
            .assert()
            .success()
            .stdout("{}")
            .stderr(predicate::str::is_empty());
    }
    fields.assert_calls(2);
    stdin.assert_calls(2);
}

#[test]
fn graphql_query_variables_and_response_remain_protocol_data() {
    let harness = Harness::new();
    let query = "query ReadNode($id: ID!) { node(id: $id) { id } }";
    let body = r#"{"data":{"node":{"id":"merchant-1","name":"Original name 원문"}}}"#;
    let request = harness.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .header("authorization", "Bearer unchanged-token.payload.signature")
            .json_body(json!({
                "query": query,
                "operationName": "ReadNode",
                "variables": {"id": "merchant-1", "filter": {"locale": "ko"}}
            }));
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });
    for language in ["en", "ko"] {
        harness
            .api(language, "graphql")
            .args(["-f", &format!("query={query}")])
            .args([
                "-f",
                "operationName=ReadNode",
                "-f",
                "id=merchant-1",
                "-f",
                "filter[locale]=ko",
            ])
            .assert()
            .success()
            .stdout(body)
            .stderr(predicate::str::is_empty());
    }
    request.assert_calls(2);
}

#[test]
fn server_error_messages_bodies_and_exit_codes_are_unchanged() {
    let harness = Harness::new();
    let rest_body = r#"{"type":"INVALID_REQUEST","message":"Original rejection 원문"}"#;
    let graphql_body = r#"{"errors":[{"message":"Original GraphQL error 원문"}]}"#;
    let rest = harness.server.mock(|when, then| {
        when.method(GET).path("/rejected");
        then.status(422)
            .header("content-type", "application/json")
            .body(rest_body);
    });
    let graphql = harness.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(graphql_body);
    });
    for language in ["en", "ko"] {
        harness
            .api(language, "/rejected")
            .assert()
            .code(1)
            .stdout(rest_body)
            .stderr("portone: Original rejection 원문 (HTTP 422)\n");
        harness
            .api(language, "graphql")
            .args(["-f", "query=query { x }"])
            .assert()
            .code(1)
            .stdout(graphql_body)
            .stderr("portone: Original GraphQL error 원문\n");
    }
    rest.assert_calls(2);
    graphql.assert_calls(2);
}

#[test]
fn saving_config_preserves_language_preference() {
    let config = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let path = config.path().join("config.toml");
    std::fs::write(
        &path,
        "language = \"ko\"\ndefault_profile = \"default\"\n[profiles.default]\nbase_url = \"https://api.example\"\n",
    )
    .unwrap();
    portone(config.path(), cache.path())
        .args(["auth", "logout"])
        .assert()
        .success()
        .stderr("'default' 프로필을 삭제했습니다.\n");
    let saved: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved["language"].as_str(), Some("ko"));
    assert!(saved.get("default_profile").is_none());
}
