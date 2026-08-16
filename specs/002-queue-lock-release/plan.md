# Implementation Plan: Queue Lock and Release Mechanism

**Branch**: `002-queue-lock-release` | **Date**: 2026-08-16 | **Spec**: [spec.md](specs/002-queue-lock-release/spec.md)

**Input**: Feature specification from `specs/002-queue-lock-release/spec.md`

## Summary

Add explicit lock/release subcommands to the `queue` CLI tool, enabling exclusive resource reservation with token-based validation. Agents or developers can lock a queue (receiving a unique token), perform exclusive work, and release the lock by presenting the token. Force-release handles crash recovery. The existing `queue run` command respects active locks, and both `lock` and `run` waiters share a single FIFO queue. This extends the existing file-based locking infrastructure with a semantic lock layer (`lock.json`) that persists beyond process lifetime.

## Technical Context

**Language/Version**: Rust, edition 2024, MSRV 1.86.0

**Primary Dependencies**: `clap` 4 (derive), `serde`/`serde_json` 1, `uuid` 1 (v4), `chrono` 0.4, `ctrlc` 3, `sysinfo` 0.39, `anyhow`, `thiserror`, `tempfile`

**Storage**: Filesystem-based — `QUEUE_STATE_DIR` env var or OS temp directory. Per-queue directory structure with JSON state files and OS-level file locks.

**Testing**: `cargo test` with `assert_cmd`, `predicates`, `serial_test` for integration tests in `tests/integration/`

**Target Platform**: Cross-platform (Windows, Linux, macOS)

**Project Type**: CLI tool (single binary `queue`)

**Performance Goals**: Lock/release operations under 1 second (SC-001)

**Constraints**: No external services, file-based only. Cross-platform file locking via existing mechanisms.

**Scale/Scope**: Local development coordination tool. Single machine, multiple processes.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|---|---|---|
| **I. CLI First & Unix Philosophy** | ✅ PASS | New `lock` and `release` subcommands follow existing CLI patterns. stdout/stderr separation. Text + JSON output formats. Exit codes consistent with existing conventions (0, 1, 2, 124, 130). `--raw` flag enables composable scripting (`TOKEN=$(queue lock --raw ...)`). |
| **II. Safe Concurrency & Mutual Exclusion** | ✅ PASS | Two-phase locking uses OS file lock for atomicity of critical section. FIFO ticket system ensures deterministic scheduling. Lock state persists to filesystem for crash resilience. |
| **III. Agent & Multi-Process Interoperability** | ✅ PASS | Lock/release designed for autonomous agent workflows. Token-based auth prevents unauthorized releases. FIFO ordering across lock/run commands. Status shows waiters for full queue inspection. |
| **IV. Test-First & Concurrency Rigor** | ✅ PASS | Validation scenarios defined in quickstart.md cover all user stories, edge cases, and concurrency scenarios. Integration tests will use existing `assert_cmd` + `serial_test` patterns. |
| **V. Observability & Minimal Overhead** | ✅ PASS | Lock state visible via `queue status`. Waiting messages go to stderr. No additional runtime overhead beyond file I/O for lock.json and waiter tickets. |
| **VI. Open Source & English-First** | ✅ PASS | All artifacts in English. |
| **VII. Relative Paths** | ✅ PASS | All spec artifact references use repo-relative paths. |

**Post-Phase 1 re-check**: All gates remain PASS. Design artifacts (data-model, contracts, quickstart) are consistent with constitution principles.

## Project Structure

### Documentation (this feature)

```text
specs/002-queue-lock-release/
├── spec.md              # Feature specification (input)
├── plan.md              # This file
├── research.md          # Phase 0 output — design decisions
├── data-model.md        # Phase 1 output — LockInfo, WaiterEntry entities
├── quickstart.md        # Phase 1 output — validation scenarios
├── contracts/
│   └── cli-contracts.md # Phase 1 output — CLI interface contracts
└── tasks.md             # Phase 2 output (NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, clap CLI dispatch
├── cli.rs               # CLI argument definitions (Commands enum)
├── signal.rs            # Global Ctrl+C signal handler
├── runner.rs            # Child process spawning + stream relay
├── display.rs           # Human / JSON output formatting
├── error.rs             # QueueError enum, exit codes
└── queue/
    ├── mod.rs           # Module re-exports
    ├── manager.rs       # Core queue logic (enqueue, wait loop, execute)
    ├── lock.rs          # File lock acquisition (OS-level locks)
    ├── state.rs         # Queue state JSON persistence
    └── cleanup.rs       # Stale PID cleanup

tests/
└── integration/
    ├── mod.rs           # Test module
    ├── cli_basic_test.rs
    ├── cli_queue_test.rs
    ├── cli_timeout_test.rs
    ├── cli_status_test.rs
    ├── cli_signal_test.rs
    └── helpers/
        └── mod.rs       # Test utilities
```

**New/Modified files for this feature**:

| File | Action | Purpose |
|---|---|---|
| `src/cli.rs` | MODIFY | Add `Lock` and `Release` variants to `Commands` enum |
| `src/main.rs` | MODIFY | Add dispatch for Lock and Release commands |
| `src/queue/lock.rs` | MODIFY | Add `LockInfo` struct, `lock.json` read/write, token validation |
| `src/queue/state.rs` | MODIFY | Add `WaiterEntry` struct, waiter directory management |
| `src/queue/manager.rs` | MODIFY | Add lock/release logic, integrate FIFO waiter system, modify wait loop to respect explicit locks |
| `src/queue/mod.rs` | MODIFY | Export new types |
| `src/display.rs` | MODIFY | Add lock/release output formatting, extend status display |
| `src/error.rs` | MODIFY | Add lock-specific error variants (InvalidToken, QueueNotLocked, etc.) |
| `tests/integration/cli_lock_test.rs` | NEW | Integration tests for lock/release scenarios |

**Structure Decision**: Single project structure (CLI tool). All new lock/release code integrates into the existing `src/queue/` module hierarchy. No new top-level modules needed.

## Complexity Tracking

No constitution violations. No complexity justifications needed.
