use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn spawn_command_shows_help() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["spawn", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Spawn a GitHub Actions workflow"));
}

#[test]
fn spawn_command_validates_inputs() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&[
        "spawn",
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
fn spawn_command_requires_workflow() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["spawn"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("WORKFLOW")));
}

#[test]
fn spawn_command_help_shows_positional_workflow() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&["spawn", "--help"])
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
fn spawn_command_rejects_follow_logs_without_await() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(&[
        "spawn",
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
    .stderr(predicate::str::contains("--await"));
}

#[test]
fn spawn_command_rejects_timeout_without_await() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(["spawn", "test.yml", "--timeout", "10"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--await"));
}

#[test]
fn spawn_command_rejects_webhook_without_await() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(["spawn", "test.yml", "--webhook"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--await"));
}


