use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use serde_json::{Value, json};

struct Harness {
    config: tempfile::TempDir,
    server: MockServer,
}

impl Harness {
    fn new() -> Self {
        Self {
            config: tempfile::tempdir().unwrap(),
            server: MockServer::start(),
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::cargo_bin("portone").unwrap();
        self.configure(&mut command);
        command
            .args(["payment", "--base-url", &self.server.base_url()])
            .args(args);
        command
    }

    fn configure(&self, command: &mut Command) {
        for key in [
            "PORTONE_STORE_ID",
            "PORTONE_API_BASE",
            "CLICOLOR_FORCE",
            "PORTONE_PAGER",
            "PAGER",
        ] {
            command.env_remove(key);
        }
        command
            .env("PORTONE_CONFIG_DIR", self.config.path())
            .env("PORTONE_LANG", "en")
            .env("NO_COLOR", "1")
            .env("PORTONE_ACCESS_TOKEN", "test-token");
    }

    #[cfg(target_os = "linux")]
    fn terminal_cancel(&self) -> Command {
        // util-linux script provides real stdin/stderr terminals without a PTY dependency.
        let mut command = Command::new("script");
        self.configure(&mut command);
        command
            .env("SHELL", "/bin/sh")
            .env("PORTONE_TEST_BINARY", env!("CARGO_BIN_EXE_portone"))
            .env("PORTONE_TEST_BASE_URL", self.server.base_url())
            .args([
                "--quiet",
                "--return",
                "--command",
                "exec \"$PORTONE_TEST_BINARY\" payment --base-url \"$PORTONE_TEST_BASE_URL\" cancel p --reason test --amount 1200 --json",
                "/dev/null",
            ])
            .timeout(std::time::Duration::from_secs(10));
        command
    }

    fn saved_store(&self, id: &str) {
        std::fs::write(
            self.config.path().join("config.toml"),
            format!("default_profile = \"work\"\n[profiles.work]\nstore_id = \"{id}\"\n"),
        )
        .unwrap();
    }
}

const RANGE: [&str; 4] = [
    "--from",
    "2026-09-01T00:00:00Z",
    "--until",
    "2026-09-08T00:00:00Z",
];

fn payment(id: &str) -> Value {
    json!({"id":id,"status":"PAID","amount":{"total":12000},"currency":"KRW","orderName":"Order","storeId":"store-a","channel":{"type":"TEST"},"statusChangedAt":"2026-09-07T01:00:00Z"})
}

#[test]
fn list_sends_schema_filters_and_returns_one_flat_json_array() {
    let h = Harness::new();
    h.saved_store("store-a");
    let mock =
        h.server.mock(|when, then| {
            when.method(GET).path("/payments").header("authorization", "Bearer test-token")
            .json_body(json!({"page":{"number":0,"size":30},"filter":{
                "storeId":"store-a","version":"V2","timestampType":"STATUS_CHANGED_AT",
                "sortBy":"STATUS_CHANGED_AT","sortOrder":"DESC","from":RANGE[1],"until":RANGE[3],
                "status":["PENDING","FAILED"],"methods":["CARD"],"isTest":true,
                "textSearch":[{"field":"PG_TX_ID","value":"pg-123"}]
            }}));
            then.status(200)
                .json_body(json!({"items":[payment("p")],"page":{"totalCount":1}}));
        });
    h.command(&[
        "list",
        "--status",
        "pending,failed",
        "--method",
        "card",
        "--test",
        "--search",
        "pg-123",
        "--search-field",
        "pg-tx-id",
        "--json",
        "id,status",
    ])
    .args(RANGE)
    .assert()
    .success()
    .stdout("[{\"id\":\"p\",\"status\":\"PAID\"}]\n")
    .stderr("");
    mock.assert_calls(1);
}

#[test]
fn list_paginates_to_limit_without_leaking_page_envelopes() {
    let h = Harness::new();
    let first = h.server.mock(|when, then| {
        when.method(GET).path("/payments")
            .json_body_includes(r#"{"page":{"number":0,"size":100},"filter":{"from":"2026-09-01T00:00:00Z","until":"2026-09-08T00:00:00Z"}}"#);
        then.status(200).json_body(json!({"items":(0..100).map(|i|payment(&format!("p{i}"))).collect::<Vec<_>>(),"page":{"totalCount":250,"size":100}}));
    });
    let second = h.server.mock(|when, then| {
        when.method(GET).path("/payments")
            .json_body_includes(r#"{"page":{"number":1,"size":100},"filter":{"from":"2026-09-01T00:00:00Z","until":"2026-09-08T00:00:00Z"}}"#);
        then.status(200).json_body(json!({"items":(100..200).map(|i|payment(&format!("p{i}"))).collect::<Vec<_>>(),"page":{"totalCount":250,"size":100}}));
    });
    let output = h
        .command(&["list", "--limit", "101", "--json"])
        .args(RANGE)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let items: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(items.as_array().unwrap().len(), 101);
    assert_eq!(items[100]["id"], "p100");
    first.assert_calls(1);
    second.assert_calls(1);
}

#[test]
fn explicit_all_stores_and_all_versions_remove_saved_and_environment_filters() {
    let h = Harness::new();
    h.saved_store("saved");
    let mock = h.server.mock(|when, then| {
        when.method(GET).path("/payments").json_body(json!({"page":{"number":0,"size":30},"filter":{
            "timestampType":"STATUS_CHANGED_AT","sortBy":"STATUS_CHANGED_AT","sortOrder":"DESC","from":RANGE[1],"until":RANGE[3]
        }}));
        then.status(200).json_body(json!({"items":[],"page":{"totalCount":0}}));
    });
    h.command(&["list", "--all-stores", "--version", "all", "--json"])
        .env("PORTONE_STORE_ID", "env")
        .args(RANGE)
        .assert()
        .success()
        .stdout("[]\n");
    mock.assert_calls(1);
}

#[test]
fn view_encodes_payment_id_and_resolves_explicit_environment_and_profile_stores() {
    let h = Harness::new();
    h.saved_store("saved");
    for (flag, env, expected) in [
        (Some("explicit"), Some("environment"), "explicit"),
        (None, Some("environment"), "environment"),
        (None, None, "saved"),
    ] {
        let mut mock = h.server.mock(|when, then| {
            when.method(GET)
                .path("/payments/order%2Fpart%3Fx%23y")
                .query_param("storeId", expected);
            then.status(200).json_body(payment("order/part?x#y"));
        });
        let mut command = h.command(&["view", "order/part?x#y", "--json", "id"]);
        if let Some(flag) = flag {
            command.args(["--store", flag]);
        }
        if let Some(env) = env {
            command.env("PORTONE_STORE_ID", env);
        }
        command
            .assert()
            .success()
            .stdout("{\"id\":\"order/part?x#y\"}\n");
        mock.assert_calls(1);
        mock.delete();
    }
}

#[test]
fn view_preserves_new_fields_and_integer_precision_and_supports_embedded_jq() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(GET).path("/payments/p");
        then.status(200).body(r#"{"id":"p","status":"PAY_PENDING","amount":{"total":9007199254740993},"futureField":{"ok":true}}"#);
    });
    let output = h
        .command(&["view", "p", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        value["amount"]["total"].as_i64(),
        Some(9_007_199_254_740_993)
    );
    assert_eq!(value["status"], "PAY_PENDING");
    assert_eq!(value["futureField"]["ok"], true);
    h.command(&["view", "p", "--json", "id,status", "--jq", ".id"])
        .assert()
        .success()
        .stdout("p\n");
    mock.assert_calls(2);
}

#[test]
fn attempts_and_webhook_lists_use_the_right_sources() {
    let h = Harness::new();
    let attempts = h.server.mock(|when, then| {
        when.method(GET)
            .path("/payments/p/transactions")
            .query_param("storeId", "s");
        then.status(200)
            .json_body(json!({"items":[{"id":"tx","paymentId":"p","status":"FAILED"}]}));
    });
    let hooks=h.server.mock(|when,then|{
        when.method(GET).path("/payments/p").query_param("storeId","s");
        then.status(200).json_body(json!({"id":"p","webhooks":[{"id":"wh","url":"https://merchant.test/hook","status":"FAILED_NOT_OK_RESPONSE","response":{"code":"500","body":"failure"}}]}));
    });
    h.command(&["transactions", "p", "--store", "s", "--json"])
        .assert()
        .success()
        .stdout("[{\"id\":\"tx\",\"paymentId\":\"p\",\"status\":\"FAILED\"}]\n");
    h.command(&[
        "webhook",
        "list",
        "p",
        "--store",
        "s",
        "--json",
        "id,status",
    ])
    .assert()
    .success()
    .stdout("[{\"id\":\"wh\",\"status\":\"FAILED_NOT_OK_RESPONSE\"}]\n");
    attempts.assert_calls(1);
    hooks.assert_calls(1);
}

#[test]
fn cancel_requires_confirmation_in_non_tty_and_sends_no_request() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.path("/payments/p/cancel");
        then.status(200)
            .json_body(json!({"cancellation":{"id":"c","status":"SUCCEEDED"}}));
    });
    h.command(&["cancel", "p", "--reason", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--yes"));
    mock.assert_calls(0);
}

#[cfg(target_os = "linux")]
#[test]
fn terminal_cancel_shows_live_context_and_requires_affirmative_confirmation() {
    let h = Harness::new();
    h.saved_store("store-a");
    let mut live_payment = payment("p");
    live_payment["channel"]["type"] = json!("LIVE");
    let lookup = h.server.mock(|when, then| {
        when.method(GET)
            .path("/payments/p")
            .query_param("storeId", "store-a");
        then.status(200).json_body(live_payment);
    });
    let cancellation = h.server.mock(|when, then| {
        when.method(POST)
            .path("/payments/p/cancel")
            .json_body(json!({"storeId":"store-a","reason":"test","amount":1200}));
        then.status(200)
            .json_body(json!({"cancellation":{"id":"c","status":"SUCCEEDED","totalAmount":1200}}));
    });
    for answer in ["\n", "n\n"] {
        h.terminal_cancel()
            .write_stdin(answer)
            .assert()
            .code(1)
            .stdout(predicate::str::contains("Profile: work"))
            .stdout(predicate::str::contains("Store: store-a"))
            .stdout(predicate::str::contains("Payment: p"))
            .stdout(predicate::str::contains("Test/live: LIVE"))
            .stdout(predicate::str::contains("1200 KRW"))
            .stdout(predicate::str::contains("Reason: test"))
            .stdout(predicate::str::contains("Cancel this payment? [y/N]"))
            .stdout(predicate::str::contains("Cancellation aborted."));
        cancellation.assert_calls(0);
    }
    h.terminal_cancel()
        .write_stdin("yes\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("SUCCEEDED"));
    lookup.assert_calls(3);
    cancellation.assert_calls(1);
}

#[test]
fn full_and_partial_cancel_send_exact_fields_and_surface_async_outcomes() {
    let h = Harness::new();
    h.saved_store("s");
    for (status, amount) in [
        ("SUCCEEDED", None),
        ("REQUESTED", Some("500")),
        ("FAILED", Some("100")),
    ] {
        let mut body = json!({"storeId":"s","reason":"test"});
        if let Some(amount) = amount {
            body["amount"] = json!(amount.parse::<i64>().unwrap());
        }
        let mut mock=h.server.mock(|when,then|{
            when.method(POST).path("/payments/p/cancel").json_body(body);
            then.status(200).json_body(json!({"cancellation":{"id":"c","status":status,"totalAmount":500,"reason":"test"}}));
        });
        let mut command = h.command(&["cancel", "p", "--reason", "test", "--yes", "--json"]);
        if let Some(amount) = amount {
            command.args(["--amount", amount]);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.success(), status != "FAILED");
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
            status
        );
        mock.assert_calls(1);
        mock.delete();
    }
}

#[test]
fn cancellation_input_preserves_refund_fields_and_explicit_store_over_defaults() {
    let h = Harness::new();
    h.saved_store("saved");
    let body = json!({"reason":"refund","storeId":"body-store","amount":9_007_199_254_740_993i64,"refundAccount":{"bank":"KOOKMIN","number":"1234","holderName":"Customer"},"skipWebhook":true});
    let mock = h.server.mock(|when, then| {
        when.method(POST)
            .path("/payments/p/cancel")
            .json_body(body.clone());
        then.status(200)
            .json_body(json!({"cancellation":{"id":"c","status":"REQUESTED"}}));
    });
    h.command(&["cancel", "p", "--input", "-", "--yes", "--json"])
        .env("PORTONE_STORE_ID", "environment")
        .write_stdin(body.to_string())
        .assert()
        .success();
    h.command(&[
        "cancel",
        "p",
        "--input",
        "-",
        "--store",
        "different",
        "--yes",
    ])
    .write_stdin(body.to_string())
    .assert()
    .failure();
    mock.assert_calls(1);
}

#[test]
fn resend_uses_body_and_reports_delivery_failure_despite_http_success() {
    let h = Harness::new();
    for (id, status) in [(None, "SUCCEEDED"), (Some("wh"), "FAILED_NOT_OK_RESPONSE")] {
        let body = id
            .map(|id| json!({"webhookId":id}))
            .unwrap_or_else(|| json!({}));
        let mut mock = h.server.mock(|when, then| {
            when.method(POST)
                .path("/payments/p/resend-webhook")
                .json_body(body);
            then.status(200).json_body(
                json!({"webhook":{"id":"wh","url":"https://merchant.test/hook","status":status}}),
            );
        });
        let mut command = h.command(&["webhook", "resend", "p", "--json"]);
        if let Some(id) = id {
            command.args(["--webhook-id", id]);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.success(), status == "SUCCEEDED");
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
            status
        );
        mock.assert_calls(1);
        mock.delete();
    }
}

