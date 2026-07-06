//! Конфигурация Модели.
use crate::Result;
use crate::llm::{LlmMessage, LlmRequest};

use serde::{Deserialize, Serialize};
use std::path::Path;

macro_rules! set_fields {
    ($r:expr, $cfg:expr, {$field0:ident,$setter0:ident} $(,{$field:ident,$setter:ident})*$(,)?) => {
        if let Some(ref x) = $cfg.$field0 {
            $r = $r.$setter0(x.to_owned());
        }
        $(
            if let Some(ref x) = $cfg.$field {
                $r = $r.$setter(x.to_owned());
            }
        )*
    }
}

/// Конфигурация которая используется для "настройки" модели.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct LlmRequestCfg {
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop_seq: Option<String>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    use_logprobs: Option<bool>,
    top_logprobs: Option<f32>,
    model: Option<String>,
    prompts: Vec<String>,
}

impl LlmRequestCfg {
    #[tracing::instrument]
    pub fn from_file<P: AsRef<Path> + std::fmt::Debug>(p: P) -> Result<Self> {
        let data = std::fs::read_to_string(p.as_ref())?;
        toml::from_str(&data).map_err(Into::into)
    }

    /// Настроить запрос
    pub fn configure<T: LlmRequest>(&self, mut req: T) -> T {
        set_fields!(
            req,
            self,
            {max_tokens, set_max_tokens},
            {temperature, set_temperature},
            {top_p, set_top_p},
            {stop_seq, set_stop},
            {frequency_penalty, set_frequency_penalty},
            {presence_penalty, set_presence_penalty},
            {use_logprobs, set_use_logprobs},
            {top_logprobs, set_top_logprobs},
            {model, set_model},
        );
        for p in self.prompts.iter() {
            req.add_message(T::Message::new_system(p));
        }
        req
    }
}

/// Конфигурация которая используется для настройки клиента.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct LlmClientCfg {
    pub host: String,
    pub chat_path: String,
}

impl LlmClientCfg {
    #[tracing::instrument]
    pub fn from_file<P: AsRef<Path> + std::fmt::Debug>(p: P) -> Result<Self> {
        let data = std::fs::read_to_string(p.as_ref())?;
        toml::from_str(&data).map_err(Into::into)
    }

    pub fn get_base_url(&self) -> &str {
        &self.host
    }

    pub fn get_chat_path(&self) -> &str {
        &self.chat_path
    }
}
