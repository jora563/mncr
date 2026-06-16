use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Telegram,
    VK,
}

#[derive(Debug, Serialize, Clone)]
pub struct UnifiedMessage {
    pub platform: Platform,
    pub user_id: String,
    pub chat_id: String,
    pub text: String,
    pub timestamp: u64,
    pub message_id: Option<String>,
}

impl UnifiedMessage {
    pub fn print(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Ошибка сериализации: {}", e),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendMessageRequest {
    pub chat_id: String,
    pub text: String,
    pub reply_to_message_id: Option<String>,
}
