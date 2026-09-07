use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use serde_json::json;

fn portone(config_dir: &std::path::Path, cache_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("portone").expect("failed to build portone binary");
    cmd.env_remove("PORTONE_ACCESS_TOKEN")
        .env_remove("PORTONE_API_BASE")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("PORTONE_PAGER")
        .env_remove("PAGER")
        .env("PORTONE_LANG", "en")
        .env("NO_COLOR", "1")
        .env("PORTONE_CONFIG_DIR", config_dir)
        .env("PORTONE_CACHE_DIR", cache_dir);
    cmd
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn write_oauth_config(
    dir: &std::path::Path,
    base_url: &str,
    token_url: &str,
    access: &str,
    refresh: &str,
    expires_at: u64,
) {
    std::fs::write(
        dir.join("config.toml"),
        format!(
            "default_profile = \"default\"\n\n[profiles.default]\nbase_url = \"{base_url}\"\n\n[profiles.default.oauth]\nstorage = \"file\"\nclient_id = \"CLI\"\ntoken_url = \"{token_url}\"\nconsole_url = \"https://console.example\"\n\n[profiles.default.oauth.tokens]\naccess_token = \"{access}\"\nrefresh_token = \"{refresh}\"\nexpires_at = {expires_at}\nscope = [\"TX_READ\"]\ntoken_type = \"Bearer\"\n"
        ),
    )
    .unwrap();
}

fn refresh_mock<'a>(
    server: &'a MockServer,
    status: u16,
    body: serde_json::Value,
) -> httpmock::Mock<'a> {
    server.mock(move |when, then| {
        when.method(POST).path("/oauth/token").json_body_includes(
            r#"{"client_id":"CLI","grant_type":"refresh_token","refresh_token":"refresh-1"}"#,
        );
        then.status(status)
            .header("content-type", "application/json")
            .json_body(body.clone());
    })
}

fn new_tokens() -> serde_json::Value {
    json!({
        "access_token": "access-2",
        "token_type": "Bearer",
        "expires_in": 1800,
        "scope": ["TX_READ"],
        "refresh_token": "refresh-2"
    })
}

fn bearer_ok<'a>(server: &'a MockServer, token: &str) -> httpmock::Mock<'a> {
    let header = format!("Bearer {token}");
    server.mock(move |when, then| {
        when.method(GET)
            .path("/payments")
            .header("authorization", header.clone());
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"ok":true}"#);
    })
}

#[test]
fn paginate_refreshes_oauth_between_rest_and_graphql_pages() {
    for graphql in [false, true] {
        let config = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let server = MockServer::start();
        let mut short_lived = new_tokens();
        short_lived["expires_in"] = json!(61);
        let initial_refresh = refresh_mock(&server, 200, short_lived);
        let next_refresh = server.mock(|when, then| {
            when.method(POST).path("/oauth/token").json_body_includes(
                r#"{"grant_type":"refresh_token","refresh_token":"refresh-2"}"#,
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "access_token": "access-3", "refresh_token": "refresh-3",
                    "expires_in": 1800, "token_type": "Bearer", "scope": ["TX_READ"]
                }));
        });
        let path = if graphql { "/graphql" } else { "/payments" };
        let first_page = server.mock(|when, then| {
            when.method(if graphql { POST } else { GET })
                .path(path)
                .header("authorization", "Bearer access-2");
            let body = if graphql {
                json!({"data": {"items": {
                    "nodes": [{"id": "first"}],
                    "pageInfo": {"hasNextPage": true, "endCursor": "cursor-1"}
                }}})
            } else {
                json!({"items": [{"id": "first"}], "page": {"size": 1, "totalCount": 2}})
            };
            then.status(200)
                .header("content-type", "application/json")
                // Move the freshly issued token into the 60-second refresh window.
                .delay(std::time::Duration::from_secs(2))
                .json_body(body);
        });
        let second_page = server.mock(|when, then| {
            let page = if graphql {
                r#"{"variables":{"endCursor":"cursor-1"}}"#
            } else {
                r#"{"page":{"number":1,"size":1}}"#
            };
            when.method(if graphql { POST } else { GET })
                .path(path)
                .header("authorization", "Bearer access-3")
                .json_body_includes(page);
            let body = if graphql {
                json!({"data": {"items": {
                    "nodes": [{"id": "second"}],
                    "pageInfo": {"hasNextPage": false, "endCursor": null}
                }}})
            } else {
                json!({"items": [{"id": "second"}], "page": {"size": 1, "totalCount": 2}})
            };
            then.status(200)
                .header("content-type", "application/json")
                .json_body(body);
        });
        write_oauth_config(
            config.path(),
            &server.base_url(),
            &server.url("/oauth/token"),
            "access-1",
            "refresh-1",
            1,
        );
        let mut cmd = portone(config.path(), cache.path());
        cmd.args([
            "api",
            if graphql { "graphql" } else { "/payments" },
            "--paginate",
            "--slurp",
        ]);
        if graphql {
            cmd.args(["-f", "query=query($endCursor: String) { items(after: $endCursor) { nodes { id } pageInfo { hasNextPage endCursor } } }"]);
        }
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("first").and(predicate::str::contains("second")))
            .stderr(predicate::str::is_empty());
        initial_refresh.assert_calls(1);
        next_refresh.assert_calls(1);
        first_page.assert_calls(1);
        second_page.assert_calls(1);
        let saved = std::fs::read_to_string(config.path().join("config.toml")).unwrap();
        assert!(saved.contains("refresh-3"));
        assert!(!saved.contains("refresh-2"));
    }
}

