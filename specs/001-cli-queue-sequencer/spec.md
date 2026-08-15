# Feature Specification: CLI Queue Sequencer

**Feature Short Name**: `cli-queue-sequencer`

**Created**: 2026-08-15

**Status**: Draft

**Input**: User description: "Develop the 'queue' CLI. A CLI that sequences commands launched simultaneously from multiple separate terminals. For now there will only be a single 'main' queue, but later it should be possible to specify a queue via a parameter."

## Clarifications

### Session 2026-08-15

- Q: Where should the queue store its state files (lock files, queue entries)? → A: OS temp directory (e.g., `$TMPDIR/queue/` or platform equivalent)
- Q: Should a `queue run` invocation wait indefinitely when the queue is busy, or should there be a configurable timeout? → A: Wait indefinitely by default, with an optional `--timeout <seconds>` flag to abort
- Q: Should the queue enforce a maximum number of pending entries? → A: No hard limit; warn to stderr when queue depth exceeds a high threshold

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Enqueue and Wait for Sequential Execution (Priority: P1)

A developer (or AI agent) opens multiple terminal windows and launches commands simultaneously via `queue run <cmd>`. Each invocation enqueues the command into the default "main" queue and blocks until the command has been dispatched, executed, and completed. Commands are executed one at a time in strict FIFO order regardless of how many terminals submitted them concurrently.

**Why this priority**: This is the fundamental value proposition — mutual exclusion of concurrent commands. Without this, the tool has no purpose.

**Independent Test**: Can be fully tested by opening two terminals, running `queue run "echo A"` in one and `queue run "echo B"` in the other simultaneously, and verifying that one completes fully before the other begins. The output of each command is streamed back to the calling terminal.

**Acceptance Scenarios**:

1. **Given** no command is currently running in the "main" queue, **When** a user runs `queue run "echo hello"`, **Then** the command executes immediately, its stdout is streamed to the calling terminal, and the process exits with the same exit code as the wrapped command.
2. **Given** a command is already running in the "main" queue, **When** a second user runs `queue run "echo world"` from another terminal, **Then** the second invocation blocks until the first command completes, and then executes the second command.
3. **Given** two commands are queued, **When** both complete, **Then** each calling terminal receives only its own command's stdout/stderr output and the correct exit code.

---

### User Story 2 - Queue Status Inspection (Priority: P2)

A developer or agent wants to see what is currently running and what is waiting in the queue. They run `queue status` to get a real-time snapshot of the queue state.

**Why this priority**: Observability is essential for debugging and trust, especially in multi-agent scenarios. Without visibility, users cannot diagnose stuck or long-running commands.

**Independent Test**: Can be tested by enqueueing a long-running command (e.g., `sleep 10`), then running `queue status` in another terminal and verifying the output shows the running command and any pending entries.

**Acceptance Scenarios**:

1. **Given** a command is currently running and two more are queued, **When** a user runs `queue status`, **Then** the output displays the currently running command (with identifier and start time) and the pending commands in FIFO order.
2. **Given** the queue is empty and no command is running, **When** a user runs `queue status`, **Then** the output indicates an empty queue with no active command.
3. **Given** a user runs `queue status --json`, **When** the queue has entries, **Then** the output is valid JSON containing the queue state for machine/agent consumption.

---

### User Story 3 - Graceful Signal Handling and Cleanup (Priority: P2)

A user sends Ctrl+C (SIGINT) or SIGTERM to a `queue run` process. The system forwards the signal to the active child process, cleans up queue state, and does not leave stale locks or zombie entries.

**Why this priority**: Without proper signal handling, interrupted commands would leave the queue in a corrupted state, blocking all subsequent commands until manual cleanup.

**Independent Test**: Can be tested by running a long command via `queue run "sleep 60"`, sending Ctrl+C, and then verifying that `queue status` shows an empty/clean state and a new `queue run` can execute immediately.

**Acceptance Scenarios**:

1. **Given** a command is running via `queue run`, **When** the user sends Ctrl+C to the `queue run` process, **Then** the signal is forwarded to the child process, the child terminates, the queue entry is cleaned up, and the next queued command (if any) proceeds.
2. **Given** a `queue run` process is waiting (blocked, not yet executing), **When** the user sends Ctrl+C, **Then** the waiting entry is removed from the queue and the process exits cleanly.
3. **Given** a `queue run` process terminates abnormally (e.g., killed by OS), **When** the next `queue run` or `queue status` is invoked, **Then** stale locks and orphaned queue entries are detected and automatically cleaned up.

---

### User Story 4 - Stream Forwarding with Exit Code Propagation (Priority: P1)

The stdout and stderr of the wrapped command are streamed in real-time back to the calling terminal. The exit code of the wrapped command is propagated as the exit code of the `queue run` process.

**Why this priority**: Transparent I/O and exit code forwarding are required for the tool to be a drop-in wrapper for any command, especially in CI/CD and agent workflows where exit codes determine success/failure.

**Independent Test**: Can be tested by running `queue run "exit 42"` and verifying the process exit code is 42, and by running `queue run "echo hello"` and verifying "hello" appears on stdout.

**Acceptance Scenarios**:

1. **Given** a command that writes to stdout and stderr, **When** executed via `queue run`, **Then** stdout and stderr are streamed to the calling terminal in real-time (not buffered until completion).
2. **Given** a command that exits with code 42, **When** executed via `queue run`, **Then** the `queue run` process also exits with code 42.
3. **Given** a command that writes to stderr, **When** executed via `queue run`, **Then** the stderr output appears on the calling terminal's stderr stream, not mixed into stdout.

