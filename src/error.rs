use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RefactorError {
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    Config(String),

    #[error("Parse error in {file}: {reason}")]
    #[allow(dead_code)]
    Parse { file: PathBuf, reason: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Path safety violation: {0}")]
    PathSafety(String),

    #[error("Migration plan error: {0}")]
    MigrationPlan(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

pub type Result<T> = std::result::Result<T, RefactorError>;

impl RefactorError {
    pub fn exit_code(&self) -> i32 {
        match self {
            RefactorError::Filesystem(_) => 4,
            RefactorError::Config(_) => 3,
            RefactorError::Parse { .. } => 1,
            RefactorError::Validation(_) => 1,
            RefactorError::Conflict(_) => 5,
            RefactorError::PathSafety(_) => 2,
            RefactorError::MigrationPlan(_) => 3,
            RefactorError::Json(_) => 3,
            RefactorError::Toml(_) => 3,
            RefactorError::Regex(_) => 2,
        }
    }
}
