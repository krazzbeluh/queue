# CLI Contract: Queue Sequencer

**Feature**: cli-queue-sequencer | **Date**: 2026-08-15

## Binary Name

```
queue
```

## Global Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--help` | `-h` | flag | — | Display help information |
| `--version` | `-V` | flag | — | Display version information |

## Subcommands

### `queue run <COMMAND> [ARGS...]`

Enqueue and execute a command in the queue. Blocks until the command has completed.

#### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `COMMAND` | string | yes | The command to execute |
| `ARGS...` | string[] | no | Arguments passed to the command |

#### Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--timeout` | `-t` | integer (seconds) | none (infinite) | Maximum time to wait in queue before aborting. Only counts waiting time, not execution time. |
| `--queue` | `-q` | string | `"main"` | Queue name (reserved for future use; only `"main"` is supported in v1) |

#### Behavior

1. Creates queue state directory if it does not exist
2. Acquires state lock, appends entry to queue, releases state lock
3. Waits until this entry is first in queue
4. Acquires execution lock
5. Updates entry status to `Running`
6. Spawns `COMMAND` with inherited stdin/stdout/stderr
7. Waits for command to complete
8. Acquires state lock, removes entry, releases state lock
9. Releases execution lock
10. Exits with the same exit code as the wrapped command

#### Exit Codes

| Code | Meaning |
|------|---------|
| 0-255 | Propagated from the wrapped command |
| 124 | Timeout expired while waiting in queue (matches GNU `timeout` convention) |
| 125 | Internal queue error (lock acquisition failure, state corruption, etc.) |
| 126 | Command found but not executable |
| 127 | Command not found |
| 130 | Interrupted by SIGINT / Ctrl+C |

#### stdout

Exclusively the wrapped command's stdout. Queue diagnostic messages MUST NOT appear on stdout.

#### stderr

- Queue diagnostic messages (e.g., "Waiting for queue...", "Queue position: 3/5") are written to stderr
- The wrapped command's stderr is also forwarded to stderr
- Warning messages (e.g., high queue depth > 100) are written to stderr

---

### `queue status`

Display the current state of the queue.

#### Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--json` | `-j` | flag | false | Output in JSON format instead of human-readable text |
| `--queue` | `-q` | string | `"main"` | Queue name (reserved for future use) |

#### Human-Readable Output Format

```
Queue: main
Status: active

Running:
  [abc12345] sleep 60  (started 2m 30s ago, PID 12345)

Pending (2):
  1. [def67890] make build  (waiting 1m 15s, PID 23456)
  2. [ghi11223] cargo test  (waiting 0m 45s, PID 34567)
```

When empty:

```
Queue: main
Status: idle

No commands running or pending.
```

#### JSON Output Format (`--json`)

```json
{
  "queue_name": "main",
  "status": "active",
  "running": {
    "id": "abc12345",
    "command": "sleep 60",
    "pid": 12345,
    "started_at": "2026-08-15T17:00:00Z",
    "elapsed_seconds": 150
  },
  "pending": [
    {
      "id": "def67890",
      "command": "make build",
      "pid": 23456,
      "enqueued_at": "2026-08-15T17:01:15Z",
      "waiting_seconds": 75,
      "position": 1
    },
    {
      "id": "ghi11223",
      "command": "cargo test",
      "pid": 34567,
      "enqueued_at": "2026-08-15T17:01:45Z",
      "waiting_seconds": 45,
      "position": 2
    }
  ],
  "total_pending": 2
}
```

When empty:

```json
{
  "queue_name": "main",
  "status": "idle",
  "running": null,
  "pending": [],
  "total_pending": 0
}
```

#### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Status displayed successfully |
| 125 | Internal queue error (cannot read state file, lock failure, etc.) |

---

## Diagnostic Messages (stderr)

All internal messages follow this format and are written to stderr only:

```
queue: <message>
```

Examples:
```
queue: Waiting for queue 'main'... (position 3/5)
queue: Acquired queue, executing command
queue: Warning: queue depth is 150 (threshold: 100)
queue: Cleaning up stale entry [abc12345] (PID 99999 no longer running)
queue: Timeout after 30s waiting in queue
```

## Shell Integration Notes

- On Unix, `COMMAND` is executed via the user's shell: `sh -c "<COMMAND> <ARGS...>"`
- On Windows, `COMMAND` is executed via: `cmd.exe /C "<COMMAND> <ARGS...>"`
- To run a binary directly without shell interpretation, future versions may add a `--exec` flag
