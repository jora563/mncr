//! Тут типы ошибки и результата.
use crate::models::Platform;

use serde::Deserialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ChatError>;

#[derive(Debug, Deserialize)]
pub struct VkError {
    error_code: i32,
    error_msg: String,
}

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Платформа отсутствует: {0:?}")]
    Platform(Platform),
    #[error("Telegram вернул ошибку: {0}")]
    TgResponse(String),
    #[error("Ошибка парсинга JSON ответа сервера ТГ: {0}")]
    TgResponseParse(serde_json::error::Error),
    #[error("Не удалось удалить вебхук: {0}")]
    TgWebhook(String),
    #[error("Неожиданный ответ VK: {0}")]
    VkUnexpected(String),
    #[error("VK API ошибка {}: {}",.0.error_code, .0.error_msg)]
    VkResponse(VkError),
    #[error("Неверный формат chat_id для VK")]
    VkChatId,
    #[error("Ключ VK Long Poll недействителен")]
    VkLongPollKey,
    #[error("Ошибка парсинга ответа VK Long Poll: {0}")]
    VkLongPoll(serde_json::error::Error),
    #[error("Ошибка парсинга сервера VK: {0}")]
    VkResponseParse(#[from] serde_json::error::Error),
    #[error("Vk token format incorrect: {0}")]
    VkToken(String),
    #[error("Reqwest client error: {0}")]
    ReqWest(#[from] reqwest::Error),
    #[error("An error occurred: {0}")]
    Other(String),
}

impl From<String> for ChatError {
    fn from(e: String) -> Self {
        Self::Other(e)
    }
}

impl ChatError {
    pub(crate) fn vk_token(t: &str) -> Self {
        Self::VkToken(t.to_string())
    }
}
