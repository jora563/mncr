use super::Messenger;
use crate::error::{ChatError, Result};
use crate::models::{Attachment, Platform, SendMessageRequest, UnifiedMessage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TgCredentials {
    pub bot_token: String,
}

impl TgCredentials {
    /// На базе данных креды содержатся как бинарные данные.
    /// Тут они переписываются как строка.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let bot_token = String::from_utf8(bytes.to_vec())
            .map_err(|e| ChatError::Other(format!("Invalid UTF-8 in token: {}", e)))?;
        Ok(Self { bot_token })
    }
}

/// Список зеркал (хостов) для API Telegram, получаемый из БД.
#[derive(Clone, Debug)]
pub struct TgMirrors {
    pub urls: Vec<String>,
}

/// Контекст сессии, объединяющий учетные данные и маршрутизацию.
/// Используется как `Credentials` в реализации трейта `Messenger`.
#[derive(Clone, Debug)]
pub struct TgSession {
    pub cred: TgCredentials,
    pub mirrors: TgMirrors,
}

// --- Структуры для десериализации ответов Telegram API ---

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
    user_id: Option<i64>,
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

// --- Основная реализация ---

#[derive(Clone, Debug, Default)]
pub struct TelegramMessenger {
    client: reqwest::Client,
}

impl TelegramMessenger {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Активирует режим Long Polling, перебирая зеркала до первого успешного ответа.
    pub async fn ensure_polling_mode(&self, session: &TgSession) -> Result<()> {
        for host in &session.mirrors.urls {
            let url = format!(
                "https://{}/bot{}/deleteWebhook?drop_pending_updates=true",
                host, session.cred.bot_token
            );
            if self.client.get(&url).send().await.is_ok() {
                tracing::info!(
                    "[TG] Вебхук удален на хосте {}, режим Long Polling активирован.",
                    host
                );
                return Ok(());
            }
        }
        Err(ChatError::Other(
            "Не удалось удалить вебхук ни на одном из зеркал".to_string(),
        ))
    }
}

impl Messenger for TelegramMessenger {
    type Credentials = TgSession;

    async fn fetch_messages(
        &self,
        offset: i64,
        cred: Self::Credentials,
    ) -> Result<(Vec<UnifiedMessage>, i64)> {
        let mut last_error = None;

        for host in &cred.mirrors.urls {
            let url = format!(
                "https://{}/bot{}/getUpdates?offset={}&timeout=25",
                host, cred.cred.bot_token, offset
            );

            match self.client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(tg_response) = serde_json::from_str::<TgUpdatesResponse>(&text) {
                            if !tg_response.ok {
                                if let Some(desc) = &tg_response.description {
                                    tracing::warn!("[TG] API Warning on {}: {}", host, desc);
                                }
                                // Ошибка API (напр. неверный токен), нет смысла пробовать другие зеркала
                                return Ok((Vec::new(), offset));
                            }

                            let mut unified_messages = Vec::new();
                            let mut max_update_id = offset;
                            let mut received_any = false;

                            if let Some(updates) = tg_response.result {
                                for update in updates {
                                    received_any = true;
                                    max_update_id = max_update_id.max(update.update_id);
                                    if let Some(msg) = update.message {
                                        let attachments = parse_attachments(&msg);
                                        let text =
                                            msg.text.as_deref().unwrap_or("[Медиа]").to_string();

                                        unified_messages.push(UnifiedMessage {
                                            platform: Platform::Telegram,
                                            user_id: msg.from.id.to_string(),
                                            chat_id: msg.chat.id.to_string(),
                                            text,
                                            timestamp: msg.date,
                                            message_id: Some(msg.message_id.to_string()),
                                            attachments,
                                        });
                                    }
                                }
                            }
                            return Ok((
                                unified_messages,
                                if received_any {
                                    max_update_id + 1
                                } else {
                                    offset
                                },
                            ));
                        } else {
                            last_error = Some(format!("JSON parse error on {}", host));
                        }
                    } else {
                        last_error = Some(format!("Read error on {}", host));
                    }
                }
                Err(e) => {
                    last_error = Some(format!("Network error on {}: {}", host, e));
                }
            }
        }

        Err(ChatError::Other(format!(
            "Все зеркала недоступны. Последняя ошибка: {}",
            last_error.unwrap_or_else(|| "Список зеркал пуст".to_string())
        )))
    }

    async fn send_message(
        &self,
        request: &SendMessageRequest,
        cred: Self::Credentials,
    ) -> Result<()> {
        let mut last_error = None;

        for host in &cred.mirrors.urls {
            let url = format!("https://{}/bot{}/sendMessage", host, cred.cred.bot_token);

            let reply_id = request
                .reply_to_message_id
                .as_ref()
                .and_then(|id| id.parse::<i64>().ok());
            let reply_markup_str = request
                .reply_markup
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| ChatError::Other(format!("Serialize markup: {}", e)))?;

            let payload = TgSendRequest {
                chat_id: &request.chat_id,
                text: &request.text,
                reply_to_message_id: reply_id,
                reply_markup: reply_markup_str,
            };

            match self.client.post(&url).json(&payload).send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(tg_response) = serde_json::from_str::<TgSendResponse>(&text) {
                            if !tg_response.ok {
                                return Err(ChatError::Other(
                                    tg_response
                                        .description
                                        .unwrap_or_else(|| "Unknown API error".to_string()),
                                ));
                            }
                            return Ok(());
                        } else {
                            last_error = Some(format!("JSON parse error on {}", host));
                        }
                    } else {
                        last_error = Some(format!("Read error on {}", host));
                    }
                }
                Err(e) => {
                    last_error = Some(format!("Network error on {}: {}", host, e));
                }
            }
        }

        Err(ChatError::Other(format!(
            "Все зеркала недоступны. Последняя ошибка: {}",
            last_error.unwrap_or_else(|| "Список зеркал пуст".to_string())
        )))
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
            user_id: contact.user_id.map(|id| id.to_string()),
        });
    }

    if let Some(photos) = &msg.photo
        && let Some(largest) = photos.last()
    {
        attachments.push(Attachment::Photo {
            file_id: largest.file_id.clone(),
            file_url: None,
            file_size: largest.file_size,
        });
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
