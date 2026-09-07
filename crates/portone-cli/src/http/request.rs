use std::io::Read;

use anyhow::{Context, anyhow};

use crate::error::CliError;
use crate::http::response::HttpResponse;

pub fn resolve_method(
    explicit: Option<&str>,
    has_body: bool,
    paginate: bool,
    graphql: bool,
) -> String {
    match explicit {
        Some(method) => method.to_ascii_uppercase(),
        None if has_body && (graphql || !paginate) => "POST".to_string(),
        None => "GET".to_string(),
    }
}

pub fn build_url(base_url: &str, endpoint: &str) -> String {
    if endpoint.contains("://") {
        endpoint.to_string()
    } else {
        format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }
}

pub fn parse_headers(raw: &[String]) -> Result<Vec<(String, String)>, CliError> {
    let mut headers = Vec::with_capacity(raw.len());
    for h in raw {
        let Some(idx) = h.find(':') else {
            return Err(CliError::Flag(format!(
                "header {h:?} requires a value separated by ':'"
            )));
        };
        let name = h[..idx].trim().to_string();
        let value = h[idx + 1..].trim().to_string();
        if ureq::http::HeaderName::from_bytes(name.as_bytes()).is_err() {
            return Err(CliError::Flag(format!("invalid header name: {name:?}")));
        }
        if name.eq_ignore_ascii_case("content-length") {
            if value.parse::<u64>().is_err() {
                return Err(CliError::Flag(format!(
                    "invalid Content-Length value: {value:?}"
                )));
            }
            continue;
        }
        headers.push((name, value));
    }
    Ok(headers)
}

pub fn same_origin(a: &str, b: &str) -> bool {
    fn origin(url: &str) -> Option<(String, String, u16)> {
        let uri: ureq::http::Uri = url.parse().ok()?;
        let scheme = uri.scheme_str()?.to_ascii_lowercase();
        let host = uri.host()?.to_ascii_lowercase();
        let port = uri.port_u16().unwrap_or(match scheme.as_str() {
            "https" => 443,
            "http" => 80,
            _ => return None,
        });
        Some((scheme, host, port))
    }
    match (origin(a), origin(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

pub fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
}

pub fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub fn apply_default_headers(
    headers: &mut Vec<(String, String)>,
    json_body: bool,
    authorization: Option<String>,
) {
    if json_body && !has_header(headers, "content-type") {
        headers.push((
            "Content-Type".to_string(),
            "application/json; charset=utf-8".to_string(),
        ));
    }
    if !has_header(headers, "accept") {
        headers.push(("Accept".to_string(), "*/*".to_string()));
    }
    if !has_header(headers, "user-agent") {
        headers.push((
            "User-Agent".to_string(),
            concat!("portone-cli/", env!("CARGO_PKG_VERSION")).to_string(),
        ));
    }
    if let Some(value) = authorization {
        headers.push(("Authorization".to_string(), value));
    }
}

pub fn read_input(path: &str) -> anyhow::Result<Vec<u8>> {
    if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .context("failed to read request body from stdin")?;
        Ok(buf)
    } else {
        std::fs::read(path).with_context(|| format!("failed to read request body from {path}"))
    }
}

pub fn build_agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(None)
        .redirect_auth_headers(ureq::config::RedirectAuthHeaders::SameHost)
        .tls_config(tls)
        .build();
    ureq::Agent::new_with_config(config)
}

