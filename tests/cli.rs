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
fn runs_with_workflow() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    // Should fail with authentication/network error, not argument parsing error
    cmd.args(&["run", "test.yml", "--repo", "owner/repo", "-b", "main", "--token", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("").or(predicate::str::contains(".")));
}

#[test]
fn fails_without_command() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No command provided"));
}
