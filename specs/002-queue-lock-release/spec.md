# Feature Specification: Queue Lock and Release Mechanism

**Feature Branch**: `002-queue-lock-release`

**Created**: 2026-08-16

**Status**: Draft

**Input**: User description: "Add lock and release mechanism to queue — the ability to lock a queue for exclusive use (e.g., reserving an iPhone simulator so an agent can run tests without being interrupted) and release it with token-based validation."

## Clarifications

### Session 2026-08-16

- Q: Should `queue lock` support `--timeout` to limit how long it waits when the queue is already locked? → A: Yes, `queue lock` supports `--timeout <seconds>` with the same semantics as `queue run --timeout`.
- Q: When both `queue run` and `queue lock` are waiting on the same locked queue, should lock requests have equal FIFO priority as run requests? → A: Strict FIFO regardless of command type — `lock` and `run` share the same wait queue, served in arrival order.
- Q: Should `queue status` display agents currently waiting to acquire a lock, or only the active lock holder? → A: Show both the active lock holder AND the list of waiting agents/commands.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Lock a Queue for Exclusive Use (Priority: P1)

An agent or developer needs exclusive access to a shared resource (e.g., an iPhone simulator, a deployment environment). They run the lock command specifying the queue name and a reason. The system locks the queue and returns a unique token. While the queue is locked, no other `queue run` commands can execute on that queue — they must wait until the lock is released.

**Why this priority**: This is the foundational capability. Without locking, the entire feature has no value. It enables the core use case of exclusive resource reservation.

**Independent Test**: Can be fully tested by running `queue lock --queue ios E2E Tests`, verifying the queue is locked, confirming a token is returned, and verifying that subsequent `queue run` commands on the same queue are blocked.

**Acceptance Scenarios**:

1. **Given** an unlocked queue named "ios", **When** the user runs `queue lock --queue ios E2E Tests`, **Then** the system locks the queue, returns a human-readable confirmation message containing a unique token, and subsequent `queue run --queue ios` commands are blocked until release.
2. **Given** a queue named "ios" that is already locked, **When** another user runs `queue lock --queue ios Other reason`, **Then** the system displays a waiting message (indicating the queue is currently locked and by whom) and blocks until the existing lock is released, then acquires its own lock and returns a new unique token.
3. **Given** an unlocked queue named "ios", **When** the user runs `queue lock --queue ios --raw E2E Tests`, **Then** the system outputs only the token to stdout with no other text, suitable for script assignment (e.g., `TOKEN=$(queue lock --queue ios --raw E2E Tests)`).

---

### User Story 2 - Release a Locked Queue with Token Validation (Priority: P1)

After completing exclusive work, the lock owner releases the queue by providing the token received during lock. The system validates the token matches the active lock before releasing.

**Why this priority**: Releasing is the mandatory counterpart to locking. Without release, locked queues would remain blocked forever. Token validation is essential to prevent unauthorized releases.

**Independent Test**: Can be fully tested by locking a queue, capturing the token, then running `queue release --queue ios --token <token>`, and verifying the queue is unlocked and subsequent commands can proceed.

**Acceptance Scenarios**:

1. **Given** a locked queue named "ios" with a known token, **When** the user runs `queue release --queue ios --token <valid-token>`, **Then** the system releases the lock, displays a confirmation message, and subsequent `queue run` commands on the queue can proceed.
2. **Given** a locked queue named "ios", **When** the user runs `queue release --queue ios --token <wrong-token>`, **Then** the system rejects the release, displays an error indicating the token does not match, and the queue remains locked.
3. **Given** an unlocked queue named "ios", **When** the user runs `queue release --queue ios --token <any-token>`, **Then** the system displays an error indicating the queue is not currently locked.

---

### User Story 3 - Force Release a Locked Queue (Priority: P2)

An administrator or developer discovers that a queue is stuck because the owning agent crashed without releasing its lock. They force-release the queue without needing the original token.

**Why this priority**: This is a critical recovery mechanism. Without it, a crashed agent could block a shared resource indefinitely. However, it is lower priority than the core lock/release flow because it addresses an exceptional scenario.

**Independent Test**: Can be fully tested by locking a queue, discarding the token, then running `queue release --queue ios --force`, and verifying the queue is unlocked.

**Acceptance Scenarios**:

1. **Given** a locked queue named "ios" whose owning agent has crashed, **When** the user runs `queue release --queue ios --force`, **Then** the system releases the lock without requiring a token and displays a confirmation message.
2. **Given** an unlocked queue named "ios", **When** the user runs `queue release --queue ios --force`, **Then** the system displays an error indicating the queue is not currently locked.

---

### Edge Cases

