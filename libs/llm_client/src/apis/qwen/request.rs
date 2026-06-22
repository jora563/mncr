#![allow(clippy::new_without_default)]

use serde::Serialize;

use super::{QwenMessage, QwenModel, QwenResponse, QwenResponseType, QwenTool};
use crate::llm::LlmRequest;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QwenReasoningEffort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

/// Represents the input to a Qwen model. De-facto this is a list
/// of input messages
#[derive(Clone, Debug, Serialize)]
pub struct QwenInput {
    pub messages: Vec<QwenMessage>,
}

impl QwenInput {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }
}

/// This represents certain parameters that qwen uses when generating
/// answers.
#[derive(Clone, Debug, Serialize)]
pub struct QwenParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<f32>,
    enable_thinking: Option<bool>,
    /// Append `reasoning_content` messages to end of output.
    preserve_thinking: Option<bool>,
    /// Maximum length of chain of thought process.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<u32>,
    reasoning_effort: QwenReasoningEffort,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_code_interpreter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repetition_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    /// Used in image generation: Increase max pixel limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    vl_high_resolution_images: Option<bool>,
    /// Used in image generation: Returns the image dimensions if generating.
    #[serde(skip_serializing_if = "Option::is_none")]
    vl_enable_image_hw_output: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    /// Determines the random seed used for generating the response.
    /// If seed is fixed, the same set of parameters should generate the same
    /// answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    /// Controls how response chunks are generated.
    /// Should be true for streaming mode, should be false otherwise.
    incremental_output: Option<bool>,
    response_format: QwenResponseType,
    result_format: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_logprobs: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<QwenTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

impl QwenParameters {
    const RESULT_FORMAT: &str = "message";
    pub fn new() -> Self {
        Self {
            temperature: Some(1.),
            top_p: None,
            top_k: None,
            enable_thinking: Some(true),
            preserve_thinking: Some(false),
            thinking_budget: None,
            reasoning_effort: QwenReasoningEffort::High,
            tool_stream: Some(false),
            enable_code_interpreter: None,
            repetition_penalty: None,
            presence_penalty: None,
            vl_high_resolution_images: None,
            vl_enable_image_hw_output: None,
            max_tokens: Some(100_000),
            seed: None,
            stream: Some(false),
            incremental_output: Some(false),
            response_format: QwenResponseType::JSON_RESPONSE,
            result_format: Self::RESULT_FORMAT,
            top_logprobs: None,
            stop: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
        }
    }
}

/// Represents a Qwen Request
#[derive(Clone, Debug, Serialize)]
pub struct QwenRequest {
    model: QwenModel,
    input: QwenInput,
    parameters: QwenParameters,
}

/// Response format is always JSON.
impl LlmRequest for QwenRequest {
    type Model = QwenModel;
    type Tool = QwenTool;
    type Message = QwenMessage;
    type Response = QwenResponse;

    fn new() -> Self {
        Self {
            model: QwenModel::Qwen25Omni7B,
            input: QwenInput::new(),
            parameters: QwenParameters::new(),
        }
    }

    fn set_stream_mode(mut self) -> Self {
        self.parameters.stream = Some(true);
        self.parameters.tool_stream = Some(true);
        self
    }
    fn set_max_tokens(mut self, max: u32) -> Self {
        self.parameters.max_tokens = Some(max);
        self
    }
    fn set_temperature(mut self, t: f32) -> Self {
        self.parameters.temperature = Some(t);
        self
    }
    fn set_top_p(mut self, top: f32) -> Self {
        self.parameters.top_p = Some(top);
        self
    }
    fn set_stop<S: AsRef<str>>(mut self, stop_seq: S) -> Self {
        self.parameters.stop.push(stop_seq.as_ref().to_string());
        self
    }
    fn set_frequency_penalty(self, _: f32) -> Self {
        self
    }
    fn set_presence_penalty(mut self, penalty: f32) -> Self {
        self.parameters.presence_penalty = Some(penalty);
        self
    }
    fn set_use_logprobs(self, _: bool) -> Self {
        self
    }
    fn set_top_logprobs(mut self, lp: f32) -> Self {
        self.parameters.top_logprobs = Some(lp);
        self
    }

    /// The model can be changed after each request.
    fn set_model<M: crate::generic::IntoLlmModel>(mut self, m: M) -> Self {
        self.model = m.to_model();
        self
    }
    /// Tools can be added or removed.
    fn set_tools<I: IntoIterator<Item = Self::Tool>>(&mut self, tools: I) {
        self.parameters.tools = tools.into_iter().collect::<Vec<_>>();
    }
    /// The messages can be added or removed.
    fn add_message(&mut self, msg: Self::Message) {
        self.input.messages.push(msg);
    }
    /// Remove and return all tools.
    fn take_tools(&mut self) -> Vec<Self::Tool> {
        std::mem::take(&mut self.parameters.tools)
    }
    /// Removes and returns all messages.
    fn take_messages(&mut self) -> Vec<Self::Message> {
        std::mem::take(&mut self.input.messages)
    }
    fn set_low(mut self) -> Self {
        self.parameters.enable_thinking = Some(false);
        self.parameters.reasoning_effort = QwenReasoningEffort::Low;
        self
    }
    fn set_high(mut self) -> Self {
        self.parameters.enable_thinking = Some(true);
        self.parameters.reasoning_effort = QwenReasoningEffort::High;
        self
    }
}
