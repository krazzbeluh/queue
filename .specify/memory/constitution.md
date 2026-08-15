<!--
Sync Impact Report:
- Version change: 1.2.0 → 1.3.0
- Modified principles: None
- Added principles / rules:
  - VII. Relative Paths & Repository-Centric References: all paths specified in Spec-Kit artifacts, specifications, plans, checklists, documentation, and codebase references must be relative to the repository root directory
- Modified sections: None
- Deferred items: None
-->

# Queue Constitution

## Core Principles

### I. CLI First & Unix Philosophy
The system MUST operate as a self-contained, composable, and universal CLI tool (`queue run <cmd>`, `queue status`, etc.). Standard I/O follows Unix philosophy: transparent streaming of `stdin`/`stdout`, error output to `stderr`, and exact propagation of wrapped process exit codes. Text and JSON formats MUST be supported for human and machine/agent consumption.

### II. Safe Concurrency & Mutual Exclusion
The fundamental objective is mutual exclusion and deterministic queue scheduling (FIFO). No resource collision (e.g., target build directories, shared file writes, locks) MUST occur between concurrent processes. File locking and IPC mechanisms MUST be atomic, robust, and immune to stale/orphaned locks on unexpected process termination.

### III. Agent & Multi-Process Interoperability
The CLI MUST be designed for seamless execution by concurrent autonomous AI agents and local developers working on the same repository. Enqueuing MUST support both blocking (wait for completion) and non-blocking modes, offer clear queue inspection (`status`, `list`), and guarantee complete process isolation without cross-contamination.

### IV. Test-First & Concurrency Rigor (NON-NEGOTIABLE)
Test-Driven Development (TDD) is strictly required. Synchronization primitives, queue dispatching, stream capture, and signal propagation MUST be validated via unit and integration tests prior to implementation. Concurrency edge cases (deadlocks, crashes, race conditions, timeouts) MUST be covered by automated test suites.

### V. Observability & Minimal Overhead
Runtime overhead and latency MUST remain minimal. The system MUST provide structured logs and clear visibility into lock and queue states without polluting the standard stdout stream of delegated child commands.

### VI. Open Source & English-First Documentation
Queue is an open-source project. All Spec-Kit artifacts, specifications, implementation plans, code comments, commit messages, and documentation MUST always be authored in English to maintain global accessibility and open-source contribution readiness. When a user provides input in any non-English language, the AI agent MUST translate that input into English before incorporating it into any project artifact (specs, plans, tasks, issues, code comments, commit messages). The original intent and technical meaning MUST be preserved faithfully during translation.

### VII. Relative Paths & Repository-Centric References
All paths specified in Spec-Kit artifacts, specifications, plans, checklists, documentation, and codebase references MUST be relative to the repository root directory (e.g., `specs/001-cli-queue-sequencer/spec.md`, `src/main.rs`). Absolute paths (including machine-specific drive letters or host filesystem prefixes) MUST NEVER be used in any project file or artifact, ensuring full portability across environments and collaborators.

## Concurrency, Resilience & Cross-Platform Constraints

- **Cross-Platform Portability**: The system MUST behave consistently across Windows, Linux, and macOS (using OS-appropriate atomic locking / IPC mechanisms).
- **Signal Forwarding**: Immediate and clean forwarding of signals (`SIGINT`, `SIGTERM`, Ctrl+C) to active child processes without leaving zombie processes or blocked queues.
- **Crash Resilience**: Automatic detection and cleanup of stale locks following abnormal terminations of parent processes or systems.

## Development Workflow & Quality Gates

- Red-Green-Refactor cycle enforced for all code changes.
- Mandatory peer/agent review: strict compliance checks for concurrency safety and deadlock freedom.
- Integration test coverage required for multi-process and multi-agent execution scenarios.

## Governance

This constitution supersedes any ad-hoc implementation choices. Any modification to core principles requires a formal update to this document, a sync impact report, and a semantic version bump (MAJOR for governance/principle removal or redefinition, MINOR for new principles or substantial guidance expansion, PATCH for wording clarifications).

**Version**: 1.3.0 | **Ratified**: 2026-08-15 | **Last Amended**: 2026-08-15
