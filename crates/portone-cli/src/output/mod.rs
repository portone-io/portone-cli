pub mod jsoncolor;
pub mod resource;

use std::io::Write;

use anyhow::anyhow;
use jaq_json::Val;
use serde_json::Value;

use crate::error::CliError;

type JqData = jaq_core::data::JustLut<Val>;
type JqFilter = jaq_core::Filter<JqData>;

pub struct Pipeline {
    jq: Option<JqFilter>,
    slurp_pages: Option<Vec<Value>>,
    color: bool,
    tty: bool,
}

impl Pipeline {
    pub fn new(jq: Option<&str>, slurp: bool, color: bool, tty: bool) -> anyhow::Result<Pipeline> {
        Ok(Pipeline {
            jq: jq.map(compile_jq).transpose()?,
            slurp_pages: slurp.then(Vec::new),
            color,
            tty,
        })
    }

    pub fn emit_json(&mut self, w: &mut dyn Write, bytes: &[u8]) -> Result<(), CliError> {
        if let Some(filter) = &self.jq {
            run_jq(filter, w, bytes, self.color, self.tty)
        } else if let Some(pages) = &mut self.slurp_pages {
            let value: Value = serde_json::from_slice(bytes).map_err(|e| {
                CliError::Other(anyhow!(crate::message!("core-response-parse", error = e)))
            })?;
            pages.push(value);
            Ok(())
        } else {
            emit_json_plain(w, bytes, self.color)
        }
    }

    pub fn finish(&mut self, w: &mut dyn Write) -> Result<(), CliError> {
        let Some(pages) = self.slurp_pages.take() else {
            return Ok(());
        };
        let array = Value::Array(pages);
        if self.color {
            jsoncolor::write_colored(w, &array, "  ", 0)?;
        } else if self.tty {
            serde_json::to_writer_pretty(&mut *w, &array).map_err(|e| CliError::Other(e.into()))?;
            writeln!(w)?;
        } else {
            serde_json::to_writer(&mut *w, &array).map_err(|e| CliError::Other(e.into()))?;
            writeln!(w)?;
        }
        Ok(())
    }
}

pub fn emit_json_plain(w: &mut dyn Write, bytes: &[u8], color: bool) -> Result<(), CliError> {
    if color {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| {
            CliError::Other(anyhow!(crate::message!("core-response-parse", error = e)))
        })?;
        jsoncolor::write_colored(w, &value, "  ", 0)?;
    } else {
        w.write_all(bytes)?;
    }
    Ok(())
}

pub fn emit_raw(
    w: &mut dyn Write,
    bytes: &[u8],
    tty: bool,
    allow_escape: bool,
) -> Result<(), CliError> {
    if !allow_escape {
        let head = &bytes[..bytes.len().min(512)];
        if head.contains(&0) {
            if tty {
                return Err(CliError::Other(anyhow!(crate::message!(
                    "core-output-binary"
                ))));
            }
        } else if bytes.contains(&0x1B) {
            return Err(CliError::Other(anyhow!(crate::message!(
                "core-output-escapes"
            ))));
        }
    }
    w.write_all(bytes)?;
    Ok(())
}

pub fn escape_controls(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        // Preserve ordinary text formatting, including separators between errors.
        if ch.is_control() && !matches!(ch, '\n' | '\t') {
            escaped.extend(ch.escape_default());
        } else {
            escaped.push(ch);
        }
    }
    escaped
}

fn compile_jq(code: &str) -> anyhow::Result<JqFilter> {
    use jaq_core::load::{Arena, File, Loader};

    let program = File { code, path: () };
    let loader = Loader::new(
        jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs()),
    );
    let arena = Arena::default();
    let modules = loader.load(&arena, program).map_err(|errs| {
        anyhow!(crate::message!(
            "core-jq-invalid",
            error = format!("{errs:?}")
        ))
    })?;
    jaq_core::Compiler::default()
        .with_funs(
            jaq_core::funs()
                .chain(jaq_std::funs())
                .chain(jaq_json::funs()),
        )
        .compile(modules)
        .map_err(|errs| {
            anyhow!(crate::message!(
                "core-jq-invalid",
                error = format!("{errs:?}")
            ))
        })
}

fn run_jq(
    filter: &JqFilter,
    w: &mut dyn Write,
    bytes: &[u8],
    color: bool,
    tty: bool,
) -> Result<(), CliError> {
    let input = jaq_json::read::parse_single(bytes)
        .map_err(|e| CliError::Other(anyhow!(crate::message!("core-response-parse", error = e))))?;
    let ctx = jaq_core::Ctx::<JqData>::new(&filter.lut, jaq_core::Vars::new([]));
    for result in filter.id.run((ctx, input)).map(jaq_core::unwrap_valr) {
        let val = result.map_err(|e| CliError::Other(anyhow!("jq: {e}")))?;
        emit_jq_val(w, &val, color, tty)?;
    }
    Ok(())
}