struct Harness {
    config: tempfile::TempDir,
    cache: tempfile::TempDir,
    server: MockServer,
}

impl Harness {
    fn new() -> Harness {
        Harness {
            config: tempfile::tempdir().unwrap(),
            cache: tempfile::tempdir().unwrap(),
            server: MockServer::start(),
        }
    }

    fn api(&self, endpoint: &str) -> Command {
        let mut cmd = portone(self.config.path(), self.cache.path());
        cmd.arg("api")
            .arg(endpoint)
            .arg("--base-url")
            .arg(self.server.base_url())
            .env("PORTONE_ACCESS_TOKEN", "test-token");
        cmd
    }
}

#[test]
fn long_help_has_examples_but_short_help_does_not() {
    let config = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();

    let long = portone(config.path(), cache.path())
        .args(["api", "--help"])
        .output()
        .unwrap();
    let long = String::from_utf8_lossy(&long.stdout);
    assert!(long.contains("authenticated HTTP request to the PortOne V2 API"));
    assert!(long.contains("$ portone api graphql"));

    let short = portone(config.path(), cache.path())
        .args(["api", "-h"])
        .output()
        .unwrap();
    let short = String::from_utf8_lossy(&short.stdout);
    assert!(short.contains("authenticated PortOne V2 API request"));
    assert!(!short.contains("$ portone api"));
}

#[test]
fn get_json_pipe_output_is_verbatim() {
    let h = Harness::new();
    let body = "{\"id\": \"abc\",\n  \"amount\": [1, 2]}";
    let mock = h.server.mock(|when, then| {
        when.method(GET).path("/payments/abc");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    h.api("/payments/abc")
        .assert()
        .success()
        .stdout(predicate::eq(body))
        .stderr(predicate::str::is_empty());
    mock.assert();
}

#[test]
fn http_422_prints_error_and_body() {
    let h = Harness::new();
    let body = r#"{"type":"INVALID_REQUEST","message":"invalid request"}"#;
    h.server.mock(|when, then| {
        when.method(GET).path("/payments");
        then.status(422)
            .header("content-type", "application/json")
            .body(body);
    });

    h.api("/payments")
        .assert()
        .code(1)
        .stdout(predicate::eq(body))
        .stderr(predicate::str::contains(
            "portone: invalid request (HTTP 422)",
        ));
}

#[test]
fn http_204_prints_nothing() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(DELETE).path("/payments/abc/schedule");
        then.status(204);
    });

    h.api("/payments/abc/schedule")
        .arg("-X")
        .arg("delete")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    mock.assert();
}

