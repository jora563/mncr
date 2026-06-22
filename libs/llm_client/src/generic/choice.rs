use super::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FinishReason {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "length")]
    Length,
    #[serde(rename = "tool_calls")]
    ToolCalls,
    #[serde(rename = "content_filter")]
    ContentFilter,
}

/// This is a structure that is used by Anthropic compatible models
/// to convey responses.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Choice<T> {
    index: usize,
    pub(crate) message: Message<T>,
    finish_reason: FinishReason,
}
