use std::io::Write;

use serde_json::Value;

use crate::http::cache::CachedResponse;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn reason(&self) -> &'static str {
        ureq::http::StatusCode::from_u16(self.status)
            .ok()
            .and_then(|s| s.canonical_reason())
            .unwrap_or("")
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn is_json(&self) -> bool {
        self.header("content-type")
            .is_some_and(is_json_content_type)
    }

    pub fn to_cached(&self) -> CachedResponse {
        CachedResponse {
            status: self.status,
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
}

impl From<CachedResponse> for HttpResponse {
    fn from(cached: CachedResponse) -> Self {
        HttpResponse {
            status: cached.status,
            headers: cached.headers,
            body: cached.body,
        }
    }
}

pub fn is_json_content_type(content_type: &str) -> bool {
    let bytes = content_type.as_bytes();
    let mut start = 0;
    while let Some(pos) = content_type[start..].find("json") {
        let i = start + pos;
        let prev_ok = i > 0 && matches!(bytes[i - 1], b'/' | b'+');
        let end = i + 4;
        let next_ok = end == bytes.len() || bytes[end] == b';';
        if prev_ok && next_ok {
            return true;
        }
        start = i + 1;
    }
    false
}

pub fn parse_error_message(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let obj = value.as_object()?;
    let message = obj
        .get("message")
        .and_then(Value::as_str)
        .filter(|m| !m.is_empty());
    let ty = obj
        .get("type")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty());
    message.or(ty).map(str::to_string)
}

pub fn write_headers(w: &mut dyn Write, resp: &HttpResponse, color: bool) -> std::io::Result<()> {
    writeln!(w, "HTTP/1.1 {} {}", resp.status, resp.reason())?;

    let mut merged: Vec<(String, Vec<&str>)> = Vec::new();
    for (name, value) in &resp.headers {
        let canon = canonical_header_name(name);
        match merged.iter_mut().find(|(n, _)| *n == canon) {
            Some((_, values)) => values.push(value),
            None => merged.push((canon, vec![value])),
        }
    }
    merged.sort_by(|a, b| a.0.cmp(&b.0));

    let (c0, c1) = if color {
        ("\x1b[1;34m", "\x1b[m")
    } else {
        ("", "")
    };
    for (name, values) in merged {
        write!(w, "{c0}{name}{c1}: {}\r\n", values.join(", "))?;
    }
    write!(w, "\r\n")
}

fn canonical_header_name(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_content_types() {
        assert!(is_json_content_type("application/json"));
        assert!(is_json_content_type("application/json; charset=utf-8"));
        assert!(is_json_content_type("application/vnd.portone+json"));
        assert!(!is_json_content_type("text/plain"));
        assert!(!is_json_content_type("application/jsonx"));
        assert!(!is_json_content_type("json"));
    }

    #[test]
    fn error_message_prefers_message_over_type() {
        let body = r#"{"type":"PAYMENT_NOT_FOUND","message":"Payment not found"}"#.as_bytes();
        assert_eq!(
            parse_error_message(body).as_deref(),
            Some("Payment not found")
        );
    }

    #[test]
    fn error_message_falls_back_to_type() {
        let body = br#"{"type":"UNAUTHORIZED"}"#;
        assert_eq!(parse_error_message(body).as_deref(), Some("UNAUTHORIZED"));
    }

    #[test]
    fn error_message_none_for_invalid_or_foreign_bodies() {
        assert_eq!(parse_error_message(b"not json"), None);
        assert_eq!(parse_error_message(br#"[1,2,3]"#), None);
        assert_eq!(parse_error_message(br#"{"error":"x"}"#), None);
    }

    #[test]
    fn write_headers_sorts_and_merges() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![
                ("x-b".to_string(), "2".to_string()),
                ("x-a".to_string(), "1".to_string()),
                ("x-b".to_string(), "3".to_string()),
            ],
            body: Vec::new(),
        };
        let mut out = Vec::new();
        write_headers(&mut out, &resp, false).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "HTTP/1.1 200 OK\nX-A: 1\r\nX-B: 2, 3\r\n\r\n"
        );
    }

    #[test]
    fn cached_roundtrip() {
        let resp = HttpResponse {
            status: 201,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: b"{}".to_vec(),
        };
        let back: HttpResponse = resp.to_cached().into();
        assert_eq!(back.status, 201);
        assert!(back.is_json());
        assert_eq!(back.body, b"{}");
    }
}
