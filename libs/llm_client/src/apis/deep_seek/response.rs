#![allow(dead_code)]

use serde::Deserialize;

use super::{DeepSeekMessage, DeepSeekModel};
use crate::generic::{Choice, GeneralRole};
use crate::llm::{LlmMessage, LlmResponse};

#[derive(Clone, Debug, Default, Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    prompt_cache_hit_tokens: u32,
    prompt_cache_miss_tokens: u32,
}

/// Represents a DeepSeek choice object.
pub type DeepSeekChoice = Choice<GeneralRole>;

/// Represents a DeepSeek output. It is not created by the client, but
/// rather received and deserialized from the DeepSeek server.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct DeepSeekOutput {
    id: String,
    object: String,
    created: u64,
    model: DeepSeekModel,
    usage: DeepSeekUsage,
    choices: Vec<DeepSeekChoice>,
}

/// Reperesents a Deep Seek error response
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct DeepSeekError {
    message: String,
    #[serde(rename = "type")]
    tpe: String,
    code: String,
}

/// Reperesents a Deep Seek Response. This includes possible error responses.
/// NB: While the error response seems to be marked in the error field,
///     the result response has no tag.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeepSeekResponse {
    Error(DeepSeekError),
    #[serde(untagged)]
    Result(DeepSeekOutput),
}

impl LlmResponse for DeepSeekResponse {
    type Message = DeepSeekMessage;
    /// Take all response messages.
    fn take_messages(self) -> Vec<Self::Message> {
        match self {
            Self::Result(r) => r.choices.into_iter().map(|x| x.message).collect::<Vec<_>>(),
            Self::Error(e) => vec![Self::Message::new_assistant(e.message)],
        }
    }
}
