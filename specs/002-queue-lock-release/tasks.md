# Tasks: Queue Lock and Release Mechanism

**Input**: Design documents from `specs/002-queue-lock-release/`

**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/cli-contracts.md, quickstart.md

**Tests**: Constitution Principle IV (Test-First & Concurrency Rigor) is **NON-NEGOTIABLE** — tests are included in every user story phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths follow the existing project structure defined in plan.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new error variants, CLI argument definitions, and module scaffolding needed by all user stories.

- [x] T001 Add lock-specific error variants (`InvalidToken`, `QueueNotLocked`, `QueueAlreadyLocked`, `LockAcquisitionTimeout`) to `src/error.rs`
- [x] T002 Add `Lock` and `Release` variants to the `Commands` enum in `src/cli.rs` with clap derive attributes per R-007 (lock: `--queue`, `--timeout`, `--raw`, `--json`, `<reason>`; release: `--queue`, `--token`, `--force`)
- [x] T003 Add dispatch arms for `Commands::Lock` and `Commands::Release` in `src/main.rs` (stub with `todo!()` initially)
- [x] T004 [P] Create `LockInfo` struct with serde derives in `src/queue/lock.rs` per data-model.md (fields: `queue_name`, `token`, `reason`, `locked_at`, `locked_by`, `pid`)
- [x] T005 [P] Create `WaiterEntry` struct with serde derives in `src/queue/state.rs` per data-model.md (fields: `id`, `command_type`, `command`, `pid`, `queued_at`)
- [x] T006 Export new types (`LockInfo`, `WaiterEntry`) from `src/queue/mod.rs`

**Checkpoint**: All new types compile, CLI parses `lock` and `release` subcommands (dispatch hits `todo!()`).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core lock.json read/write, waiter ticket system, and FIFO ordering — infrastructure that ALL user stories depend on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T007 Implement `lock.json` read/write functions in `src/queue/lock.rs`: `read_lock_info(queue_dir) -> Option<LockInfo>`, `write_lock_info(queue_dir, &LockInfo)`, `remove_lock_info(queue_dir)` with atomic file operations (write to temp then rename)
- [x] T008 Implement waiter ticket management in `src/queue/state.rs`: `create_waiter_ticket(queue_dir, &WaiterEntry) -> PathBuf`, `remove_waiter_ticket(path)`, `list_waiters(queue_dir) -> Vec<WaiterEntry>` (sorted by filename = FIFO), `is_my_turn(queue_dir, my_ticket) -> bool`
- [x] T009 Implement stale PID detection helper in `src/queue/lock.rs`: `is_lock_stale(lock_info: &LockInfo) -> bool` using `sysinfo` crate to check if PID is still alive (R-006)
- [x] T010 [P] Add integration test scaffolding — create `tests/integration/cli_lock_test.rs` with module declaration and test helper imports

**Checkpoint**: Foundation ready — lock.json persistence, waiter tickets, and stale detection are unit-testable.

---

## Phase 3: User Story 1 — Lock a Queue for Exclusive Use (Priority: P1) 🎯 MVP

**Goal**: An agent or developer can run `queue lock --queue <name> <reason>` to acquire exclusive access, receive a UUIDv4 token, and block subsequent `queue run` commands on that queue.

**Independent Test**: Run `queue lock --queue ios E2E Tests`, verify token is returned, run `queue status --queue ios` to confirm locked, verify `queue run --queue ios --timeout 2 -- echo blocked` times out with exit code 124.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation (Constitution Principle IV)**

- [x] T011 [P] [US1] Integration test: lock an unlocked queue returns exit 0 and outputs a UUIDv4-formatted token (validate with regex `^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`) in `tests/integration/cli_lock_test.rs`
- [x] T012 [P] [US1] Integration test: lock with `--raw` outputs only the token (no decoration) to stdout in `tests/integration/cli_lock_test.rs`
- [x] T013 [P] [US1] Integration test: `queue run` on a locked queue with `--timeout 2` exits with code 124 in `tests/integration/cli_lock_test.rs`

