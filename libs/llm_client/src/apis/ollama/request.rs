#![allow(dead_code)]

use serde::Serialize;

use super::{OllamaMessage, OllamaResponseFormat, OllamaResult, OllamaTool};
use crate::llm::LlmRequest;

#[derive(Clone, Debug, Serialize)]
pub struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OllamaTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<OllamaResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    think: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_logprobs: Option<u32>,
}

impl OllamaRequest {
    fn options_or_initiate(&mut self) -> &mut OllamaOptions {
        if self.options.is_none() {
            self.options = Some(OllamaOptions::default());
        }
        self.options.as_mut().expect("Is some")
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_p: Option<f32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    stop: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

/// Response format is always JSON.
impl LlmRequest for OllamaRequest {
    type Model = String;
    type Tool = OllamaTool;
    type Message = OllamaMessage;
    type Response = OllamaResult;

    fn new() -> Self {
        OllamaRequest {
            model: "qwen2.5".to_string(),
            messages: Vec::new(),
            tools: Vec::new(),
            options: Some(OllamaOptions {
                num_ctx: Some(6_000),
                temperature: Some(0.7),
                ..Default::default()
            }),
            format: None,
            stream: Some(false),
            think: Some(false),
            keep_alive: None,
            logprobs: Some(false),
            top_logprobs: None,
        }
    }

    fn set_stream_mode(mut self) -> Self {
        self.stream = Some(true);
        self
    }
    fn set_max_tokens(self, _max: u32) -> Self {
        self
    }
    fn set_temperature(mut self, t: f32) -> Self {
        self.options_or_initiate().temperature = Some(t);
        self
    }
    fn set_top_p(mut self, top: f32) -> Self {
        self.options_or_initiate().top_p = Some(top);
        self
    }
    fn set_stop<S: AsRef<str>>(mut self, stop_seq: S) -> Self {
        self.options_or_initiate().stop = stop_seq.as_ref().to_string();
        self
    }
    fn set_frequency_penalty(self, _penalty: f32) -> Self {
        self
    }
    fn set_presence_penalty(self, _penalty: f32) -> Self {
        self
    }
    fn set_use_logprobs(mut self, uselp: bool) -> Self {
        self.logprobs = Some(uselp);
        self
    }
    fn set_top_logprobs(mut self, lp: f32) -> Self {
        self.top_logprobs = Some(lp as u32);
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
        self.think = Some(false);
        self
    }
    fn set_high(mut self) -> Self {
        self.think = Some(true);
        self
    }
}
