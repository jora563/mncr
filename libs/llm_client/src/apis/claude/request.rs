#![allow(dead_code)]

use serde::Serialize;

use super::{ClaudeMessage, ClaudeModel, ClaudeResponse, ClaudeTool};
use crate::llm::LlmRequest;

const EPHEMERAL: &str = "ephemeral";

#[derive(Clone, Debug, Serialize)]
pub struct ClaudeCacheControl {
    #[serde(rename = "type")]
    tpe: &'static str,
    ttl: &'static str,
}

impl ClaudeCacheControl {
    fn five_m() -> Self {
        let (tpe, ttl) = (EPHEMERAL, "5m");
        Self { tpe, ttl }
    }
    fn one_h() -> Self {
        let (tpe, ttl) = (EPHEMERAL, "1h");
        Self { tpe, ttl }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeEffort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

/// Should have `effort` and `format` fields.
/// The `format` field is skipped (TODO: Understand `format` field better).
#[derive(Clone, Debug, Serialize)]
struct ClaudeOutputConfig {
    effort: ClaudeEffort,
}

impl ClaudeOutputConfig {
    fn high() -> Self {
        Self {
            effort: ClaudeEffort::Max,
        }
    }
    fn low() -> Self {
        Self {
            effort: ClaudeEffort::Low,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum ClaudeDisplayKind {
    Summarized,
    Omitted,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClaudeThinkingParam {
    Disabled,
    Enabled {
        budget_tokens: u32,
        display: ClaudeDisplayKind,
    },
    Adaptive {
        display: ClaudeDisplayKind,
    },
}

impl ClaudeThinkingParam {
    fn adaptive() -> Self {
        Self::Adaptive {
            display: ClaudeDisplayKind::Omitted,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClaudeToolChoice {
    Auto {
        disable_parallel_tool_use: bool,
    },
    Any {
        disable_parallel_tool_use: bool,
    },
    Tool {
        name: String,
        disable_parallel_tool_use: bool,
    },
    None,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClaudeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    messages: Vec<ClaudeMessage>,
    model: ClaudeModel,
    cache_control: Option<ClaudeCacheControl>,
    /// The identifier of a container (instance?) of a model to use, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    container: Option<String>,
    /// Geographical region for inference (possibly irrelevant to us).
    #[serde(skip_serializing_if = "Option::is_none")]
    inference_geo: Option<String>,
    /// This determines how to make the output (format and effort)
    #[serde(skip_serializing_if = "Option::is_none")]
    output_config: Option<ClaudeOutputConfig>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    /// System prompt, if any.
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    thinking: ClaudeThinkingParam,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ClaudeTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ClaudeToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

/// Response format is always JSON.
impl LlmRequest for ClaudeRequest {
    type Model = ClaudeModel;
    type Tool = ClaudeTool;
    type Message = ClaudeMessage;
    type Response = ClaudeResponse;

    fn new() -> Self {
        ClaudeRequest {
            max_tokens: None,
            messages: vec![],
            model: ClaudeModel::Opus47,
            cache_control: None,
            container: None,
            inference_geo: None,
            output_config: Some(ClaudeOutputConfig::high()),
            stop_sequences: vec![],
            stream: Some(false),
            system: None,
            temperature: None,
            thinking: ClaudeThinkingParam::adaptive(),
            tools: vec![],
            tool_choice: None,
            top_k: None,
            top_p: None,
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
        self.stop_sequences.push(stop_seq.as_ref().to_string());
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
        self.output_config = Some(ClaudeOutputConfig::low());
        self
    }
    fn set_high(mut self) -> Self {
        self.output_config = Some(ClaudeOutputConfig::high());
        self
    }
}
