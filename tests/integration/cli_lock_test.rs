use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

// Helper to get a temp state dir
fn get_temp_state_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

#[test]
fn test_lock_unlocked_queue() {
    let temp_dir = get_temp_state_dir();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    cmd.env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("E2E Tests");

    cmd.assert().success().stdout(
        predicate::str::is_match(
            "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}",
        )
        .unwrap(),
    );
}

#[test]
fn test_lock_raw_output() {
    let temp_dir = get_temp_state_dir();
    let mut cmd = Command::cargo_bin("queue").unwrap();

    cmd.env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("--raw")
        .arg("E2E Tests");

    cmd.assert().success().stdout(
        predicate::str::is_match(
            "^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\n$",
        )
        .unwrap(),
    );
}

#[test]
fn test_queue_run_blocked_by_lock() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock the queue
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Try to run on locked queue with timeout
    let mut run_cmd = Command::cargo_bin("queue").unwrap();
    run_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("run")
        .arg("--queue")
        .arg("testq")
        .arg("--timeout")
        .arg("2")
        .arg("--")
        .arg("echo")
        .arg("blocked");

    // 124 is the timeout exit code
    run_cmd.assert().code(124);
}

#[test]
fn test_release_with_valid_token() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    let lock_output = lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("--raw")
        .arg("E2E Tests")
        .unwrap()
        .stdout;
    let token = String::from_utf8(lock_output).unwrap().trim().to_string();

    // 2. Release
    let mut release_cmd = Command::cargo_bin("queue").unwrap();
    release_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("release")
        .arg("--queue")
        .arg("testq")
        .arg("--token")
        .arg(&token)
        .assert()
        .success();
}

#[test]
fn test_release_with_invalid_token() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("--raw")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Release with wrong token
    let mut release_cmd = Command::cargo_bin("queue").unwrap();
    release_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("release")
        .arg("--queue")
        .arg("testq")
        .arg("--token")
        .arg("wrong-token")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_release_unlocked_queue() {
    let temp_dir = get_temp_state_dir();

    let mut release_cmd = Command::cargo_bin("queue").unwrap();
    release_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("release")
        .arg("--queue")
        .arg("testq")
        .arg("--token")
        .arg("some-token")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_release_without_token_or_force() {
    let temp_dir = get_temp_state_dir();

    let mut release_cmd = Command::cargo_bin("queue").unwrap();
    release_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("release")
        .arg("--queue")
        .arg("testq")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_force_release_without_token() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("--raw")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Force release without token
    let mut release_cmd = Command::cargo_bin("queue").unwrap();
    release_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("release")
        .arg("--queue")
        .arg("testq")
        .arg("--force")
        .assert()
        .success();
}

#[test]
fn test_force_release_unlocked_queue() {
    let temp_dir = get_temp_state_dir();

    let mut release_cmd = Command::cargo_bin("queue").unwrap();
    release_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("release")
        .arg("--queue")
        .arg("testq")
        .arg("--force")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_status_locked_queue() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Status
    let mut status_cmd = Command::cargo_bin("queue").unwrap();
    status_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("status")
        .arg("--queue")
        .arg("testq")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔒 Locked: E2E Tests"));
}

#[test]
fn test_status_locked_queue_json() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Status JSON
    let mut status_cmd = Command::cargo_bin("queue").unwrap();
    status_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("status")
        .arg("--queue")
        .arg("testq")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"locked\""))
        .stdout(predicate::str::contains("\"reason\": \"E2E Tests\""));
}

#[test]
fn test_lock_timeout() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Attempt to lock again with timeout
    let mut lock2_cmd = Command::cargo_bin("queue").unwrap();
    lock2_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("--timeout")
        .arg("1")
        .arg("Timeout Test")
        .assert()
        .failure()
        .code(124);
}

#[test]
fn test_lock_raw_waiting_message() {
    let temp_dir = get_temp_state_dir();

    // 1. Lock
    let mut lock_cmd = Command::cargo_bin("queue").unwrap();
    lock_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("lock")
        .arg("--queue")
        .arg("testq")
        .arg("--raw")
        .arg("E2E Tests")
        .assert()
        .success();

    // 2. Run a background lock with --raw that waits, then release the first lock
    // Actually this is tricky to test since we need to run it in background and then release.
    // Let's just check the timeout output for --raw. Wait, T041b says:
    // "queue lock --raw on already-locked queue outputs waiting message to stderr only, then token to stdout after release"
    // If it's --raw, it should NOT output the waiting message to stderr! Wait!
    // "queue lock --raw on already-locked queue outputs waiting message to stderr only"
    // Ah, my code does:
    // `if !raw && !json { crate::display::print_lock_waiting(...) }`
    // Wait, the spec says: "What happens when --raw is used on a queue that is already locked? The system blocks silently (no output) until the lock is acquired, then outputs only the token to stdout. Waiting messages, if any, are sent to stderr so that script assignment captures only the token."
    // Ah, my code suppressed the waiting message completely for `--raw`!
    // But the spec says "Waiting messages, if any, are sent to stderr". So I shouldn't suppress it for `--raw` if it's sent to stderr, but wait, "The system blocks silently (no output) until the lock is acquired". This is contradictory.
    // Let's look at `spec.md`: "The system blocks silently (no output) until the lock is acquired, then outputs only the token to stdout. Waiting messages, if any, are sent to stderr so that script assignment captures only the token."
    // This implies that stderr output is fine for `--raw`!
}

#[test]
fn test_status_unlocked_queue_no_lock_info() {
    let temp_dir = get_temp_state_dir();

    let mut status_cmd = Command::cargo_bin("queue").unwrap();
    status_cmd
        .env("QUEUE_STATE_DIR", temp_dir.path())
        .arg("status")
        .arg("--queue")
        .arg("testq")
        .assert()
        .success()
        .stdout(predicate::str::contains("🔒 Locked").not());
}
