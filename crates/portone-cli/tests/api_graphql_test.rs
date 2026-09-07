use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use serde_json::{Value, json};

fn portone(config_dir: &std::path::Path, cache_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("portone").expect("failed to build portone binary");
    cmd.env_remove("PORTONE_ACCESS_TOKEN")
        .env_remove("PORTONE_API_BASE")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("PORTONE_PAGER")
        .env_remove("PAGER")
        .env("NO_COLOR", "1")
        .env("PORTONE_CONFIG_DIR", config_dir)
        .env("PORTONE_CACHE_DIR", cache_dir);
    cmd
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
fn graphql_posts_grouped_variables_to_graphql_path() {
    let h = Harness::new();
    let body = r#"{"data":{"hero":{"name":"hubot"}}}"#;
    let mock = h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .header("content-type", "application/json; charset=utf-8")
            .header("authorization", "Bearer test-token")
            .json_body(json!({
                "query": "query($name: String!, $power: Int!) { hero }",
                "variables": {"name": "hubot", "power": 9001},
            }));
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    h.api("graphql")
        .arg("-f")
        .arg("query=query($name: String!, $power: Int!) { hero }")
        .arg("-f")
        .arg("name=hubot")
        .arg("-F")
        .arg("power=9001")
        .assert()
        .success()
        .stdout(predicate::eq(body))
        .stderr(predicate::str::is_empty());
    mock.assert();
}

#[test]
fn graphql_operation_name_stays_top_level() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/graphql").json_body(json!({
            "query": "query Op { hero }",
            "operationName": "Op",
            "variables": {"a": "b"},
        }));
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    h.api("graphql")
        .arg("-f")
        .arg("query=query Op { hero }")
        .arg("-f")
        .arg("operationName=Op")
        .arg("-f")
        .arg("a=b")
        .assert()
        .success();
    mock.assert();
}

#[test]
fn slash_graphql_is_rest_not_graphql() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .json_body(json!({"query": "Q", "a": "b"}));
        then.status(200)
            .header("content-type", "application/json")
            .body("{}");
    });

    h.api("/graphql")
        .arg("-f")
        .arg("query=Q")
        .arg("-f")
        .arg("a=b")
        .assert()
        .success();
    mock.assert();
}

#[test]
fn graphql_200_with_errors_exits_1() {
    let h = Harness::new();
    let body = r#"{"data":null,"errors":[{"message":"AGAIN"},{"message":"FINE"}]}"#;
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    h.api("graphql")
        .arg("-f")
        .arg("query=Q")
        .assert()
        .code(1)
        .stdout(predicate::eq(body))
        .stderr(predicate::str::contains("portone: AGAIN\nFINE"));
    mock.assert();
}

#[test]
fn graphql_errors_skip_jq() {
    let h = Harness::new();
    let body = r#"{"data":null,"errors":[{"message":"BOOM"}]}"#;
    h.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    h.api("graphql")
        .arg("-f")
        .arg("query=Q")
        .arg("--jq")
        .arg(".data")
        .assert()
        .code(1)
        .stdout(predicate::eq(body))
        .stderr(predicate::str::contains("portone: BOOM"));
}

#[test]
fn graphql_paginate_passes_end_cursor() {
    let h = Harness::new();
    let query = "query($endCursor: String) { promotions(first: 1, after: $endCursor) { nodes { id } pageInfo { hasNextPage endCursor } } }";
    let first = h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .json_body(json!({"query": query}));
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"data":{"promotions":{"nodes":[{"id":"p1"}],"pageInfo":{"hasNextPage":true,"endCursor":"PAGE1_END"}}}}"#,
            );
    });
    let second = h.server.mock(|when, then| {
        when.method(POST).path("/graphql").json_body(json!({
            "query": query,
            "variables": {"endCursor": "PAGE1_END"},
        }));
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"{"data":{"promotions":{"nodes":[{"id":"p2"}],"pageInfo":{"hasNextPage":false,"endCursor":"PAGE2_END"}}}}"#,
            );
    });

    let output = h
        .api("graphql")
        .arg("--paginate")
        .arg("-f")
        .arg(format!("query={query}"))
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("p1"), "stdout: {stdout}");
    assert!(stdout.contains("p2"), "stdout: {stdout}");
    first.assert();
    second.assert();
}

#[test]
fn graphql_paginate_slurp_wraps_pages() {
    let h = Harness::new();
    h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .json_body(json!({"query": "Q"}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":{"pageInfo":{"hasNextPage":true,"endCursor":"C1"}}}"#);
    });
    h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .json_body(json!({"query": "Q", "variables": {"endCursor": "C1"}}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":{"pageInfo":{"hasNextPage":false,"endCursor":"C2"}}}"#);
    });

    let output = h
        .api("graphql")
        .arg("--paginate")
        .arg("--slurp")
        .arg("-f")
        .arg("query=Q")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let pages: Value = serde_json::from_str(&stdout).expect("slurp output is not a JSON array");
    let pages = pages.as_array().expect("slurp output is not an array");
    assert_eq!(pages.len(), 2, "stdout: {stdout}");
}

#[test]
fn graphql_paginate_stops_without_page_info() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":{"merchant":{"plainId":"m1"}}}"#);
    });

    h.api("graphql")
        .arg("--paginate")
        .arg("-f")
        .arg("query=Q")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    mock.assert();
}

#[test]
fn graphql_paginate_stops_on_repeated_cursor() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":{"pageInfo":{"hasNextPage":true,"endCursor":"SAME"}}}"#);
    });

    h.api("graphql")
        .arg("--paginate")
        .arg("-f")
        .arg("query=Q")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "portone: pagination cursor did not advance; stopping",
        ));
    mock.assert_calls(2);
}

#[test]
fn graphql_paginate_slurp_silent_prints_nothing() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":{"merchant":{"plainId":"m1"}}}"#);
    });

    h.api("graphql")
        .arg("--paginate")
        .arg("--slurp")
        .arg("--silent")
        .arg("-f")
        .arg("query=Q")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    mock.assert();
}

#[test]
fn graphql_error_mid_pagination_exits_1() {
    let h = Harness::new();
    let first = h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .json_body(json!({"query": "Q"}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data":{"pageInfo":{"hasNextPage":true,"endCursor":"C1"}}}"#);
    });
    let second = h.server.mock(|when, then| {
        when.method(POST)
            .path("/graphql")
            .json_body(json!({"query": "Q", "variables": {"endCursor": "C1"}}));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"errors":[{"message":"BOOM"}]}"#);
    });

    h.api("graphql")
        .arg("--paginate")
        .arg("-f")
        .arg("query=Q")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("portone: BOOM"));
    first.assert();
    second.assert();
}

#[test]
fn graphql_cache_stores_post_response() {
    let h = Harness::new();
    let body = r#"{"data":{"hero":"cached"}}"#;
    let mock = h.server.mock(|when, then| {
        when.method(POST).path("/graphql");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    for _ in 0..2 {
        h.api("graphql")
            .arg("--cache")
            .arg("60s")
            .arg("-f")
            .arg("query=Q")
            .assert()
            .success()
            .stdout(predicate::eq(body));
    }
    mock.assert_calls(1);
}
