use std::io::Write;

use crate::http::response::{HttpResponse, is_json_content_type};

pub fn log_request(
    w: &mut dyn Write,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> std::io::Result<()> {
    writeln!(w, "* Request to {url}")?;
    writeln!(w, "> {method} {url}")?;
    for (name, value) in headers {
        if name.trim().eq_ignore_ascii_case("authorization") {
            writeln!(w, "> {name}: {}", mask_authorization(value))?;
        } else {
            writeln!(w, "> {name}: {value}")?;
        }
    }
    writeln!(w)?;
    if let Some(bytes) = body {
        write_body(w, bytes, true)?;
    }
    Ok(())
}

pub fn log_response(w: &mut dyn Write, resp: &HttpResponse) -> std::io::Result<()> {
    writeln!(w, "< HTTP/1.1 {} {}", resp.status, resp.reason())?;
    for (name, value) in &resp.headers {
        writeln!(w, "< {name}: {value}")?;
    }
    writeln!(w)?;
    let texty = resp
        .header("content-type")
        .is_none_or(|ct| is_json_content_type(ct) || ct.starts_with("text/"));
    write_body(w, &resp.body, texty)
}

fn mask_authorization(value: &str) -> String {
    match value.split_whitespace().next() {
        Some(scheme) if value.trim().contains(char::is_whitespace) => format!("{scheme} ████"),
        _ => "████".to_string(),
    }
}

fn write_body(w: &mut dyn Write, bytes: &[u8], texty_hint: bool) -> std::io::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    let head = &bytes[..bytes.len().min(512)];
    if texty_hint && !head.contains(&0) {
        w.write_all(bytes)?;
        writeln!(w)?;
        writeln!(w)
    } else {
        writeln!(w, "* body of {} bytes omitted", bytes.len())?;
        writeln!(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_log_masks_authorization() {
        let mut out = Vec::new();
        let headers = vec![
            (
                "Authorization".to_string(),
                "PortOne secret-value".to_string(),
            ),
            ("Accept".to_string(), "*/*".to_string()),
        ];
        log_request(
            &mut out,
            "GET",
            "https://api.portone.io/payments",
            &headers,
            None,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("> Authorization: PortOne ████"));
        assert!(!text.contains("secret-value"));
        assert!(text.contains("> GET https://api.portone.io/payments"));
    }

    #[test]
    fn authorization_mask_keeps_scheme_only() {
        assert_eq!(mask_authorization("Bearer eyJhbGciOi"), "Bearer ████");
        assert_eq!(mask_authorization("PortOne sk_test"), "PortOne ████");
        assert_eq!(mask_authorization("opaque-token-without-scheme"), "████");
        assert_eq!(mask_authorization(""), "████");
    }

    #[test]
    fn response_log_includes_status_headers_and_json_body() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: br#"{"ok":true}"#.to_vec(),
        };
        let mut out = Vec::new();
        log_response(&mut out, &resp).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("< HTTP/1.1 200 OK"));
        assert!(text.contains("< content-type: application/json"));
        assert!(text.contains(r#"{"ok":true}"#));
    }

    #[test]
    fn response_log_omits_binary_body() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![(
                "content-type".to_string(),
                "application/octet-stream".to_string(),
            )],
            body: vec![0, 1, 2, 3],
        };
        let mut out = Vec::new();
        log_response(&mut out, &resp).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("* body of 4 bytes omitted"));
    }
}
