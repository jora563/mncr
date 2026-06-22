use chat::messengers::{TgCredentials, VkCredentials};
use chat::{InboundHandler, MessengerGateway, Platform, UnifiedMessage};

struct QueueProcessor {
    _gateway: MessengerGateway,
}

impl InboundHandler for QueueProcessor {
    async fn handle_inbound_message(&self, message: UnifiedMessage) {
        message.print();
    }
}

#[tokio::main]
async fn main() {
    let gateway = MessengerGateway::new();
    let processor = QueueProcessor {
        _gateway: gateway.clone(),
    };
    let vk_c = b"some-server-string::some-auth-token".to_vec();
    let tg_c = b"some:telegram-auth-token".to_vec();

    let vk_credentials = VkCredentials::from_bytes(&vk_c).unwrap();
    let tg_credentials = TgCredentials::from_bytes(&tg_c).unwrap();

    gateway
        .start_inbound_polling(processor, tg_credentials.clone(), vk_credentials.clone())
        .await;

    let _vk = gateway
        .send_text(Platform::VK, "", "Тест ВК", None, &vk_c)
        .await;

    let _tg = gateway
        .send_text(Platform::Telegram, "", "Тест ТГ", None, &tg_c)
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(100)).await;
}
