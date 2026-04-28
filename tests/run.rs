use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn run_command_shows_help() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run a GitHub Actions workflow"));
}

#[test]
fn run_command_validates_inputs() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&[
        "run",
        "--workflow",
        "test.yml",
        "--repo",
        "owner/repo",
        "--ref",
        "main",
        "--token",
        "test-token",
        "--arg",
        "invalid_format",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Invalid input format"));
}

#[test]
fn run_command_requires_workflow() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("workflow")));
}

