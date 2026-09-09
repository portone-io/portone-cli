//! Exercise the real compiler, including schema-only incremental rebuilds.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"));
        for directory in ["macros/src", "macros/schema", "consumer/src"] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        for path in ["Cargo.toml", "src/lib.rs", "src/expand.rs"] {
            fs::copy(source.join(path), root.join("macros").join(path)).unwrap();
        }
        fs::copy(source.join("../../Cargo.lock"), root.join("Cargo.lock")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[workspace]
members = ["macros", "consumer"]
resolver = "3"
[workspace.package]
edition = "2024"
license = "MIT"
"#,
        )
        .unwrap();
        fs::write(
            root.join("consumer/Cargo.toml"),
            r#"
[package]
name = "schema-consumer"
version = "0.0.0"
edition = "2024"
[dependencies]
portone-schema-macros = { path = "../macros" }
clap = { version = "4.6.6", features = ["derive"] }
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
"#,
        )
        .unwrap();
        Self { _dir: dir, root }
    }

    fn source(&self, source: &str) {
        fs::write(self.root.join("consumer/src/main.rs"), source).unwrap();
    }

    fn schema(&self, schema: &Value) {
        fs::write(
            self.root.join("macros/schema/v2.openapi.json"),
            serde_json::to_vec(schema).unwrap(),
        )
        .unwrap();
    }

    fn cargo(&self, command: &str) -> Output {
        Command::new(env!("CARGO"))
            .args([
                command,
                "--offline",
                "--message-format=json",
                "-p",
                "schema-consumer",
            ])
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", self.root.join("target"))
            .env("CARGO_TERM_COLOR", "never")
            .output()
            .unwrap()
    }

    fn run(&self) -> String {
        let build = self.cargo("build");
        assert!(
            build.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let output = Command::new(
            self.root
                .join("target/debug")
                .join(format!("schema-consumer{}", std::env::consts::EXE_SUFFIX)),
        )
        .output()
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn rejects(&self, invocation: &str, expected: &str) {
        self.source(&format!(
            "use portone_schema_macros::schema_enum;\n{invocation}\nfn main() {{}}\n"
        ));
        let output = self.cargo("check");
        assert!(
            !output.status.success(),
            "unexpectedly accepted {invocation}"
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let diagnostic = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|message| {
                message["reason"] == "compiler-message"
                    && message["message"]["level"] == "error"
                    && message["message"]["message"]
                        .as_str()
                        .is_some_and(|text| text.contains(expected))
            })
            .unwrap_or_else(|| {
                panic!(
                    "missing `{expected}` in {stdout}\n{}",
                    String::from_utf8_lossy(&output.stderr)
                )
            });
        assert!(
            diagnostic["message"]["spans"]
                .as_array()
                .unwrap()
                .iter()
                .any(|span| span["is_primary"] == true && span["line_start"] == 2),
            "{diagnostic}"
        );
    }
}

#[test]
fn schema_enums_compile_and_schema_edits_invalidate_expansion() {
    let project = Project::new();
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/v2.openapi.json")).unwrap();
    let mut declarations =
        String::from("use clap::ValueEnum;\nuse portone_schema_macros::schema_enum;\n");
    let mut checks = String::new();
    // Discover types from the input, so additions outside the CLI's current
    // selections are also compiled and checked without a test-side allowlist.
    for (name, definition) in schema["components"]["schemas"].as_object().unwrap() {
        if definition["type"] != "string" || definition.get("enum").is_none() {
            continue;
        }
        writeln!(declarations, "schema_enum!(pub {name});").unwrap();
        let expected = serde_json::to_string(&definition["enum"]).unwrap();
        writeln!(
            checks,
            r#"
    let expected: Vec<String> = serde_json::from_str({expected:?}).unwrap();
    assert_eq!({name}::value_variants().len(), expected.len());
    for (variant, wire) in {name}::value_variants().iter().zip(expected) {{
        assert_eq!(variant.as_api_str(), wire);
        assert_eq!(serde_json::to_value(variant).unwrap(), wire);
        let cli = wire.replace('_', "-").to_ascii_lowercase();
        assert_eq!({name}::from_str(&cli, false).unwrap(), *variant);
    }}
"#
        )
        .unwrap();
    }
    declarations.push_str("schema_enum!(pub Incremental);\n");
    project.source(&format!(r#"{declarations}
fn main() {{
{checks}
    println!("{{}}", Incremental::value_variants().iter().map(Incremental::as_api_str).collect::<Vec<_>>().join(","));
}}
"#));
    schema["components"]["schemas"]["Incremental"] = json!({"type":"string","enum":["READY"]});
    schema["components"]["schemas"]["Collision"] = json!({"type":"string","enum":["A_B","A-B"]});
    project.schema(&schema);
    assert_eq!(project.run(), "READY\n");

    // Leave consumer source untouched: only the embedded schema changes.
    schema["components"]["schemas"]["Incremental"]["enum"] = json!(["FUTURE_VALUE"]);
    project.schema(&schema);
    assert_eq!(project.run(), "FUTURE_VALUE\n");

    project.rejects(
        "schema_enum!(pub MissingSchema);",
        "schema type `MissingSchema` does not exist",
    );
    project.rejects(
        "schema_enum!(pub PaymentFilterInput);",
        "must be a direct string enum",
    );
    project.rejects("schema_enum!(pub Collision);", "conflicting Rust variant");
    project.rejects(
        "schema_enum!(pub Currency, cli_case = \"invalid\");",
        "expected `kebab-case` or `preserve`",
    );
}
