# Data Model: Queue Lock and Release Mechanism

**Feature**: 002-queue-lock-release  
**Date**: 2026-08-16

## Entities

### LockInfo

Represents an explicit lock held on a queue, persisted as `lock.json` in the queue's state directory.

| Field | Type | Description | Constraints |
|---|---|---|---|
| `queue_name` | `String` | Name of the locked queue | Required, non-empty |
| `token` | `String` | UUIDv4 token for release authorization | Required, generated at lock time |
| `reason` | `String` | Human-readable reason for locking | Required (from positional args) |
| `locked_at` | `DateTime<Utc>` (ISO-8601 string) | Timestamp when lock was acquired | Required, auto-generated |
| `locked_by` | `Option<String>` | Identity of the lock holder, formatted as `"PID <id>"` from `std::process::id()` | Optional, populated automatically at lock time |
| `pid` | `u32` | PID of the process that acquired the lock | Required, for stale detection |

**Serialized format** (`lock.json`):
```json
{
  "queue_name": "ios",
  "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "reason": "E2E Tests",
  "locked_at": "2026-08-16T09:30:00Z",
  "locked_by": "agent-42",
  "pid": 12345
}
```

**Validation rules**:
- `token` must be a valid UUIDv4 string
- `queue_name` must match the parent directory name
- `locked_at` must be a valid ISO-8601 timestamp

**State transitions**:
```
Unlocked ──[queue lock]──► Locked (lock.json created)
Locked ──[queue release --token <valid>]──► Unlocked (lock.json removed)
Locked ──[queue release --force]──► Unlocked (lock.json removed)
Locked ──[invalid token release]──► Locked (no change)
```

---

### WaiterEntry

Represents a process waiting to acquire either a lock or to run a command. Persisted as individual files in `<state_dir>/<queue_name>/waiters/`.

| Field | Type | Description | Constraints |
|---|---|---|---|
| `id` | `String` | Unique identifier for this waiter (UUIDv4) | Required, auto-generated |
| `command_type` | `String` | Either `"lock"` or `"run"` | Required, enum |
| `command` | `String` | The command or lock reason | Required |
| `pid` | `u32` | PID of the waiting process | Required |
| `queued_at` | `DateTime<Utc>` (ISO-8601 string) | Timestamp when the waiter joined the queue | Required, auto-generated |

**File naming**: `<timestamp_nanos>-<uuid>.json` — the nanosecond-precision timestamp prefix ensures lexicographic sort equals FIFO order.

**Serialized format** (e.g., `1723799400000000000-a1b2c3d4.json`):
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "command_type": "lock",
  "command": "E2E Tests",
  "pid": 12346,
  "queued_at": "2026-08-16T09:30:05Z"
}
```

**Lifecycle**:
```
Process arrives ──► WaiterEntry file created in waiters/
Process acquires lock/run ──► WaiterEntry file removed
Process times out ──► WaiterEntry file removed
Process interrupted ──► WaiterEntry file removed
```

---

### Extended QueueStateEntry (modification to existing)

The existing `QueueStateEntry` in `state.json` is extended with optional lock-related fields for `queue status` output.

| New Field | Type | Description |
|---|---|---|
| `locked` | `bool` | Whether the queue has an active explicit lock |
| `lock_reason` | `Option<String>` | The lock reason, if locked |
| `lock_token` | `Option<String>` | The lock token (shown in status) |
| `locked_at` | `Option<String>` | When the lock was acquired |
| `locked_by` | `Option<String>` | Who holds the lock |
| `lock_pid` | `Option<u32>` | PID of lock holder |
| `lock_stale` | `Option<bool>` | Whether lock appears stale (PID dead) |
| `waiters` | `Vec<WaiterInfo>` | List of waiting agents/commands |

**Note**: These fields are computed at status-read time from `lock.json` and `waiters/` directory — they are NOT persisted in `state.json` itself.

---

## Relationships

```
Queue (directory)
├── queue.lock          # OS-level file lock (atomicity, existing)
├── state.json          # Run state metadata (existing)
├── lock.json           # Explicit lock metadata (NEW)
└── waiters/            # FIFO wait queue (NEW)
    ├── <ts>-<uuid>.json  # WaiterEntry 1
    ├── <ts>-<uuid>.json  # WaiterEntry 2
    └── ...
```

- A `Queue` has zero or one `LockInfo` (1:0..1)
- A `Queue` has zero or more `WaiterEntry` instances (1:0..*)
- A `LockInfo` is independent of `QueueStateEntry` — a queue can be locked without any command running
- `WaiterEntry` instances can be for either `lock` or `run` command types
