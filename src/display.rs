use crate::queue::state::{EntryStatus, QueueState};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct StatusJson {
    pub queue_name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<LockJson>,
    pub running: Option<RunningJson>,
    pub pending: Vec<PendingJson>,
    pub total_pending: usize,
    pub waiters: Vec<WaiterJson>,
}

#[derive(Serialize)]
pub struct LockJson {
    pub reason: String,
    pub token: String,
    pub locked_at: String,
    pub locked_by: Option<String>,
    pub pid: u32,
    pub is_stale: bool,
}

#[derive(Serialize)]
pub struct WaiterJson {
    pub id: String,
    pub command_type: String,
    pub command: String,
    pub pid: u32,
    pub queued_at: String,
}

#[derive(Serialize)]
pub struct RunningJson {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub started_at: String,
    pub elapsed_seconds: i64,
}

#[derive(Serialize)]
pub struct PendingJson {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub enqueued_at: String,
    pub waiting_seconds: i64,
    pub position: usize,
}

fn parse_time(t: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(t)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn print_status(state: &QueueState, as_json: bool) {
    let now = Utc::now();

    let mut running = None;
    let mut pending = Vec::new();

    for entry in &state.entries {
        match entry.status {
            EntryStatus::Running => {
                let elapsed = entry
                    .started_at
                    .as_ref()
                    .and_then(|t| parse_time(t))
                    .map(|t| (now - t).num_seconds())
                    .unwrap_or(0);

                running = Some(RunningJson {
                    id: entry.id.clone(),
                    command: entry.command.clone(),
                    pid: entry.pid,
                    started_at: entry.started_at.clone().unwrap_or_default(),
                    elapsed_seconds: elapsed,
                });
            }
            EntryStatus::Pending => {
                let waiting = parse_time(&entry.enqueued_at)
                    .map(|t| (now - t).num_seconds())
                    .unwrap_or(0);

                pending.push(PendingJson {
                    id: entry.id.clone(),
                    command: entry.command.clone(),
                    pid: entry.pid,
                    enqueued_at: entry.enqueued_at.clone(),
                    waiting_seconds: waiting,
                    position: pending.len() + 1,
                });
            }
            _ => {}
        }
    }

    let is_idle = running.is_none() && pending.is_empty();

    if as_json {
        let lock = if state.locked {
            Some(LockJson {
                reason: state.lock_reason.clone().unwrap_or_default(),
                token: state.lock_token.clone().unwrap_or_default(),
                locked_at: state.locked_at.clone().unwrap_or_default(),
                locked_by: state.locked_by.clone(),
                pid: state.lock_pid.unwrap_or(0),
                is_stale: state.lock_stale.unwrap_or(false),
            })
        } else {
            None
        };

        let waiters: Vec<WaiterJson> = state
            .waiters
            .iter()
            .map(|w| WaiterJson {
                id: w.id.clone(),
                command_type: w.command_type.clone(),
                command: w.command.clone(),
                pid: w.pid,
                queued_at: w.queued_at.to_rfc3339(),
            })
            .collect();

        let out = StatusJson {
            queue_name: state.queue_name.clone(),
            status: if state.locked {
                "locked".to_string()
            } else if is_idle {
                "idle".to_string()
            } else {
                "active".to_string()
            },
            lock,
            running,
            total_pending: pending.len(),
            pending,
            waiters,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("Queue: {}", state.queue_name);
        println!(
            "Status: {}",
            if state.locked {
                "locked"
            } else if is_idle {
                "idle"
            } else {
                "active"
            }
        );

        if state.locked {
            println!();
            println!("🔒 Locked: {}", state.lock_reason.as_deref().unwrap_or(""));
            println!("   Token: {}", state.lock_token.as_deref().unwrap_or(""));
            if let Some(by) = &state.locked_by {
                println!("   By:    {}", by);
            }
            if state.lock_stale.unwrap_or(false) {
                println!(
                    "   State: STALE (PID {} is dead)",
                    state.lock_pid.unwrap_or(0)
                );
            }
        }
        println!();

        if is_idle && !state.locked && state.waiters.is_empty() {
            println!("No commands running or pending.");
            return;
        }

        if let Some(r) = running {
            println!("Running:");
            println!(
                "  [{}] {}  (started {}m {}s ago, PID {})",
                r.id.split('-').next().unwrap_or(&r.id),
                r.command,
                r.elapsed_seconds / 60,
                r.elapsed_seconds % 60,
                r.pid
            );
            println!();
        }

        if !pending.is_empty() {
            println!("Pending ({}):", pending.len());
            for p in pending {
                println!(
                    "  {}. [{}] {}  (waiting {}m {}s, PID {})",
                    p.position,
                    p.id.split('-').next().unwrap_or(&p.id),
                    p.command,
                    p.waiting_seconds / 60,
                    p.waiting_seconds % 60,
                    p.pid
                );
            }
            println!();
        }

        if !state.waiters.is_empty() {
            println!("Waiters ({}):", state.waiters.len());
            for (i, w) in state.waiters.iter().enumerate() {
                let waiting = (now - w.queued_at).num_seconds();
                println!(
                    "  {}. [{}] {} ({})  (waiting {}m {}s, PID {})",
                    i + 1,
                    w.id.split('-').next().unwrap_or(&w.id),
                    w.command,
                    w.command_type,
                    waiting / 60,
                    waiting % 60,
                    w.pid
                );
            }
        }
    }
}

use crate::queue::lock::LockInfo;

pub fn print_lock_success(lock_info: &LockInfo, raw: bool, json: bool) {
    if raw {
        println!("{}", lock_info.token);
    } else if json {
        #[derive(Serialize)]
        struct LockSuccessJson<'a> {
            status: &'a str,
            queue: &'a str,
            token: &'a str,
            reason: &'a str,
            locked_at: String,
        }
        let out = LockSuccessJson {
            status: "locked",
            queue: &lock_info.queue_name,
            token: &lock_info.token,
            reason: &lock_info.reason,
            locked_at: lock_info.locked_at.to_rfc3339(),
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("🔒 Acquired lock on queue \"{}\"", lock_info.queue_name);
        println!("Token:  {}", lock_info.token);
        println!("Reason: {}", lock_info.reason);
    }
}

pub fn print_lock_waiting(queue_name: &str, reason: &str) {
    eprintln!(
        "⏳ Queue \"{}\" is currently locked. Reason: {}. Waiting...",
        queue_name, reason
    );
}

pub fn print_queue_busy_waiting(queue_name: &str) {
    eprintln!(
        "⏳ Queue \"{}\" is currently active. Waiting for it to become available...",
        queue_name
    );
}

pub fn print_lock_timeout(queue_name: &str, timeout_secs: u64) {
    eprintln!(
        "⏱️ Timeout: could not acquire lock on queue \"{}\" within {} seconds.",
        queue_name, timeout_secs
    );
}

pub fn print_release_success(queue_name: &str, json: bool) {
    if json {
        println!("{{\"status\":\"released\",\"queue\":\"{}\"}}", queue_name);
    } else {
        println!("🔓 Released lock on queue \"{}\"", queue_name);
    }
}
