pub mod cli;
pub mod display;
pub mod error;
pub mod queue;
pub mod runner;
pub mod signal;

use clap::Parser;
use cli::{Cli, Commands};
use queue::manager::QueueManager;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            timeout,
            queue,
            command,
            args,
        } => {
            let manager = match QueueManager::new(&queue) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            };

            let mut cmd_str = command.clone();
            for arg in &args {
                cmd_str.push(' ');
                cmd_str.push_str(arg);
            }

            let running = signal::setup_signal_handler();

            match manager.enqueue_and_wait(&cmd_str, timeout, running.clone()) {
                Ok((entry, _exec_lock)) => {
                    let exit_code = match runner::run_command(&command, &args, running) {
                        Ok(code) => code,
                        Err(e) => {
                            eprintln!("queue: {}", e);
                            125
                        }
                    };

                    let _ = manager.complete_entry(&entry.id, exit_code);
                    std::process::exit(exit_code);
                }
                Err(error::QueueError::Timeout) => {
                    eprintln!("queue: Timeout expired while waiting in queue");
                    std::process::exit(124);
                }
                Err(error::QueueError::Cancelled) => {
                    eprintln!("queue: Cancelled by user");
                    std::process::exit(130);
                }
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            }
        }
        Commands::Status { json, queue } => {
            let manager = match QueueManager::new(&queue) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            };

            match manager.status_snapshot() {
                Ok(state) => {
                    display::print_status(&state, json);
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            }
        }
        Commands::Lock {
            timeout,
            raw,
            json,
            queue,
            reason,
        } => {
            let manager = match QueueManager::new(&queue) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            };

            let running = signal::setup_signal_handler();
            let reason_str = reason.join(" ");

            match manager.acquire_lock(&reason_str, timeout, raw, json, running) {
                Ok(lock_info) => {
                    display::print_lock_success(&lock_info, raw, json);
                    std::process::exit(0);
                }
                Err(error::QueueError::Timeout) => {
                    display::print_lock_timeout(&queue, timeout.unwrap_or(0));
                    std::process::exit(124);
                }
                Err(error::QueueError::Cancelled) => {
                    eprintln!("queue: Cancelled by user");
                    std::process::exit(130);
                }
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            }
        }
        Commands::Release {
            queue,
            token,
            force,
        } => {
            let manager = match QueueManager::new(&queue) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(125);
                }
            };

            match manager.release_lock(token.as_deref(), force) {
                Ok(()) => {
                    // We don't have a json flag on the release command in cli.rs, but we can default to false
                    display::print_release_success(&queue, false);
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("queue: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
