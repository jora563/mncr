//! Модуль логики работы с чатами
use ahash::AHashMap;
use chat::messengers::{Messenger, TelegramMessenger, TgCredentials, VkCredentials, VkMessenger};
use chat::models::{SendMessageRequest, UnifiedMessage};
use db::core_schema::{ApiId, CoreDbCrud, DbBotAccountWithMeta, DbChat, DbFullMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{CoreError, Result};

/// Драйвер в которым модуль чата может жить своей лучшей жизнью
#[derive(Debug, Default)]
pub(crate) struct ChatDriver {
    tg: Arc<TelegramMessenger>,
    /// ИД бота (ключ) против оффсета (значение). Оффсет это измерение которым пользуются
    ///  многие месенджеры чтобы 'понять' с какого сообщения начинать возврощать сообщения.
    tg_offsets: Arc<RwLock<AHashMap<i64, i64>>>,
    vk: Arc<VkMessenger>,
    /// ИД бота (ключ) против оффсета (значение). Оффсет это измерение которым пользуются
    ///  многие месенджеры чтобы 'понять' с какого сообщения начинать возврощать сообщения.
    vk_offsets: Arc<RwLock<AHashMap<i64, i64>>>,
}

/// TODO: Replace with the real thing.
#[derive(Debug)]
pub(crate) struct ChatMessages(Vec<UnifiedMessage>);

impl ChatMessages {
    pub(crate) fn get_user_external_id(&self) -> &str {
        self.0.iter().last().map_or("", |v| &v.user_id)
    }
    pub(crate) fn get_last_msg_external_id(&self) -> Option<String> {
        self.0.iter().last().and_then(|v| v.message_id.clone())
    }
    pub(crate) fn get_user_nick(&self) -> &str {
        "Unknown"
    }
    pub(crate) fn get_chat_external_id(&self) -> &str {
        self.0.iter().last().map_or("", |v| &v.chat_id)
    }
    pub(crate) fn get_ticket_number(&self) -> Option<i32> {
        None
    }
    /// Если ничего другого нет, то берёмся за телефон.
    pub(crate) fn get_phone(&self) -> Option<String> {
        Some("+79452200022".to_string())
    }

    pub(crate) fn get_text(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|x| x.text.as_ref())
    }

    /// Пришла ли нам пустышка.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Пришла ли нам пустышка.
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

impl ChatDriver {
    #[tracing::instrument(skip_all)]
    pub(crate) async fn initialise(&self, platform: &DbBotAccountWithMeta) -> Result<()> {
        if matches!(platform.platform.platform.api_id, ApiId::Telegram) {
            let bot_acc = &platform.account;
            let cred = TgCredentials::from_bytes(&bot_acc.token)?;
            self.tg.ensure_polling_mode(&cred).await?;
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
            ApiId::Max => return Err(CoreError::ChatApiDisconnected(ApiId::Max)),
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
        let request = SendMessageRequest {
            chat_id: chat.external_id.to_owned(),
            text: message.message.content.unwrap_or_default(),
            reply_to_message_id: original.get_last_msg_external_id(),
        };
        match platform.platform.platform.api_id {
            ApiId::Telegram => self
                .tg
                .send_message(
                    &request,
                    TgCredentials::from_bytes(&platform.account.token)?,
                )
                .await
                .map_err(CoreError::ChatLib)?,
            ApiId::Vk => self
                .vk
                .send_message(
                    &request,
                    VkCredentials::from_bytes(&platform.account.token)?,
                )
                .await
                .map_err(CoreError::ChatLib)?,
            ApiId::Max => return Err(CoreError::ChatApiDisconnected(ApiId::Max)),
        };
        Ok(())
    }
}

#[tracing::instrument(skip_all)]
async fn get_telegram(
    messenger: &TelegramMessenger,
    offsets: &Arc<RwLock<AHashMap<i64, i64>>>,
    platform: &DbBotAccountWithMeta,
) -> Result<ChatMessages> {
    let bot_acc = &platform.account;
    tracing::info!(
        "Polling for Telegram-Api messages for {}",
        bot_acc.external_id
    );

    let offset = offsets
        .read()
        .await
        .get(&platform.account.pkey())
        .copied()
        .unwrap_or(0);
    let cred = TgCredentials::from_bytes(&bot_acc.token)?;

    let (messages, new_offset) = messenger
        .fetch_messages(offset, cred)
        .await
        .map_err(CoreError::ChatLib)?;

    let mut guard = offsets.write().await;
    let offset = guard.entry(bot_acc.pkey()).or_insert(0);
    *offset = new_offset;

    Ok(ChatMessages(messages))
}

#[tracing::instrument(skip_all)]
async fn get_vk(
    messenger: &VkMessenger,
    offsets: &Arc<RwLock<AHashMap<i64, i64>>>,
    platform: &DbBotAccountWithMeta,
) -> Result<ChatMessages> {
    let bot_acc = &platform.account;
    tracing::info!("Polling for Vk-Api messages for {}", bot_acc.external_id);

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

    let mut guard = offsets.write().await;
    let offset = guard.entry(bot_acc.pkey()).or_insert(0);
    *offset = new_offset;

    Ok(ChatMessages(messages))
}
