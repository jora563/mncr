//! Модуль логики работы с чатами

use ahash::AHashMap;
use chat::messengers::{
    Messenger, TelegramMessenger, TgCredentials, TgMirrors, TgSession, VkCredentials, VkMessenger,
};
use chat::models::{ReplyMarkup, SendMessageRequest, UnifiedMessage};
use chat::verification;
use db::core_schema::{ApiId, DbBotAccountWithMeta, DbChat, DbFullMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{CoreError, Result};

/// Драйвер в которым модуль чата может жить своей лучшей жизнью
#[derive(Debug, Default)]
pub(crate) struct ChatDriver {
    tg: Arc<TelegramMessenger>,
    /// ИД бота (ключ) против оффсета (значение). Оффсет это измерение которым пользуются
    ///  многие месенджеры чтобы 'понять' с какого сообщения начинать возврощать сообщения.
    tg_offsets: Arc<RwLock<AHashMap<String, i64>>>,
    vk: Arc<VkMessenger>,
    /// ИД бота (ключ) против оффсета (значение). Оффсет это измерение которым пользуются
    ///  многие месенджеры чтобы 'понять' с какого сообщения начинать возврощать сообщения.
    vk_offsets: Arc<RwLock<AHashMap<String, i64>>>,
}

/// TODO: Replace with the real thing.
#[derive(Debug, Clone)]
pub(crate) struct ChatMessages(Vec<UnifiedMessage>);

impl ChatMessages {
    pub(crate) fn user_external_id(&self) -> &str {
        self.0.last().map_or("", |v| &v.user_id)
    }

    pub(crate) fn last_msg_external_id(&self) -> Option<String> {
        self.0.last().and_then(|v| v.message_id.clone())
    }

    pub(crate) fn chat_external_id(&self) -> &str {
        self.0.last().map_or("", |v| &v.chat_id)
    }

    pub(crate) fn phone(&self) -> Option<String> {
        self.0.iter().find_map(verification::extract_phone)
    }

    pub(crate) fn user_name(&self) -> String {
        self.0
            .iter()
            .find_map(verification::extract_name)
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(crate) fn is_contact(&self) -> bool {
        self.0.iter().any(verification::is_contact)
    }

    /// Получить user_id из контакта (если контакт соответствует пользователю)
    pub(crate) fn contact_user_id(&self) -> Option<String> {
        self.0
            .iter()
            .find_map(verification::extract_contact_user_id)
    }

    pub(crate) fn texts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|x| x.text.as_ref())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

impl ChatDriver {
    #[tracing::instrument(skip_all)]
    pub(crate) async fn initialise(&self, platform: &DbBotAccountWithMeta) -> Result<()> {
        if matches!(platform.platform.platform.api_id, ApiId::Telegram) {
            let mirrors = TgMirrors {
                urls: platform
                    .platform
                    .mirrors
                    .iter()
                    .map(|m| m.url.clone())
                    .collect(),
            };
            let cred =
                TgCredentials::from_bytes(&platform.account.token).map_err(CoreError::ChatLib)?;
            let session = TgSession { cred, mirrors };

            self.tg
                .ensure_polling_mode(&session)
                .await
                .map_err(CoreError::ChatLib)?;
        }
        Ok(())
    }

    /// Принимаем сообщение из мессенджеров.
    /// Внутри сидит логика из библиотеки `chat`.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn get_messages(
        &self,
        platform: &DbBotAccountWithMeta,
    ) -> Result<ChatMessages> {
        match platform.platform.platform.api_id {
            ApiId::Telegram => get_telegram(&self.tg, &self.tg_offsets, platform).await,
            ApiId::Vk => get_vk(&self.vk, &self.vk_offsets, platform).await,
            ApiId::Max => Err(CoreError::ChatApiDisconnected(ApiId::Max)),
        }
    }

    /// Send messages back to a platform.
    /// Внутри сидит логика из библиотеки `chat`.
    #[tracing::instrument(skip_all)]
    pub(crate) async fn send_messages(
        &self,
        platform: &DbBotAccountWithMeta,
        chat: &DbChat,
        message: DbFullMessage,
        original: ChatMessages,
    ) -> Result<()> {
        self.send(
            platform,
            &chat.external_id,
            &message.message.content.unwrap_or_default(),
            original.last_msg_external_id(),
            None,
        )
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn send(
        &self,
        platform: &DbBotAccountWithMeta,
        chat_id: &str,
        text: &str,
        reply_to: Option<String>,
        reply_markup: Option<ReplyMarkup>,
    ) -> Result<()> {
        let request = SendMessageRequest {
            chat_id: chat_id.to_owned(),
            text: text.to_owned(),
            reply_to_message_id: reply_to,
            reply_markup,
        };

        match platform.platform.platform.api_id {
            ApiId::Telegram => {
                let mirrors = TgMirrors {
                    urls: platform
                        .platform
                        .mirrors
                        .iter()
                        .map(|m| m.url.clone())
                        .collect(),
                };
                let cred = TgCredentials::from_bytes(&platform.account.token)?;
                let session = TgSession { cred, mirrors };

                self.tg
                    .send_message(&request, session)
                    .await
                    .map_err(CoreError::ChatLib)
            }
            ApiId::Vk => {
                let mirrors: Vec<String> = platform
                    .platform
                    .mirrors
                    .iter()
                    .map(|m| m.url.clone())
                    .collect();
                let cred = VkCredentials::from_bytes(&platform.account.token, mirrors)
                    .map_err(CoreError::ChatLib)?;
                self.vk
                    .send_message(&request, cred)
                    .await
                    .map_err(CoreError::ChatLib)
            }
            ApiId::Max => Err(CoreError::ChatApiDisconnected(ApiId::Max)),
        }
    }
}

#[tracing::instrument(skip_all)]
async fn get_telegram(
    messenger: &TelegramMessenger,
    offsets: &Arc<RwLock<AHashMap<String, i64>>>,
    platform: &DbBotAccountWithMeta,
) -> Result<ChatMessages> {
    let bot_acc = &platform.account;
    tracing::info!("Polling for Telegram messages for {}", bot_acc.external_id);

    let offset = offsets
        .read()
        .await
        .get(&bot_acc.external_id)
        .copied()
        .unwrap_or(0);

    let mirrors = TgMirrors {
        urls: platform
            .platform
            .mirrors
            .iter()
            .map(|m| m.url.clone())
            .collect(),
    };
    let cred = TgCredentials::from_bytes(&bot_acc.token).map_err(CoreError::ChatLib)?;
    let session = TgSession { cred, mirrors };

    let (messages, new_offset) = messenger
        .fetch_messages(offset, session)
        .await
        .map_err(CoreError::ChatLib)?;

    *offsets
        .write()
        .await
        .entry(bot_acc.external_id.clone())
        .or_insert(0) = new_offset;

    Ok(ChatMessages(messages))
}

#[tracing::instrument(skip_all)]
async fn get_vk(
    messenger: &VkMessenger,
    offsets: &Arc<RwLock<AHashMap<String, i64>>>,
    platform: &DbBotAccountWithMeta,
) -> Result<ChatMessages> {
    let bot_acc = &platform.account;
    tracing::info!("Polling for VK messages for {}", bot_acc.external_id);

    let offset = offsets
        .read()
        .await
        .get(&bot_acc.external_id)
        .copied()
        .unwrap_or(0);

    let mirrors: Vec<String> = platform
        .platform
        .mirrors
        .iter()
        .map(|m| m.url.clone())
        .collect();
    let cred = VkCredentials::from_bytes(&bot_acc.token, mirrors).map_err(CoreError::ChatLib)?;

    let (messages, new_offset) = messenger
        .fetch_messages(offset, cred)
        .await
        .map_err(CoreError::ChatLib)?;

    *offsets
        .write()
        .await
        .entry(bot_acc.external_id.clone())
        .or_insert(0) = new_offset;

    Ok(ChatMessages(messages))
}
