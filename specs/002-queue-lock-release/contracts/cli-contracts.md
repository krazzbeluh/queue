# CLI Contract: `queue lock`

**Feature**: 002-queue-lock-release  
**Date**: 2026-08-16

## Synopsis

```
queue lock --queue <name> [--timeout <seconds>] [--raw] [--json] <reason>...
```

## Arguments

| Argument | Type | Required | Description |
|---|---|---|---|
| `--queue`, `-q` | `String` | Yes | Name of the queue to lock |
| `--timeout`, `-t` | `u64` | No | Maximum seconds to wait if queue is already locked |
| `--raw` | `flag` | No | Output only the token to stdout (no formatting) |
| `--json` | `flag` | No | Output result in JSON format |
| `<reason>` | `Vec<String>` (positional, trailing) | Yes | Human-readable reason for locking (joined with spaces) |

## Behavior

### Success — Queue Unlocked

**Human output** (stdout):
```
🔒 Queue "ios" locked successfully.
Token: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Reason: E2E Tests
```

**Raw output** (`--raw`, stdout):
```
a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

**JSON output** (`--json`, stdout):
```json
{
  "status": "locked",
  "queue": "ios",
  "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "reason": "E2E Tests",
  "locked_at": "2026-08-16T09:30:00Z"
}
```

**Exit code**: `0`

### Waiting — Queue Already Locked

**Human output** (stderr, while waiting):
```
⏳ Queue "ios" is currently locked.
   Reason: Integration tests
   Waiting for lock to be released...
```

**Raw output** (`--raw`): waiting messages go to stderr, stdout is silent until token acquired.

Once lock is acquired, outputs as per "Success" above.

### Timeout

**Human output** (stderr):
```
⏱️  Timeout: could not acquire lock on queue "ios" within 30 seconds.
```

**Exit code**: `124` (consistent with existing timeout convention)

### Interrupted (Ctrl+C)

**Exit code**: `130` (consistent with existing signal convention)

---

# CLI Contract: `queue release`

## Synopsis

```
queue release --queue <name> (--token <token> | --force)
```

## Arguments

| Argument | Type | Required | Description |
|---|---|---|---|
| `--queue`, `-q` | `String` | Yes | Name of the queue to release |
| `--token` | `String` | Conditional | Token received from `queue lock` (required unless `--force`) |
| `--force` | `flag` | No | Bypass token validation and release unconditionally |

## Behavior

### Success — Valid Token

**Human output** (stdout):
```
🔓 Queue "ios" released successfully.
```

**JSON output** (`--json`, stdout):
```json
{
  "status": "released",
  "queue": "ios"
}
```

**Exit code**: `0`

### Success — Force Release

**Human output** (stdout):
```
🔓 Queue "ios" force-released.
```

**Exit code**: `0`

### Error — Invalid Token

**Human output** (stderr):
```
❌ Error: token does not match the active lock on queue "ios".
```

**Exit code**: `1`

### Error — Queue Not Locked

**Human output** (stderr):
```
❌ Error: queue "ios" is not currently locked.
```

**Exit code**: `1`

### Error — Neither `--token` nor `--force` Provided

**Human output** (stderr):
```
❌ Error: either --token <token> or --force is required.
```

**Exit code**: `2` (usage error)

---

# CLI Contract: `queue status` (Extended)

## Extended Output

### Human-Readable (when queue is locked)

```
Queue: ios
Status: locked
Lock reason: E2E Tests
Lock token: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Locked at: 2026-08-16T09:30:00Z
Locked by: PID 12345

Waiters (2):
  1. [lock] "Deploy staging" (PID 12346, queued at 09:30:05Z)
  2. [run]  "cargo test" (PID 12347, queued at 09:30:10Z)
```

### JSON (when queue is locked)

```json
{
  "queue": "ios",
  "status": "locked",
  "lock": {
    "reason": "E2E Tests",
    "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "locked_at": "2026-08-16T09:30:00Z",
    "pid": 12345,
    "stale": false
  },
  "waiters": [
    {
      "type": "lock",
      "command": "Deploy staging",
      "pid": 12346,
      "queued_at": "2026-08-16T09:30:05Z"
    },
    {
      "type": "run",
      "command": "cargo test",
      "pid": 12347,
      "queued_at": "2026-08-16T09:30:10Z"
    }
  ]
}
```
