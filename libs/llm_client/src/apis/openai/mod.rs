//! OpenAi is a "runner" for LLM models which uses a fixesd API
//! which is not dissimilar to the anthropic and openAI APis,
//! but has its own peculiarities.
//! The advantage of OpenAi is that it is useful for running
//! LMMs locally on low power machines.
use crate::generic::GeneralRole;
use crate::llm::{LlmMessage, LlmTool};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OpenAiResponseType {
    #[serde(rename = "type")]
    tpe: &'static str,
}

impl OpenAiResponseType {
    pub const JSON_RESPONSE: OpenAiResponseType = OpenAiResponseType { tpe: "json" };
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum OpenAiResponseFormat {
    #[serde(rename = "json")]
    #[default]
    Json,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenAiPrompt {
    description: String,
}

/// Defines a specific OpenAi tool to use.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenAiTool {
    id: String,
    index: u64,
    #[serde(rename = "type")]
    tpe: String,
    function: OpenAiToolCall,
}

/// This represents a qwen tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenAiToolCall {
    name: String,
    arguments: String,
}

/// This represents a qwen tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenAiFunctionCall {
    name: String,
    description: String,
    arguments: Vec<String>,
}

/// This represents a qwen tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenAiFunction {
    name: String,
    description: String,
    parameters: Vec<String>,
}

impl OpenAiTool {
    /// Creates a new tool.
    pub fn new<S, T>(name: S, desc: T) -> Self
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        Self {
            id: String::new(),
            index: 5,
            tpe: "function".to_string(),
            function: OpenAiToolCall {
                name: name.as_ref().to_string(),
                arguments: desc.as_ref().to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OpenAiMessageContent {
    Outgoing { description: String },
    Incoming(String),
}

impl Default for OpenAiMessageContent {
    fn default() -> Self {
        Self::Incoming(String::default())
    }
}

/// This represents a qwen specific message.
/// OpenAi user message specification is especially interesting.
/// The serialized representation should be something like:
/// ```ignore
/// {
///   "role": "system",
///   "content": "You are a cat-girl working in customer services."
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct OpenAiMessage {
    pub content: OpenAiMessageContent,
    pub role: GeneralRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<OpenAiFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<OpenAiToolCall>,
}

impl Default for OpenAiMessage {
    fn default() -> Self {
        Self::new_text("", GeneralRole::Assistant)
    }
}
impl OpenAiMessage {
    fn new_text<S: AsRef<str>>(desc: S, role: GeneralRole) -> Self {
        let content = OpenAiMessageContent::Incoming(desc.as_ref().to_string());
        Self {
            role,
            content,
            function_call: None,
            name: None,
            images: Vec::new(),
            tool_calls: Vec::new(),
        }
    }
}

impl LlmTool for OpenAiTool {}
impl LlmMessage for OpenAiMessage {
    fn new_assistant<S: AsRef<str>>(desc: S) -> Self {
        Self::new_text(desc, GeneralRole::Assistant)
    }
    fn new_system<S: AsRef<str>>(desc: S) -> Self {
        Self::new_text(desc, GeneralRole::System)
    }
    fn new_user<S: AsRef<str>>(desc: S) -> Self {
        Self::new_text(desc, GeneralRole::User)
    }
    fn new_tool<S: AsRef<str>>(desc: S) -> Self {
        Self::new_text(desc, GeneralRole::Assistant)
    }
    fn content(&self) -> &str {
        match &self.content {
            OpenAiMessageContent::Outgoing { description } => description,
            OpenAiMessageContent::Incoming(x) => x,
        }
    }
}

pub mod client;
pub mod request;
pub mod response;

pub use client::OpenAiClient;
pub use request::OpenAiRequest;
pub use response::OpenAiResult;
