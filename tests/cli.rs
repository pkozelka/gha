use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn runs_with_name() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["run", "--name", "Alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, Alice!"));
}

#[test]
fn fails_without_command() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No command provided"));
}
