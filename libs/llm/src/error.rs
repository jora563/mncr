use thiserror::Error;

use crate::messages::{ErrorDetail, ErrorResponse};

pub type Result<T> = std::result::Result<T, LlmError>;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Error from LLM: {} (code: {})", .0.message, .0.code)]
    AiOmniLlm(ErrorDetail),
    #[error("AI Core Server is misconfigured. Field: {field}, value: {value}")]
    ConfigError { field: String, value: String },
    #[error("Environmental variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Join error: {0}")]
    Join(#[from] tokio::task::JoinError),
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
    #[error("Error in reqwest client: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Bad slice: {0}")]
    TryFromSlice(#[from] std::array::TryFromSliceError),
    #[error("Cannot parse Url: {0}")]
    Url(String),
}

impl LlmError {
    pub(crate) fn url<D: std::fmt::Display>(e: D) -> Self {
        Self::Url(e.to_string())
    }

    pub(crate) fn from_error_response(e: ErrorResponse) -> Self {
        Self::AiOmniLlm(e.error)
    }
}
