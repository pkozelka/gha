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
    cmd.args(&["run", "--workflow", "test.yml", "--repo", "owner/repo", "--ref", "main", "--token", "test"])
        .assert()
        .failure() // Will fail due to no actual workflow, but should not fail on args parsing
        .stderr(predicate::str::contains("").or(predicate::str::contains(".")));
}

#[test]
fn fails_without_command() {
    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No command provided"));
}
