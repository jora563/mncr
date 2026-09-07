use serde::{Deserialize, Serialize};

/// Модель одного сообщения в истории диалога.
///
/// Используется для передачи контекста предыдущих сообщений в запросе /chat.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HistoryMessage {
    /// Роль отправителя: "user" (пользователь) или "assistant" (ассистент).
    pub role: String,
    /// Текст сообщения.
    pub content: String,
}

/// Детальная информация об ошибке.
#[derive(Clone, Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    /// Код ошибки (например, "PROJECT_NOT_FOUND", "INVALID_REQUEST").
    pub code: String,
    /// Человекочитаемое описание ошибки.
    pub message: String,
}

pub mod replies;
pub mod requests;

pub use replies::*;
pub use requests::*;
