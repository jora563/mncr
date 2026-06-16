use crate::client::Client;
use crate::messengers::Messenger;
use crate::models::{Platform, SendMessageRequest, UnifiedMessage};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const VK_GROUP_ID: &str = "239313626";
const VK_ACCESS_TOKEN: &str = "vk1.a.jwA-jvAvRFcBKquRzTP3gcVClxBVU-8sg4lnObaVV2M-SK4QH1wrAtJMR9aStekMbNetpgp-qQJAsnBqT7nokW2Kox8JLUV0kqrBoD6xjBo67mxIUfcB4mnqcFk7wTbJmfDU51-Crcq2brwbiCqnqoCIwvVgrqbc5_N5eUtkGXwztQUI0KXl4skbMis4usgpAOh3A0-4slvbvOpSbpgc0w";
const VK_API_VERSION: &str = "5.199";

#[derive(Debug, Clone, Deserialize)]
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

    async fn fetch_server_info(&self) -> Result<VkServerInfo, String> {
        let url = format!(
            "https://api.vk.com/method/messages.getLongPollServer?group_id={}&access_token={}&v={}&lp_version=3",
            VK_GROUP_ID, VK_ACCESS_TOKEN, VK_API_VERSION
        );

        let response_text = self.client.get(&url).await.map_err(|e| e.to_string())?;

        #[derive(Debug, Deserialize)]
        struct VkApiResponse { response: VkServerInfo }

        let parsed: VkApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Ошибка парсинга сервера VK: {}", e))?;

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

impl Messenger for VkMessenger {
    async fn fetch_messages(&self, offset: i64) -> Result<(Vec<UnifiedMessage>, i64), String> {
        let current_info = {
            let guard = self.server_info.lock().unwrap();
            guard.clone()
        };

        let server_info = if current_info.is_none() || offset == 0 {
            let new_info = self.fetch_server_info().await?;
            {
                let mut guard = self.server_info.lock().unwrap();
                *guard = Some(new_info.clone());
            }
            new_info
        } else {
            current_info.unwrap()
        };

        let poll_url = format!(
            "https://{}?act=a_check&key={}&ts={}&wait=25&version=3",
            server_info.server, server_info.key, server_info.ts
        );

        let response_text = self.client.get(&poll_url).await.map_err(|e| e.to_string())?;

        #[derive(Debug, Deserialize)]
        struct VkPollResponse {
            #[serde(default)]
            ts: Option<u64>,
            failed: Option<u8>,
            #[serde(default)]
            updates: Option<Vec<serde_json::Value>>,
        }

        let poll_response: VkPollResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Ошибка парсинга ответа VK Long Poll: {}", e))?;

        if let Some(failed_code) = poll_response.failed {
            if failed_code == 1 {
                if let Some(new_ts) = poll_response.ts {
                    self.update_ts(new_ts);
                }
                return Ok((Vec::new(), self.get_current_ts() as i64));
            } else {
                self.reset_cache();
                return Err("Ключ VK Long Poll недействителен".to_string());
            }
        }

        if let Some(new_ts) = poll_response.ts {
            self.update_ts(new_ts);
        }

        let mut unified_messages = Vec::new();

        if let Some(updates) = poll_response.updates {
            for update in updates {
                if let Some(update_array) = update.as_array() {
                    if update_array.get(0).and_then(|v| v.as_u64()) == Some(4) {
                        let message_id = update_array.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
                        let flags = update_array.get(2).and_then(|v| v.as_u64()).unwrap_or(0);
                        let peer_id = update_array.get(3).and_then(|v| v.as_i64()).unwrap_or(0);
                        let timestamp = update_array.get(4).and_then(|v| v.as_u64()).unwrap_or(0);
                        let text = update_array.get(5).and_then(|v| v.as_str()).unwrap_or("").to_string();

                        let is_incoming = (flags & 2) == 0;
                        if !is_incoming {
                            continue;
                        }

                        let from_id = if let Some(extra) = update_array.get(6).and_then(|v| v.as_object()) {
                            extra.get("from_id").and_then(|v| v.as_i64()).unwrap_or(peer_id)
                        } else {
                            peer_id
                        };

                        if from_id > 0 {
                            unified_messages.push(UnifiedMessage {
                                platform: Platform::VK,
                                user_id: from_id.to_string(),
                                chat_id: peer_id.to_string(),
                                text: if text.is_empty() { "[Медиа]".to_string() } else { text },
                                timestamp,
                                message_id: Some(message_id.to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok((unified_messages, self.get_current_ts() as i64))
    }

    async fn send_message(&self, request: &SendMessageRequest) -> Result<(), String> {
        let url = "https://api.vk.com/method/messages.send";
        
        let peer_id = request.chat_id.parse::<i64>().map_err(|_| "Неверный формат chat_id для VK".to_string())?;
        
        let random_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let payload = VkSendRequest {
            access_token: VK_ACCESS_TOKEN,
            v: VK_API_VERSION,
            peer_id,
            message: &request.text,
            random_id,
        };

        let response_text = self.client.post_form(&url, &payload).await.map_err(|e| e.to_string())?;
        
        #[derive(Debug, Deserialize)]
        struct VkSendResponse {
            response: Option<i64>,
            error: Option<VkError>,
        }
        #[derive(Debug, Deserialize)]
        struct VkError {
            error_code: i32,
            error_msg: String,
        }

        let vk_response: VkSendResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Ошибка парсинга ответа VK: {} (Response: {})", e, response_text))?;

        if let Some(error) = vk_response.error {
            Err(format!("VK API ошибка {}: {}", error.error_code, error.error_msg))
        } else if vk_response.response.is_some() {
            Ok(())
        } else {
            Err(format!("Неожиданный ответ VK: {}", response_text))
        }
    }
}
