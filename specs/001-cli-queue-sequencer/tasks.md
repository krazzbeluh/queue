# Tasks: CLI Queue Sequencer

**Feature**: `cli-queue-sequencer`
**Plan**: [plan.md](specs/001-cli-queue-sequencer/plan.md)
**Spec**: [spec.md](specs/001-cli-queue-sequencer/spec.md)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependency management, and testing infrastructure.

- [x] T001 Initialize Cargo binary crate with dependencies (`clap`, `serde`, `serde_json`, `ctrlc`, `sysinfo`, `tempfile`, `anyhow`, `thiserror`, `chrono`, `uuid`) in `Cargo.toml`
- [x] T002 [P] Configure development and test dependencies (`assert_cmd`, `predicates`) in `Cargo.toml`
- [x] T003 [P] Set up error handling infrastructure and custom error types in `src/error.rs`
- [x] T004 [P] Set up shared test helpers and temporary directory fixtures in `tests/common/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data models, locking primitives, state management, and CLI argument parsing that MUST be complete before ANY user story can be implemented.

⚠️ **CRITICAL**: No user story work can begin until this phase is complete.

- [x] T005 [P] Implement `EntryStatus` and `QueueEntry` data models with serde serialization in `src/queue/state.rs`
- [x] T006 [P] Implement `QueueState` schema and atomic serialization/deserialization in `src/queue/state.rs`
- [x] T007 Implement two-level file locking primitives (`File::lock()`, state lock and execution lock) in `src/queue/lock.rs`
- [x] T008 [P] Implement PID liveness check and process start time verification using `sysinfo` in `src/queue/cleanup.rs`
- [x] T009 Implement stale lock and orphaned entry detection and cleanup in `src/queue/cleanup.rs`
- [x] T010 Implement queue state directory resolution and queue manager core in `src/queue/manager.rs`
- [x] T010b [P] Validate queue name isolation in API to support future named queues (FR-017)
- [x] T011 Export queue module public interface in `src/queue/mod.rs`
- [x] T012 Define CLI arguments and subcommands structure using `clap` in `src/cli.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel.

---

## Phase 3: User Story 1 - Enqueue and Wait for Sequential Execution (Priority: P1) 🎯 MVP

**Goal**: Commands enqueued via `queue run` execute sequentially one at a time in strict FIFO order across terminals.

**Independent Test**: Spawn multiple concurrent `queue run` invocations from tests and verify mutual exclusion and FIFO completion order.

### Tests for User Story 1 (TDD) ⚠️
> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T013 [P] [US1] Integration tests for sequential execution and FIFO ordering in `tests/integration/concurrency_test.rs`
- [x] T014 [P] [US1] Integration tests for `queue run` basic command execution in `tests/integration/run_test.rs`
- [x] T015 [P] [US1] Integration tests for `--timeout` wait abort in `tests/integration/timeout_test.rs`

### Implementation for User Story 1

- [x] T016 [US1] Implement enqueue, queue position waiting loop, and execution lock acquisition in `src/queue/manager.rs`
- [x] T017 [US1] Implement basic command execution and process spawning in `src/runner.rs`
- [x] T018 [US1] Implement timeout handling on wait queue in `src/queue/manager.rs`
- [x] T019 [US1] Implement CLI entry point dispatch for `queue run` in `src/main.rs`

**Checkpoint**: User Story 1 is functional and testable independently (MVP ready).

---

## Phase 4: User Story 4 - Stream Forwarding with Exit Code Propagation (Priority: P1)

**Goal**: Child process stdout, stderr, and stdin are streamed in real-time without buffering; exact child exit code is propagated to parent process.

**Independent Test**: Run commands writing to stdout/stderr and exiting with specific codes (0, 42, etc.) and verify live streaming and exit code forwarding.

### Tests for User Story 4 (TDD) ⚠️
> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T020 [P] [US4] Integration tests for stdout/stderr streaming separation and exit code propagation in `tests/integration/run_test.rs`
- [x] T020b [P] [US4] Performance test for I/O stream forwarding latency <100ms (SC-003)

### Implementation for User Story 4

- [x] T021 [US4] Implement real-time stdout and stderr stream forwarding and stdin piping in `src/runner.rs`
- [x] T022 [US4] Implement exit code capture and mapping to CLI exit codes (0-255, 124, 125, 126, 127) in `src/runner.rs`
- [x] T023 [US4] Ensure all queue internal diagnostic messages route exclusively to stderr in `src/display.rs`

**Checkpoint**: User Stories 1 and 4 work together seamlessly with full transparent I/O and exit codes.

---

## Phase 5: User Story 2 - Queue Status Inspection (Priority: P2)

**Goal**: Real-time inspection of running and pending commands via human-readable text or machine-readable JSON.

