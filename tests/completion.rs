use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild, PathCreateDir};
use predicates::prelude::*;

#[test]
fn zsh_completion_includes_dynamic_spawn_helpers_from_local_workflows_dir() {
    let temp = assert_fs::TempDir::new().expect("temp dir");
    let workflows_dir = temp.child(".github/workflows");
    workflows_dir.create_dir_all().expect("workflows dir");

    workflows_dir
        .child("deploy.yml")
        .write_str(include_str!("deploy.yml"))
        .expect("fixture workflow");

    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.current_dir(temp.path())
        .args(["completion", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef gha"))
        .stdout(predicate::str::contains("_gha_spawn_args()"))
        .stdout(predicate::str::contains("functions[_gha__spawn]"))
        .stdout(predicate::str::contains("_gha_choices[environment]"))
        .stdout(predicate::str::contains("'dev'"))
        .stdout(predicate::str::contains("'staging'"))
        .stdout(predicate::str::contains("'prod'"))
        .stdout(predicate::str::contains("_gha_choices[version]").not());
}

#[test]
fn zsh_completion_without_workflows_omits_dynamic_helpers() {
    let temp = assert_fs::TempDir::new().expect("temp dir");

    let mut cmd = Command::new(assert_cmd::cargo_bin!("gha"));
    cmd.current_dir(temp.path())
        .args(["completion", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef gha"))
        .stdout(predicate::str::contains("# --- gha dynamic workflow input completions ---").not())
        .stdout(predicate::str::contains("_gha_spawn_args()").not());
}


