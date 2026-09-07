use std::process::{Command, Stdio};

use url::Url;

pub fn open(url: &str) -> std::io::Result<()> {
    let parsed = Url::parse(url).map_err(|err| invalid(format!("invalid URL {url}: {err}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(invalid(format!("refusing to open non-http URL: {url}")));
    }
    let (program, args) = launcher();
    Command::new(program)
        .args(args)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn launcher() -> (String, Vec<String>) {
    for name in ["PORTONE_BROWSER", "BROWSER"] {
        if let Some(command) = std::env::var(name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            let mut words = command.split_whitespace().map(str::to_string);
            if let Some(program) = words.next() {
                return (program, words.collect());
            }
        }
    }
    platform_launcher()
}

#[cfg(target_os = "macos")]
fn platform_launcher() -> (String, Vec<String>) {
    ("open".to_string(), Vec::new())
}

#[cfg(target_os = "windows")]
fn platform_launcher() -> (String, Vec<String>) {
    (
        "rundll32".to_string(),
        vec!["url.dll,FileProtocolHandler".to_string()],
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_launcher() -> (String, Vec<String>) {
    ("xdg-open".to_string(), Vec::new())
}

fn invalid(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::with_env;

    #[test]
    fn rejects_non_http_schemes() {
        let err = open("file:///etc/passwd").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(open("not a url").is_err());
    }

    #[test]
    fn env_launcher_splits_program_and_args() {
        with_env(
            &[
                ("PORTONE_BROWSER", Some("  my-browser --new-window ")),
                ("BROWSER", Some("ignored")),
            ],
            || {
                let (program, args) = launcher();
                assert_eq!(program, "my-browser");
                assert_eq!(args, vec!["--new-window".to_string()]);
            },
        );
        with_env(
            &[("PORTONE_BROWSER", None), ("BROWSER", Some("firefox"))],
            || {
                let (program, args) = launcher();
                assert_eq!(program, "firefox");
                assert!(args.is_empty());
            },
        );
    }

    #[test]
    fn missing_launcher_is_reported_not_panicked() {
        with_env(
            &[("PORTONE_BROWSER", Some("definitely-missing-browser-xyz"))],
            || {
                let err = open("https://example.com").unwrap_err();
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
            },
        );
    }
}
