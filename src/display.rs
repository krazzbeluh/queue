use crate::queue::state::{EntryStatus, QueueState};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct StatusJson {
    pub queue_name: String,
    pub status: String,
    pub running: Option<RunningJson>,
    pub pending: Vec<PendingJson>,
    pub total_pending: usize,
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
        let out = StatusJson {
            queue_name: state.queue_name.clone(),
            status: if is_idle {
                "idle".to_string()
            } else {
                "active".to_string()
            },
            running,
            total_pending: pending.len(),
            pending,
        };
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("Queue: {}", state.queue_name);
        println!("Status: {}", if is_idle { "idle" } else { "active" });
        println!();

        if is_idle {
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
        }
    }
}
