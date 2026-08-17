//! Модуль мессенджеров.
//! Содержит трейты и реализации для работы с различными платформами.

pub mod telegram;
pub mod vk;

// Экспортируем публичные типы, чтобы они были доступны через `chat::messengers::...`
pub use telegram::{TelegramMessenger, TgCredentials, TgMirrors, TgSession};
pub use vk::{VkCredentials, VkMessenger};

use crate::error::Result;
use crate::models::{SendMessageRequest, UnifiedMessage};

/// Общий трейт для всех мессенджеров.
/// Определяет базовый контракт для получения и отправки сообщений.
pub trait Messenger {
    /// Тип учетных данных, специфичный для конкретной реализации мессенджера.
    type Credentials;

    /// Получить новые сообщения с заданного смещения (offset).
    /// Возвращает вектор унифицированных сообщений и новый offset для следующего запроса.
    fn fetch_messages(
        &self,
        offset: i64,
        cred: Self::Credentials,
    ) -> impl std::future::Future<Output = Result<(Vec<UnifiedMessage>, i64)>> + Send;

    /// Отправить сообщение в чат.
    fn send_message(
        &self,
        request: &SendMessageRequest,
        cred: Self::Credentials,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}
