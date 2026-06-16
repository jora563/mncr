use crate::client::Client;
use crate::messengers::Messenger;
use crate::models::{Platform, SendMessageRequest, UnifiedMessage};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const TG_BOT_TOKEN: &str = "5197913983:AAF7vSQpinqjvjQkMYwiG7TiDv_pxk4XjCE";

#[derive(Debug, Deserialize)]
struct TgUpdatesResponse {
    ok: bool,
    #[serde(default)]
    result: Option<Vec<TgUpdate>>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TgSendResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TgUpdate {
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Debug, Deserialize)]
struct TgMessage {
    message_id: i64,
    from: TgUser,
    chat: TgChat,
    date: u64,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TgUser { id: i64 }
#[derive(Debug, Deserialize)]
struct TgChat { id: i64 }

#[derive(Debug, Serialize)]
struct TgSendRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
}

pub struct TelegramMessenger {
    client: Client,
}

impl TelegramMessenger {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    pub async fn ensure_polling_mode(&self) -> Result<(), String> {
        let url = format!("https://api.telegram.org/bot{}/deleteWebhook?drop_pending_updates=true", TG_BOT_TOKEN);
        match self.client.get(&url).await {
            Ok(_) => { println!("[TG] Вебхук удален, режим Long Polling активирован."); Ok(()) }
            Err(e) => Err(format!("Не удалось удалить вебхук: {}", e)),
        }
    }
}

impl Messenger for TelegramMessenger {
    async fn fetch_messages(&self, offset: i64) -> Result<(Vec<UnifiedMessage>, i64), String> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=25", TG_BOT_TOKEN, offset);
        
        let response_text = match self.client.get(&url).await {
            Ok(text) => text,
            Err(e) => {
                eprintln!("[TG] Сетевая ошибка: {}", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
                return Ok((Vec::new(), offset)); 
            }
        };

        let tg_response: TgUpdatesResponse = match serde_json::from_str(&response_text) {
            Ok(parsed) => parsed,
            Err(e) => return Err(format!("Ошибка парсинга JSON: {}", e)),
        };

        if !tg_response.ok {
            if let Some(desc) = tg_response.description {
                eprintln!("[TG] API Warning: {}", desc);
            }
            return Ok((Vec::new(), offset));
        }

        let mut unified_messages = Vec::new();
        let mut max_update_id = offset;
        let mut received_any_update = false;

        if let Some(updates) = tg_response.result {
            for update in updates {
                received_any_update = true;
                if update.update_id >= max_update_id {
                    max_update_id = update.update_id;
                }

                if let Some(msg) = update.message {
                    unified_messages.push(UnifiedMessage {
                        platform: Platform::Telegram,
                        user_id: msg.from.id.to_string(),
                        chat_id: msg.chat.id.to_string(),
                        text: msg.text.unwrap_or_else(|| "[Медиа]".to_string()),
                        timestamp: msg.date,
                        message_id: Some(msg.message_id.to_string()),
                    });
                }
            }
        }

        if received_any_update {
            Ok((unified_messages, max_update_id + 1))
        } else {
            Ok((unified_messages, offset))
        }
    }

    async fn send_message(&self, request: &SendMessageRequest) -> Result<(), String> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", TG_BOT_TOKEN);
        
        let reply_id = request.reply_to_message_id.as_ref().and_then(|id| id.parse::<i64>().ok());

        let payload = TgSendRequest {
            chat_id: &request.chat_id,
            text: &request.text,
            reply_to_message_id: reply_id,
        };

        let response_text = self.client.post_json(&url, &payload).await.map_err(|e| e.to_string())?;
        
        let tg_response: TgSendResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Ошибка парсинга ответа отправки: {} (Response: {})", e, response_text))?;

        if !tg_response.ok {
            let desc = tg_response.description.as_deref().unwrap_or("Неизвестная ошибка");
            Err(format!("Telegram вернул ошибку: {}", desc))
        } else {
            Ok(())
        }
    }
}
