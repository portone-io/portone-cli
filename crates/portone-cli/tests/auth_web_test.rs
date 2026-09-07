use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use httpmock::prelude::*;
use serde_json::json;
use sha2::{Digest, Sha256};

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn spawn_login(
    config_dir: &std::path::Path,
    server: &MockServer,
    port: u16,
    extra: &[&str],
) -> (Child, BufReader<ChildStderr>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_portone"))
        .env_remove("PORTONE_ACCESS_TOKEN")
        .env_remove("PORTONE_API_BASE")
        .env_remove("PORTONE_OAUTH_CLIENT_ID")
        .env_remove("PORTONE_BROWSER")
        .env_remove("BROWSER")
        .env("NO_COLOR", "1")
        .env("PORTONE_CONFIG_DIR", config_dir)
        .env("PORTONE_CONSOLE_URL", server.base_url())
        .env("PORTONE_MERCHANT_SERVICE_URL", server.base_url())
        .env(
            "PORTONE_OAUTH_REDIRECT_URI",
            format!("http://127.0.0.1:{port}/oauth/cli"),
        )
        .args([
            "auth",
            "login",
            "--no-browser",
            "--insecure-storage",
            "--base-url",
            &server.base_url(),
        ])
        .args(extra)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run portone");
    let stderr = child.stderr.take().unwrap();
    (child, BufReader::new(stderr))
}

fn read_authorize_url(reader: &mut BufReader<ChildStderr>) -> url::Url {
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).unwrap();
        assert!(n > 0, "process exited before printing the login URL");
        let trimmed = line.trim();
        if trimmed.starts_with("http") {
            return url::Url::parse(trimmed).unwrap();
        }
    }
}

fn callback(port: u16, query: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET /oauth/cli?{query} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
    let status: u16 = status_line.split(' ').nth(1).unwrap().parse().unwrap();
    let mut body = String::new();
    let _ = reader.read_to_string(&mut body);
    (status, body)
}

fn finish(mut child: Child, reader: &mut BufReader<ChildStderr>) -> (bool, String) {
    let status = child.wait().unwrap();
    let mut rest = String::new();
    reader.read_to_string(&mut rest).unwrap();
    (status.success(), rest)
}

fn probe_mock<'a>(server: &'a MockServer, typename: &str) -> httpmock::Mock<'a> {
    let typename = typename.to_string();
    server.mock(move |when, then| {
        when.method(POST)
            .path("/graphql")
            .header("authorization", "Bearer access-1");
        then.status(200).json_body(json!({
            "data": { "merchant": { "__typename": typename, "plainId": "merchant-1" } }
        }));
    })
}

