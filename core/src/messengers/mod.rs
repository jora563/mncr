//! Модуль логики работы с чатами
use ahash::AHashMap;
use chat::messengers::{Messenger, TelegramMessenger, TgCredentials, VkCredentials, VkMessenger};
use chat::models::{ReplyMarkup, SendMessageRequest, UnifiedMessage};
use chat::verification;
use db::core_schema::{ApiId, CoreDbCrud, DbBotAccountWithMeta, DbChat, DbFullMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{CoreError, Result};

#[derive(Debug, Default)]
pub(crate) struct ChatDriver {
    tg: Arc<TelegramMessenger>,
    tg_offsets: Arc<RwLock<AHashMap<i64, i64>>>,
    vk: Arc<VkMessenger>,
    vk_offsets: Arc<RwLock<AHashMap<i64, i64>>>,
}

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
        self.0.iter().find_map(|msg| verification::extract_phone(msg))
    }

    pub(crate) fn user_name(&self) -> String {
        self.0
            .iter()
            .find_map(|msg| verification::extract_name(msg))
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub(crate) fn is_contact_only(&self) -> bool {
        self.0.iter().any(|msg| verification::is_contact_only(msg))
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
            let cred = TgCredentials::from_bytes(&platform.account.token)?;
            self.tg.ensure_polling_mode(&cred).await?;
        }
        Ok(())
    }

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
                let cred = TgCredentials::from_bytes(&platform.account.token)?;
                self.tg
                    .send_message(&request, cred)
                    .await
                    .map_err(CoreError::ChatLib)
            }
            ApiId::Vk => {
                let cred = VkCredentials::from_bytes(&platform.account.token)?;
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
    offsets: &Arc<RwLock<AHashMap<i64, i64>>>,
    platform: &DbBotAccountWithMeta,
) -> Result<ChatMessages> {
    let bot_acc = &platform.account;
    tracing::info!("Polling for Telegram messages for {}", bot_acc.external_id);

    let offset = offsets
        .read()
        .await
        .get(&bot_acc.pkey())
        .copied()
        .unwrap_or(0);
    let cred = TgCredentials::from_bytes(&bot_acc.token)?;

    let (messages, new_offset) = messenger
        .fetch_messages(offset, cred)
        .await
        .map_err(CoreError::ChatLib)?;

    *offsets.write().await.entry(bot_acc.pkey()).or_insert(0) = new_offset;

    Ok(ChatMessages(messages))
}

#[tracing::instrument(skip_all)]
async fn get_vk(
    messenger: &VkMessenger,
    offsets: &Arc<RwLock<AHashMap<i64, i64>>>,
    platform: &DbBotAccountWithMeta,
) -> Result<ChatMessages> {
    let bot_acc = &platform.account;
    tracing::info!("Polling for VK messages for {}", bot_acc.external_id);

    let offset = offsets
        .read()
        .await
        .get(&bot_acc.pkey())
        .copied()
        .unwrap_or(0);
    let cred = VkCredentials::from_bytes(&bot_acc.token)?;

    let (messages, new_offset) = messenger
        .fetch_messages(offset, cred)
        .await
        .map_err(CoreError::ChatLib)?;

    *offsets.write().await.entry(bot_acc.pkey()).or_insert(0) = new_offset;

    Ok(ChatMessages(messages))
}
