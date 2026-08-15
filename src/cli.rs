use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "queue")]
#[command(about = "CLI Queue Sequencer - Serialize concurrent commands")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Enqueue and execute a command in the queue. Blocks until the command has completed.
    Run {
        /// Maximum time to wait in queue before aborting (in seconds).
        #[arg(short, long)]
        timeout: Option<u64>,

        /// Queue name
        #[arg(short, long, default_value = "main")]
        queue: String,

        /// The command to execute
        #[arg(required = true)]
        command: String,

        /// Arguments passed to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Display the current state of the queue
    Status {
        /// Output in JSON format instead of human-readable text
        #[arg(short, long)]
        json: bool,

        /// Queue name
        #[arg(short, long, default_value = "main")]
        queue: String,
    },
}
