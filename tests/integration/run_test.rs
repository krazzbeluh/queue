use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_basic_queue_run() {
    let temp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    // Use cross-platform echo-like behavior via python, or just basic echo which works on cmd and sh
    cmd.env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--")
        .arg("rustc")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rustc"));
}

#[test]
fn test_exit_code_propagation() {
    let temp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    // We expect rustc with bad args to fail
    cmd.env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--")
        .arg("rustc")
        .arg("--does-not-exist")
        .assert()
        .failure(); // exit code != 0
}

#[test]
fn test_stream_separation() {
    let temp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    // We use a small rust script or simply standard tools.
    // On Windows, `cmd /c "echo hello && echo error 1>&2"`
    // On Unix, `sh -c "echo hello && echo error >&2"`
    let (shell, arg) = if cfg!(windows) {
        ("cmd.exe", "/C")
    } else {
        ("sh", "-c")
    };
    let script = if cfg!(windows) {
        "echo hello && echo error 1>&2"
    } else {
        "echo hello && echo error >&2"
    };

    cmd.env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--")
        .arg(shell)
        .arg(arg)
        .arg(script)
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"))
        .stderr(predicate::str::contains("error"));
}
