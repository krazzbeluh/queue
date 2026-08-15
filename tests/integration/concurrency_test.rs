use assert_cmd::cargo::CommandCargoExt;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn test_sequential_execution() {
    let temp = tempdir().unwrap();

    let (sleep_cmd, sleep_args) = if cfg!(windows) {
        ("powershell", vec!["-Command", "Start-Sleep -Seconds 2"])
    } else {
        ("sleep", vec!["2"])
    };

    let start = Instant::now();

    let mut child1 = std::process::Command::cargo_bin("queue")
        .unwrap()
        .env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--")
        .arg(sleep_cmd)
        .args(&sleep_args)
        .spawn()
        .unwrap();

    // Small delay to ensure child1 gets in queue first
    std::thread::sleep(Duration::from_millis(100));

    let mut child2 = std::process::Command::cargo_bin("queue")
        .unwrap()
        .env("QUEUE_STATE_DIR", temp.path())
        .arg("run")
        .arg("--")
        .arg(sleep_cmd)
        .args(&sleep_args)
        .spawn()
        .unwrap();

    let status1 = child1.wait().unwrap();
    let status2 = child2.wait().unwrap();

    let elapsed = start.elapsed();

    assert!(status1.success());
    assert!(status2.success());

    // Each takes 2 seconds, running sequentially should take at least 4 seconds
    assert!(
        elapsed.as_secs() >= 4,
        "Commands ran in parallel, elapsed: {:?}",
        elapsed
    );
}
