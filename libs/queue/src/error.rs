//! Модуль ошибок.
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, QueueError>;

/// Внутренние варианты ошибки.
#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Error parsing CoreDbConfig: {0}")]
    ConfigParse(#[from] toml::de::Error),
    #[error("Db error: {0}")]
    Db(#[from] db::error::DbError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Sqlx transaction related error: {0}")]
    Transaction(#[from] sqlx::error::Error),
    #[error("General queue error: {0}")]
    Other(String),
}

/// NB: Это извращение.
#[cfg(test)]
impl From<&str> for QueueError {
    fn from(e: &str) -> Self {
        Self::Other(e.to_string())
    }
}
