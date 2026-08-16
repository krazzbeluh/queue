# Research: Queue Lock and Release Mechanism

**Feature**: 002-queue-lock-release  
**Date**: 2026-08-16  
**Status**: Complete

## R-001: Lock State Persistence Strategy

**Decision**: Extend the existing per-queue state directory with a dedicated `lock.json` file alongside the existing `state.json`.

**Rationale**: The project already uses `<state_dir>/<queue_name>/` with `queue.lock` (OS file lock) and `state.json` (metadata). Adding a `lock.json` file for explicit lock metadata (token, reason, holder, timestamp) keeps the architecture consistent. The OS-level `queue.lock` file continues to serve as the atomicity mechanism — `lock.json` adds the semantic layer for token-based validation.

**Alternatives considered**:
- **Embed lock state in `state.json`**: Rejected because `state.json` is cleared on each run completion. Lock state must persist independently of run state, since a lock can outlive any single polling cycle.
- **Separate lock file with OS lock**: Rejected as unnecessary complexity. One OS lock file (`queue.lock`) is sufficient for atomicity; the semantic lock info belongs in a data file.

## R-002: Lock/Run FIFO Ordering Mechanism

**Decision**: Use a file-based ticket system in `<state_dir>/<queue_name>/waiters/` where each waiting process creates a timestamped ticket file (e.g., `<timestamp>-<uuid>.json`). The polling loop checks whether the current process's ticket is the oldest in the directory before attempting to acquire the lock.

**Rationale**: The spec requires strict FIFO ordering across both `lock` and `run` commands (FR-013). The current polling loop (`try_acquire_lock` every 500ms) has no ordering guarantee — whichever process happens to poll at the right moment wins. A ticket directory provides a deterministic ordering mechanism that is consistent with the file-based architecture.

**Alternatives considered**:
- **Named pipes / Unix domain sockets**: Rejected for cross-platform complexity (Windows named pipes have different semantics).
- **In-memory queue with IPC**: Rejected because it requires a persistent daemon process, violating the self-contained CLI principle.
- **Polling without FIFO**: Rejected because FR-013 explicitly requires strict FIFO.

## R-003: Token Generation

**Decision**: Use `uuid::Uuid::new_v4()` which is already a project dependency.

**Rationale**: UUIDv4 provides 122 bits of randomness, more than sufficient for a local coordination tool (spec assumption). The `uuid` crate is already in `Cargo.toml`.

**Alternatives considered**:
- **Cryptographic random bytes (hex-encoded)**: Equivalent security, but UUID is more readable and the crate is already present.
- **Sequential / timestamp-based**: Rejected because FR-010 requires tokens to be unguessable.

## R-004: Integration with `queue run` Wait Loop

**Decision**: Modify `QueueRunner::wait_for_lock()` to check for an active explicit lock (presence of `lock.json` with a valid token) in addition to the OS file lock. When an explicit lock is active, `queue run` must enter the FIFO waiter queue and wait for both the explicit lock to be released AND the OS file lock to be available.

**Rationale**: FR-008 requires `queue run` to wait when a queue is locked. The current `wait_for_lock` only checks the OS file lock via `try_acquire_lock()`. An explicit lock (via `queue lock`) must also be respected. Since both `lock` and `run` share the same FIFO queue (FR-013), the waiter ticket system handles ordering for both.

**Alternatives considered**:
- **Hold OS file lock for the entire duration of `queue lock`**: This would work but prevents the lock holder from releasing their CLI process. The `queue lock` command exits after acquiring the lock and returning the token — the lock must persist beyond the process lifetime. OS file locks are released when the process exits.
- **Separate OS lock file for explicit locks**: Adds complexity without benefit since `lock.json` presence is sufficient.

## R-005: `queue status` Enhancement for Lock Visibility

**Decision**: Extend `StatusInfo` struct to include optional lock fields (`locked: bool`, `lock_reason: Option<String>`, `lock_token: Option<String>`, `locked_at: Option<String>`, `locked_by: Option<String>`) and a `waiters: Vec<WaiterInfo>` field showing queued agents/commands.

**Rationale**: FR-009 requires `queue status` to show lock state and waiting agents in both text and JSON formats. The existing `StatusInfo` already supports JSON serialization via serde.

**Alternatives considered**:
- **Separate `queue lock-status` subcommand**: Rejected because the spec explicitly requires this info in `queue status`.

## R-006: Stale Lock Detection

**Decision**: When `lock.json` exists but the locking process PID (if recorded) is no longer alive, mark the lock as potentially stale. `queue status` will indicate staleness. `--force` release always works regardless of staleness. Normal operations (run, lock) will treat a stale lock as still valid — only `--force` can clear it.

**Rationale**: The edge case spec mentions corrupted or missing lock files and crashed processes. The `sysinfo` crate is already available for PID checks. However, PID-based detection is best-effort (PIDs can be reused), so it's informational rather than automatic cleanup.

**Alternatives considered**:
- **Automatic stale lock cleanup**: Rejected because it risks releasing a lock that was intentionally held by a process that was restarted with the same PID.
- **TTL-based expiry**: Rejected because the spec doesn't mention time-based lock expiry, and lock durations are unpredictable (e.g., a full E2E test suite).

## R-007: CLI Subcommand Structure

**Decision**: Add two new subcommands to the `Commands` enum:
- `Lock { queue: String, timeout: Option<u64>, raw: bool, reason: Vec<String> }`
- `Release { queue: String, token: Option<String>, force: bool }`

**Rationale**: Matches the spec's CLI interface (FR-001, FR-004, FR-006). Uses clap derive macros consistent with existing subcommands. `reason` is a `Vec<String>` to capture multiple positional words without requiring quotes.

**Alternatives considered**:
- **Single `lock` subcommand with `--release` flag**: Rejected because lock and release are semantically distinct operations with different arguments.

## R-008: Atomic Lock Acquisition

**Decision**: Use a two-phase locking approach:
1. Acquire OS file lock on `queue.lock` (atomic via `fs4`)
2. While holding OS lock, check for existing `lock.json`, write new `lock.json` if absent, release OS lock

The OS file lock serves as a mutex for the brief critical section of reading/writing `lock.json`. The semantic lock (represented by `lock.json`) persists beyond the process lifetime.

**Rationale**: FR-007 requires atomic lock acquisition. The OS file lock provides the atomicity guarantee for the critical section. The semantic lock (`lock.json`) persists after the process exits, enabling the token-based release workflow.

**Alternatives considered**:
- **Hold OS lock for entire lock duration**: Impossible because `queue lock` exits after printing the token. The lock must survive process termination.
- **Atomic file rename**: Could work for creating `lock.json` atomically, but doesn't help with the read-check-write race condition.
