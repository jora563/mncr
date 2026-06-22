use serde::Serialize;

use super::{DeepSeekMessage, DeepSeekModel, DeepSeekResponse, DeepSeekResponseType, DeepSeekTool};
use crate::llm::LlmRequest;

#[derive(Clone, Debug, Serialize)]
pub enum DeepSeekReasoningEffort {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "high")]
    High,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeepSeekThinkingParams {
    thinking: DeepSeekThinking,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeepSeekThinking {
    #[serde(rename = "type")]
    tpe: DeepSeekThinkingType,
}
#[derive(Clone, Debug, Serialize)]
pub enum DeepSeekThinkingType {
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
}
impl DeepSeekThinkingParams {
    fn enabled() -> Self {
        let tpe = DeepSeekThinkingType::Enabled;
        let thinking = DeepSeekThinking { tpe };
        Self { thinking }
    }
    fn disabled() -> Self {
        let tpe = DeepSeekThinkingType::Disabled;
        let thinking = DeepSeekThinking { tpe };
        Self { thinking }
    }
}

/// Reperesents a DeepSeek Request
#[derive(Clone, Debug, Serialize)]
pub struct DeepSeekRequest {
    model: DeepSeekModel,
    messages: Vec<DeepSeekMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    response_format: DeepSeekResponseType,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<DeepSeekTool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_logprobs: Option<f32>,
    reasoning_effort: DeepSeekReasoningEffort,
    extra_body: DeepSeekThinkingParams,
}

/// Response format is always JSON.
impl LlmRequest for DeepSeekRequest {
    type Model = DeepSeekModel;
    type Tool = DeepSeekTool;
    type Message = DeepSeekMessage;
    type Response = DeepSeekResponse;

    fn new() -> Self {
        Self {
            model: DeepSeekModel::V4Flash,
            messages: Vec::new(),
            stream: None,
            max_tokens: Some(100_000),
            temperature: Some(1.),
            top_p: None,
            response_format: DeepSeekResponseType::JSON_RESPONSE,
            tools: Vec::new(),
            stop: Vec::new(),
            frequency_penalty: None,
            presence_penalty: None,
            logprobs: None,
            top_logprobs: None,
            reasoning_effort: DeepSeekReasoningEffort::High,
            extra_body: DeepSeekThinkingParams::enabled(),
        }
    }

    fn set_stream_mode(mut self) -> Self {
        self.stream = Some(true);
        self
    }
    fn set_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }
    fn set_temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }
    fn set_top_p(mut self, top: f32) -> Self {
        self.top_p = Some(top);
        self
    }
    fn set_stop<S: AsRef<str>>(mut self, stop_seq: S) -> Self {
        self.stop.push(stop_seq.as_ref().to_string());
        self
    }
    fn set_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }
    fn set_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }
    fn set_use_logprobs(mut self, uselp: bool) -> Self {
        self.logprobs = Some(uselp);
        self
    }
    fn set_top_logprobs(mut self, lp: f32) -> Self {
        self.top_logprobs = Some(lp);
        self
    }

    /// The model can be changed after each request.
    fn set_model<M: crate::generic::IntoLlmModel>(mut self, m: M) -> Self {
        self.model = m.to_model();
        self
    }
    /// Tools can be added or removed.
    fn set_tools<I: IntoIterator<Item = Self::Tool>>(&mut self, tools: I) {
        self.tools = tools.into_iter().collect::<Vec<_>>();
    }
    /// The messages can be added or removed.
    fn add_message(&mut self, msg: Self::Message) {
        self.messages.push(msg);
    }
    /// Remove and return all tools.
    fn take_tools(&mut self) -> Vec<Self::Tool> {
        std::mem::take(&mut self.tools)
    }
    /// Removes and returns all messages.
    fn take_messages(&mut self) -> Vec<Self::Message> {
        std::mem::take(&mut self.messages)
    }
    fn set_low(mut self) -> Self {
        self.extra_body = DeepSeekThinkingParams::disabled();
        self.reasoning_effort = DeepSeekReasoningEffort::None;
        self
    }
    fn set_high(mut self) -> Self {
        self.extra_body = DeepSeekThinkingParams::enabled();
        self.reasoning_effort = DeepSeekReasoningEffort::High;
        self
    }
}
