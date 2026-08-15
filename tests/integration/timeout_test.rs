use assert_cmd::Command;
use assert_cmd::cargo::CommandCargoExt;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn test_timeout_aborts_wait() {
    let temp = tempdir().unwrap();

    let (sleep_cmd, sleep_args_3) = if cfg!(windows) {
        ("powershell", vec!["-Command", "Start-Sleep -Seconds 3"])
    } else {
        ("sleep", vec!["3"])
    };

    let sleep_args_1 = if cfg!(windows) {
        vec!["-Command", "Start-Sleep -Seconds 1"]
    } else {
        vec!["1"]
    };

    // Spawn a long running command
    let mut cmd1 = std::process::Command::cargo_bin("queue").unwrap();
    let mut child1 = cmd1
        .env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--")
        .arg(sleep_cmd)
        .args(&sleep_args_3)
        .spawn()
        .unwrap();

    // Wait slightly to ensure it acquires the lock
    std::thread::sleep(Duration::from_millis(500));

    // Spawn a second command with a 1 second timeout
    let mut cmd2 = Command::cargo_bin("queue").unwrap();
    let start = Instant::now();
    cmd2.env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--timeout")
        .arg("1")
        .arg("--")
        .arg(sleep_cmd)
        .args(&sleep_args_1)
        .assert()
        .failure()
        .code(124); // Custom timeout code

    let elapsed = start.elapsed();
    assert!(elapsed.as_secs() < 3); // It should have timed out early

    child1.wait().unwrap();
}
