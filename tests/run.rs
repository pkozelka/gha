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
        "test.yml",
        "--repo",
        "owner/repo",
        "-b",
        "main",
        "--token",
        "test-token",
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
        .stderr(predicate::str::contains("required").or(predicate::str::contains("WORKFLOW")));
}

#[test]
fn run_command_help_shows_positional_workflow() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<WORKFLOW>"))
        .stdout(predicate::str::contains("-b, --ref"))
        .stdout(predicate::str::contains("--timeout"))
        .stdout(predicate::str::contains("--poll-interval"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--webhook"))
        .stdout(predicate::str::contains("--follow-logs"));
}

#[test]
fn run_command_rejects_follow_logs_without_wait() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&[
        "run",
        "test.yml",
        "--repo",
        "owner/repo",
        "-b",
        "main",
        "--token",
        "test-token",
        "--follow-logs",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("requires --wait"));
}