#[test]
fn include_prints_status_line_and_sorted_headers() {
    let h = Harness::new();
    h.server.mock(|when, then| {
        when.method(GET).path("/meta");
        then.status(200)
            .header("content-type", "application/json")
            .header("x-zeta", "1")
            .header("x-alpha", "2")
            .body("{}");
    });

    let output = h.api("/meta").arg("-i").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.starts_with("HTTP/1.1 200 OK\n"), "stdout: {stdout}");
    let alpha = stdout
        .find("X-Alpha: 2\r\n")
        .expect("X-Alpha header missing");
    let zeta = stdout.find("X-Zeta: 1\r\n").expect("X-Zeta header missing");
    assert!(alpha < zeta, "headers are not sorted by name: {stdout}");
    assert!(stdout.ends_with("\r\n\r\n{}"), "stdout: {stdout}");
}

#[test]
fn magic_and_raw_fields_build_typed_json_body_even_for_get() {
    let h = Harness::new();
    let note_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(note_file.path(), "hello world").unwrap();

    let mock = h.server.mock(|when, then| {
        when.method(GET)
            .path("/typed")
            .header("content-type", "application/json; charset=utf-8")
            .json_body(json!({
                "raw_num": "42",
                "count": 3,
                "ok": true,
                "none": null,
                "name": "foo",
                "note": "hello world",
            }));
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    h.api("/typed")
        .arg("-X")
        .arg("GET")
        .arg("-f")
        .arg("raw_num=42")
        .arg("-F")
        .arg("count=3")
        .arg("-F")
        .arg("ok=true")
        .arg("-F")
        .arg("none=null")
        .arg("-F")
        .arg("name=foo")
        .arg("-F")
        .arg(format!("note=@{}", note_file.path().display()))
        .assert()
        .success();
    mock.assert();
}

#[test]
fn fields_default_to_post_but_paginate_keeps_get() {
    let h = Harness::new();
    let post = h.server.mock(|when, then| {
        when.method(POST)
            .path("/things")
            .json_body(json!({"a": "b"}));
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });
    h.api("/things").arg("-f").arg("a=b").assert().success();
    post.assert();

    let get = h.server.mock(|when, then| {
        when.method(GET)
            .path("/things")
            .json_body(json!({"page": {"number": 0, "size": 2}}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"items":[{"id":"x"}],"page":{"number":0,"size":2,"totalCount":1}}"#);
    });
    h.api("/things")
        .arg("--paginate")
        .arg("-F")
        .arg("page[number]=0")
        .arg("-F")
        .arg("page[size]=2")
        .assert()
        .success();
    get.assert();
}

#[test]
fn paginate_offset_requests_three_pages() {
    let h = Harness::new();
    let mut mocks = Vec::new();
    for n in 0..3u64 {
        mocks.push(h.server.mock(|when, then| {
            when.method(GET)
                .path("/payments")
                .json_body(json!({"page": {"number": n, "size": 100}}));
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    r#"{{"items":[{{"id":"p{n}"}}],"page":{{"number":{n},"size":100,"totalCount":250}}}}"#
                ));
        }));
    }

    let output = h.api("/payments").arg("--paginate").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    for n in 0..3 {
        assert!(stdout.contains(&format!("p{n}")), "stdout: {stdout}");
    }
    for mock in &mocks {
        mock.assert();
    }
}

#[test]
fn paginate_cursor_passes_cursor_and_stops_on_short_page() {
    let h = Harness::new();
    let first = h.server.mock(|when, then| {
        when.method(GET)
            .path("/transactions")
            .json_body(json!({"size": 2}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"items":[{"id":"t1","cursor":"c1"},{"id":"t2","cursor":"c2"}]}"#);
    });
    let second = h.server.mock(|when, then| {
        when.method(GET)
            .path("/transactions")
            .json_body(json!({"size": 2, "cursor": "c2"}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"items":[{"id":"t3","cursor":"c3"}]}"#);
    });

    let output = h
        .api("/transactions")
        .arg("--paginate")
        .arg("-F")
        .arg("size=2")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    for id in ["t1", "t2", "t3"] {
        assert!(stdout.contains(id), "stdout: {stdout}");
    }
    first.assert();
    second.assert();
}

#[test]
fn slurp_wraps_all_pages_in_single_array() {
    let h = Harness::new();
    for n in 0..2u64 {
        h.server.mock(|when, then| {
            when.method(GET)
                .path("/payments")
                .json_body(json!({"page": {"number": n, "size": 2}}));
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    r#"{{"items":[{n}],"page":{{"number":{n},"size":2,"totalCount":4}}}}"#
                ));
        });
    }

    let output = h
        .api("/payments")
        .arg("--paginate")
        .arg("--slurp")
        .arg("-F")
        .arg("page[number]=0")
        .arg("-F")
        .arg("page[size]=2")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("expected a JSON array");
    let pages = value.as_array().expect("expected a top-level array");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0]["items"], json!([0]));
    assert_eq!(pages[1]["items"], json!([1]));
}

