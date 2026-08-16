use std::fs::{File, OpenOptions};
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::error::QueueError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub queue_name: String,
    pub token: String,
    pub reason: String,
    pub locked_at: DateTime<Utc>,
    pub locked_by: Option<String>,
    pub pid: u32,
}

pub struct StateLock {
    _file: File,
}

impl StateLock {
    pub fn acquire(path: &Path) -> Result<Self, QueueError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        file.lock()
            .map_err(|e| QueueError::LockError(format!("Failed to acquire state lock: {}", e)))?;

        Ok(Self { _file: file })
    }
}

pub struct ExecutionLock {
    _file: File,
}

impl ExecutionLock {
    pub fn acquire(path: &Path) -> Result<Self, QueueError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        file.lock().map_err(|e| {
            QueueError::LockError(format!("Failed to acquire execution lock: {}", e))
        })?;

        Ok(Self { _file: file })
    }

    pub fn try_acquire(path: &Path) -> Result<Self, QueueError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        match file.try_lock() {
            Ok(_) => Ok(Self { _file: file }),
            Err(e) => Err(QueueError::LockError(format!(
                "Failed to try_lock execution lock: {}",
                e
            ))),
        }
    }
}

pub fn read_lock_info(queue_dir: &Path, queue_name: &str) -> Option<LockInfo> {
    let path = queue_dir.join(format!("{}.lock.json", queue_name));
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).ok()
    } else {
        None
    }
}

pub fn write_lock_info(queue_dir: &Path, info: &LockInfo) -> Result<(), QueueError> {
    let path = queue_dir.join(format!("{}.lock.json", info.queue_name));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| QueueError::LockError(format!("Failed to create dir: {}", e)))?;
    }
    let parent = path.parent().unwrap_or(Path::new("."));

    let temp_file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| QueueError::LockError(format!("Failed to create temp file: {}", e)))?;

    serde_json::to_writer(&temp_file, info)
        .map_err(|e| QueueError::LockError(format!("Failed to write lock info: {}", e)))?;

    temp_file
        .persist(&path)
        .map_err(|e| QueueError::LockError(format!("Failed to persist lock info: {}", e)))?;

    Ok(())
}

pub fn remove_lock_info(queue_dir: &Path, queue_name: &str) -> Result<(), QueueError> {
    let path = queue_dir.join(format!("{}.lock.json", queue_name));
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| QueueError::LockError(format!("Failed to remove lock info: {}", e)))?;
    }
    Ok(())
}

pub fn is_lock_stale(lock_info: &LockInfo) -> bool {
    let mut sys = System::new();
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(lock_info.pid)]),
        true,
    );
    sys.process(sysinfo::Pid::from_u32(lock_info.pid)).is_none()
}
