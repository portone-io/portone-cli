use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use url::Url;

const REQUEST_LINE_LIMIT: usize = 8 * 1024;
const HEADER_LIMIT: usize = 64 * 1024;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub struct CallbackServer {
    listener: TcpListener,
    path: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callback {
    Code(String),
    Denied {
        error: String,
        description: Option<String>,
    },
}

impl CallbackServer {
    pub fn bind(redirect_uri: &Url) -> anyhow::Result<Self> {
        let host = redirect_uri.host_str().unwrap_or_default();
        if !matches!(host, "127.0.0.1" | "localhost") {
            bail!("redirect URI host must be 127.0.0.1 or localhost: {redirect_uri}");
        }
        let port = redirect_uri
            .port()
            .with_context(|| format!("redirect URI is missing a port: {redirect_uri}"))?;
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|err| {
            if err.kind() == std::io::ErrorKind::AddrInUse {
                anyhow!(
                    "port {port} is already in use; stop any other portone login or MCP server using it, then try again"
                )
            } else {
                anyhow!("failed to start callback server on 127.0.0.1:{port}: {err}")
            }
        })?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            path: redirect_uri.path().to_string(),
            port,
        })
    }

    pub fn wait(
        &self,
        expected_state: &str,
        timeout: Duration,
        err: &mut dyn Write,
    ) -> anyhow::Result<Callback> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(callback) = self.handle(stream, expected_state, err) {
                        return Ok(callback);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        bail!(
                            "timed out after waiting {} minutes for console login",
                            timeout.as_secs().div_ceil(60)
                        );
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(e) => return Err(e).context("failed to accept callback connection"),
            }
        }
    }

    fn handle(
        &self,
        mut stream: TcpStream,
        expected_state: &str,
        err: &mut dyn Write,
    ) -> Option<Callback> {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(CONNECTION_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CONNECTION_TIMEOUT));

        let request_line = match read_request_line(&mut stream) {
            Ok(line) => line,
            Err(_) => {
                respond(
                    &mut stream,
                    400,
                    "Bad Request",
                    &page("Invalid request", ""),
                );
                return None;
            }
        };
        let Some((method, target)) = parse_request_line(&request_line) else {
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page("Invalid request", ""),
            );
            return None;
        };
        if method != "GET" {
            respond(
                &mut stream,
                405,
                "Method Not Allowed",
                &page("Request not allowed", ""),
            );
            return None;
        }
        let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page("Invalid request", ""),
            );
            return None;
        };
        if url.path() != self.path {
            respond(&mut stream, 404, "Not Found", &page("Not found", ""));
            return None;
        }
        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
        if params.get("state").map(String::as_str) != Some(expected_state) {
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page(
                    "Unable to verify login request",
                    "The state value does not match. Restart login from the terminal.",
                ),
            );
            let _ = writeln!(err, "portone: ignored callback with mismatched state");
            return None;
        }
        if let Some(error) = params.get("error") {
            let description = params
                .get("error_description")
                .filter(|d| !d.is_empty())
                .cloned();
            respond(
                &mut stream,
                400,
                "Bad Request",
                &page(
                    "Login denied",
                    "Close this window and follow the instructions in your terminal.",
                ),
            );
            return Some(Callback::Denied {
                error: error.clone(),
                description,
            });
        }
        match params.get("code").filter(|code| !code.is_empty()) {
            Some(code) => {
                respond(
                    &mut stream,
                    200,
                    "OK",
                    &page(
                        "Login complete",
                        "Close this window and return to your terminal.",
                    ),
                );
                Some(Callback::Code(code.clone()))
            }
            None => {
                respond(
                    &mut stream,
                    400,
                    "Bad Request",
                    &page(
                        "Unable to verify login request",
                        "The code value is missing.",
                    ),
                );
                None
            }
        }
    }
}

