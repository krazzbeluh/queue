# Research: CLI Queue Sequencer

**Feature**: cli-queue-sequencer | **Date**: 2026-08-15

## Decision Log

### 1. Language & Toolchain

- **Decision**: Rust (stable, minimum 1.89.0)
- **Rationale**: User requirement. Rust provides zero-cost abstractions, memory safety without GC, cross-platform compilation, and excellent CLI tooling. The 1.89.0 minimum is required for `std::fs::File::lock()` stabilization.
- **Alternatives considered**:
  - Go — simpler concurrency model but user explicitly chose Rust
  - C/C++ — lower-level but no memory safety guarantees

### 2. File Locking Mechanism

- **Decision**: `std::fs::File::lock()` / `File::try_lock()` (standard library, stabilized in Rust 1.89.0)
- **Rationale**: The Rust standard library now provides cross-platform advisory file locking using native OS primitives (`flock(2)` on Unix, `LockFileEx` on Windows). This eliminates the need for external crates like `fs4`, `fd-lock`, or `file_guard`. The API includes `lock()` (blocking exclusive), `lock_shared()`, `try_lock()` (non-blocking), and `unlock()`. Locks are automatically released when the `File` is dropped.
- **Alternatives considered**:
  - `fs4` crate — well-maintained but unnecessary now that stdlib supports locking
  - `fd-lock` crate — lightweight but adds an external dependency for no benefit
  - `filelock` crate — simple RAII guard but stdlib is simpler and zero-dependency

### 3. CLI Framework

- **Decision**: `clap` v4 with derive API
- **Rationale**: Industry standard for Rust CLI applications. The derive macro provides type-safe argument parsing, automatic help generation from doc comments, and excellent subcommand support via `#[derive(Subcommand)]`. Minimal boilerplate with maximum functionality.
- **Alternatives considered**:
  - `argh` — lighter weight but less ecosystem support
  - Manual parsing — too error-prone for production CLI
  - `structopt` — superseded by clap v4 derive

### 4. Queue State Storage

- **Decision**: JSON state file in OS temp directory (`std::env::temp_dir()/queue/`), protected by a separate lock file per queue name
- **Rationale**: A single JSON file per queue stores the ordered list of entries (pending and running). A companion `.lock` file provides mutual exclusion for reading/writing the state file. This is simpler than IPC channels or shared memory, works across unrelated processes, and survives individual process crashes. Atomic writes (write to temp file + rename) prevent corruption.
- **Alternatives considered**:
  - `interprocess` crate (local sockets) — more complex, requires a daemon process to coordinate
  - Shared memory (`shared_memory` + `raw_sync`) — overkill for the throughput requirements, requires `unsafe`
  - `ipc-channel` — designed for parent-child communication, not arbitrary processes
  - SQLite — heavyweight for a simple FIFO queue

### 5. FIFO Ordering Strategy

- **Decision**: Two-level locking with a state file. The **state lock** (short-held) protects read/write of the queue state file. The **execution lock** (long-held) ensures only one command runs at a time. A process: (1) acquires state lock, (2) appends itself to queue, (3) releases state lock, (4) polls/waits for its turn (it is first in queue), (5) acquires execution lock, (6) updates state to "running", (7) executes command, (8) acquires state lock, (9) removes itself from queue, (10) releases execution lock.
- **Rationale**: Pure single-lock approaches cannot distinguish between "waiting for turn" and "executing". A two-lock design cleanly separates queue metadata management from execution serialization while maintaining strict FIFO order.
- **Alternatives considered**:
  - Single lock file (prototype approach) — simpler but does not support `queue status` or FIFO visibility; blocking `lock()` does not guarantee FIFO order across waiters
  - Named semaphore — not portable across all three target platforms
  - Daemon-based broker — adds complexity of a long-running process

### 6. Process Spawning & I/O

- **Decision**: `std::process::Command` with inherited stdio
- **Rationale**: The standard library `Command` API natively supports inheriting stdin/stdout/stderr from the parent process, which satisfies the real-time streaming requirement (FR-005, FR-007). No buffering or pipe management needed. Exit code is retrieved via `Child::wait()`. Cross-platform by design.
- **Alternatives considered**:
  - `duct` crate — useful for pipe chaining but unnecessary when inheriting stdio
  - `shared_child` crate — useful for concurrent waiting but stdlib suffices for our use case
  - Manual fork/exec — not portable

