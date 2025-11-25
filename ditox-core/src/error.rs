use thiserror::Error;

#[derive(Error, Debug)]
pub enum DitoxError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, DitoxError>;
