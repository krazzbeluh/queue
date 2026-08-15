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
    }
}