### 7. Signal Handling

- **Decision**: `ctrlc` crate for Ctrl+C (SIGINT) handling + cleanup logic
- **Rationale**: On Unix, child processes in the same process group automatically receive SIGINT when the user presses Ctrl+C. On Windows, console processes in the same console also receive the Ctrl+C event. The `ctrlc` crate provides a cross-platform handler to perform cleanup (release locks, update queue state) after the child terminates. The child process is killed explicitly if it hasn't terminated within a grace period.
- **Alternatives considered**:
  - `signal-hook` crate — more powerful but Unix-only for advanced features
  - `tokio::signal` — requires async runtime, unnecessary overhead
  - Raw platform APIs — not portable

### 8. PID Liveness Detection (Stale Lock Cleanup)

- **Decision**: `sysinfo` crate for cross-platform PID existence checks, combined with process start time comparison
- **Rationale**: When a `queue run` or `queue status` invocation detects entries in the queue state file, it verifies that the owning process is still alive by checking PID existence and comparing the recorded start time. This prevents false positives from PID reuse. `sysinfo` provides a unified API across Linux, macOS, and Windows.
- **Alternatives considered**:
  - `kill(pid, 0)` on Unix + `OpenProcess` on Windows — requires platform-specific code
  - Lock file age heuristic — unreliable for long-running commands
  - No detection (manual cleanup) — violates FR-012

### 9. Serialization

- **Decision**: `serde` + `serde_json`
- **Rationale**: Industry-standard serialization framework for Rust. JSON is human-readable (useful for debugging queue state), well-supported, and sufficient for the small data volumes involved. Also needed for `--json` output (FR-009).
- **Alternatives considered**:
  - `bincode` — faster but not human-readable
  - `toml` — less common for state files
  - Plain text — harder to parse reliably

### 10. Error Handling

- **Decision**: `anyhow` for application-level errors
- **Rationale**: Provides ergonomic error handling with context chaining (`context()`, `with_context()`). Ideal for CLI applications where errors are displayed to users rather than matched programmatically. Pairs well with the `?` operator.
- **Alternatives considered**:
  - `thiserror` — better for libraries exposing typed errors; can be used alongside `anyhow` for core error types
  - `eyre` — similar to `anyhow` but less widely adopted
  - Raw `Result<T, Box<dyn Error>>` — less ergonomic

### 11. Timeout Implementation

- **Decision**: Poll loop with `File::try_lock()` and `std::thread::sleep()`, checking elapsed time against `--timeout` value
- **Rationale**: Simple and portable. The process checks if it is first in queue, attempts to acquire the execution lock with `try_lock()`, and if unsuccessful, sleeps briefly before retrying. If `--timeout` is specified, it checks `Instant::elapsed()` against the timeout and exits with a distinct error code if exceeded. No async runtime needed.
- **Alternatives considered**:
  - Blocking `lock()` with a separate timeout thread — more complex, harder to clean up
  - Async runtime with `tokio::time::timeout` — overkill for this use case

### 12. Testing Strategy

- **Decision**: `cargo test` with built-in test framework; integration tests using `std::process::Command` to spawn multiple `queue` instances; `assert_cmd` + `predicates` crates for CLI testing
- **Rationale**: Constitution mandates TDD (Principle IV). Unit tests validate individual modules (queue state, locking, serialization). Integration tests spawn real processes to verify FIFO ordering, signal handling, and exit code propagation under real concurrency conditions. `assert_cmd` provides ergonomic CLI assertion helpers.
- **Alternatives considered**:
  - `nextest` — faster parallel test runner, can be used as a drop-in replacement but not a dependency
  - Manual shell scripts — harder to maintain and not cross-platform

## Dependency Summary

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.x | CLI argument parsing (derive) |
| `serde` | 1.x | Serialization framework |
| `serde_json` | 1.x | JSON serialization |
| `ctrlc` | 3.x | Cross-platform Ctrl+C handling |
| `sysinfo` | 0.x | Cross-platform PID liveness checks |
| `anyhow` | 1.x | Error handling |
| `tempfile` | 3.x | Atomic file writes (write + persist) |
| `assert_cmd` | 2.x | CLI integration testing (dev) |
| `predicates` | 3.x | Test assertion helpers (dev) |

> **Note**: File locking uses `std::fs::File::lock()` (stabilized in Rust 1.89.0). No external locking crate is required.