#[test]
fn login_web_no_browser_stores_tokens_in_file() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    let port = free_port();
    let expected_challenge: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let challenge_for_mock = Arc::clone(&expected_challenge);
    let token = server.mock(move |when, then| {
        when.method(POST)
            .path("/oauth/token")
            .json_body_includes(
                r#"{"client_id":"CLI","grant_type":"authorization_code","code":"abc"}"#,
            )
            .is_true(move |req| {
                let body: serde_json::Value =
                    serde_json::from_slice(req.body().as_ref()).unwrap_or_default();
                let verifier = body["code_verifier"].as_str().unwrap_or("");
                let actual = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
                challenge_for_mock.lock().unwrap().as_deref() == Some(actual.as_str())
            });
        then.status(200).json_body(json!({
            "access_token": "access-1",
            "token_type": "Bearer",
            "expires_in": 1800,
            "scope": ["TX_READ", "STORE_READ"],
            "refresh_token": "refresh-1"
        }));
    });
    let probe = probe_mock(&server, "Merchant");

    let (child, mut reader) = spawn_login(
        dir.path(),
        &server,
        port,
        &["--scopes", "TX_READ,STORE_READ,MERCHANT_READ"],
    );
    let url = read_authorize_url(&mut reader);
    assert_eq!(url.path(), "/oauth/authorize");
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    assert_eq!(params["client_id"], "CLI");
    assert_eq!(
        params["redirect_uri"],
        format!("http://127.0.0.1:{port}/oauth/cli")
    );
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["scope"], "TX_READ STORE_READ MERCHANT_READ");
    assert_eq!(params["code_challenge_method"], "S256");
    *expected_challenge.lock().unwrap() = Some(params["code_challenge"].clone());

    let (status, body) = callback(port, &format!("code=abc&state={}", params["state"]));
    assert_eq!(status, 200);
    assert!(body.contains("Login complete"), "{body}");

    let (ok, stderr) = finish(child, &mut reader);
    assert!(ok, "stderr: {stderr}");
    assert!(
        stderr.contains("Console login complete (merchant merchant-1)"),
        "{stderr}"
    );
    assert!(
        stderr.contains("some requested scopes were not granted: MERCHANT_READ"),
        "{stderr}"
    );
    assert!(stderr.contains("Storage: config file"), "{stderr}");
    token.assert();
    probe.assert();

    let path = dir.path().join("config.toml");
    let contents = std::fs::read_to_string(&path).unwrap();
    for expected in [
        "default_profile = \"default\"",
        &format!("base_url = \"{}\"", server.base_url()),
        "storage = \"file\"",
        "client_id = \"CLI\"",
        &format!("token_url = \"{}/oauth/token\"", server.base_url()),
        &format!("console_url = \"{}\"", server.base_url()),
        "access_token = \"access-1\"",
        "refresh_token = \"refresh-1\"",
    ] {
        assert!(
            contents.contains(expected),
            "missing {expected}: {contents}"
        );
    }
    assert!(!contents.contains("credential_id"), "{contents}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[test]
fn login_web_rejects_wrong_state_then_accepts() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    let port = free_port();
    server.mock(|when, then| {
        when.method(POST).path("/oauth/token");
        then.status(200).json_body(json!({
            "access_token": "access-1",
            "token_type": "Bearer",
            "expires_in": 1800,
            "scope": ["TX_READ"],
            "refresh_token": "refresh-1"
        }));
    });
    probe_mock(&server, "Merchant");

    let (child, mut reader) = spawn_login(dir.path(), &server, port, &[]);
    let url = read_authorize_url(&mut reader);
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let (status, _) = callback(port, "code=evil&state=wrong");
    assert_eq!(status, 400);
    let (status, _) = callback(port, &format!("code=good&state={}", params["state"]));
    assert_eq!(status, 200);

    let (ok, stderr) = finish(child, &mut reader);
    assert!(ok, "stderr: {stderr}");
    assert!(
        stderr.contains("callback with mismatched state"),
        "{stderr}"
    );
    assert!(stderr.contains("Console login complete"), "{stderr}");
}

#[test]
fn login_web_reports_denied_callback() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    let port = free_port();
    let token = server.mock(|when, then| {
        when.method(POST).path("/oauth/token");
        then.status(200);
    });

    let (child, mut reader) = spawn_login(dir.path(), &server, port, &[]);
    let url = read_authorize_url(&mut reader);
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let (status, _) = callback(
        port,
        &format!(
            "error=access_denied&error_description=nope&state={}",
            params["state"]
        ),
    );
    assert_eq!(status, 400);

    let (ok, stderr) = finish(child, &mut reader);
    assert!(!ok);
    assert!(
        stderr.contains("console login was denied: access_denied (nope)"),
        "{stderr}"
    );
    token.assert_calls(0);
    assert!(!dir.path().join("config.toml").exists());
}

#[test]
fn login_web_fails_when_probe_rejects_token() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    let port = free_port();
    server.mock(|when, then| {
        when.method(POST).path("/oauth/token");
        then.status(200).json_body(json!({
            "access_token": "access-1",
            "token_type": "Bearer",
            "expires_in": 1800,
            "scope": ["TX_READ"],
            "refresh_token": "refresh-1"
        }));
    });
    probe_mock(&server, "UnauthorizedError");

    let (child, mut reader) = spawn_login(dir.path(), &server, port, &[]);
    let url = read_authorize_url(&mut reader);
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let (status, _) = callback(port, &format!("code=abc&state={}", params["state"]));
    assert_eq!(status, 200);

    let (ok, stderr) = finish(child, &mut reader);
    assert!(!ok);
    assert!(
        stderr.contains("console and API environments match"),
        "{stderr}"
    );
    assert!(!dir.path().join("config.toml").exists());
}

#[test]
fn login_web_fails_fast_when_port_is_busy() {
    let dir = tempfile::tempdir().unwrap();
    let server = MockServer::start();
    let taken = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();

    let (child, mut reader) = spawn_login(dir.path(), &server, port, &[]);
    let (ok, stderr) = finish(child, &mut reader);
    assert!(!ok);
    assert!(stderr.contains(&format!("port {port}")), "{stderr}");
    assert!(!stderr.contains("PORTONE_OAUTH_REDIRECT_URI"), "{stderr}");
}
