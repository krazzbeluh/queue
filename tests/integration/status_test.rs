use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_status_empty() {
    let temp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    cmd.env("QUEUE_STATE_DIR", temp.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: idle"))
        .stdout(predicate::str::contains("No commands running or pending."));
}

#[test]
fn test_status_empty_json() {
    let temp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    cmd.env("QUEUE_STATE_DIR", temp.path())
        .arg("status")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"idle\""));
}
