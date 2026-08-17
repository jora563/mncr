#![allow(dead_code)]
//! Модуль ошибок. В основном обарачивает ошибки из внешних библиотек.
use chat::error::ChatError;
use db::core_schema::ApiId;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub(crate) enum CoreError {
    #[error("No Access: No access for {0} \"{1}\".")]
    NoAccess(&'static str, String),
    #[error("AI Core Server is misconfigured. Field: {field}, value: {value}")]
    ConfigError { field: String, value: String },
    #[error("Environmental variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error initiating logger: {0}")]
    Log(#[from] tracing::dispatcher::SetGlobalDefaultError),
    #[error("Serialization error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("DB error: {0}")]
    DbError(#[from] db::error::DbError),
    #[error("Chat interface error: {0}")]
    ChatLib(#[from] ChatError),
    #[error("Empty set of messages received.")]
    EmptyChat,
    #[error("Chat API is not connected: {0}")]
    ChatApiDisconnected(ApiId),
    #[error("Cannot validate chat: {0}")]
    ChatValidation(String),
    #[error("Llm interface error")]
    LlmError(llm_client::LlmError),
    #[error("An error occurred: {0}")]
    Other(String),
    #[error("Http server error: {0}")]
    Http(#[from] actix_web::error::HttpError),
    #[error("Cannot parse string as integer: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Queue interface error")]
    QueueError,
    #[error("Uzor plugin error: {0}")]
    UzorPlugin(#[from] uzor_plugin::error::UzorPluginError),
}

impl CoreError {
    /// TODO: Decide which errors are critical and which are not.
    pub(crate) fn is_critical(&self) -> bool {
        false
    }
}

// impl From<actix_web::error::HttpError> for CoreError {
//     fn from(a: actix_web::error::HttpError) -> Self {
//         Self::Http(a.to_string())
//     }
// }

impl From<llm_client::LlmError> for CoreError {
    fn from(e: llm_client::LlmError) -> Self {
        Self::LlmError(e)
    }
}

impl From<String> for CoreError {
    fn from(e: String) -> Self {
        Self::Other(e)
    }
}
