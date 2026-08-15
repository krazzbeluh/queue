use sysinfo::System;

use crate::queue::state::{EntryStatus, QueueState};

pub fn is_process_alive(pid: u32, expected_start_time: u64) -> bool {
    let sys = System::new_all();

    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
        let start_time = process.start_time();
        let diff = (start_time as i64 - expected_start_time as i64).abs();
        diff <= 5 // Allow 5 seconds of drift
    } else {
        false
    }
}

pub fn cleanup_stale_entries(state: &mut QueueState) -> usize {
    let mut removed = 0;

    state.entries.retain(|entry| match entry.status {
        EntryStatus::Pending | EntryStatus::Running => {
            let alive = is_process_alive(entry.pid, entry.process_start_time);
            if !alive {
                eprintln!(
                    "Warning: Removing stale entry {} (PID {} is dead)",
                    entry.id, entry.pid
                );
                removed += 1;
                false
            } else {
                true
            }
        }
        _ => true,
    });

    removed
}
