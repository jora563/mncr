//! Ollama is a "runner" for LLM models which uses a fixesd API
//! which is not dissimilar to the anthropic and openAI APis,
//! but has its own peculiarities.
//! The advantage of Ollama is that it is useful for running
//! LMMs locally on low power machines.
use crate::generic::GeneralRole;
use crate::llm::{LlmMessage, LlmTool};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaResponseType {
    #[serde(rename = "type")]
    tpe: &'static str,
}

impl OllamaResponseType {
    pub const JSON_RESPONSE: OllamaResponseType = OllamaResponseType { tpe: "json" };
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum OllamaResponseFormat {
    #[serde(rename = "json")]
    Json,
}

/// Defines a specific Ollama tool to use.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaToolCall {
    function: OllamaFunctionCall,
}

/// Defines a specific Ollama tool to use.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaTool {
    #[serde(rename = "type")]
    tpe: String,
    function: OllamaFunction,
}

/// This represents a qwen tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaFunctionCall {
    name: String,
    description: String,
    arguments: Vec<String>,
}

/// This represents a qwen tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaFunction {
    name: String,
    description: String,
    parameters: Vec<String>,
}

impl OllamaTool {
    /// Creates a new tool.
    pub fn new<S, T, U, I>(name: S, desc: T, params: I) -> Self
    where
        S: AsRef<str>,
        T: AsRef<str>,
        U: AsRef<str>,
        I: IntoIterator<Item = U>,
    {
        let parameters = params
            .into_iter()
            .map(|x| x.as_ref().to_string())
            .collect::<Vec<_>>();

        Self {
            tpe: "function".to_string(),
            function: OllamaFunction {
                name: name.as_ref().to_string(),
                description: desc.as_ref().to_string(),
                parameters,
            },
        }
    }
}

/// This represents a qwen specific message.
/// Ollama user message specification is especially interesting.
/// The serialized representation should be something like:
/// ```ignore
/// {
///   "role": "system",
///   "content": "You are a cat-girl working in customer services."
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct OllamaMessage {
    pub role: GeneralRole,
    pub content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<OllamaToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

impl Default for OllamaMessage {
    fn default() -> Self {
        Self::new_text("", GeneralRole::Assistant)
    }
}
impl OllamaMessage {
    fn new_text<S: AsRef<str>>(desc: S, role: GeneralRole) -> Self {
        let content = desc.as_ref().to_string();
        Self {
            role,
            content,
            images: Vec::new(),
            tool_calls: Vec::new(),
            thinking: None,
        }
    }
}

impl LlmTool for OllamaTool {}
impl LlmMessage for OllamaMessage {
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
        &self.content
    }
}

pub mod client;
pub mod request;
pub mod response;

pub use client::OllamaClient;
pub use request::OllamaRequest;
pub use response::OllamaResult;
