use super::Messenger;
use crate::client::Client;
use crate::error::{ChatError, Result, VkError};
use crate::models::{Platform, SendMessageRequest, UnifiedMessage};

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// TODO: Decide whether this is a valid format or not.
/// [vk_group_id]::[vk_access_token]
#[derive(Clone)]
pub struct VkCredentials {
    group_id: String,
    access_token: String,
}

impl VkCredentials {
    const API_VERSION: &str = "5.199";
    /// На базе данных креды содержатся как бинарные данные.
    /// Тут они переписываются как строка.
    #[tracing::instrument(skip_all)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let uncut = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
        let (a, b) = uncut
            .split_once("::")
            .ok_or_else(|| ChatError::vk_token(&uncut))?;
        Ok(Self {
            group_id: a.to_string(),
            access_token: b.to_string(),
        })
    }

    fn get_info_address(&self) -> String {
        format!(
            "https://api.vk.com/method/messages.getLongPollServer?group_id={}&access_token={}&v={}&lp_version=3",
            self.group_id,
            self.access_token,
            VkCredentials::API_VERSION
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct VkServerInfo {
    server: String,
    key: String,
    ts: u64,
}

#[derive(Debug, Serialize)]
struct VkSendRequest<'a> {
    access_token: &'a str,
    v: &'a str,
    peer_id: i64,
    message: &'a str,
    random_id: u64,
}

#[derive(Debug, Default)]
pub struct VkMessenger {
    client: Client,
    server_info: Mutex<Option<VkServerInfo>>,
}

impl VkMessenger {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            server_info: Mutex::new(None),
        }
    }

    fn get_current_ts(&self) -> u64 {
        let guard = self.server_info.lock().unwrap();
        guard.as_ref().map(|info| info.ts).unwrap_or(0)
    }

    #[tracing::instrument(skip_all)]
    async fn fetch_server_info(&self, cred: VkCredentials) -> Result<VkServerInfo> {
        let url = cred.get_info_address();

        let response_text = self.client.get(&url).await.map_err(|e| e.to_string())?;

        #[derive(Debug, Deserialize)]
        struct VkApiResponse {
            response: VkServerInfo,
        }

        let parsed: VkApiResponse = serde_json::from_str(&response_text)?;

        Ok(parsed.response)
    }

    fn update_ts(&self, new_ts: u64) {
        let mut guard = self.server_info.lock().unwrap();
        if let Some(info) = guard.as_mut() {
            info.ts = new_ts;
        }
    }

    fn reset_cache(&self) {
        let mut guard = self.server_info.lock().unwrap();
        *guard = None;
    }
}

#[derive(Debug, Deserialize)]
struct VkSendResponse {
    response: Option<u64>,
    error: Option<VkError>,
}

impl Messenger for VkMessenger {
    type Credentials = VkCredentials;

    #[tracing::instrument(skip_all)]
    async fn fetch_messages(
        &self,
        offset: i64,
        cred: Self::Credentials,
    ) -> Result<(Vec<UnifiedMessage>, i64)> {
        #[derive(Debug, Deserialize)]
        struct VkPollResponse {
            #[serde(default)]
            ts: Option<u64>,
            failed: Option<i64>,
            #[serde(default)]
            updates: Option<Vec<serde_json::Value>>,
        }

        let current_info = {
            let guard = self.server_info.lock().unwrap();
            guard.clone()
        };

        let server_info: VkServerInfo = if let Some(info) = current_info
            && offset != 0
        {
            info
        } else {
            let new_info = self.fetch_server_info(cred).await?;
            {
                let mut guard = self.server_info.lock().unwrap();
                *guard = Some(new_info.clone());
            }
            new_info
        };

        let poll_url = format!(
            "https://{}?act=a_check&key={}&ts={}&wait=25&version=3",
            server_info.server, server_info.key, server_info.ts
        );

        let response_text = self
            .client
            .get(&poll_url)
            .await
            .map_err(|e| e.to_string())?;

        let poll_response: VkPollResponse =
            serde_json::from_str(&response_text).map_err(ChatError::VkLongPoll)?;

        if let Some(failed_code) = poll_response.failed {
            if failed_code == 1 {
                if let Some(new_ts) = poll_response.ts {
                    self.update_ts(new_ts);
                }
                return Ok((Vec::new(), self.get_current_ts() as i64));
            } else {
                self.reset_cache();
                return Err(ChatError::VkLongPollKey);
            }
        }

        if let Some(new_ts) = poll_response.ts {
            self.update_ts(new_ts);
        }

        let mut unified_messages = Vec::new();

        if let Some(updates) = poll_response.updates {
            for update in updates {
                if let Some(update_array) = update.as_array()
                    && update_array.first().and_then(|v| v.as_u64()) == Some(4)
                {
                    let message_id = update_array.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
                    let flags = update_array.get(2).and_then(|v| v.as_u64()).unwrap_or(0);
                    let peer_id = update_array.get(3).and_then(|v| v.as_i64()).unwrap_or(0);
                    let timestamp = update_array.get(4).and_then(|v| v.as_u64()).unwrap_or(0);
                    let text = update_array
                        .get(5)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let is_incoming = (flags & 2) == 0;
                    if !is_incoming {
                        continue;
                    }

                    let from_id =
                        if let Some(extra) = update_array.get(6).and_then(|v| v.as_object()) {
                            extra
                                .get("from_id")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(peer_id)
                        } else {
                            peer_id
                        };

                    if from_id > 0 {
                        unified_messages.push(UnifiedMessage {
                            platform: Platform::VK,
                            user_id: from_id.to_string(),
                            chat_id: peer_id.to_string(),
                            text: if text.is_empty() {
                                "[Медиа]".to_string()
                            } else {
                                text
                            },
                            timestamp,
                            message_id: Some(message_id.to_string()),
                            attachments: Vec::new(), // VK: вложения пока не парсим
                        });
                    }
                }
            }
        }

        Ok((unified_messages, self.get_current_ts() as i64))
    }

    #[tracing::instrument(skip_all)]
    async fn send_message(
        &self,
        request: &SendMessageRequest,
        cred: Self::Credentials,
    ) -> Result<()> {
        let url = "https://api.vk.com/method/messages.send";

        let peer_id = request
            .chat_id
            .parse::<i64>()
            .map_err(|_| ChatError::VkChatId)?;

        let random_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let payload = VkSendRequest {
            access_token: &cred.access_token,
            v: VkCredentials::API_VERSION,
            peer_id,
            message: &request.text,
            random_id,
        };

        let response_text = self
            .client
            .post_form(url, &payload)
            .await
            .map_err(|e| e.to_string())?;

        let vk_response: VkSendResponse = serde_json::from_str(&response_text)?;

        if let Some(error) = vk_response.error {
            Err(ChatError::VkResponse(error))
        } else if vk_response.response.is_some() {
            Ok(())
        } else {
            Err(ChatError::VkUnexpected(response_text))
        }
    }
}
