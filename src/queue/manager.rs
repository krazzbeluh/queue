use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::QueueError;
use crate::queue::cleanup::cleanup_stale_entries;
use crate::queue::lock::{
    ExecutionLock, LockInfo, StateLock, is_lock_stale, read_lock_info, remove_lock_info,
    write_lock_info,
};
use crate::queue::state::{
    EntryStatus, QueueEntry, QueueState, WaiterEntry, WaiterGuard, is_my_turn,
};
use uuid::Uuid;

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

        let waiter_entry = WaiterEntry {
            id: entry.id.clone(),
            command_type: "run".to_string(),
            command: command_str.to_string(),
            pid,
            queued_at: chrono::Utc::now(),
        };
        let _waiter_guard = WaiterGuard::new(self.state_dir.clone(), &waiter_entry)?;
        let ticket_path = &_waiter_guard.ticket_path;

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

            let turn = is_my_turn(&self.state_dir, ticket_path).unwrap_or(false);

            if is_first && turn {
                match ExecutionLock::try_acquire(&self.exec_lock_path()) {
                    Ok(exec_lock) => {
                        let mut can_run = true;
                        if let Some(existing_lock) =
                            read_lock_info(&self.state_dir, &self.queue_name)
                        {
                            if is_lock_stale(&existing_lock) {
                                let _ = remove_lock_info(&self.state_dir, &self.queue_name);
                            } else {
                                can_run = false;
                            }
                        }

                        if can_run {
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

        if let Some(lock_info) = read_lock_info(&self.state_dir, &self.queue_name) {
            state.locked = true;
            state.lock_reason = Some(lock_info.reason.clone());
            state.lock_token = Some(lock_info.token.clone());
            state.locked_at = Some(lock_info.locked_at.to_rfc3339());
            state.locked_by = lock_info.locked_by.clone();
            state.lock_pid = Some(lock_info.pid);
            state.lock_stale = Some(is_lock_stale(&lock_info));
        }

        state.waiters = crate::queue::state::list_waiters(&self.state_dir).unwrap_or_default();

        Ok(state)
    }

    pub fn acquire_lock(
        &self,
        reason: &str,
        timeout_secs: Option<u64>,
        raw: bool,
        json: bool,
        running_flag: Arc<AtomicBool>,
    ) -> Result<LockInfo, QueueError> {
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(std::process::id())]),
            true,
        );
        let pid = if let Some(process) = sys.process(sysinfo::Pid::from_u32(std::process::id())) {
            process
                .parent()
                .map(|p| p.as_u32())
                .unwrap_or_else(std::process::id)
        } else {
            std::process::id()
        };

        let entry = WaiterEntry {
            id: Uuid::new_v4().to_string(),
            command_type: "lock".to_string(),
            command: reason.to_string(),
            pid: std::process::id(), // Waiter uses its own PID since it's actively waiting
            queued_at: chrono::Utc::now(),
        };

        let _waiter_guard = WaiterGuard::new(self.state_dir.clone(), &entry)?;
        let ticket_path = &_waiter_guard.ticket_path;

        let mut has_printed_wait = false;
        let start_wait = Instant::now();

        loop {
            if !running_flag.load(Ordering::SeqCst) {
                return Err(QueueError::Cancelled);
            }

            if let Some(t) = timeout_secs
                && start_wait.elapsed().as_secs() >= t
            {
                return Err(QueueError::Timeout);
            }

            let existing_lock_info = read_lock_info(&self.state_dir, &self.queue_name);
            let mut is_locked = false;
            let mut locked_reason = String::new();
            if let Some(lock) = &existing_lock_info
                && !is_lock_stale(lock)
            {
                is_locked = true;
                locked_reason = lock.reason.clone();
            }

            if is_my_turn(&self.state_dir, ticket_path).unwrap_or(false)
                && let Ok(_exec_lock) = ExecutionLock::try_acquire(&self.exec_lock_path())
                && !is_locked
            {
                if let Some(lock) = &existing_lock_info
                    && is_lock_stale(lock)
                {
                    let _ = remove_lock_info(&self.state_dir, &self.queue_name);
                }

                let token = Uuid::new_v4().to_string();
                let locked_at = chrono::Utc::now();
                let lock_info = LockInfo {
                    queue_name: self.queue_name.clone(),
                    reason: reason.to_string(),
                    token: token.clone(),
                    locked_at,
                    locked_by: Some(format!("PID {}", pid)),
                    pid,
                };

                write_lock_info(&self.state_dir, &lock_info)?;
                return Ok(lock_info);
            }

            if !raw && !json && !has_printed_wait {
                if is_locked {
                    crate::display::print_lock_waiting(&self.queue_name, &locked_reason);
                } else {
                    crate::display::print_queue_busy_waiting(&self.queue_name);
                }
                has_printed_wait = true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn release_lock(&self, token: Option<&str>, force: bool) -> Result<(), QueueError> {
        let _exec_lock = ExecutionLock::acquire(&self.exec_lock_path())?;

        if let Some(existing_lock) = read_lock_info(&self.state_dir, &self.queue_name) {
            if force || token == Some(existing_lock.token.as_str()) {
                remove_lock_info(&self.state_dir, &self.queue_name)?;
                Ok(())
            } else {
                Err(QueueError::InvalidToken)
            }
        } else {
            Err(QueueError::QueueNotLocked)
        }
    }
}
