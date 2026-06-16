use crate::models::{UnifiedMessage, SendMessageRequest};

pub mod telegram;
pub mod vk;

#[allow(async_fn_in_trait)]
pub trait Messenger {
    /// Возвращает список сообщений и новый offset
    async fn fetch_messages(&self, offset: i64) -> Result<(Vec<UnifiedMessage>, i64), String>;
    
    /// Отправляет сообщение в чат
    async fn send_message(&self, request: &SendMessageRequest) -> Result<(), String>;
}
