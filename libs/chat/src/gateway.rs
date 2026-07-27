use crate::error::{ChatError, Result};
use crate::messengers::Messenger;
use crate::messengers::telegram::{TelegramMessenger, TgCredentials};
use crate::messengers::vk::{VkCredentials, VkMessenger};
use crate::models::{Platform, SendMessageRequest, UnifiedMessage};

use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct MessengerGateway {
    tg: Arc<TelegramMessenger>,
    vk: Arc<VkMessenger>,
}

impl MessengerGateway {
    pub fn new() -> Self {
        Self {
            tg: Arc::new(TelegramMessenger::new()),
            vk: Arc::new(VkMessenger::new()),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn send(
        &self,
        platform: Platform,
        request: SendMessageRequest,
        credentials: &[u8],
        mirrors: Vec<String>,
    ) -> Result<()> {
        match platform {
            Platform::Telegram => {
                let cred = TgCredentials::from_bytes(credentials, mirrors)?;
                self.tg.send_message(&request, cred).await
            }
            Platform::VK => {
                let cred = VkCredentials::from_bytes(credentials, mirrors)?;
                self.vk.send_message(&request, cred).await
            }
            platform => Err(ChatError::Platform(platform)),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn send_text(
        &self,
        platform: Platform,
        chat_id: impl Into<String>,
        text: impl Into<String>,
        reply_to_message_id: Option<String>,
        credentials: &[u8],
        mirrors: Vec<String>,
    ) -> Result<()> {
        let request = SendMessageRequest {
            chat_id: chat_id.into(),
            text: text.into(),
            reply_to_message_id,
            reply_markup: None,
        };
        self.send(platform, request, credentials, mirrors).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn start_inbound_polling<H>(
        &self,
        handler: H,
        tg_credentials: TgCredentials,
        vk_credentials: VkCredentials,
    ) where
        H: InboundHandler + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);

        // --- TELEGRAM ---
        let tg = self.tg.clone();
        let h_tg = handler.clone();
        tokio::spawn(async move {
            // TODO: Think of a better way.
            let _ = tg.ensure_polling_mode(&tg_credentials).await;
            let mut offset: i64 = 0;
            loop {
                match tg.fetch_messages(offset, tg_credentials.clone()).await {
                    Ok((messages, new_offset)) => {
                        for msg in messages {
                            h_tg.handle_inbound_message(msg).await;
                        }
                        if new_offset > offset {
                            offset = new_offset;
                        }
                    }
                    Err(e) => {
                        tracing::error!("[TG Gateway Error] {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    }
                }
            }
        });

        // --- VK ---
        let vk = self.vk.clone();
        let h_vk = handler;
        tokio::spawn(async move {
            let mut offset: i64 = 0;
            loop {
                match vk.fetch_messages(offset, vk_credentials.clone()).await {
                    Ok((messages, new_offset)) => {
                        for msg in messages {
                            h_vk.handle_inbound_message(msg).await;
                        }
                        if new_offset > offset {
                            offset = new_offset;
                        }
                    }
                    Err(e) => {
                        tracing::error!("[VK Gateway Error] {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    }
                }
            }
        });
    }
}

#[allow(async_fn_in_trait)]
pub trait InboundHandler: Send + Sync {
    fn handle_inbound_message(
        &self,
        message: UnifiedMessage,
    ) -> impl std::future::Future<Output = ()> + Send;
}