fn read_request_line(stream: &mut TcpStream) -> anyhow::Result<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let line_end = loop {
        if let Some(pos) = find(&buf, b"\r\n") {
            break pos;
        }
        if buf.len() >= REQUEST_LINE_LIMIT {
            bail!("request line too long");
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            bail!("connection closed before request line");
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let line = String::from_utf8_lossy(&buf[..line_end]).into_owned();
    while find(&buf, b"\r\n\r\n").is_none() && buf.len() < HEADER_LIMIT {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    Ok(line)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split(' ');
    let method = parts.next()?;
    let target = parts.next()?;
    if method.is_empty() || !target.starts_with('/') {
        return None;
    }
    Some((method.to_string(), target.to_string()))
}

fn respond(stream: &mut TcpStream, status: u16, reason: &str, html: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

fn page(title: &str, detail: &str) -> String {
    format!(
        "<!doctype html><html lang=\"ko\"><head><meta charset=\"utf-8\"><title>{title} - PortOne CLI</title><style>body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:560px;margin:80px auto;padding:0 24px;color:#222}}h1{{font-size:20px}}p{{color:#555}}</style></head><body><h1>{title}</h1><p>{detail}</p></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};

    fn server(path: &str) -> CallbackServer {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        CallbackServer {
            listener,
            path: path.to_string(),
            port,
        }
    }

    fn send(port: u16, raw: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader.read_line(&mut status_line).unwrap();
        let status: u16 = status_line.split(' ').nth(1).unwrap().parse().unwrap();
        let mut body = String::new();
        let _ = reader.read_to_string(&mut body);
        (status, body)
    }

    fn wait_in_thread(
        server: CallbackServer,
        state: &str,
        timeout: Duration,
    ) -> std::thread::JoinHandle<(anyhow::Result<Callback>, String)> {
        let state = state.to_string();
        std::thread::spawn(move || {
            let mut err = Vec::new();
            let result = server.wait(&state, timeout, &mut err);
            (result, String::from_utf8_lossy(&err).into_owned())
        })
    }

    #[test]
    fn bind_rejects_non_loopback_and_missing_port() {
        assert!(CallbackServer::bind(&Url::parse("http://example.com:1271/cb").unwrap()).is_err());
        assert!(CallbackServer::bind(&Url::parse("http://127.0.0.1/cb").unwrap()).is_err());
    }

    #[test]
    fn bind_reports_port_in_use() {
        let taken = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = taken.local_addr().unwrap().port();
        let url = Url::parse(&format!("http://127.0.0.1:{port}/oauth/cli")).unwrap();
        let err = CallbackServer::bind(&url).unwrap_err().to_string();
        assert!(err.contains(&format!("port {port}")), "{err}");
        assert!(!err.contains("PORTONE_OAUTH_REDIRECT_URI"));
    }

    #[test]
    fn valid_callback_returns_code() {
        let s = server("/oauth/cli");
        let port = s.port;
        let handle = wait_in_thread(s, "st", Duration::from_secs(5));
        let (status, body) = send(
            port,
            "GET /oauth/cli?code=abc&state=st HTTP/1.1\r\nHost: x\r\n\r\n",
        );
        assert_eq!(status, 200);
        assert!(body.contains("Login complete"));
        let (result, err) = handle.join().unwrap();
        assert_eq!(result.unwrap(), Callback::Code("abc".to_string()));
        assert!(err.is_empty());
    }

    #[test]
    fn wrong_state_is_rejected_then_correct_one_accepted() {
        let s = server("/oauth/cli");
        let port = s.port;
        let handle = wait_in_thread(s, "good", Duration::from_secs(5));
        let (status, _) = send(port, "GET /oauth/cli?code=evil&state=bad HTTP/1.1\r\n\r\n");
        assert_eq!(status, 400);
        let (status, _) = send(port, "GET /oauth/cli?code=ok&state=good HTTP/1.1\r\n\r\n");
        assert_eq!(status, 200);
        let (result, err) = handle.join().unwrap();
        assert_eq!(result.unwrap(), Callback::Code("ok".to_string()));
        assert!(err.contains("callback with mismatched state"));
    }

    #[test]
    fn path_mismatch_method_and_oversized_requests_keep_waiting() {
        let s = server("/oauth/cli");
        let port = s.port;
        let handle = wait_in_thread(s, "st", Duration::from_secs(5));
        let (status, _) = send(port, "GET /favicon.ico HTTP/1.1\r\n\r\n");
        assert_eq!(status, 404);
        let (status, _) = send(port, "POST /oauth/cli?code=a&state=st HTTP/1.1\r\n\r\n");
        assert_eq!(status, 405);
        let long = format!("GET /oauth/cli?x={} HTTP/1.1\r\n\r\n", "a".repeat(9000));
        let (status, _) = send(port, &long);
        assert_eq!(status, 400);
        let (status, _) = send(port, "GET /oauth/cli?state=st HTTP/1.1\r\n\r\n");
        assert_eq!(status, 400);
        let (status, _) = send(port, "GET /oauth/cli?code=fin&state=st HTTP/1.1\r\n\r\n");
        assert_eq!(status, 200);
        let (result, _) = handle.join().unwrap();
        assert_eq!(result.unwrap(), Callback::Code("fin".to_string()));
    }

    #[test]
    fn error_parameter_returns_denied() {
        let s = server("/oauth/cli");
        let port = s.port;
        let handle = wait_in_thread(s, "st", Duration::from_secs(5));
        let (status, _) = send(
            port,
            "GET /oauth/cli?error=access_denied&error_description=nope&state=st HTTP/1.1\r\n\r\n",
        );
        assert_eq!(status, 400);
        let (result, _) = handle.join().unwrap();
        assert_eq!(
            result.unwrap(),
            Callback::Denied {
                error: "access_denied".to_string(),
                description: Some("nope".to_string()),
            }
        );
    }

    #[test]
    fn times_out_without_callback() {
        let s = server("/oauth/cli");
        let handle = wait_in_thread(s, "st", Duration::from_millis(200));
        let (result, _) = handle.join().unwrap();
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timed out"), "{err}");
    }

    #[test]
    fn silent_client_does_not_block_forever() {
        let s = server("/oauth/cli");
        let port = s.port;
        let handle = wait_in_thread(s, "st", Duration::from_secs(20));
        let idle = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let started = Instant::now();
        std::thread::sleep(Duration::from_millis(200));
        let (status, _) = send(port, "GET /oauth/cli?code=late&state=st HTTP/1.1\r\n\r\n");
        assert_eq!(status, 200);
        assert!(started.elapsed() < Duration::from_secs(10));
        drop(idle);
        let (result, _) = handle.join().unwrap();
        assert_eq!(result.unwrap(), Callback::Code("late".to_string()));
    }
}