#[test]
fn slurp_with_silent_prints_nothing() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(GET).path("/payments");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"items":[1],"page":{"number":0,"size":100,"totalCount":1}}"#);
    });

    h.api("/payments")
        .arg("--paginate")
        .arg("--slurp")
        .arg("--silent")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    mock.assert();
}

#[test]
fn input_dash_sends_stdin_as_body() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/imports").body(r#"{"raw":"body"}"#);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"ok":true}"#);
    });

    h.api("/imports")
        .arg("--input")
        .arg("-")
        .write_stdin(r#"{"raw":"body"}"#)
        .assert()
        .success()
        .stdout(predicate::eq(r#"{"ok":true}"#));
    mock.assert();
}

#[test]
fn cache_serves_second_request_without_network() {
    let h = Harness::new();
    let body = r#"{"cached":true}"#;
    let mock = h.server.mock(|when, then| {
        when.method(GET).path("/cached");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    for _ in 0..2 {
        h.api("/cached")
            .arg("--cache")
            .arg("60s")
            .assert()
            .success()
            .stdout(predicate::eq(body));
    }
    mock.assert_calls(1);
}

#[test]
fn jq_extracts_values_and_prints_strings_raw() {
    let h = Harness::new();
    h.server.mock(|when, then| {
        when.method(GET).path("/payments");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"items":[{"id":"a"},{"id":"b"}]}"#);
    });

    h.api("/payments")
        .arg("--jq")
        .arg(".items[].id")
        .assert()
        .success()
        .stdout(predicate::eq("a\nb\n"));
}

#[test]
fn authorization_header_is_injected_and_overridable() {
    let h = Harness::new();
    let injected = h.server.mock(|when, then| {
        when.method(GET)
            .path("/auth-default")
            .header("authorization", "Bearer test-token");
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });
    h.api("/auth-default").assert().success();
    injected.assert();

    let overridden = h.server.mock(|when, then| {
        when.method(GET)
            .path("/auth-override")
            .header("authorization", "Bearer custom-token");
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/auth-override")
        .arg("--base-url")
        .arg(h.server.base_url())
        .arg("-H")
        .arg("Authorization: Bearer custom-token")
        .assert()
        .success();
    overridden.assert();
}

#[test]
fn missing_credentials_fail_before_request() {
    let h = Harness::new();
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .arg("--base-url")
        .arg(h.server.base_url())
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "portone: no credentials found. run `portone auth login` or set PORTONE_ACCESS_TOKEN",
        ));
}

#[test]
fn verbose_output_masks_access_token() {
    let h = Harness::new();
    h.server.mock(|when, then| {
        when.method(GET).path("/payments");
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    let output = portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .arg("--base-url")
        .arg(h.server.base_url())
        .env("PORTONE_ACCESS_TOKEN", "super-secret-value")
        .arg("--verbose")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
    assert!(
        stdout.contains("> Authorization: Bearer ████"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("super-secret-value") && !stderr.contains("super-secret-value"),
        "unmasked secret was exposed in output"
    );
}

#[test]
fn full_url_to_foreign_origin_does_not_leak_secret() {
    let h = Harness::new();
    let other = MockServer::start();
    let leak_guard = other.mock(|when, then| {
        when.method(GET).path("/x").header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"ok":true}"#);
    });

    h.api(&format!("{}/x", other.base_url())).assert().success();
    leak_guard.assert_calls(1);

    let same = h.server.mock(|when, then| {
        when.method(GET)
            .path("/y")
            .header("authorization", "Bearer test-token");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"ok":true}"#);
    });
    h.api(&format!("{}/y", h.server.base_url()))
        .assert()
        .success();
    same.assert_calls(1);
}

#[test]
fn api_uses_bearer_for_oauth_profile() {
    let h = Harness::new();
    write_oauth_config(
        h.config.path(),
        &h.server.base_url(),
        &h.server.url("/oauth/token"),
        "access-1",
        "refresh-1",
        now() + 3600,
    );
    let payments = bearer_ok(&h.server, "access-1");
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .assert()
        .success();
    payments.assert();
}

#[test]
fn api_refreshes_expired_token_before_request() {
    let h = Harness::new();
    write_oauth_config(
        h.config.path(),
        &h.server.base_url(),
        &h.server.url("/oauth/token"),
        "access-1",
        "refresh-1",
        1,
    );
    let refresh = refresh_mock(&h.server, 200, new_tokens());
    let payments = bearer_ok(&h.server, "access-2");
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .assert()
        .success();
    refresh.assert_calls(1);
    payments.assert();
    let contents = std::fs::read_to_string(h.config.path().join("config.toml")).unwrap();
    assert!(
        contents.contains("access_token = \"access-2\""),
        "{contents}"
    );
    assert!(
        contents.contains("refresh_token = \"refresh-2\""),
        "{contents}"
    );
}

#[test]
fn api_refresh_invalid_grant_suggests_relogin() {
    let h = Harness::new();
    write_oauth_config(
        h.config.path(),
        &h.server.base_url(),
        &h.server.url("/oauth/token"),
        "access-1",
        "refresh-1",
        1,
    );
    let refresh = refresh_mock(
        &h.server,
        400,
        json!({"error": "invalid_grant", "detail": "Invalid refresh_token"}),
    );
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("`portone auth login`"));
    refresh.assert_calls(1);
}

#[test]
fn api_refresh_5xx_keeps_valid_token_with_warning() {
    let h = Harness::new();
    write_oauth_config(
        h.config.path(),
        &h.server.base_url(),
        &h.server.url("/oauth/token"),
        "access-1",
        "refresh-1",
        now() + 30,
    );
    refresh_mock(&h.server, 503, json!({}));
    let payments = bearer_ok(&h.server, "access-1");
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "continuing with the current token",
        ));
    payments.assert();
}

#[test]
fn api_refresh_5xx_with_expired_token_fails() {
    let h = Harness::new();
    write_oauth_config(
        h.config.path(),
        &h.server.base_url(),
        &h.server.url("/oauth/token"),
        "access-1",
        "refresh-1",
        1,
    );
    refresh_mock(&h.server, 503, json!({}));
    let payments = bearer_ok(&h.server, "access-1");
    portone(h.config.path(), h.cache.path())
        .arg("api")
        .arg("/payments")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("token refresh failed"));
    payments.assert_calls(0);
}

