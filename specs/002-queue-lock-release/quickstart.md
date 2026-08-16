# Quickstart Validation Guide: Queue Lock and Release

**Feature**: 002-queue-lock-release  
**Date**: 2026-08-16

## Prerequisites

- Rust toolchain installed (MSRV 1.86.0)
- Project built: `rtk proxy ./gradlew build` from repo root
- Binary available at `target/debug/queue` (or `cargo run --`)

## Setup

Set a temporary state directory to avoid polluting the default `~/.queue`:

```bash
export QUEUE_STATE_DIR=$(mktemp -d)
```

On Windows PowerShell:
```powershell
$env:QUEUE_STATE_DIR = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
```

---

## Scenario 1: Basic Lock and Release

**Validates**: US-1 (lock), US-2 (release with token), FR-001, FR-004, FR-010

```bash
# Lock the queue — should print token and confirmation
TOKEN=$(queue lock --queue ios --raw E2E Tests)
echo "Got token: $TOKEN"

# Verify queue is locked
queue status --queue ios
# Expected: Status = locked, reason = "E2E Tests", token shown

# Release with correct token
queue release --queue ios --token "$TOKEN"
# Expected: success message, exit code 0

# Verify queue is unlocked
queue status --queue ios
# Expected: Status = idle (or "not locked")
```

**Expected outcome**: Lock acquired instantly, token is a UUID, status shows locked state, release succeeds, status returns to idle.

---

## Scenario 2: Invalid Token Rejection

**Validates**: US-2 scenario 2, FR-005

```bash
# Lock the queue
TOKEN=$(queue lock --queue ios --raw Test run)

# Try to release with wrong token
queue release --queue ios --token "wrong-token-12345"
# Expected: Error message on stderr, exit code 1

echo "Exit code: $?"
# Expected: 1

# Release properly
queue release --queue ios --token "$TOKEN"
```

**Expected outcome**: Wrong token is rejected with clear error. Queue remains locked. Correct token releases successfully.

---

## Scenario 3: Force Release

**Validates**: US-3, FR-006

```bash
# Lock the queue
queue lock --queue ios --raw Recovery test > /dev/null

# Force release without token
queue release --queue ios --force
# Expected: success message, exit code 0

# Verify released
queue status --queue ios
```

**Expected outcome**: Force release succeeds without needing the token.

---

## Scenario 4: Queue Run Blocked by Lock

**Validates**: FR-008, FR-013

```bash
# Lock the queue
TOKEN=$(queue lock --queue ios --raw Blocking test)

# In a separate terminal / background:
queue run --queue ios --timeout 5 -- echo "should wait"
# Expected: waits, then times out after 5 seconds (exit code 124)

# Release the lock
queue release --queue ios --token "$TOKEN"

# Now run should succeed:
queue run --queue ios -- echo "success"
# Expected: "success" printed, exit code 0
```

**Expected outcome**: `queue run` is blocked while lock is active, respects timeout. After release, runs normally.

---

## Scenario 5: FIFO Ordering

**Validates**: FR-007, FR-013

```bash
# Lock the queue
TOKEN=$(queue lock --queue ios --raw First lock)

# Start multiple waiters in background (in order):
queue lock --queue ios Second lock &
PID1=$!
sleep 0.2
queue run --queue ios -- echo "Run command" &
PID2=$!
sleep 0.2
queue lock --queue ios Third lock &
PID3=$!

# Check status — should show 3 waiters in FIFO order
queue status --queue ios --json

# Release the first lock
queue release --queue ios --token "$TOKEN"

# Waiters should acquire in order: Second lock → Run command → Third lock
wait $PID1 $PID2 $PID3
```

**Expected outcome**: Status shows 3 waiters in arrival order. After release, they proceed in strict FIFO.

---

## Scenario 6: Lock Timeout

**Validates**: FR-012

```bash
# Lock the queue
TOKEN=$(queue lock --queue ios --raw Timeout test)

# Try to lock with timeout
queue lock --queue ios --timeout 3 Another lock
# Expected: waits 3 seconds, then exits with code 124

echo "Exit code: $?"
# Expected: 124

# Cleanup
queue release --queue ios --token "$TOKEN"
```

**Expected outcome**: Second lock attempt times out with exit code 124 (same as `queue run --timeout`).

---

## Scenario 7: Status with Lock and Waiters (JSON)

**Validates**: FR-009

```bash
# Lock the queue
TOKEN=$(queue lock --queue ios --raw Status test)

# Add a waiter
queue lock --queue ios --timeout 30 Waiting lock &

sleep 1

# Check JSON status
queue status --queue ios --json
# Expected: JSON with lock info and waiters array

# Cleanup
queue release --queue ios --token "$TOKEN"
wait
```

**Expected outcome**: JSON output includes `lock` object with reason/token/timestamp and `waiters` array with the waiting command.

---

## Scenario 8: Release Non-Locked Queue

**Validates**: US-2 scenario 3, US-3 scenario 2

```bash
# Try to release an unlocked queue
queue release --queue ios --token "any-token"
# Expected: error "queue is not currently locked", exit code 1

queue release --queue ios --force
# Expected: error "queue is not currently locked", exit code 1
```

**Expected outcome**: Both token-based and force release fail gracefully on an unlocked queue.

---

## Exit Code Reference

See [cli-contracts.md](contracts/cli-contracts.md) for complete exit code definitions.

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error (invalid token, queue not locked) |
| 2 | Usage error (missing required args) |
| 124 | Timeout exceeded |
| 130 | Interrupted (Ctrl+C) |
