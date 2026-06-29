use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Platform {
    Telegram,
    VK,
    Max,
}

/// Вложение к сообщению. Поддерживаются только три типа: контакт, фото, документ.
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
        file_url: Option<String>,
        file_size: Option<i64>,
    },
    Document {
        file_id: String,
        file_url: Option<String>,
        file_size: Option<i64>,
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

/// Разметка ответа (клавиатура) для мессенджеров.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReplyMarkup {
    Telegram(TelegramKeyboard),
}

/// Клавиатура Telegram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramKeyboard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<Vec<Vec<TelegramKeyboardButton>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resize_keyboard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_time_keyboard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_keyboard: Option<bool>,
}

/// Кнопка клавиатуры Telegram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_contact: Option<bool>,
}

impl TelegramKeyboard {
    /// Создать клавиатуру с кнопкой "Поделиться контактом".
    pub fn request_contact() -> Self {
        Self {
            keyboard: Some(vec![vec![TelegramKeyboardButton {
                text: "📱 Поделиться номером телефона".to_string(),
                request_contact: Some(true),
            }]]),
            resize_keyboard: Some(true),
            one_time_keyboard: Some(true),
            remove_keyboard: None,
        }
    }

    /// Убрать клавиатуру.
    pub fn remove() -> Self {
        Self {
            keyboard: None,
            resize_keyboard: None,
            one_time_keyboard: None,
            remove_keyboard: Some(true),
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
