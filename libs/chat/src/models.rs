use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Platform {
    Telegram,
    VK,
    Max,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Attachment {
    Contact {
        phone: String,
        first_name: String,
        last_name: Option<String>,
    },
    Photo {
        file_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_size: Option<i64>,
    },
    Document {
        file_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_size: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_name: Option<String>,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct UnifiedMessage {
    pub platform: Platform,
    pub user_id: String,
    pub chat_id: String,
    pub text: String,
    pub timestamp: u64,
    pub message_id: Option<String>,
    pub attachments: Vec<Attachment>,
}

impl UnifiedMessage {
    pub fn print(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Ошибка сериализации: {}", e),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReplyMarkup {
    Telegram(TelegramKeyboard),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelegramKeyboard {
    Show {
        keyboard: Vec<Vec<TelegramKeyboardButton>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        resize_keyboard: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        one_time_keyboard: Option<bool>,
    },
    Remove {
        remove_keyboard: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_contact: Option<bool>,
}

impl TelegramKeyboard {
    pub fn request_contact() -> Self {
        Self::Show {
            keyboard: vec![vec![TelegramKeyboardButton {
                text: "📱 Поделиться номером телефона".to_string(),
                request_contact: Some(true),
            }]],
            resize_keyboard: Some(true),
            one_time_keyboard: Some(true),
        }
    }

    pub fn remove() -> Self {
        Self::Remove {
            remove_keyboard: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendMessageRequest {
    pub chat_id: String,
    pub text: String,
    pub reply_to_message_id: Option<String>,
    pub reply_markup: Option<ReplyMarkup>,
}
