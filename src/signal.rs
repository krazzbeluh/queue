use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn setup_signal_handler() -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Ignore error if handler is already set (useful in tests)
    let _ = ctrlc::set_handler(move || {
        eprintln!("queue: Interrupted by user");
        r.store(false, Ordering::SeqCst);
    });

    running
}