#[test]
fn validation_errors_do_not_send_mutations() {
    let h = Harness::new();
    let mock = h.server.mock(|when, then| {
        when.method(POST);
        then.status(200).json_body(json!({}));
    });
    for args in [
        vec!["cancel", "p", "--reason", "test", "--amount", "0", "--yes"],
        vec![
            "cancel",
            "p",
            "--reason",
            "test",
            "--amount",
            "9223372036854775808",
            "--yes",
        ],
        vec!["cancel", "p", "--reason", "", "--yes"],
        vec![
            "cancel", "p", "--reason", "test", "--yes", "--json", "badField",
        ],
        vec![
            "cancel", "p", "--reason", "test", "--yes", "--json", "--jq", ".[",
        ],
    ] {
        h.command(&args).assert().failure();
    }
    mock.assert_calls(0);
}

#[test]
fn http_errors_preserve_discriminator_and_pg_diagnostics() {
    let h = Harness::new();
    for code in [401, 403, 409] {
        let mut mock=h.server.mock(|when,then|{
            when.method(POST).path("/payments/p/cancel");
            then.status(code).json_body(json!({"type":"PG_PROVIDER","message":"cannot cancel","pgCode":"E42","pgMessage":"provider declined"}));
        });
        h.command(&["cancel", "p", "--reason", "test", "--yes", "--json"])
            .assert()
            .failure()
            .stdout("")
            .stderr(
                predicate::str::contains("PG_PROVIDER")
                    .and(predicate::str::contains("E42"))
                    .and(predicate::str::contains(code.to_string())),
            );
        mock.assert_calls(1);
        mock.delete();
    }
}

