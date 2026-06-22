//! This module contains the types for using the Qwen LLM.
use crate::llm::{LlmMessage, LlmModel, LlmTool};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QwenResponseType {
    #[serde(rename = "type")]
    tpe: &'static str,
}

impl QwenResponseType {
    pub const JSON_RESPONSE: QwenResponseType = QwenResponseType { tpe: "json_object" };
}

/// This is an enum that defines which specific Qwen model to use.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum QwenModel {
    #[serde(rename = "qwen3.6-plus")]
    Qwen36Plus,
    #[serde(rename = "qwen3.6-flash")]
    Qwen36Flash,
    #[serde(rename = "qwen3.5-plus")]
    Qwen35Plus,
    #[serde(rename = "qwen3.5-flash")]
    Qwen35Flash,
    #[serde(rename = "qwen3.5-27b")]
    Qwen35_27B,
    /// Cannot use tools
    #[default]
    #[serde(rename = "qwen2.5-omni-7b")]
    Qwen25Omni7B,
    /// Can use tools
    #[serde(rename = "qwen2.5-instruct-7b")]
    Qwen25Instruct7B,
    /// Ollama's qwen2.5
    #[serde(rename = "qwen2.5")]
    Qwen25Ollama,
}

/// Defines a specific Qwen tool to use.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QwenTool {
    #[serde(rename = "type")]
    tpe: String,
    function: QwenFunction,
}

/// This represents a qwen tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QwenFunction {
    name: String,
    description: String,
    parameters: Vec<String>,
}

impl QwenTool {
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
            function: QwenFunction {
                name: name.as_ref().to_string(),
                description: desc.as_ref().to_string(),
                parameters,
            },
        }
    }
}

/// This represents a qwen specific message.
/// Qwen user message specification is especially interesting.
/// The serialized representation should be something like:
/// ```ignore
/// {
///   "role": "system",
///   "content": "You are a cat-girl working in customer services."
/// }
/// ```
/// or:
/// ```ignore
/// {
///   "role": "user",
///   "content": {
///     "text": "I came here for a meeting. Where is meeting room 3?",
///   }
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "role", content = "content", rename_all = "lowercase")]
pub enum QwenMessage {
    User(QwenUserMsg),
    System(String),
    Assistant(QwenAssistantMsg),
    /// TODO: Decide how to work with this, since it's not really supported.
    Tool,
}
/// This is an implementation that should not be used.
impl Default for QwenMessage {
    fn default() -> Self {
        Self::Tool
    }
}

/// A qwen user message. By default we only need the `text` prompt field,
/// but images and videos can also be provided to some (but not all) models.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct QwenUserMsg {
    pub text: String,
    /// Provides path or base64 encoded data.
    pub image: Option<String>,
    /// Provide video file path. (TODO: Can also be image list...)
    pub video: Option<String>,
    /// FPS setting for video 0.1 - 10.
    pub fps: Option<f32>,
    pub max_frames: Option<u32>,
    pub min_pixels: Option<u32>,
    pub max_pixels: Option<u32>,
    pub total_pixels: Option<u32>,
}

impl QwenUserMsg {
    fn new_text(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            ..Default::default()
        }
    }
}

/// A qwen assistant message. It is returned by the LLM.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QwenAssistantMsg {
    pub text: String,
    pub image_hw: Option<[u32; 2]>,
}

impl LlmModel for QwenModel {}
impl LlmTool for QwenTool {}
impl LlmMessage for QwenMessage {
    fn new_assistant<S: AsRef<str>>(description: S) -> Self {
        Self::Assistant(QwenAssistantMsg {
            text: description.as_ref().to_string(),
            image_hw: None,
        })
    }
    fn new_system<S: AsRef<str>>(description: S) -> Self {
        Self::System(description.as_ref().to_string())
    }
    fn new_user<S: AsRef<str>>(description: S) -> Self {
        Self::User(QwenUserMsg::new_text(description.as_ref()))
    }
    fn new_tool<S: AsRef<str>>(_: S) -> Self {
        Self::Tool
    }
    fn content(&self) -> &str {
        match self {
            Self::System(x) => x.as_ref(),
            Self::User(x) => x.text.as_ref(),
            Self::Assistant(x) => x.text.as_ref(),
            Self::Tool => "",
        }
    }
}

pub mod client;
pub mod request;
pub mod response;

pub use self::client::QwenClient;
pub use self::request::QwenRequest;
pub use self::response::QwenResponse;
