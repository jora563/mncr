#![allow(async_fn_in_trait)]
//! This is a library with an API for posting requests to LLMs and returning an answer which
//! can then be moved on to other functional parts of a system.
//!
//! The library has the following components:
//! 1. A trait [`Llm`] which describes requests and responses from an LLm.
//! 2. A type [`LlmClient`] which takes [`Llm::Request`] and returns [`Llm::Response`].
//! 3. Implementation of [`Llm`] for a number of different LLMs.
pub mod apis;
pub mod config;
pub mod generic;
pub mod llm;

pub use apis::claude;
pub use apis::deep_seek;
pub use apis::ollama;
pub use apis::openai;
pub use apis::qwen;

pub type Result<T> = std::result::Result<T, LlmError>;

/// TODO: Do a proper error
#[derive(Debug)]
pub struct LlmError(pub std::io::Error);

impl LlmError {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(std::io::Error::other(s.as_ref()))
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: std::error::Error> From<E> for LlmError {
    fn from(e: E) -> Self {
        Self(std::io::Error::other(e.to_string()))
    }
}
