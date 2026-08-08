use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    Database(rusqlite::Error),
    Io {
        operation: &'static str,
        source: std::io::Error,
    },
    Serialization(String),
    ResetRequired(String),
    CorruptStorage(String),
    RecoveryFailed(String),
    NotFound(String),
    Validation(String),
    Conflict(String),
    Git(String),
    Unauthorized {
        operation: &'static str,
    },
    ProjectNotOpen,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(f, "database error: {error}"),
            Self::Io { operation, source } => write!(f, "{operation} failed: {source}"),
            Self::Serialization(message) => write!(f, "serialization error: {message}"),
            Self::ResetRequired(message)
            | Self::CorruptStorage(message)
            | Self::RecoveryFailed(message) => f.write_str(message),
            Self::NotFound(message)
            | Self::Validation(message)
            | Self::Conflict(message)
            | Self::Git(message) => f.write_str(message),
            Self::Unauthorized { operation } => write!(f, "unauthorized operation: {operation}"),
            Self::ProjectNotOpen => f.write_str("no project is open"),
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for CoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<std::io::Error> for CoreError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            operation: "filesystem operation",
            source,
        }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error.to_string())
    }
}

impl From<String> for CoreError {
    fn from(message: String) -> Self {
        Self::Validation(message)
    }
}

impl From<&str> for CoreError {
    fn from(message: &str) -> Self {
        Self::Validation(message.to_string())
    }
}
