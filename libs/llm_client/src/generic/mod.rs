//! This module contains generic components which look the same in different models.

pub mod choice;
pub mod message;

pub use choice::Choice;
pub use message::{GeneralRole, Message};

use crate::llm::LlmModel;

impl LlmModel for String {}

pub trait IntoLlmModel {
    fn to_model<T: LlmModel>(&self) -> T;
}

impl IntoLlmModel for String {
    fn to_model<T: LlmModel>(&self) -> T {
        serde_json::from_str(self).unwrap_or_default()
    }
}
