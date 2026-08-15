# queue - A minimal cross-platform command sequencer

`queue` is a lightweight, zero-dependency CLI tool that forces commands to run sequentially using file-based locking. 

## Features
- **Zero dependencies**: No daemon, no background service, no server needed.
- **Cross-platform**: Works on Linux, macOS, and Windows.
- **Transparent**: Full exit code propagation and real-time I/O stream forwarding.
- **Timeout support**: Automatically aborts waiting if a command takes too long to get its turn.
- **Graceful termination**: Handles Ctrl+C safely, terminating child processes and cleaning up lock states.

## Installation

```bash
cargo install --path .
```

## Usage

### Run a command sequentially
```bash
queue run -- <COMMAND> [ARGS...]
```

Example:
```bash
queue run -- npm install
queue run -- npm run build
```
These two commands will execute strictly sequentially, even if spawned concurrently in different terminal windows.

### Named Queues
Use a different named queue for separate execution lanes:
```bash
queue run --queue deploys -- ./deploy.sh
```

### Timeouts
Specify a max wait time before giving up (useful for CI/CD):
```bash
queue run --timeout 60 -- npm install
```

### Check Queue Status
View currently running and pending commands:
```bash
queue status
queue status --json
```

## Architecture
`queue` relies exclusively on local filesystem locking (`std::fs::File::lock`). All state is maintained in `~/.queue/` (or the directory specified by `QUEUE_STATE_DIR`).