### Implementation for User Story 1

- [x] T014 [US1] Implement `acquire_lock(queue_name, reason, timeout, raw, json)` in `src/queue/manager.rs` — two-phase locking (R-008): acquire OS file lock, check for existing lock.json, if absent write new lock.json with UUIDv4 token, release OS lock. If locked, create waiter ticket and enter polling loop (500ms, check `is_my_turn` + lock released). Respect `--timeout`.
- [x] T015 [US1] Add lock success output formatting in `src/display.rs` — human (🔒 emoji + token + reason), raw (token only), JSON (`{"status":"locked","queue":"...","token":"...","reason":"...","locked_at":"..."}`) per cli-contracts.md
- [x] T016 [US1] Add lock waiting output (stderr) in `src/display.rs` — `⏳ Queue "<name>" is currently locked. Reason: <reason>. Waiting...` per cli-contracts.md
- [x] T017 [US1] Add lock timeout output (stderr) in `src/display.rs` — `⏱️ Timeout: could not acquire lock on queue "<name>" within <N> seconds.` per cli-contracts.md
- [x] T018 [US1] Wire `Commands::Lock` dispatch in `src/main.rs` to call `acquire_lock` and format output, replacing the `todo!()`
- [x] T019 [US1] Modify `queue run` wait loop in `src/queue/manager.rs` to check for active explicit lock (lock.json present) and create a waiter ticket (command_type = "run") before polling, ensuring `queue run` is blocked by explicit locks (FR-008, FR-013)

**Checkpoint**: `queue lock` works end-to-end. `queue run` is blocked by explicit locks. All US1 tests pass.

---

## Phase 4: User Story 2 — Release a Locked Queue with Token Validation (Priority: P1)

**Goal**: The lock owner releases the queue by providing the token. The system validates the token matches before releasing. Invalid tokens are rejected.

**Independent Test**: Lock a queue, capture token, run `queue release --queue ios --token <valid-token>` → exit 0 and queue unlocked. Run `queue release --queue ios --token wrong-token` → exit 1, queue stays locked.

### Tests for User Story 2

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T020 [P] [US2] Integration test: release with valid token succeeds (exit 0, queue unlocked) in `tests/integration/cli_lock_test.rs`
- [x] T021 [P] [US2] Integration test: release with wrong token fails (exit 1, queue stays locked) in `tests/integration/cli_lock_test.rs`
- [x] T022 [P] [US2] Integration test: release on unlocked queue fails (exit 1, error message) in `tests/integration/cli_lock_test.rs`
- [x] T023 [P] [US2] Integration test: release with neither `--token` nor `--force` fails (exit 2, usage error) in `tests/integration/cli_lock_test.rs`

### Implementation for User Story 2

- [x] T024 [US2] Implement `release_lock(queue_name, token, force)` in `src/queue/manager.rs` — acquire OS file lock, read lock.json, validate token matches, if valid remove lock.json and notify next waiter, if invalid return `InvalidToken` error. Handle queue-not-locked case.
- [x] T025 [US2] Add release success output formatting in `src/display.rs` — human (🔓 emoji), JSON (`{"status":"released","queue":"..."}`) per cli-contracts.md
- [x] T026 [US2] Add release error output (stderr) in `src/display.rs` — invalid token error, queue-not-locked error per cli-contracts.md
- [x] T027 [US2] Wire `Commands::Release` dispatch in `src/main.rs` to call `release_lock` and format output, replacing the `todo!()`
- [ ] T028 [US2] Add clap validation in `src/cli.rs`: `release` requires at least one of `--token` or `--force` (use clap group or manual validation), exit 2 on usage error

**Checkpoint**: Full lock → release cycle works with token validation. All US2 tests pass. Combined with US1, the core lock/release workflow is complete.

