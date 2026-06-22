//! This file contains a structure for storing system prompts which can be used
//! to store system prompts which are initially loaded.
use super::LlmMessage;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SystemPrompts {
    /// This contains the system prompts.
    pub prompts: Vec<String>,
}

impl SystemPrompts {
    /// Convert the loaded prompts into system messages.
    pub fn into_messages<T: LlmMessage>(self) -> impl Iterator<Item = T> {
        Box::new(self.prompts.into_iter().map(|x| T::new_system(x)))
    }
}
