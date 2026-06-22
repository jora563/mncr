//! This module contains the types for using the DeepSeek LLM.
use crate::generic::{GeneralRole, Message};
use crate::llm::{LlmModel, LlmTool};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeepSeekResponseType {
    #[serde(rename = "type")]
    tpe: &'static str,
}

impl DeepSeekResponseType {
    pub const JSON_RESPONSE: DeepSeekResponseType = DeepSeekResponseType { tpe: "json_object" };
}

/// This is an enum that defines which specific DeepSeek model to use.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum DeepSeekModel {
    #[default]
    #[serde(rename = "deepseek-v4-flash")]
    V4Flash,
    #[serde(rename = "deepseek-v4-pro")]
    V4Pro,
    #[serde(rename = "deepseek-chat")]
    Chat,
    #[serde(rename = "deepseek-reasoner")]
    Reasoner,
}

/// Defines a specific DeepSeek tool to use.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeepSeekTool {
    #[serde(rename = "type")]
    tpe: String,
    function: DeepSeekFunction,
}

/// This represents a deep seek tool
/// (TODO: What is the format of the function?)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeepSeekFunction {
    name: String,
    description: String,
    parameters: Vec<String>,
}

impl DeepSeekTool {
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
            function: DeepSeekFunction {
                name: name.as_ref().to_string(),
                description: desc.as_ref().to_string(),
                parameters,
            },
        }
    }
}

/// This represents a deep seek message
/// (TODO: What is the format of the function?)
pub type DeepSeekMessage = Message<GeneralRole>;

impl LlmModel for DeepSeekModel {}
impl LlmTool for DeepSeekTool {}

pub mod client;
pub mod request;
pub mod response;

pub use self::client::DeepSeekClient;
pub use self::request::DeepSeekRequest;
pub use self::response::DeepSeekResponse;
