#![allow(dead_code)]

use serde::Deserialize;

use super::OllamaMessage;
use crate::llm::{LlmMessage, LlmResponse};

/// This is an element of the response.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct OllamaLogProbs {
    token: String,
    logprob: u128,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OllamaTopLogProbs {
    token: String,
    logprob: u128,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct OllamaResult {
    pub error: Option<String>,
    pub model: String,
    pub created_at: String,
    pub message: OllamaMessage,
    pub done: bool,
    pub done_reason: Option<String>,
    pub total_duration: Option<u128>,
    pub load_duration: Option<u128>,
    pub prompt_eval_count: Option<u128>,
    pub prompt_eval_duration: Option<u128>,
    pub eval_count: Option<u128>,
    pub eval_duration: Option<u128>,
    pub logprobs: Vec<OllamaLogProbs>,
}

impl LlmResponse for OllamaResult {
    type Message = OllamaMessage;
    /// Take all response messages.
    /// NB: We only get text for now. Everything else is ignored for now.
    /// TODO: Fill in everything else.
    fn take_messages(self) -> Vec<Self::Message> {
        match self.error {
            Some(e) => vec![Self::Message::new_assistant(e)],
            None => vec![self.message],
        }
    }
}