#[test]
fn api_uses_access_token_env_and_masks_it_in_verbose() {
    let h = Harness::new();
    let payments = bearer_ok(&h.server, "env-token");
    let output = portone(h.config.path(), h.cache.path())
        .env("PORTONE_ACCESS_TOKEN", "env-token")
        .arg("api")
        .arg("/payments")
        .arg("--base-url")
        .arg(h.server.base_url())
        .arg("--verbose")
        .assert()
        .success();
    payments.assert();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("> Authorization: Bearer ████"),
        "stdout: {stdout}"
    );
    assert!(!stdout.contains("env-token"), "unmasked token was exposed");
}

#[test]
fn api_concurrent_calls_refresh_once() {
    let h = Harness::new();
    write_oauth_config(
        h.config.path(),
        &h.server.base_url(),
        &h.server.url("/oauth/token"),
        "access-1",
        "refresh-1",
        1,
    );
    let refresh = refresh_mock(&h.server, 200, new_tokens());
    let payments = bearer_ok(&h.server, "access-2");

    let children: Vec<_> = (0..4)
        .map(|_| {
            std::process::Command::new(env!("CARGO_BIN_EXE_portone"))
                .env_remove("PORTONE_ACCESS_TOKEN")
                .env_remove("PORTONE_API_BASE")
                .env("PORTONE_LANG", "en")
                .env("NO_COLOR", "1")
                .env("PORTONE_CONFIG_DIR", h.config.path())
                .env("PORTONE_CACHE_DIR", h.cache.path())
                .args(["api", "/payments"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    refresh.assert_calls(1);
    payments.assert_calls(4);
}
