use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::error::QueueError;

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
            .truncate(true)
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
            .truncate(true)
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
            .truncate(true)
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
