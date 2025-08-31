use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help() {
    let mut cmd = Command::cargo_bin("gha").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn runs_with_name() {
    let mut cmd = Command::cargo_bin("gha").unwrap();
    cmd.args(&["run", "--name", "Alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Alice!"));
}

#[test]
fn fails_without_command() {
    let mut cmd = Command::cargo_bin("gha").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No command provided"));
}