- What happens when the lock token file is corrupted or missing on disk while the queue reports as locked? The system must detect and handle this gracefully, treating it as a stale lock and allowing `--force` release.
- What happens when multiple agents attempt to lock the same queue simultaneously? Only one acquires the lock immediately; the others block and wait their turn (FIFO). The locking operation must be atomic.
- What happens when the process holding the lock is terminated via `SIGKILL` or a system crash? The lock persists on disk. The `--force` flag or an automatic stale-lock detection mechanism must allow recovery.
- What happens when `queue run` is invoked on a queue that is locked? The command must wait (respecting existing timeout behavior) or fail if the queue remains locked beyond the timeout.
- What happens when `--raw` is used on a queue that is already locked? The system blocks silently (no output) until the lock is acquired, then outputs only the token to stdout. Waiting messages, if any, are sent to stderr so that script assignment captures only the token.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `queue lock` subcommand that accepts `--queue <name>`, an optional `--timeout <seconds>`, and a positional `<reason>` argument, locks the specified queue, and returns a unique token.
- **FR-002**: System MUST provide a `--raw` flag on `queue lock` that outputs only the token to stdout with no additional formatting, enabling script assignment.
- **FR-003**: When a queue is already locked, `queue lock` MUST block and wait until the existing lock is released, displaying a waiting message with the existing lock reason and holder identity (if available), then acquire its own lock and return a new token.
- **FR-004**: System MUST provide a `queue release` subcommand that accepts `--queue <name>` and `--token <token>`, validates the token against the active lock, and releases the queue only if the token matches.
- **FR-005**: System MUST reject release attempts with an invalid or mismatched token, displaying a clear error message while keeping the queue locked.
- **FR-006**: System MUST provide a `--force` flag on `queue release` that bypasses token validation and releases the queue unconditionally.
- **FR-007**: The lock operation MUST be atomic — when multiple agents attempt to lock the same queue simultaneously, exactly one acquires the lock immediately and all others block and wait their turn (FIFO order).
- **FR-008**: When a queue is locked, `queue run` commands targeting that queue MUST wait (following existing timeout behavior) until the lock is released before executing.
- **FR-009**: The `queue status` subcommand MUST display lock state information (locked/unlocked, reason, timestamp, holder identity) and the list of waiting agents/commands (in FIFO order) for the inspected queue, in both human-readable and JSON output formats.
- **FR-010**: Lock tokens MUST be cryptographically random or sufficiently unique to prevent guessing (e.g., UUIDv4).
- **FR-011**: System MUST output errors to stderr and use appropriate non-zero exit codes for failure scenarios (invalid token, queue not locked, timeout exceeded while waiting for lock).
- **FR-012**: When `queue lock --timeout <seconds>` is specified and the lock cannot be acquired within the given duration, the command MUST exit with the same timeout exit code used by `queue run --timeout`.
- **FR-013**: `queue lock` and `queue run` commands waiting on the same queue MUST share a single FIFO wait queue and be served strictly in arrival order, with no priority distinction between command types.

### Key Entities

- **Lock**: Represents an exclusive hold on a queue. Key attributes: queue name, token, reason, creation timestamp, holder identity (PID of the locking process; populated automatically).
- **Token**: A unique, opaque string generated at lock time, used to authorize the release of a lock.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can lock and release a queue in under 1 second per operation under normal conditions.
- **SC-002**: When two agents attempt to lock the same queue simultaneously, exactly one acquires the lock immediately and the other blocks until the first lock is released, then acquires its own lock, with 100% consistency across repeated runs.
- **SC-003**: A crashed agent's lock can be recovered using `--force` release in under 5 seconds of operator intervention.
- **SC-004**: Script-based workflows can programmatically capture the lock token using `--raw` and pass it to `release` without any manual parsing or text processing.
- **SC-005**: Queue status accurately reflects lock state (locked/unlocked, reason, timestamp) in both text and JSON output formats immediately after any lock or release operation.

## Assumptions

- The existing file-based locking infrastructure in `~/.queue/` (or `QUEUE_STATE_DIR`) will be extended to support the new lock/release mechanism alongside the existing run-queue file locks.
- Lock state will be persisted to the filesystem, consistent with the existing architecture (no external database or service required).
- The `queue run` command's existing timeout mechanism will naturally apply when waiting for a locked queue, requiring no separate timeout feature for lock waits.
- Token generation will use standard platform-provided randomness (e.g., UUID) which is sufficient for this use case — this is a local coordination tool, not a networked security system.
- Signal handling (Ctrl+C, SIGTERM) during a `queue run` wait-for-lock scenario follows the same behavior as the existing wait-for-queue logic: the waiting command is interrupted and exits cleanly without affecting the lock.
