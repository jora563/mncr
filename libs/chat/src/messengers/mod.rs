use crate::error::Result;
use crate::models::{SendMessageRequest, UnifiedMessage};

pub mod telegram;
pub mod vk;

pub use telegram::{TelegramMessenger, TgCredentials};
pub use vk::{VkCredentials, VkMessenger};

#[allow(async_fn_in_trait)]
pub trait Messenger {
    type Credentials;

    /// Возвращает список сообщений и новый offset
    async fn fetch_messages(
        &self,
        offset: i64,
        cred: Self::Credentials,
    ) -> Result<(Vec<UnifiedMessage>, i64)>;

    /// Отправляет сообщение в чат
    async fn send_message(
        &self,
        request: &SendMessageRequest,
        cred: Self::Credentials,
    ) -> Result<()>;
}