#[test]
fn payment_client_refreshes_profile_tokens_before_request_and_preserves_store() {
    let h = Harness::new();
    std::fs::write(
        h.config.path().join("config.toml"),
        format!(
            r#"
default_profile = "work"
[profiles.work]
store_id = "saved"
base_url = "{}"
[profiles.work.oauth]
storage = "file"
client_id = "CLI"
token_url = "{}/oauth/token"
console_url = "https://console.example"
[profiles.work.oauth.tokens]
access_token = "old"
refresh_token = "refresh-old"
expires_at = 1
scope = ["TX_READ"]
token_type = "Bearer"
"#,
            h.server.base_url(),
            h.server.base_url()
        ),
    )
    .unwrap();
    let refresh=h.server.mock(|when,then|{
        when.method(POST).path("/oauth/token").json_body_includes(r#"{"refresh_token":"refresh-old"}"#);
        then.status(200).json_body(json!({"access_token":"new","refresh_token":"refresh-new","expires_in":1800,"scope":["TX_READ"],"token_type":"Bearer"}));
    });
    let request = h.server.mock(|when, then| {
        when.method(GET)
            .path("/payments/p")
            .query_param("storeId", "saved")
            .header("authorization", "Bearer new");
        then.status(200).json_body(payment("p"));
    });
    h.command(&["view", "p", "--json"])
        .env_remove("PORTONE_ACCESS_TOKEN")
        .assert()
        .success();
    refresh.assert_calls(1);
    request.assert_calls(1);
    let config = std::fs::read_to_string(h.config.path().join("config.toml")).unwrap();
    assert!(config.contains("refresh-new") && config.contains("store_id = \"saved\""));
}
