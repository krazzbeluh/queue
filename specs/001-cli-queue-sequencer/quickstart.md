# Quickstart Validation Guide: CLI Queue Sequencer

**Feature**: cli-queue-sequencer | **Date**: 2026-08-15

## Prerequisites

- Rust toolchain ≥ 1.89.0 (`rustup update stable`)
- Two terminal windows (for concurrency tests)
- The `queue` binary built and available on PATH

## Setup

```bash
# Clone and build
cd queue
cargo build --release

# Add to PATH (or use target/release/queue directly)
export PATH="$PWD/target/release:$PATH"   # Unix
# $env:PATH = "$PWD\target\release;$env:PATH"  # Windows PowerShell
```

## Validation Scenarios

### Scenario 1: Basic Command Execution (FR-001, FR-005, FR-006)

Verifies that a single command executes immediately with proper I/O streaming and exit code propagation.

```bash
# Run a simple command
queue run echo "hello world"
# Expected stdout: hello world
# Expected exit code: 0

# Verify exit code propagation
queue run sh -c "exit 42"    # Unix
# queue run cmd /C "exit 42"  # Windows
echo $?  # Should print: 42
```

**Expected outcome**: Output appears immediately (real-time), exit code matches wrapped command.

---

### Scenario 2: Sequential Execution / Mutual Exclusion (FR-002, FR-003, FR-004)

Verifies FIFO ordering when two commands are submitted simultaneously.

**Terminal 1:**
```bash
queue run sh -c "echo 'A started'; sleep 5; echo 'A finished'"
```

**Terminal 2** (run within 1 second of Terminal 1):
```bash
queue run sh -c "echo 'B started'; sleep 2; echo 'B finished'"
```

**Expected outcome**:
- Terminal 1 shows "A started" immediately, "A finished" after 5s
- Terminal 2 shows "B started" only after Terminal 1's command finishes (~5s delay), "B finished" after ~7s total
- No interleaving of output between A and B

---

### Scenario 3: Queue Status Inspection (FR-008, FR-009)

Verifies the `status` subcommand shows accurate queue state.

**Terminal 1:**
```bash
queue run sleep 30
```

**Terminal 2:**
```bash
# Human-readable
queue status
# Expected: Shows "sleep 30" as running, queue name "main"

# JSON format
queue status --json
# Expected: Valid JSON with "status": "active", running entry with command "sleep 30"
```

**Expected outcome**: Status reflects real-time queue state; JSON output is parseable.

---

### Scenario 4: Signal Handling / Ctrl+C (FR-010, FR-011)

Verifies graceful termination and lock cleanup on interrupt.

**Terminal 1:**
```bash
queue run sleep 60
# Press Ctrl+C after 2 seconds
```

**Terminal 2** (after Ctrl+C in Terminal 1):
```bash
queue status
# Expected: Queue is idle, no stale entries

queue run echo "should work immediately"
# Expected: Executes without delay
```

**Expected outcome**: Ctrl+C terminates child, cleans up queue state, next command runs immediately.

---

### Scenario 5: Stale Lock Recovery (FR-012)

Verifies automatic cleanup of entries from crashed processes.

```bash
# Start a long command
queue run sleep 300 &
QPID=$!

# Kill it forcefully (simulates crash)
kill -9 $QPID  # Unix
# Stop-Process -Id $QPID -Force  # Windows

# Verify recovery
queue status
# Expected: No stale entries (or cleaned up on inspection)

queue run echo "recovery works"
# Expected: Executes immediately
```

**Expected outcome**: Stale entries detected and cleaned automatically; no manual intervention needed.

---

### Scenario 6: Timeout Behavior (FR-018)

Verifies `--timeout` flag aborts waiting after specified duration.

**Terminal 1:**
```bash
queue run sleep 60
```

**Terminal 2:**
```bash
queue run --timeout 5 echo "should timeout"
echo $?  # Should print: 124
```

**Expected outcome**: Terminal 2 exits after 5 seconds with exit code 124 and a timeout error on stderr.

---

### Scenario 7: stderr Separation (FR-015)

Verifies that queue diagnostics go to stderr, not stdout.

```bash
# Capture stdout only
queue run echo "only this on stdout" > /tmp/stdout.txt 2>/dev/null
cat /tmp/stdout.txt
# Expected: "only this on stdout" (no queue messages)

# Capture stderr only
queue run echo "test" 2> /tmp/stderr.txt > /dev/null
cat /tmp/stderr.txt
# Expected: Queue diagnostic messages (if any) appear here
```

**Expected outcome**: stdout contains only the wrapped command's output; queue messages appear only on stderr.

---

## Test Suite Commands

```bash
# Run all unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run all tests with output
cargo test -- --nocapture

# Run with nextest (faster, if installed)
cargo nextest run
```

## Cross-Platform Verification Matrix

| Scenario | Linux | macOS | Windows |
|----------|-------|-------|---------|
| Basic execution | ☐ | ☐ | ☐ |
| Sequential execution | ☐ | ☐ | ☐ |
| Queue status | ☐ | ☐ | ☐ |
| Signal handling | ☐ | ☐ | ☐ |
| Stale lock recovery | ☐ | ☐ | ☐ |
| Timeout | ☐ | ☐ | ☐ |
| stderr separation | ☐ | ☐ | ☐ |