---

## Phase 5: User Story 3 — Force Release a Locked Queue (Priority: P2)

**Goal**: An administrator can force-release a stuck queue without the original token, enabling recovery from crashed agents.

**Independent Test**: Lock a queue, discard token, run `queue release --queue ios --force` → exit 0, queue unlocked. Run `queue release --queue ios --force` on unlocked queue → exit 1.

### Tests for User Story 3

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T029 [P] [US3] Integration test: force release without token succeeds (exit 0, queue unlocked) in `tests/integration/cli_lock_test.rs`
- [x] T030 [P] [US3] Integration test: force release on unlocked queue fails (exit 1) in `tests/integration/cli_lock_test.rs`

### Implementation for User Story 3

- [x] T031 [US3] Add `--force` branch to `release_lock()` in `src/queue/manager.rs` — skip token validation, remove lock.json unconditionally (if it exists), notify next waiter
- [ ] T032 [US3] Add force-release output formatting in `src/display.rs` — human (🔓 force-released), JSON per cli-contracts.md

**Checkpoint**: Force release works. All US3 tests pass. Complete crash recovery path validated.

---

## Phase 6: Status Enhancement — Lock & Waiter Visibility

**Goal**: Extend `queue status` to display lock state information (locked/unlocked, reason, token, timestamp, holder, staleness) and waiting agents/commands in FIFO order.

**Independent Test**: Lock a queue, start a background waiter, run `queue status --queue ios` and `queue status --queue ios --json` — verify lock info and waiter list are displayed.

### Tests for Status Enhancement

- [x] T033 [P] Integration test: status of a locked queue shows lock info (reason, token, locked_at) in `tests/integration/cli_lock_test.rs`
- [x] T034 [P] Integration test: status JSON of a locked queue includes `lock` object and `waiters` array in `tests/integration/cli_lock_test.rs`
- [x] T035 [P] Integration test: status of an unlocked queue does not show lock info in `tests/integration/cli_lock_test.rs`

### Implementation for Status Enhancement

- [x] T036 Extend `StatusInfo` struct in `src/display.rs` with optional lock fields (`locked`, `lock_reason`, `lock_token`, `locked_at`, `locked_by`, `lock_pid`, `lock_stale`) and `waiters: Vec<WaiterInfo>` per data-model.md
- [x] T037 Modify status-gathering logic (in `src/queue/manager.rs` or wherever `queue status` is assembled) to read `lock.json` and `waiters/` directory, populate `StatusInfo` lock fields, and check stale PID
- [x] T038 Update human-readable status output in `src/display.rs` to show lock info and waiters list per cli-contracts.md format
- [x] T039 Update JSON status output in `src/display.rs` to include `lock` object and `waiters` array per cli-contracts.md format

**Checkpoint**: `queue status` fully reflects lock state and waiters. All status tests pass.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories, edge cases, and validation.

