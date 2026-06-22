#![allow(dead_code)]

use serde::Deserialize;

use super::QwenMessage;
use crate::llm::{LlmMessage, LlmResponse};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct QwenInputTokenDetails {
    text_tokens: u32,
    image_tokens: u32,
    video_tokens: u32,
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct QwenOutputTokenDetails {
    text_tokens: u32,
    reasoning_tokens: u32,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "cache_type", content = "cache_creation_input_tokens")]
pub enum QwenCacheCreation {
    #[serde(rename = "lowercase")]
    Ephemeral(u32),
}
impl Default for QwenCacheCreation {
    fn default() -> Self {
        Self::Ephemeral(0)
    }
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct QwenPromptTokenDetails {
    cached_tokens: u32,
    cache_creation: QwenCacheCreation,
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct QwenUsage {
    input_tokens: u32,
    output_tokens: u32,
    prompt_tokens: u32,
    total_tokens: u32,
    image_tokens: u32,
    video_tokens: u32,
    audio_tokens: u32,
    input_token_details: QwenInputTokenDetails,
    output_token_details: QwenOutputTokenDetails,
    prompt_token_details: QwenPromptTokenDetails,
}

/// Represents a Qwen choice object.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct QwenChoice {
    finish_reason: String,
    message: QwenMessage,
    // Logprobs also exist, but for now we ignore them
    // logprobs: Vec<QwenLogProb>,
}

/// Reperesents a successful Qwen Response. It is not created by the client, but
/// rather received and deserialized from the Qwen server.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct QwenOutput {
    text: String,
    finish_reason: String,
    choices: Vec<QwenChoice>,
}
/// Reperesents a Qwen Response. This includes possible error responses.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct QwenResponse {
    status_code: u16,
    request_id: String,
    code: String,
    message: String,
    usage: QwenUsage,
    output: Option<QwenOutput>,
}

impl LlmResponse for QwenResponse {
    type Message = QwenMessage;
    /// Take all response messages.
    fn take_messages(self) -> Vec<Self::Message> {
        if let Some(output) = self.output {
            output
                .choices
                .into_iter()
                .map(|x| x.message)
                .collect::<Vec<_>>()
        } else {
            vec![Self::Message::new_assistant(self.message)]
        }
    }
}
