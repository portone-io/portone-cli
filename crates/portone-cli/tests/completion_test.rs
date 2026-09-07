use assert_cmd::Command;
use predicates::prelude::*;

fn portone() -> Command {
    Command::cargo_bin("portone").expect("portone binary not found")
}

#[test]
fn zsh_script_starts_with_compdef() {
    portone()
        .arg("completion")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("#compdef portone"));
}

#[test]
fn bash_script_defines_portone_function() {
    portone()
        .arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_portone"));
}

#[test]
fn fish_script_completes_portone() {
    portone()
        .arg("completion")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c portone"));
}

#[test]
fn powershell_and_elvish_succeed() {
    portone()
        .arg("completion")
        .arg("powershell")
        .assert()
        .success();
    portone().arg("completion").arg("elvish").assert().success();
}

#[test]
fn unknown_shell_fails_with_usage_error() {
    portone().arg("completion").arg("nushell").assert().code(2);
}
