use crate::generic::{GeneralRole, Message};
use crate::llm::{LlmModel, LlmTool};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum ClaudeModel {
    #[serde(rename = "claude-haiku-4-5")]
    Haiku45,
    #[serde(rename = "claude-mythos-preview")]
    MythosPreview,
    #[serde(rename = "claude-opus-4-6")]
    Opus46,
    #[default]
    #[serde(rename = "claude-opus-4-7")]
    Opus47,
    #[serde(rename = "claude-sonnet-4-6")]
    Sonnet46,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClaudeTool;

impl LlmModel for ClaudeModel {}
impl LlmTool for ClaudeTool {}

pub type ClaudeMessage = Message<GeneralRole>;

pub mod client;
pub mod request;
pub mod response;

pub use self::client::ClaudeClient;
pub use self::request::ClaudeRequest;
pub use self::response::ClaudeResponse;
