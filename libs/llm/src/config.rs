//! Модуль конфигурации ЛЛМ клиент Aiomni.
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::Result;
use crate::methods::AiomniLlmClient;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AiomniLlmConfig {
    host: String,
    chat_path: String,
}

impl AiomniLlmConfig {
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Self> {
        let file = std::fs::read_to_string(p)?;
        toml::from_str(&file).map_err(Into::into)
    }

    pub fn get_client(&self) -> Result<AiomniLlmClient> {
        AiomniLlmClient::new()?.set_base_uri(&self.host)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn chat_path(&self) -> &str {
        &self.chat_path
    }
}
