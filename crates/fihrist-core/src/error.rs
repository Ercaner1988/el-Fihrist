use thiserror::Error;

#[derive(Error, Debug)]
pub enum FihristError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration or path error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, FihristError>;
