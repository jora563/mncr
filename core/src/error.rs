#![allow(dead_code)]
//! Модуль ошибок. В основном обарачивает ошибки из внешних библиотек.
use chat::error::ChatError;
use db::core_schema::ApiId;
use llm::error::LlmError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub(crate) enum CoreError {
    #[error("An error occurred: {0} ({1})")]
    ActixError(String, actix_web::http::StatusCode),
    #[error("Chat API is not connected: {0}")]
    ChatApiDisconnected(ApiId),
    #[error("Chat interface error: {0}")]
    ChatLib(#[from] ChatError),
    #[error("Cannot validate chat: {0}")]
    ChatValidation(String),
    #[error("AI Core Server is misconfigured. Field: {field}, value: {value}")]
    ConfigError { field: String, value: String },
    #[error("DB error: {0}")]
    DbError(#[from] db::error::DbError),
    #[error("Empty set of messages received.")]
    EmptyChat,
    #[error("Environmental variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Http server error: {0}")]
    Http(#[from] actix_web::error::HttpError),
    #[error("WS binary item kind is invalid ({0})")]
    InvalidWsBinItemKind(u8),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("Llm interface error: {0}")]
    LlmError(LlmError),
    #[error("Error initiating logger: {0}")]
    Log(#[from] tracing::dispatcher::SetGlobalDefaultError),
    #[error("No Access: No access for {0} \"{1}\".")]
    NoAccess(&'static str, String),
    #[error("An error occurred: {0}")]
    Other(String),
    #[error("Cannot parse string as integer: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Cannot parse string as Json Object: {0}")]
    ParseJson(#[from] serde_json::Error),
    #[error("Queue interface error: {0}")]
    QueueError(#[from] queue::error::QueueError),
    #[error("Error in Database: {0}")]
    RawSql(#[from] sqlx::Error),
    #[error("Ticket {0} already in use by operator.")]
    TicketInUse(i64),
    #[error("Internal communication error: {0}")]
    TokioError(String),
    #[error("Serialization error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Bad slice: {0}")]
    TryFromSlice(#[from] std::array::TryFromSliceError),
    #[error("Uzor plugin error: {0}")]
    UzorPlugin(#[from] uzor_plugin::error::UzorPluginError),
    #[error("WS binary message is too short ({0})")]
    WsAttachmentTooShort(usize),
    #[error("WS binary message is too short ({0})")]
    WsBytesTooShort(usize),
    #[error("WS closed ({0:?})")]
    WSClose(actix_ws::CloseReason),
    #[error("WS closed error({0:?})")]
    WSCloseError(#[from] actix_ws::Closed),
    #[error("WS Handshake error: {0}")]
    WsHandShakeError(#[from] actix_http::ws::HandshakeError),
}

impl CoreError {
    /// TODO: Decide which errors are critical and which are not.
    pub(crate) fn is_critical(&self) -> bool {
        false
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for CoreError {
    fn from(e: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::TokioError(e.to_string())
    }
}

impl From<LlmError> for CoreError {
    fn from(e: LlmError) -> Self {
        Self::LlmError(e)
    }
}

/// TODO: Найти как заставить актикс нормально код возвращать а не плеваться 500-ым.
impl From<actix_web::error::Error> for CoreError {
    fn from(e: actix_web::error::Error) -> Self {
        let code = e.as_response_error().status_code();
        Self::ActixError(e.to_string(), code)
    }
}

impl From<String> for CoreError {
    fn from(e: String) -> Self {
        Self::Other(e)
    }
}

impl From<actix_ws::CloseReason> for CoreError {
    fn from(e: actix_ws::CloseReason) -> Self {
        Self::WSClose(e)
    }
}
