#![allow(dead_code)]

use serde::Serialize;

use super::*;
use crate::llm::LlmRequest;

#[derive(Clone, Debug, Default, Serialize)]
pub struct OpenAiRequest {
    messages: Vec<OpenAiMessage>,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    echo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<OpenAiFunctionCall>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    functions: Vec<OpenAiFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grammar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ignore_eos: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_base_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_keep: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_prompt_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<OpenAiPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OpenAiTool>,
}
/// Response format is always JSON.
impl LlmRequest for OpenAiRequest {
    type Model = String;
    type Tool = OpenAiTool;
    type Message = OpenAiMessage;
    type Response = OpenAiResult;

    fn new() -> Self {
        OpenAiRequest {
            model: "llama-3.2-sun-2.5b-chat".to_string(),
            messages: Vec::new(),
            tools: Vec::new(),
            temperature: Some(0.7),
            stream: Some(false),
            ..Default::default()
        }
    }

    fn set_stream_mode(mut self) -> Self {
        self.stream = Some(true);
        self
    }
    fn set_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max as u64);
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
    fn set_stop<S: AsRef<str>>(self, _stop_seq: S) -> Self {
        self
    }
    fn set_frequency_penalty(self, _penalty: f32) -> Self {
        self
    }
    fn set_presence_penalty(self, _penalty: f32) -> Self {
        self
    }
    fn set_use_logprobs(self, _uselp: bool) -> Self {
        self
    }
    fn set_top_logprobs(self, _lp: f32) -> Self {
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
        self.quality = Some("low".to_string());
        self
    }
    fn set_high(mut self) -> Self {
        self.quality = Some("high".to_string());
        self
    }
}
