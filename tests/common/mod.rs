use tempfile::TempDir;
use std::path::PathBuf;

pub struct TestEnv {
    pub temp_dir: TempDir,
    pub queue_name: String,
}

impl TestEnv {
    pub fn new(queue_name: &str) -> Self {
        Self {
            temp_dir: tempfile::tempdir().expect("Failed to create temp dir"),
            queue_name: queue_name.to_string(),
        }
    }

    pub fn state_dir(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }
}
