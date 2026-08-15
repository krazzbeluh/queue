use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::error::QueueError;

pub fn run_command(
    command: &str,
    args: &[String],
    running_flag: Arc<AtomicBool>,
) -> Result<i32, QueueError> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    let mut child = cmd
        .spawn()
        .map_err(|e| QueueError::Execution(format!("Failed to spawn command: {}", e)))?;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if let Some(code) = status.code() {
                    return Ok(code);
                }
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    if let Some(signal) = status.signal() {
                        return Ok(128 + signal);
                    }
                }
                return Ok(125);
            }
            Ok(None) => {
                if !running_flag.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(130); // SIGINT exit code
                }
            }
            Err(e) => {
                return Err(QueueError::Execution(format!(
                    "Failed to wait for command: {}",
                    e
                )));
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
