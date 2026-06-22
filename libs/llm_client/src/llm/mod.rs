//! The [`Llm`] trait which provides requests and replies for an LLM.
use crate::Result;

use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};

/// The trait is used to post an organised request.
pub trait Llm: Sized {
    /// The service which typically represents the LLM
    const DEFAULT_SERVICE: &str;

    /// A new instance of an Llm Client
    fn new() -> Result<Self>;

    /// Add the base url it uses.
    fn set_base_uri<U: IntoUrl>(self, path: U) -> Result<Self>;

    /// Add an authentication/autherisation token.
    fn set_auth<S: AsRef<str>>(self, token: S) -> Self;
}

pub trait CallLlmService: LlmRequest {
    type Client: Llm;
    /// The posting path for chat completions.
    const DEFAULT_PATH: &str;

    /// Post a request. NB: The request is not consumed, which allows it to be appended
    /// with additional messages afterwards.
    async fn post(
        &self,
        client: &Self::Client,
        path: &str,
    ) -> Result<<Self as LlmRequest>::Response>;
}

/// Response format is always JSON.
pub trait LlmRequest: Serialize + std::fmt::Debug {
    type Model: LlmModel;
    type Tool: LlmTool;
    type Message: LlmMessage;
    type Response: LlmResponse;

    fn new() -> Self;
    fn set_stream_mode(self) -> Self;
    fn set_max_tokens(self, max: u32) -> Self;
    fn set_temperature(self, t: f32) -> Self;
    fn set_top_p(self, top: f32) -> Self;
    fn set_stop<S: AsRef<str>>(self, stop_seq: S) -> Self;
    fn set_frequency_penalty(self, penalty: f32) -> Self;
    fn set_presence_penalty(self, penalty: f32) -> Self;
    fn set_use_logprobs(self, uselp: bool) -> Self;
    fn set_top_logprobs(self, lp: f32) -> Self;

    /// The model can be changed after each request.
    fn set_model<M: crate::generic::IntoLlmModel>(self, m: M) -> Self;
    /// Tools can be added or removed.
    fn set_tools<I: IntoIterator<Item = Self::Tool>>(&mut self, tools: I);
    /// The messages can be added or removed.
    fn add_message(&mut self, msg: Self::Message);
    /// Remove and return all tools.
    fn take_tools(&mut self) -> Vec<Self::Tool>;
    /// Removes and returns all messages.
    fn take_messages(&mut self) -> Vec<Self::Message>;

    /// Sets reasoning to lowest possible settings. (Done this way since different models
    /// might have radically different internals here)
    fn set_low(self) -> Self;
    /// Sets reasoning to highest possible settings. (Done this way since different models
    /// might have radically different internals here)
    fn set_high(self) -> Self;
}

pub trait LlmResponse: for<'a> Deserialize<'a> + std::fmt::Debug {
    type Message: LlmMessage;
    /// Take all response messages.
    fn take_messages(self) -> Vec<Self::Message>;
}

/// A definition of models used by an LLM. For different LLMs, a different
/// set of models are used by different LLMs.
pub trait LlmModel:
    Serialize + std::default::Default + for<'a> Deserialize<'a> + std::fmt::Debug + Clone
{
}

/// Different LLM interfaces have different tool formats which might not be compatible.
pub trait LlmTool: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug + Clone {}

/// Different LLMs may use different message formats, thus it is a trait.
pub trait LlmMessage: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug {
    fn new_assistant<S: AsRef<str>>(description: S) -> Self;
    fn new_system<S: AsRef<str>>(description: S) -> Self;
    fn new_user<S: AsRef<str>>(description: S) -> Self;
    fn new_tool<S: AsRef<str>>(description: S) -> Self;
    fn content(&self) -> &str;
    fn to_assist<T: LlmMessage>(&self) -> T {
        T::new_assistant(self.content())
    }
}

pub mod system_prompts;
pub use system_prompts::*;
