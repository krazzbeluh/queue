use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::error::QueueError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaiterEntry {
    pub id: String,
    pub command_type: String,
    pub command: String,
    pub pid: u32,
    pub queued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: String,
    pub command: String,
    pub status: EntryStatus,
    pub pid: u32,
    pub process_start_time: u64,
    pub enqueued_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub exit_code: Option<i32>,
}

impl QueueEntry {
    pub fn new(command: String, pid: u32, process_start_time: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            command,
            status: EntryStatus::Pending,
            pid,
            process_start_time,
            enqueued_at: Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            exit_code: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueState {
    pub queue_name: String,
    pub entries: Vec<QueueEntry>,
    pub version: u32,

    // Computed fields (not persisted to state.json)
    #[serde(skip, default)]
    pub locked: bool,
    #[serde(skip, default)]
    pub lock_reason: Option<String>,
    #[serde(skip, default)]
    pub lock_token: Option<String>,
    #[serde(skip, default)]
    pub locked_at: Option<String>,
    #[serde(skip, default)]
    pub locked_by: Option<String>,
    #[serde(skip, default)]
    pub lock_pid: Option<u32>,
    #[serde(skip, default)]
    pub lock_stale: Option<bool>,
    #[serde(skip, default)]
    pub waiters: Vec<WaiterEntry>,
}

impl QueueState {
    pub fn new(queue_name: String) -> Self {
        Self {
            queue_name,
            entries: Vec::new(),
            version: 1,
            locked: false,
            lock_reason: None,
            lock_token: None,
            locked_at: None,
            locked_by: None,
            lock_pid: None,
            lock_stale: None,
            waiters: Vec::new(),
        }
    }

    pub fn load(path: &Path, queue_name: &str) -> Result<Self, QueueError> {
        if path.exists() {
            let data = fs::read_to_string(path)?;
            let state: QueueState = serde_json::from_str(&data)?;
            Ok(state)
        } else {
            Ok(Self::new(queue_name.to_string()))
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), QueueError> {
        let parent = path.parent().ok_or_else(|| {
            QueueError::State("Invalid state path: no parent directory".to_string())
        })?;
        fs::create_dir_all(parent)?;

        let temp_file = NamedTempFile::new_in(parent)
            .map_err(|e| QueueError::State(format!("Failed to create temp file: {}", e)))?;

        serde_json::to_writer(&temp_file, self)?;
        temp_file
            .persist(path)
            .map_err(|e| QueueError::State(format!("Failed to persist state file: {}", e)))?;
        Ok(())
    }
}

pub fn create_waiter_ticket(
    queue_dir: &Path,
    entry: &WaiterEntry,
) -> Result<std::path::PathBuf, QueueError> {
    let waiters_dir = queue_dir.join("waiters");
    fs::create_dir_all(&waiters_dir)
        .map_err(|e| QueueError::State(format!("Failed to create waiters dir: {}", e)))?;

    let ts = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let filename = format!("{:020}-{}.json", ts, entry.id);
    let path = waiters_dir.join(&filename);

    let temp_file = NamedTempFile::new_in(&waiters_dir)
        .map_err(|e| QueueError::State(format!("Failed to create temp file: {}", e)))?;

    serde_json::to_writer(&temp_file, entry)
        .map_err(|e| QueueError::State(format!("Failed to write waiter entry: {}", e)))?;

    temp_file
        .persist(&path)
        .map_err(|e| QueueError::State(format!("Failed to persist waiter entry: {}", e)))?;

    Ok(path)
}

pub fn remove_waiter_ticket(state_dir: &Path, id: &str) -> std::io::Result<()> {
    let waiters_dir = state_dir.join("waiters");
    for entry in std::fs::read_dir(waiters_dir)? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(&format!("-{}.json", id)) {
                    return std::fs::remove_file(path);
                }
            }
        }
    }
    Ok(())
}

pub struct WaiterGuard {
    state_dir: std::path::PathBuf,
    pub ticket_path: std::path::PathBuf,
    id: String,
}

impl WaiterGuard {
    pub fn new(state_dir: std::path::PathBuf, entry: &WaiterEntry) -> std::io::Result<Self> {
        let ticket_path = create_waiter_ticket(&state_dir, entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(Self {
            state_dir,
            ticket_path,
            id: entry.id.clone(),
        })
    }
}

impl Drop for WaiterGuard {
    fn drop(&mut self) {
        let _ = remove_waiter_ticket(&self.state_dir, &self.id);
    }
}

pub fn list_waiters(queue_dir: &Path) -> Result<Vec<WaiterEntry>, QueueError> {
    let waiters_dir = queue_dir.join("waiters");
    if !waiters_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(&waiters_dir)
        .map_err(|e| QueueError::State(format!("Failed to read waiters dir: {}", e)))?
    {
        if let Ok(entry) = entry {
            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "json")
            {
                files.push(entry.path());
            }
        }
    }

    files.sort();

    for file in files {
        if let Ok(data) = fs::read_to_string(&file) {
            if let Ok(waiter) = serde_json::from_str(&data) {
                entries.push(waiter);
            }
        }
    }
    Ok(entries)
}

pub fn is_my_turn(queue_dir: &Path, my_ticket: &Path) -> Result<bool, QueueError> {
    let waiters_dir = queue_dir.join("waiters");
    if !waiters_dir.exists() {
        return Ok(true);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&waiters_dir)
        .map_err(|e| QueueError::State(format!("Failed to read waiters dir: {}", e)))?
    {
        if let Ok(entry) = entry {
            if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "json")
            {
                files.push(entry.path());
            }
        }
    }

    if files.is_empty() {
        return Ok(true);
    }

    files.sort();

    Ok(files[0] == my_ticket)
}
