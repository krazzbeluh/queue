use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::QueueError;
use crate::queue::cleanup::cleanup_stale_entries;
use crate::queue::lock::{ExecutionLock, StateLock};
use crate::queue::state::{EntryStatus, QueueEntry, QueueState};

pub struct QueueManager {
    queue_name: String,
    state_dir: PathBuf,
}

impl QueueManager {
    pub fn new(queue_name: &str) -> Result<Self, QueueError> {
        if !queue_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(QueueError::State(
                "Invalid queue name: must be alphanumeric".to_string(),
            ));
        }

        let state_dir = env::var("QUEUE_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::temp_dir().join("queue"));

        Ok(Self {
            queue_name: queue_name.to_string(),
            state_dir,
        })
    }

    pub fn state_file_path(&self) -> PathBuf {
        self.state_dir
            .join(format!("{}.state.json", self.queue_name))
    }

    pub fn state_lock_path(&self) -> PathBuf {
        self.state_dir
            .join(format!("{}.state.lock", self.queue_name))
    }

    pub fn exec_lock_path(&self) -> PathBuf {
        self.state_dir.join(format!("{}.lock", self.queue_name))
    }

    pub fn get_process_start_time() -> u64 {
        let sys = sysinfo::System::new_all();
        if let Some(process) = sys.process(sysinfo::Pid::from_u32(std::process::id())) {
            process.start_time()
        } else {
            0
        }
    }

    pub fn enqueue_and_wait(
        &self,
        command_str: &str,
        timeout_secs: Option<u64>,
        running_flag: Arc<AtomicBool>,
    ) -> Result<(QueueEntry, ExecutionLock), QueueError> {
        let pid = std::process::id();
        let start_time = Self::get_process_start_time();

        let mut entry = QueueEntry::new(command_str.to_string(), pid, start_time);

        // 1. Enqueue
        {
            let _lock = StateLock::acquire(&self.state_lock_path())?;
            let mut state = QueueState::load(&self.state_file_path(), &self.queue_name)?;
            cleanup_stale_entries(&mut state);
            if state.entries.len() >= 100 {
                eprintln!(
                    "queue: Warning: High queue depth ({} entries). Commands may take a long time to execute.",
                    state.entries.len()
                );
            }
            state.entries.push(entry.clone());
            state.save(&self.state_file_path())?;
        }

        eprintln!(
            "queue: Enqueued in '{}'. Waiting for turn...",
            self.queue_name
        );

        let start_wait = Instant::now();
        loop {
            if !running_flag.load(Ordering::SeqCst) {
                let _lock = StateLock::acquire(&self.state_lock_path())?;
                let mut state = QueueState::load(&self.state_file_path(), &self.queue_name)?;
                state.entries.retain(|e| e.id != entry.id);
                state.save(&self.state_file_path())?;
                return Err(QueueError::Cancelled);
            }

            if let Some(t) = timeout_secs
                && start_wait.elapsed().as_secs() >= t
            {
                let _lock = StateLock::acquire(&self.state_lock_path())?;
                let mut state = QueueState::load(&self.state_file_path(), &self.queue_name)?;
                state.entries.retain(|e| e.id != entry.id);
                state.save(&self.state_file_path())?;
                return Err(QueueError::Timeout);
            }

            let is_first = {
                let _lock = StateLock::acquire(&self.state_lock_path())?;
                let mut state = QueueState::load(&self.state_file_path(), &self.queue_name)?;
                let removed = cleanup_stale_entries(&mut state);
                if removed > 0 {
                    let _ = state.save(&self.state_file_path());
                }

                state
                    .entries
                    .first()
                    .map(|e| e.id == entry.id)
                    .unwrap_or(false)
            };

            if is_first {
                match ExecutionLock::try_acquire(&self.exec_lock_path()) {
                    Ok(exec_lock) => {
                        let _lock = StateLock::acquire(&self.state_lock_path())?;
                        let mut state =
                            QueueState::load(&self.state_file_path(), &self.queue_name)?;
                        if let Some(e) = state.entries.iter_mut().find(|e| e.id == entry.id) {
                            e.status = EntryStatus::Running;
                            e.started_at = Some(chrono::Utc::now().to_rfc3339());
                            entry = e.clone();
                        }
                        state.save(&self.state_file_path())?;
                        eprintln!("queue: Acquired queue, executing command");
                        return Ok((entry, exec_lock));
                    }
                    Err(QueueError::LockError(_)) => {
                        // Another process holds execution lock, keep waiting
                    }
                    Err(e) => return Err(e),
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn complete_entry(&self, entry_id: &str, exit_code: i32) -> Result<(), QueueError> {
        let _lock = StateLock::acquire(&self.state_lock_path())?;
        let mut state = QueueState::load(&self.state_file_path(), &self.queue_name)?;

        if let Some(e) = state.entries.iter_mut().find(|e| e.id == entry_id) {
            e.status = if exit_code == 0 {
                EntryStatus::Completed
            } else {
                EntryStatus::Failed
            };
            e.completed_at = Some(chrono::Utc::now().to_rfc3339());
            e.exit_code = Some(exit_code);
        }

        // Remove completed entries from queue
        state.entries.retain(|e| e.id != entry_id);
        state.save(&self.state_file_path())?;

        Ok(())
    }

    pub fn status_snapshot(&self) -> Result<QueueState, QueueError> {
        let _lock = StateLock::acquire(&self.state_lock_path())?;
        let mut state = QueueState::load(&self.state_file_path(), &self.queue_name)?;
        let removed = cleanup_stale_entries(&mut state);
        if removed > 0 {
            let _ = state.save(&self.state_file_path());
        }
        Ok(state)
    }
}
