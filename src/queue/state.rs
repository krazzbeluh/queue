use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::error::QueueError;

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
}

impl QueueState {
    pub fn new(queue_name: String) -> Self {
        Self {
            queue_name,
            entries: Vec::new(),
            version: 1,
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
