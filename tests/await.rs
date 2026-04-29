use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn await_command_help_shows_new_flags() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(["await", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--timeout"))
        .stdout(predicate::str::contains("--poll-interval"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--webhook"))
        .stdout(predicate::str::contains("--follow-logs"));
}

#[test]
fn await_command_rejects_invalid_output_value() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(["await", "--repo", "owner/repo", "123", "--output", "xml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("possible values"));
}

#[test]
fn await_command_parses_without_repo_flag() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.args(["await", "123", "--output", "xml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("possible values"));
}