---

### User Story 5 - Queue List Inspection (Priority: Deferred/Future)

A user or agent wants to see all active named queues on the system. They run `queue list` to get an overview of all queues, their current running commands, and pending counts.

**Why this priority**: Deferred to a future release because this initial version only supports a single implicit "main" queue. It will be required when named queues are introduced.

**Acceptance Scenarios**:

1. **Given** multiple named queues exist, **When** a user runs `queue list`, **Then** the output displays a summary of all active queues.

---

### Edge Cases

- What happens when two `queue run` invocations start at the exact same instant? → FIFO ordering is still guaranteed; one will acquire the lock first.
- How does the system handle a command that produces no output? → The `queue run` process exits silently with the command's exit code.
- What happens if the queue state file is corrupted or manually edited? → The system detects corruption, logs a warning to stderr, resets the queue state, and proceeds.
- What happens when the wrapped command reads from stdin? → stdin from the calling terminal is forwarded to the child process.
- What happens if the machine reboots while a command is queued? → On next invocation, stale entries are detected (via PID liveness checks or lock file age) and cleaned up automatically.
- What happens when `--timeout` expires while waiting? → The waiting entry is removed from the queue, a timeout error is printed to stderr, and the process exits with a non-zero exit code distinct from wrapped command exit codes.
- What happens if the queue grows very large (e.g., >100 pending entries)? → The system logs a warning to stderr on enqueue but does not reject the entry. No hard cap is enforced.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `queue run <command>` subcommand that enqueues and executes the given command within the default "main" queue.
- **FR-002**: System MUST enforce strict FIFO ordering — commands enqueued first MUST execute first.
- **FR-003**: System MUST guarantee mutual exclusion — only one command executes at a time within a given queue.
- **FR-004**: System MUST block the calling process until its enqueued command has completed execution.
- **FR-005**: System MUST stream stdout and stderr of the wrapped command back to the calling terminal in real-time.
- **FR-006**: System MUST propagate the exit code of the wrapped command as its own exit code.
- **FR-007**: System MUST forward stdin from the calling terminal to the wrapped command's process.
- **FR-008**: System MUST provide a `queue status` subcommand that displays the current queue state (running command, pending commands).
- **FR-009**: System MUST support a `--json` flag on `queue status` to output machine-readable JSON.
- **FR-010**: System MUST forward termination signals (SIGINT, SIGTERM, Ctrl+C) to the active child process.
- **FR-011**: System MUST clean up queue state (locks, entries) when a process terminates normally or abnormally.
- **FR-012**: System MUST detect and clean up stale locks and orphaned queue entries from previously crashed processes.
- **FR-013**: System MUST use atomic file locking or IPC mechanisms that are safe across concurrent processes.
- **FR-014**: System MUST work consistently across Windows, Linux, and macOS.
- **FR-015**: System MUST NOT pollute the stdout stream of the wrapped command with its own diagnostic output; all queue-internal messages MUST go to stderr.
- **FR-016**: System MUST use a single default queue named "main" for this initial version.
- **FR-017**: System architecture MUST be designed to support named queues (specified via a parameter) in a future version, without requiring a rewrite.
- **FR-018**: System MUST support an optional `--timeout <seconds>` flag on `queue run` that aborts the waiting process with exit code 124 (timeout) if the command has not started execution within the specified duration. When no timeout is specified, the process waits indefinitely.

### Key Entities

- **Queue**: A named FIFO sequence of command entries. For this version, only the "main" queue exists. Represents the scheduling boundary for mutual exclusion.
- **Queue Entry**: A single enqueued command with metadata: unique identifier, enqueue timestamp, command string, process identifier of the caller, and execution status (pending, running, completed, failed).
- **Lock**: A mechanism ensuring only one command runs at a time within a queue. Must be atomic, cross-process safe, and self-healing on stale state. Lock files and queue state are stored in the OS temporary directory.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Two commands submitted simultaneously from separate terminals are executed sequentially — the second does not start until the first completes, with zero overlap in execution.
- **SC-002**: The exit code of any wrapped command is propagated exactly to the calling terminal, with 100% fidelity across exit codes 0–255.
- **SC-003**: stdout and stderr of the wrapped command appear in the calling terminal within 100ms of being written by the child process (real-time streaming, not post-execution buffering).
- **SC-004**: After a `queue run` process is killed (Ctrl+C, SIGTERM, or abnormal termination), the next `queue run` invocation succeeds without manual cleanup within 2 seconds.
- **SC-005**: `queue status` returns an accurate snapshot of the queue state within 500ms of invocation.
- **SC-006**: The tool operates identically on Windows, Linux, and macOS for all supported subcommands.
- **SC-007**: Queue overhead adds less than 100ms of latency to the total execution time of a wrapped command.

## Assumptions

- The tool is invoked as a standalone CLI binary (`queue`) available on the user's PATH.
- Users have filesystem write access to the OS temporary directory where queue state files are stored (e.g., `$TMPDIR/queue/`, `%TEMP%\queue\`, or platform equivalent).
- The "main" queue is implicit and does not need to be specified by the user in this version.
- All concurrent `queue run` invocations target the same machine and filesystem (distributed/networked queues are out of scope).
- The internal architecture (queue naming, state storage) will be designed from the start to accommodate multiple named queues in a future release, but only "main" is exposed in this version.
- stdin forwarding is best-effort; interactive TUI applications may not work perfectly through the queue wrapper.
- There is no hard limit on the number of pending queue entries. The system warns on high queue depth but does not reject enqueue requests.
