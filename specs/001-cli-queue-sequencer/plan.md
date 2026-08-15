# Implementation Plan: CLI Queue Sequencer

**Branch**: `001-cli-queue-sequencer` | **Date**: 2026-08-15 | **Spec**: [spec.md](specs/001-cli-queue-sequencer/spec.md)

**Input**: Feature specification from `specs/001-cli-queue-sequencer/spec.md`

## Summary

Build `queue`, a cross-platform CLI tool in Rust that sequences commands launched simultaneously from multiple terminals. Commands are enqueued into a named FIFO queue (default: "main") and executed one at a time with full I/O transparency. The tool uses file-based locking (Rust 1.89+ `std::fs::File::lock()`) and a JSON state file for queue coordination, with robust signal handling and stale-lock recovery.

## Technical Context

**Language/Version**: Rust stable ≥ 1.89.0 (required for `std::fs::File::lock()` stabilization)

**Primary Dependencies**: `clap` 4.x (CLI parsing), `serde`/`serde_json` (serialization), `ctrlc` (signal handling), `sysinfo` (PID liveness), `anyhow` (error handling), `tempfile` (atomic writes)

**Storage**: JSON state files in `{OS_TEMP_DIR}/queue/` — one `.state.json` + two lock files per queue

**Testing**: `cargo test` (unit + integration), `assert_cmd` + `predicates` (CLI assertions), TDD per Constitution Principle IV

**Target Platform**: Linux, macOS, Windows (cross-platform via std library + portable crates)

**Project Type**: CLI binary

**Performance Goals**: < 100ms queue overhead (SC-007), < 100ms I/O streaming latency (SC-003), < 500ms status response (SC-005)

**Constraints**: Zero external runtime dependencies (single static binary), advisory locking only, no daemon process

**Scale/Scope**: Single-machine, multi-process coordination. Queue depth unbounded with warning at > 100 entries.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| **I. CLI First & Unix Philosophy** | ✅ PASS | `queue run` / `queue status` subcommands. stdout/stderr separation (FR-015). Exit code propagation (FR-006). JSON output (FR-009). |
| **II. Safe Concurrency & Mutual Exclusion** | ✅ PASS | Two-level file locking: state lock (short) + execution lock (long). Atomic state writes via tempfile+rename. `std::fs::File::lock()` uses OS-native primitives. |
| **III. Agent & Multi-Process Interoperability** | ✅ PASS | Blocking by default (FR-004), `--timeout` for non-blocking (FR-018). `queue status --json` for machine consumption (FR-009). Process isolation via separate entries. |
| **IV. Test-First & Concurrency Rigor** | ✅ PASS | TDD with `cargo test`. Integration tests spawn real concurrent processes. Concurrency edge cases covered: deadlocks, crashes, races, timeouts. `assert_cmd` for CLI testing. |
| **V. Observability & Minimal Overhead** | ✅ PASS | Diagnostics on stderr only. < 100ms overhead target. `queue status` for queue inspection. Structured JSON output. |
| **VI. Open Source & English-First Documentation** | ✅ PASS | All artifacts in English. User's French input translated to English in specs. |
| **Cross-Platform Portability** | ✅ PASS | Rust stdlib + portable crates. `File::lock()` maps to `flock`/`LockFileEx`. `sysinfo` for PID checks. `std::env::temp_dir()` for state directory. |
| **Signal Forwarding** | ✅ PASS | `ctrlc` crate for Ctrl+C. Unix: child in same process group receives SIGINT. Windows: console event forwarding. Cleanup on handler. |
| **Crash Resilience** | ✅ PASS | PID liveness + start time via `sysinfo`. Stale entry cleanup on every state read. Atomic writes prevent corruption. |

**Post-Phase-1 Re-check**: All gates still pass. Design artifacts (data-model, contracts, quickstart) are consistent with constitution.

## Project Structure

### Documentation (this feature)

```text
specs/001-cli-queue-sequencer/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0 output — technology decisions
├── data-model.md        # Phase 1 output — entities, state, invariants
├── quickstart.md        # Phase 1 output — validation scenarios
├── contracts/
│   └── cli.md           # Phase 1 output — CLI command schema
└── checklists/
    └── requirements.md  # Spec quality checklist
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, clap CLI definition
├── cli.rs               # CLI argument structs (clap derive)
├── queue/
│   ├── mod.rs           # Queue module public interface
│   ├── state.rs         # QueueState, QueueEntry, serialization
│   ├── lock.rs          # Lock management (state lock + execution lock)
│   ├── manager.rs       # Queue operations: enqueue, dequeue, status, cleanup
│   └── cleanup.rs       # Stale entry detection and removal
├── runner.rs            # Command execution: spawn, wait, signal forwarding
├── signal.rs            # Signal handler setup (ctrlc integration)
├── display.rs           # Human-readable and JSON output formatting
└── error.rs             # Error types (anyhow + thiserror for core types)

tests/
├── integration/
│   ├── run_test.rs      # End-to-end `queue run` tests
│   ├── status_test.rs   # `queue status` tests
│   ├── signal_test.rs   # Ctrl+C / signal forwarding tests
│   ├── timeout_test.rs  # --timeout behavior tests
│   └── concurrency_test.rs  # Multi-process FIFO ordering tests
└── common/
    └── mod.rs           # Shared test utilities

Cargo.toml
```

**Structure Decision**: Single binary crate. Source organized by responsibility: CLI parsing, queue state management (with sub-modules for lock/state/cleanup), command execution, and output formatting. Integration tests in `tests/` directory use real process spawning to validate cross-process behavior.

## Complexity Tracking

No constitution violations — table not needed.
