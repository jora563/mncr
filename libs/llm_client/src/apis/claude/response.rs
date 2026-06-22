#![allow(dead_code)]

use serde::Deserialize;

use super::ClaudeMessage;
use crate::generic::{Choice, GeneralRole};
use crate::llm::{LlmMessage, LlmResponse};

/// Represents a Claude choice object.
pub type ClaudeChoice = Choice<GeneralRole>;

#[derive(Clone, Debug, Deserialize)]
pub struct CacheCreation {
    ephemeral_1h_input_tokens: u32,
    ephemeral_5m_input_tokens: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerToolUsage {
    web_fetch_requests: u32,
    web_search_requests: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeServiceTier {
    Standard,
    Priority,
    Batch,
}

/// This is an element of the response.
#[derive(Clone, Debug, Deserialize)]
pub struct ClaudeUsage {
    cache_creation: CacheCreation,
    cache_creation_input_tokens: u32,
    cache_read_input_tokens: u32,
    inference_geo: String,
    input_tokens: u32,
    output_tokens: u32,
    server_tool_use: ServerToolUsage,
    service_tier: ClaudeServiceTier,
}

/// Explanation why Claude ended the turn.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeStopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    PauseTurn,
    Refusal,
}

/// The reason for stopping.
#[derive(Clone, Debug, Deserialize)]
pub struct ClaudeStopDetails {
    /// Usually "cyber" or "bio"
    category: String,
    /// Any string.
    explanation: String,
}

/// The container used.
#[derive(Clone, Debug, Deserialize)]
pub struct ClaudeContainer {
    /// Identifier of the container.
    id: String,
    /// Expiry date of container
    expires_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClaudeResponse {
    id: String,
    container: ClaudeContainer,
    content: Vec<ClaudeContent>,
    role: GeneralRole,
    stop_details: ClaudeStopDetails,
    stop_reason: ClaudeStopReason,
    stop_sequence: String,
    usage: ClaudeUsage,
}

impl LlmResponse for ClaudeResponse {
    type Message = ClaudeMessage;
    /// Take all response messages.
    /// NB: We only get text for now. Everything else is ignored for now.
    /// TODO: Fill in everything else.
    fn take_messages(self) -> Vec<Self::Message> {
        self.content
            .into_iter()
            .filter_map(|x| match x {
                ClaudeContent::Text { text } => Some(Self::Message::new_assistant(text)),
                _ => None,
            })
            .collect::<Vec<_>>()
    }
}

/// The content is the kind of content that it has returned.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeContent {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    RedactedThinking {
        data: String,
    },
    /// Also contains `input` and `caller`
    ToolUse {
        id: String,
        name: String,
    },
    ServerToolUse {
        id: String,
        name: String,
    },
    WebSearchToolResult {
        tool_use_id: String,
    },
    WebFetchToolResult {
        tool_use_id: String,
    },
    CodeExecutionToolResult {
        tool_use_id: String,
    },
    BashCodeExecutionToolResult {
        tool_use_id: String,
    },
    TextEditorCodeExecutionToolResult {
        tool_use_id: String,
    },
    ToolSearchToolResult {
        tool_use_id: String,
    },
    ContainerUpload {
        file_id: String,
    },
}
