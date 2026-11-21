use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RfError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid component id: {0}")]
    InvalidComponentId(String),

    #[error("path escapes workspace boundary: {0}")]
    PathEscape(PathBuf),

    #[error("TOML parse error in {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON serialization error: {0}")]
    JsonSerialize(#[from] serde_json::Error),

    #[error("RON parse error in {path}: {source}")]
    RonParse {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("adapter error: {0}")]
    Adapter(String),
}

pub type RfResult<T> = std::result::Result<T, RfError>;
