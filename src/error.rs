use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Queue state error: {0}")]
    State(String),

    #[error("Lock acquisition failed: {0}")]
    LockError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Process execution error: {0}")]
    Execution(String),

    #[error("Timeout waiting for lock")]
    Timeout,

    #[error("Invalid token provided for lock release")]
    InvalidToken,

    #[error("Queue is not currently locked")]
    QueueNotLocked,

    #[error("Queue is already locked")]
    QueueAlreadyLocked,

    #[error("Timeout acquiring explicit lock")]
    LockAcquisitionTimeout,

    #[error("Cancelled by user signal")]
    Cancelled,
}