pub fn send(
    agent: &ureq::Agent,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> anyhow::Result<HttpResponse> {
    use ureq::http;

    let http_method = http::Method::from_bytes(method.as_bytes())
        .map_err(|_| anyhow!("invalid HTTP method: {method}"))?;
    let mut builder = http::Request::builder().method(http_method).uri(url);
    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    let response = match body {
        Some(bytes) => agent.run(
            builder
                .body(bytes.to_vec())
                .map_err(|e| anyhow!("failed to build request: {e}"))?,
        ),
        None => agent.run(
            builder
                .body(())
                .map_err(|e| anyhow!("failed to build request: {e}"))?,
        ),
    }?;

    let status = response.status().as_u16();
    let resp_headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let body = response
        .into_body()
        .into_with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .context("failed to read response body")?;

    Ok(HttpResponse {
        status,
        headers: resp_headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn method_defaults_to_get() {
        assert_eq!(resolve_method(None, false, false, false), "GET");
    }

    #[test]
    fn method_switches_to_post_with_body() {
        assert_eq!(resolve_method(None, true, false, false), "POST");
    }

    #[test]
    fn method_stays_get_with_body_when_paginating() {
        assert_eq!(resolve_method(None, true, true, false), "GET");
    }

    #[test]
    fn method_posts_for_graphql_even_when_paginating() {
        assert_eq!(resolve_method(None, true, true, true), "POST");
    }

    #[test]
    fn graphql_without_body_stays_get() {
        assert_eq!(resolve_method(None, false, false, true), "GET");
    }

    #[test]
    fn explicit_method_is_uppercased() {
        assert_eq!(resolve_method(Some("delete"), true, false, false), "DELETE");
    }

    #[test]
    fn build_url_joins_base_and_path() {
        assert_eq!(
            build_url("https://api.portone.io/", "/payments"),
            "https://api.portone.io/payments"
        );
        assert_eq!(
            build_url("https://api.portone.io", "payments"),
            "https://api.portone.io/payments"
        );
    }

    #[test]
    fn build_url_passes_through_full_url() {
        assert_eq!(
            build_url("https://api.portone.io", "https://example.com/x"),
            "https://example.com/x"
        );
    }

    #[test]
    fn parse_headers_splits_on_first_colon_and_trims_value() {
        let parsed = parse_headers(&strings(&["X-Foo:  bar ", "Accept: a:b"])).unwrap();
        assert_eq!(
            parsed,
            vec![
                ("X-Foo".to_string(), "bar".to_string()),
                ("Accept".to_string(), "a:b".to_string()),
            ]
        );
    }

    #[test]
    fn parse_headers_requires_colon() {
        let err = parse_headers(&strings(&["X-Foo bar"])).unwrap_err();
        match err {
            CliError::Flag(msg) => {
                assert_eq!(
                    msg,
                    "header \"X-Foo bar\" requires a value separated by ':'"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_headers_validates_and_drops_content_length() {
        let parsed = parse_headers(&strings(&["content-length: 12", "X-A: b"])).unwrap();
        assert_eq!(parsed, vec![("X-A".to_string(), "b".to_string())]);
        assert!(matches!(
            parse_headers(&strings(&["Content-Length: abc"])),
            Err(CliError::Flag(_))
        ));
        assert!(matches!(
            parse_headers(&strings(&["Content-Length: -5"])),
            Err(CliError::Flag(_))
        ));
    }

    #[test]
    fn parse_headers_trims_and_validates_names() {
        let parsed = parse_headers(&strings(&[" Authorization : token x"])).unwrap();
        assert_eq!(
            parsed,
            vec![("Authorization".to_string(), "token x".to_string())]
        );
        assert!(matches!(
            parse_headers(&strings(&["Bad Name: v"])),
            Err(CliError::Flag(_))
        ));
        assert!(matches!(
            parse_headers(&strings(&[": v"])),
            Err(CliError::Flag(_))
        ));
    }

    #[test]
    fn same_origin_compares_scheme_host_port() {
        assert!(same_origin(
            "https://api.portone.io/payments",
            "https://api.portone.io"
        ));
        assert!(same_origin(
            "https://API.portone.io:443/x",
            "https://api.portone.io/y"
        ));
        assert!(!same_origin(
            "https://evil.example.com/x",
            "https://api.portone.io"
        ));
        assert!(!same_origin(
            "http://api.portone.io/x",
            "https://api.portone.io"
        ));
        assert!(!same_origin(
            "https://api.portone.io:8443/x",
            "https://api.portone.io"
        ));
        assert!(!same_origin("not a url", "https://api.portone.io"));
    }

    #[test]
    fn default_headers_do_not_override_user_values() {
        let mut headers = vec![("accept".to_string(), "application/json".to_string())];
        apply_default_headers(&mut headers, true, Some("PortOne secret".to_string()));
        assert_eq!(header_value(&headers, "Accept"), Some("application/json"));
        assert_eq!(
            header_value(&headers, "content-type"),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(
            header_value(&headers, "authorization"),
            Some("PortOne secret")
        );
        assert!(
            header_value(&headers, "user-agent").is_some_and(|v| v.starts_with("portone-cli/"))
        );
    }

    #[test]
    fn content_type_only_added_for_json_body() {
        let mut headers = Vec::new();
        apply_default_headers(&mut headers, false, None);
        assert!(!has_header(&headers, "content-type"));
        assert!(has_header(&headers, "accept"));
    }
}