fn emit_jq_val(w: &mut dyn Write, val: &Val, color: bool, tty: bool) -> Result<(), CliError> {
    match val {
        Val::TStr(bytes) | Val::BStr(bytes) => {
            w.write_all(bytes.as_ref())?;
            writeln!(w)?;
        }
        Val::Null => writeln!(w)?,
        Val::Bool(_) | Val::Num(_) => writeln!(w, "{val}")?,
        Val::Arr(_) | Val::Obj(_) => {
            if color || tty {
                let value: Value = serde_json::from_str(&val.to_string()).map_err(|e| {
                    CliError::Other(anyhow!(crate::message!("core-jq-render", error = e)))
                })?;
                if color {
                    jsoncolor::write_colored(w, &value, "  ", 0)?;
                } else {
                    serde_json::to_writer_pretty(&mut *w, &value)
                        .map_err(|e| CliError::Other(e.into()))?;
                    writeln!(w)?;
                }
            } else {
                writeln!(w, "{val}")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_controls_preserves_text_formatting_but_escapes_terminal_controls() {
        assert_eq!(
            escape_controls("first\nsecond\tCafé\0\r\u{1b}\u{7}\u{7f}\u{9b}"),
            "first\nsecond\tCafé\\u{0}\\r\\u{1b}\\u{7}\\u{7f}\\u{9b}"
        );
    }

    fn run_pipeline(jq: Option<&str>, slurp: bool, pages: &[&str]) -> String {
        let mut pipeline = Pipeline::new(jq, slurp, false, false).unwrap();
        let mut out = Vec::new();
        for page in pages {
            pipeline.emit_json(&mut out, page.as_bytes()).unwrap();
        }
        pipeline.finish(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn jq_string_result_prints_raw() {
        assert_eq!(
            run_pipeline(Some(".name"), false, &[r#"{"name":"foo"}"#]),
            "foo\n"
        );
    }

    #[test]
    fn jq_null_result_prints_empty_line() {
        assert_eq!(run_pipeline(Some(".missing"), false, &[r#"{}"#]), "\n");
    }

    #[test]
    fn jq_scalar_results_print_to_string() {
        assert_eq!(run_pipeline(Some(".n"), false, &[r#"{"n":42}"#]), "42\n");
        assert_eq!(
            run_pipeline(Some(".b"), false, &[r#"{"b":true}"#]),
            "true\n"
        );
    }

    #[test]
    fn jq_non_scalar_prints_compact_json_when_piped() {
        assert_eq!(
            run_pipeline(Some(".items"), false, &[r#"{"items":[1,2]}"#]),
            "[1,2]\n"
        );
    }

    #[test]
    fn jq_iterates_multiple_results() {
        assert_eq!(
            run_pipeline(
                Some(".items[].id"),
                false,
                &[r#"{"items":[{"id":"a"},{"id":"b"}]}"#]
            ),
            "a\nb\n"
        );
    }

    #[test]
    fn invalid_jq_filter_fails_at_compile() {
        assert!(Pipeline::new(Some(".["), false, false, false).is_err());
    }

    #[test]
    fn slurp_wraps_pages_in_array() {
        assert_eq!(
            run_pipeline(None, true, &[r#"{"a":1}"#, r#"{"b":2}"#]),
            "[{\"a\":1},{\"b\":2}]\n"
        );
    }

    #[test]
    fn slurp_outputs_empty_array_without_pages() {
        assert_eq!(run_pipeline(None, true, &[]), "[]\n");
    }

    #[test]
    fn plain_json_passthrough_is_verbatim() {
        let mut out = Vec::new();
        emit_json_plain(&mut out, b"{\"a\": 1}", false).unwrap();
        assert_eq!(out, b"{\"a\": 1}");
    }

    #[test]
    fn raw_guard_rejects_escape_sequences_even_when_piped() {
        let mut out = Vec::new();
        let err = emit_raw(&mut out, b"hello \x1b[31mred", false, false).unwrap_err();
        assert!(out.is_empty());
        match err {
            CliError::Other(e) => assert!(e.to_string().contains("escape sequences")),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn raw_guard_rejects_binary_on_tty_only() {
        let mut out = Vec::new();
        assert!(emit_raw(&mut out, b"\x00binary", true, false).is_err());
        assert!(out.is_empty());

        let mut out = Vec::new();
        emit_raw(&mut out, b"\x00binary", false, false).unwrap();
        assert_eq!(out, b"\x00binary");
    }

    #[test]
    fn raw_guard_disabled_by_allow_flag() {
        let mut out = Vec::new();
        emit_raw(&mut out, b"\x1b[31mred", true, true).unwrap();
        assert_eq!(out, b"\x1b[31mred");
    }
}
