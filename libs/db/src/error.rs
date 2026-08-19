//! Ошибки которые могут возникнуть в БД.
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, DbError>;

/// Внутренние варианты ошибки.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Bot {0} does not belong to chat with id {1}")]
    AlienBot(String, i64),
    #[error("User {0} does not belong to chat with id {1}")]
    AlienUser(String, i64),
    #[error("Invalid Ticket status: {0}")]
    BadTicketStatus(i16),
    #[error("Error parsing CoreDbConfig: {0}")]
    ConfigParse(#[from] toml::de::Error),
    #[error("Cannot validate {entity}: {reason}")]
    FailedValidation { entity: Box<str>, reason: Box<str> },
    #[error("User account ({0}) and Bot account ({2}) from different platforms ({1} vs {3}).")]
    IncompatibleAccountPlatforms(String, i64, String, i64),
    #[error("Bot account ({0}) and chat platform incompatible ({1} vs {2}).")]
    IncompatibleBotChatPlatforms(String, i64, i64),
    #[error("User account ({0}) and chat platform incompatible ({1} vs {2}).")]
    IncompatibleUserChatPlatforms(String, i64, i64),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid DB config: Max connection count is 0")]
    MaxConnectionZero,
    #[error("Invalid DB config: No log connection counter higher than max")]
    MaxNoLogConnectionHigh,
    #[error("Error in Database: {0}")]
    MigrateSql(#[from] sqlx::migrate::MigrateError),
    #[error("{object} with `{field}` = {value} not found")]
    NotFound {
        object: &'static str,
        field: &'static str,
        value: String,
    },
    #[error("Error in Database: {0}")]
    RawSql(#[from] sqlx::Error),
}

/// NB: Это извращение.
#[cfg(test)]
impl From<DbError> for sqlx::Error {
    fn from(e: DbError) -> Self {
        sqlx::Error::from(std::io::Error::other(e.to_string()))
    }
}

impl DbError {
    /// Create a generic validation failed error.
    pub(crate) fn validation_fail(entity: &str, reason: &str) -> Self {
        Self::FailedValidation {
            entity: entity.into(),
            reason: reason.into(),
        }
    }

    /// Создать вариант "не найдено"
    pub(crate) fn not_found<T: std::fmt::Display>(
        object: &'static str,
        field: &'static str,
        value: T,
    ) -> Self {
        let value = value.to_string();
        Self::NotFound {
            object,
            field,
            value,
        }
    }

    /// The error corresponds to a row not found.
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::NotFound { .. } | Self::RawSql(sqlx::Error::RowNotFound)
        )
    }
}