- [x] T040 [P] Integration test: FIFO ordering — lock queue, start 3 background waiters (lock, run, lock) in order, release first lock, verify they proceed in FIFO order in `tests/integration/cli_lock_test.rs`
- [x] T041 [P] Integration test: lock timeout — lock queue, attempt second lock with `--timeout 2`, verify exit code 124 in `tests/integration/cli_lock_test.rs`
- [x] T041b [P] Integration test: `queue lock --raw` on already-locked queue outputs waiting message to stderr only, then token to stdout after release, in `tests/integration/cli_lock_test.rs`
- [x] T042 Ensure waiter ticket cleanup on process exit/signal (Ctrl+C) — verify `ctrlc` handler or Drop impl removes waiter ticket file in `src/queue/manager.rs` or `src/queue/state.rs`
- [x] T042b [P] Integration test: Ctrl+C during `queue lock` wait exits with code 130 in `tests/integration/cli_lock_test.rs`
- [x] T042c [P] Integration test: Ctrl+C during `queue lock` wait does not corrupt existing lock.json — verify original lock intact after signal in `tests/integration/cli_lock_test.rs`
- [x] T043 Handle corrupted/missing lock.json edge case — if lock.json is unreadable or has invalid JSON, treat as stale lock in `src/queue/lock.rs`
- [ ] T044 [P] Ensure `waiters/` directory is created lazily (on first use) and cleaned up when empty in `src/queue/state.rs`
- [ ] T045 [P] Run `quickstart.md` validation scenarios end-to-end (Scenarios 1–8) and fix any issues
- [ ] T046 Clean up temp file `setup-tasks-output.json` from repo root

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (Phase 2) completion
- **User Story 2 (Phase 4)**: Depends on Foundational (Phase 2) completion. Depends on US1 for the `acquire_lock` function used in test setup.
- **User Story 3 (Phase 5)**: Depends on US2 (`release_lock` function with `--force` branch)
- **Status Enhancement (Phase 6)**: Depends on Foundational (Phase 2). Can start after Phase 2, independent of US1/US2/US3 for struct changes, but integration tests need US1.
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) — No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) — Tests use US1's `queue lock` for setup, but implementation is independent
- **User Story 3 (P2)**: Extends US2's `release_lock()` — starts after US2 implementation
- **Status Enhancement**: Independent struct/format work can start after Phase 2; integration tests need US1

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution Principle IV)
- Data structs (models) before logic (services)
- Core implementation before output formatting
- Output formatting before CLI wiring
- Story complete before moving to next priority

### Parallel Opportunities

- T004 and T005 (LockInfo, WaiterEntry structs) can run in parallel
- All test tasks within a phase marked [P] can run in parallel
- US1 and US2 can largely proceed in parallel once Phase 2 is done (US2 tests use `queue lock` but that's an integration-level dependency)
- Status Enhancement (Phase 6) struct changes can run in parallel with US3

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "Integration test: lock an unlocked queue returns exit 0 and UUID token in tests/integration/cli_lock_test.rs"
Task: "Integration test: lock with --raw outputs only token in tests/integration/cli_lock_test.rs"
Task: "Integration test: queue run on locked queue times out in tests/integration/cli_lock_test.rs"

# After tests fail (TDD), implement:
# T014 (acquire_lock) first, then T015-T017 (display) in parallel, then T018 (wire), T019 (run integration)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001–T006)
2. Complete Phase 2: Foundational (T007–T010)
3. Complete Phase 3: User Story 1 (T011–T019)
4. **STOP and VALIDATE**: `queue lock` works, `queue run` respects locks, tests pass
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → lock works (MVP!)
3. Add User Story 2 → Test independently → release works → full lock/release cycle
4. Add User Story 3 → Test independently → crash recovery
5. Add Status Enhancement → Full observability
6. Polish → FIFO validation, edge cases, quickstart scenarios

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (lock)
   - Developer B: User Story 2 (release) — can work on implementation while A finishes lock
   - Developer C: Status Enhancement (Phase 6) — struct/format work
3. After US1+US2: Developer A takes US3 (extends release_lock)
4. Final: Polish phase together

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- TDD is **mandatory** per Constitution Principle IV — verify tests fail before implementing
- All paths are relative to repo root per Constitution Principle VII
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence

## Phase 8: Convergence

- [ ] T047 Impl�menter m�canisme de verrouillage (acquire_lock) et persistance lock.json per US1 (missing)
- [ ] T048 Impl�menter lib�ration avec validation de jeton (release_lock) per US2 (missing)
- [ ] T049 Impl�menter les tests d'int�gration pour lock/release per Constitution IV (missing)
- [ ] T050 Ajouter la lib�ration forc�e (--force) per US3 (missing)
- [ ] T051 Mettre � jour l'affichage du statut (lock info et waiters) per FR-009 (missing)

