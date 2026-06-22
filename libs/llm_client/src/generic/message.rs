//! This codes for generic messages of the type
//! ```ignore
//! {
//!   "content": "string",
//!   "role": "string",
//! }
//! ```
//! which are found in anthropic-like APIs. Thus this module exists for DRY.
use crate::llm::LlmMessage;

use serde::{Deserialize, Serialize};

/// This structure stands for a generic message.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message<T> {
    /// The content (prompt or answer) of a message.
    pub content: String,
    /// The role in which the message is being sent. Different models may have different roles,
    /// so this is made generic.
    pub role: T,
}

/// This is an enum that defines general roles (other models may have specific roles).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum GeneralRole {
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "user")]
    User,
}

impl LlmMessage for Message<GeneralRole> {
    fn new_assistant<S: AsRef<str>>(description: S) -> Self {
        let role = GeneralRole::Assistant;
        let content = description.as_ref().to_string();
        Self { role, content }
    }
    fn new_system<S: AsRef<str>>(description: S) -> Self {
        let role = GeneralRole::System;
        let content = description.as_ref().to_string();
        Self { role, content }
    }
    fn new_user<S: AsRef<str>>(description: S) -> Self {
        let role = GeneralRole::User;
        let content = description.as_ref().to_string();
        Self { role, content }
    }
    fn new_tool<S: AsRef<str>>(description: S) -> Self {
        let role = GeneralRole::Tool;
        let content = description.as_ref().to_string();
        Self { role, content }
    }
    fn content(&self) -> &str {
        &self.content
    }
}
