use super::Messenger;
use crate::client::Client;
use crate::error::{ChatError, Result};
use crate::models::{Attachment, Platform, SendMessageRequest, UnifiedMessage};

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct TgCredentials {
    bot_token: String,
}

impl TgCredentials {
    #[tracing::instrument(skip_all)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let bot_token = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
        Ok(Self { bot_token })
    }

    fn get_fetch_address(&self, offset: i64) -> String {
        format!(
            "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=25",
            self.bot_token, offset
        )
    }

    fn get_info_address(&self) -> String {
        format!(
            "https://api.telegram.org/bot{}/deleteWebhook?drop_pending_updates=true",
            self.bot_token
        )
    }

    fn get_send_address(&self) -> String {
        format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token)
    }
}

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
    contact: Option<TgContact>,
    photo: Option<Vec<TgPhotoSize>>,
    document: Option<TgDocument>,
}

#[derive(Debug, Deserialize)]
struct TgUser {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TgChat {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct TgContact {
    phone_number: String,
    first_name: String,
    last_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TgPhotoSize {
    file_id: String,
    file_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TgDocument {
    file_id: String,
    file_size: Option<i64>,
    file_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct TgSendRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TelegramMessenger {
    client: Client,
}

impl TelegramMessenger {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn ensure_polling_mode(&self, cred: &TgCredentials) -> Result<()> {
        let url = cred.get_info_address();
        self.client
            .get(&url)
            .await
            .map(|_| {
                tracing::info!("[TG] Вебхук удален, режим Long Polling активирован.");
            })
            .map_err(ChatError::tg_webhook)
    }
}

impl Messenger for TelegramMessenger {
    type Credentials = TgCredentials;

    #[tracing::instrument(skip_all)]
    async fn fetch_messages(
        &self,
        offset: i64,
        cred: Self::Credentials,
    ) -> Result<(Vec<UnifiedMessage>, i64)> {
        let url = cred.get_fetch_address(offset);

        let response_text = match self.client.get(&url).await {
            Ok(text) => text,
            Err(e) => {
                tracing::error!("[TG] Сетевая ошибка: {}", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
                return Ok((Vec::new(), offset));
            }
        };

        let tg_response: TgUpdatesResponse =
            serde_json::from_str(&response_text).map_err(ChatError::TgResponseParse)?;

        if !tg_response.ok {
            if let Some(desc) = tg_response.description {
                tracing::warn!("[TG] API Warning: {}", desc);
            }
            return Ok((Vec::new(), offset));
        }

        let mut unified_messages = Vec::new();
        let mut max_update_id = offset;
        let mut received_any_update = false;

        if let Some(updates) = tg_response.result {
            for update in updates {
                received_any_update = true;
                max_update_id = max_update_id.max(update.update_id);

                if let Some(msg) = update.message {
                    let attachments = parse_attachments(&msg);

                    unified_messages.push(UnifiedMessage {
                        platform: Platform::Telegram,
                        user_id: msg.from.id.to_string(),
                        chat_id: msg.chat.id.to_string(),
                        text: msg.text.unwrap_or_else(|| "[Медиа]".to_string()),
                        timestamp: msg.date,
                        message_id: Some(msg.message_id.to_string()),
                        attachments,
                    });
                }
            }
        }

        let new_offset = if received_any_update {
            max_update_id + 1
        } else {
            offset
        };

        Ok((unified_messages, new_offset))
    }

    #[tracing::instrument(skip_all)]
    async fn send_message(
        &self,
        request: &SendMessageRequest,
        cred: Self::Credentials,
    ) -> Result<()> {
        let url = cred.get_send_address();

        let reply_id = request
            .reply_to_message_id
            .as_ref()
            .and_then(|id| id.parse::<i64>().ok());

        let reply_markup_str = request
            .reply_markup
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()
            .map_err(|e| ChatError::tg_response(Some(e.to_string())))?;

        let payload = TgSendRequest {
            chat_id: &request.chat_id,
            text: &request.text,
            reply_to_message_id: reply_id,
            reply_markup: reply_markup_str,
        };

        let response_text = self
            .client
            .post_json(&url, &payload)
            .await
            .map_err(|e| e.to_string())?;

        let tg_response: TgSendResponse =
            serde_json::from_str(&response_text).map_err(ChatError::TgResponseParse)?;

        if !tg_response.ok {
            Err(ChatError::tg_response(tg_response.description))
        } else {
            Ok(())
        }
    }
}

/// Парсит вложения из сообщения Telegram
fn parse_attachments(msg: &TgMessage) -> Vec<Attachment> {
    let mut attachments = Vec::new();

    if let Some(contact) = &msg.contact {
        attachments.push(Attachment::Contact {
            phone: contact.phone_number.clone(),
            first_name: contact.first_name.clone(),
            last_name: contact.last_name.clone(),
        });
    }

    if let Some(photos) = &msg.photo {
        if let Some(largest) = photos.last() {
            attachments.push(Attachment::Photo {
                file_id: largest.file_id.clone(),
                file_url: None,
                file_size: largest.file_size,
            });
        }
    }

    if let Some(doc) = &msg.document {
        attachments.push(Attachment::Document {
            file_id: doc.file_id.clone(),
            file_url: None,
            file_size: doc.file_size,
            file_name: doc.file_name.clone(),
        });
    }

    attachments
}
