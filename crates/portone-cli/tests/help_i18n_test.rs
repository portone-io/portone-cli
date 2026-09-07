use assert_cmd::Command;
use clap::{CommandFactory, FromArgMatches};
use portone_cli::cmd::{self, Cli};
use portone_cli::i18n::Localizer;

fn portone(language: &str) -> Command {
    let mut command = Command::cargo_bin("portone").expect("portone binary not found");
    command.env("PORTONE_LANG", language).env("NO_COLOR", "1");
    command
}

fn help(language: &str, args: &[&str]) -> String {
    let output = portone(language)
        .args(args)
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn english_help_preserves_clap_output_for_every_command() {
    for path in [
        vec![],
        vec!["api"],
        vec!["auth"],
        vec!["auth", "login"],
        vec!["auth", "logout"],
        vec!["auth", "status"],
        vec!["auth", "token"],
        vec!["setup"],
        vec!["completion"],
    ] {
        for flag in ["-h", "--help"] {
            let args = [vec!["portone"], path.clone(), vec![flag]].concat();
            let expected = Cli::command()
                .color(clap::ColorChoice::Never)
                .try_get_matches_from(&args)
                .unwrap_err()
                .to_string();
            let actual = cmd::help::command(&Localizer::english())
                .color(clap::ColorChoice::Never)
                .try_get_matches_from(&args)
                .unwrap_err()
                .to_string();
            assert_eq!(actual, expected, "{args:?}");
        }
    }
}

#[test]
fn korean_help_translates_headings_flags_and_descriptions() {
    let root = help("ko", &["--help"]);
    assert!(root.contains("사용법: portone <COMMAND>"), "{root}");
    assert!(root.contains("명령어:"), "{root}");
    assert!(root.contains("옵션:"), "{root}");
    assert!(root.contains("도움말 출력"), "{root}");
    assert!(root.contains("버전 출력"), "{root}");
    assert!(root.contains("PortOne 인증 관리"), "{root}");
    assert!(!root.contains("Usage:"));
    assert!(!root.contains("Print help"));
    assert!(!root.contains("Print this message"));

    let api = help("ko", &["api", "--help"]);
    assert!(api.contains("인자:"), "{api}");
    assert!(api.contains("환경 변수:"), "{api}");
    assert!(api.contains("표준 입력"), "{api}");
    assert!(api.contains("# 결제 조회"), "{api}");
    assert!(api.contains("pageInfo { hasNextPage endCursor }"), "{api}");
    assert!(api.contains("<ENDPOINT>"), "{api}");
    assert!(!api.contains('\u{2068}'));
    assert!(!api.contains('\u{2069}'));
}

#[test]
fn korean_short_and_long_help_keep_their_distinct_behavior() {
    let short = help("ko", &["api", "-h"]);
    assert!(short.contains("'--help'로 자세히 보기"), "{short}");
    assert!(!short.contains("# 결제 조회"), "{short}");
    let long = help("ko", &["api", "--help"]);
    assert!(long.contains("'-h'로 요약 보기"), "{long}");
    assert!(long.contains("# 결제 조회"), "{long}");
}

#[test]
fn help_subcommands_render_the_selected_language() {
    for language in ["en", "ko"] {
        assert_eq!(
            help(language, &["help", "auth", "login"]),
            help(language, &["auth", "login", "--help"])
        );
        assert_eq!(
            help(language, &["auth", "help", "login"]),
            help(language, &["auth", "login", "--help"])
        );
    }
}

#[test]
fn korean_help_preserves_shell_values_and_localizes_their_label() {
    let output = help("ko", &["completion", "--help"]);
    assert!(
        output.contains("[가능한 값: bash, elvish, fish, powershell, zsh]"),
        "{output}"
    );
    assert!(output.contains("<SHELL>"), "{output}");
    assert!(!output.contains("possible values"), "{output}");
}

#[test]
fn clap_generated_argument_errors_remain_english() {
    let output = portone("ko")
        .args(["auth", "login", "--unknown"])
        .assert()
        .code(2)
        .get_output()
        .clone();
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("unexpected argument '--unknown' found"),
        "{error}"
    );
    assert!(error.contains("Usage:"), "{error}");
}

#[test]
fn localizers_preserve_typed_argument_parsing_and_remain_independent() {
    let english = Localizer::english();
    let korean = Localizer::korean();
    for localizer in [&korean, &english, &korean, &english] {
        let matches = cmd::help::command(localizer)
            .try_get_matches_from([
                "portone",
                "api",
                "/payments",
                "-X",
                "GET",
                "--profile",
                "merchant",
                "-f",
                "reason=Customer request",
            ])
            .unwrap();
        let cmd::Command::Api(args) = Cli::from_arg_matches(&matches).unwrap().command else {
            panic!("wrong command variant");
        };
        assert_eq!(args.endpoint, "/payments");
        assert_eq!(args.method.as_deref(), Some("GET"));
        assert_eq!(args.auth.profile.as_deref(), Some("merchant"));
        assert_eq!(args.raw_fields, ["reason=Customer request"]);

        let output = cmd::help::command(localizer).render_help().to_string();
        assert_eq!(output.contains("사용법:"), localizer.lang() == "ko");
    }
}

#[test]
fn completion_scripts_and_version_are_identical_across_languages() {
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        assert_eq!(
            help("ko", &["completion", shell]),
            help("en", &["completion", shell]),
            "{shell}"
        );
    }
    assert_eq!(help("ko", &["--version"]), help("en", &["--version"]));
}

#[test]
fn corrupt_config_or_invalid_path_does_not_prevent_help_version_or_completion() {
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(config_dir.path().join("config.toml"), "invalid = [").unwrap();
    let blocker = config_dir.path().join("not-a-directory");
    std::fs::write(&blocker, "").unwrap();
    for path in [config_dir.path(), blocker.as_path()] {
        for args in [
            vec!["--help"],
            vec!["auth", "login", "--help"],
            vec!["--version"],
            vec!["completion", "fish"],
        ] {
            portone("ko")
                .env_remove("PORTONE_LANG")
                .env_remove("LANGUAGE")
                .env("LC_ALL", "ko_KR.UTF-8")
                .env("PORTONE_CONFIG_DIR", path)
                .args(args)
                .assert()
                .success();
        }
    }
}
