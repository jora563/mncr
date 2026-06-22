#![allow(dead_code)]

use serde::Deserialize;

use super::OpenAiMessage;
use crate::llm::{LlmMessage, LlmResponse};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct OpenAiDataItem {
    b64_json: String,
    embedding: Vec<f32>,
    index: u64,
    object: String,
    url: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct OpenAiUsage {
    pub completion_tokens: u64,
    pub prompt_tokens: u64,
    pub timing_prompt_processing: Option<f32>,
    pub timing_token_generation: Option<f32>,
    pub total_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct OpenAiChoice {
    pub delta: Option<OpenAiMessage>,
    pub finish_reason: String,
    pub index: u64,
    pub message: OpenAiMessage,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct OpenAiError {
    code: u16,
    message: String,
    #[serde(rename = "type")]
    tpe: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct OpenAiResult {
    pub error: Option<OpenAiError>,
    pub choices: Vec<OpenAiChoice>,
    pub created: i64,
    pub data: Vec<OpenAiDataItem>,
    pub id: String,
    pub model: String,
    pub object: String,
    pub usage: OpenAiUsage,
}

impl LlmResponse for OpenAiResult {
    type Message = OpenAiMessage;
    /// Take all response messages.
    /// NB: We only get text for now. Everything else is ignored for now.
    /// TODO: Fill in everything else.
    fn take_messages(self) -> Vec<Self::Message> {
        match self.error {
            Some(e) => vec![Self::Message::new_assistant(e.message)],
            None => self.choices.into_iter().map(|x| x.message).collect(),
        }
    }
}