**Independent Test**: Enqueue commands, run `queue status` and `queue status --json`, verify structured outputs match active state.

### Tests for User Story 2 (TDD) ⚠️
> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T024 [P] [US2] Integration tests for `queue status` human-readable and JSON format output in `tests/integration/status_test.rs`
- [x] T024b [P] [US2] Performance test for queue status response <500ms (SC-005)

### Implementation for User Story 2

- [x] T025 [P] [US2] Implement queue status snapshot query method in `src/queue/manager.rs`
- [x] T026 [US2] Implement human-readable text and JSON formatting for status in `src/display.rs`
- [x] T027 [US2] Implement CLI handler and dispatch for `queue status` in `src/main.rs`

**Checkpoint**: Queue state observability is fully functional in both text and JSON formats.

---

## Phase 6: User Story 3 - Graceful Signal Handling and Cleanup (Priority: P2)

**Goal**: Ctrl+C / SIGINT / SIGTERM signals are forwarded to active child, state is cleaned up, and waiting entries are cancelled cleanly.

**Independent Test**: Send SIGINT/Ctrl+C to waiting and running `queue run` processes and verify graceful termination and queue state cleanup.

### Tests for User Story 3 (TDD) ⚠️
> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T028 [P] [US3] Integration tests for Ctrl+C / signal termination and queue cleanup in `tests/integration/signal_test.rs`

### Implementation for User Story 3

- [x] T029 [US3] Implement signal handler with `ctrlc` crate and child process signal forwarding in `src/signal.rs`
- [x] T030 [US3] Implement cancel entry cleanup on signal interception for waiting and running commands in `src/queue/manager.rs`
- [x] T031 [US3] Integrate signal handling into command runner lifecycle in `src/runner.rs`

**Checkpoint**: All user stories functional, robust against interruption and crashes.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: High depth warnings, validation against scenarios, and comprehensive quality checks.

### Final Polish

- [x] T032 [P] Code cleanup, standard `rustfmt` formatting
- [x] T033 [P] Final end-to-end testing and QA (SC-004, SC-006)
- [x] T034 [P] Generate `docs/README.md` and basic man page/help usage texts per `specs/001-cli-queue-sequencer/quickstart.md`
- [x] T034 Run full test suite (`cargo test`) across all unit and integration tests
- [x] T035 Performance benchmark to verify queue scheduling overhead <100ms (SC-007)

## Phase 8: Convergence

- [x] T036 Add high queue depth warning (>100 entries) on enqueue per `spec.md` (missing)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories.
- **User Stories (Phase 3+)**: All depend on Foundational phase completion.
  - User Story 1 (P1): Depends on Phase 2.
  - User Story 4 (P1): Depends on Phase 2 and US1 basic runner.
  - User Story 2 (P2): Depends on Phase 2.
  - User Story 3 (P2): Depends on Phase 2, US1 runner, and US4 streaming.
- **Polish (Final Phase)**: Depends on all user stories being complete.

### Parallel Opportunities

- **Phase 1**: T002, T003, T004 can run in parallel.
- **Phase 2**: T005, T006, T008 can run in parallel.
- **User Story Tests**: All test creation tasks marked `[P]` can run in parallel.
- **Across Stories**: Once Foundational (Phase 2) is complete, US2 (status inspection) can be implemented in parallel with US1/US4.

---

## Parallel Example: User Story 1

```bash
# Launch test creation for User Story 1 in parallel:
Task: "Integration tests for sequential execution and FIFO ordering in tests/integration/concurrency_test.rs"
Task: "Integration tests for queue run basic command execution in tests/integration/run_test.rs"
Task: "Integration tests for --timeout wait abort in tests/integration/timeout_test.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001 - T004)
2. Complete Phase 2: Foundational (T005 - T012)
3. Complete Phase 3: User Story 1 (T013 - T019)
4. **STOP and VALIDATE**: Verify basic FIFO sequential execution with `cargo test --test concurrency_test`
5. MVP ready for local command sequencing.

### Incremental Delivery

1. Setup + Foundational → Primitives ready
2. User Story 1 → Sequential execution (MVP)
3. User Story 4 → Real-time streaming & exit code forwarding
4. User Story 2 → Queue status inspection (`queue status`, `--json`)
5. User Story 3 → Signal handling (Ctrl+C) & recovery
6. Polish → Depth warnings, quickstart validation, full regression test pass

---

## Phase 9: Convergence

- [x] T037 Fix argument quoting loss (reconstruct command safely) per FR-001 (partial)
- [x] T038 Propagate Unix signal exit codes accurately per SC-002 (partial)
- [x] T039 Clean up compiler warnings (unused_mut, unused imports) per Polish (unrequested)
