use std::io::{BufWriter, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

enum Output<'a> {
    Pager {
        child: Child,
        stdin: Option<BufWriter<ChildStdin>>,
    },
    Passthrough(&'a mut dyn Write),
}

pub struct Pager<'a> {
    output: Output<'a>,
}

impl<'a> Pager<'a> {
    pub fn start(
        out: &'a mut dyn Write,
        err: &mut dyn Write,
        tty: bool,
        enabled: bool,
    ) -> Pager<'a> {
        if enabled && tty {
            let command = resolve_command(
                std::env::var("PORTONE_PAGER").ok(),
                std::env::var("PAGER").ok(),
            );
            if let Some(command) = command {
                match spawn_pager(&command) {
                    Ok((child, stdin)) => {
                        return Pager {
                            output: Output::Pager {
                                child,
                                stdin: Some(BufWriter::new(stdin)),
                            },
                        };
                    }
                    Err(e) => {
                        let _ = writeln!(err, "failed to start pager: {e}");
                    }
                }
            }
        }
        Pager {
            output: Output::Passthrough(out),
        }
    }

    pub fn writer(&mut self) -> &mut dyn Write {
        match &mut self.output {
            Output::Pager { stdin, .. } => stdin.as_mut().expect("pager already finished"),
            Output::Passthrough(out) => &mut **out,
        }
    }

    pub fn finish(&mut self) -> std::io::Result<()> {
        match &mut self.output {
            Output::Pager { child, stdin } => {
                let flushed = match stdin.take() {
                    Some(mut writer) => writer.flush(),
                    None => Ok(()),
                };
                let waited = child.wait();
                flushed?;
                waited.map(|_| ())
            }
            Output::Passthrough(out) => out.flush(),
        }
    }
}

impl Write for Pager<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer().flush()
    }
}

fn resolve_command(portone_pager: Option<String>, pager: Option<String>) -> Option<String> {
    let command = match portone_pager {
        Some(value) => value,
        None => pager.unwrap_or_default(),
    };
    if command.is_empty() || command == "cat" {
        None
    } else {
        Some(command)
    }
}

fn spawn_pager(pager_command: &str) -> std::io::Result<(Child, ChildStdin)> {
    let invalid =
        |msg: &str| std::io::Error::new(std::io::ErrorKind::InvalidInput, msg.to_string());
    let words = split_shell_words(pager_command).ok_or_else(|| invalid("invalid pager command"))?;
    let Some((program, args)) = words.split_first() else {
        return Err(invalid("empty pager command"));
    };
    let mut command = Command::new(program);
    command.args(args);
    command.env_remove("PAGER");
    if std::env::var_os("LESS").is_none() {
        command.env("LESS", "FRX");
    }
    if std::env::var_os("LV").is_none() {
        command.env("LV", "-c");
    }
    command.stdin(Stdio::piped());
    let mut child = command.spawn()?;
    let stdin = child.stdin.take().expect("piped stdin");
    Ok((child, stdin))
}

fn split_shell_words(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => current.push(ch),
                        None => return None,
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(escaped @ ('"' | '\\' | '$' | '`')) => current.push(escaped),
                            Some(other) => {
                                current.push('\\');
                                current.push(other);
                            }
                            None => return None,
                        },
                        Some(ch) => current.push(ch),
                        None => return None,
                    }
                }
            }
            '\\' => {
                in_word = true;
                current.push(chars.next()?);
            }
            ch => {
                in_word = true;
                current.push(ch);
            }
        }
    }
    if in_word {
        words.push(current);
    }
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(value: &str) -> Option<String> {
        Some(value.to_string())
    }

    #[test]
    fn resolve_prefers_portone_pager() {
        assert_eq!(
            resolve_command(some("less -R"), some("more")),
            some("less -R")
        );
    }

    #[test]
    fn resolve_falls_back_to_pager() {
        assert_eq!(resolve_command(None, some("more")), some("more"));
    }

    #[test]
    fn resolve_empty_portone_pager_disables_without_fallback() {
        assert_eq!(resolve_command(some(""), some("less")), None);
    }

    #[test]
    fn resolve_cat_and_unset_disable() {
        assert_eq!(resolve_command(None, None), None);
        assert_eq!(resolve_command(some("cat"), some("less")), None);
        assert_eq!(resolve_command(None, some("cat")), None);
        assert_eq!(resolve_command(None, some("")), None);
    }

    #[test]
    fn non_tty_is_noop() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut pager = Pager::start(&mut out, &mut err, false, true);
        assert!(matches!(pager.output, Output::Passthrough(_)));
        assert!(pager.finish().is_ok());
        assert!(err.is_empty());
    }

    #[test]
    fn disabled_is_passthrough_on_tty_without_env_lookup() {
        crate::config::paths::with_env(
            &[("PORTONE_PAGER", Some("definitely-missing-pager-xyz"))],
            || {
                let mut out = Vec::new();
                let mut err = Vec::new();
                let mut pager = Pager::start(&mut out, &mut err, true, false);
                assert!(matches!(pager.output, Output::Passthrough(_)));
                assert!(pager.finish().is_ok());
                assert!(err.is_empty());
            },
        );
    }

    #[test]
    fn passthrough_writes_reach_target() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut pager = Pager::start(&mut out, &mut err, false, true);
        pager.writer().write_all(b"body").unwrap();
        pager.finish().unwrap();
        drop(pager);
        assert_eq!(out, b"body");
    }

    #[test]
    fn split_shell_words_handles_quotes_and_escapes() {
        assert_eq!(
            split_shell_words("less -R"),
            Some(vec!["less".to_string(), "-R".to_string()])
        );
        assert_eq!(
            split_shell_words(r#"delta --pager 'less -R'"#),
            Some(vec![
                "delta".to_string(),
                "--pager".to_string(),
                "less -R".to_string()
            ])
        );
        assert_eq!(
            split_shell_words(r#""my pager" a\ b"#),
            Some(vec!["my pager".to_string(), "a b".to_string()])
        );
        assert_eq!(split_shell_words("'unclosed"), None);
        assert_eq!(split_shell_words("   "), Some(vec![]));
    }

    #[test]
    fn spawn_fails_immediately_for_missing_command() {
        assert!(spawn_pager("definitely-missing-pager-xyz").is_err());
        assert!(spawn_pager("'unclosed").is_err());
        assert!(spawn_pager("").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn spawn_write_finish() {
        let (child, stdin) = spawn_pager("cat").unwrap();
        let mut pager = Pager {
            output: Output::Pager {
                child,
                stdin: Some(BufWriter::new(stdin)),
            },
        };
        pager.writer().write_all("pager test\n".as_bytes()).unwrap();
        pager.finish().unwrap();
        pager.finish().unwrap();
    }
}
