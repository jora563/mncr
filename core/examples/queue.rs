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
    let processor = QueueProcessor { _gateway: gateway.clone() };

    gateway.start_inbound_polling(processor).await;

    let _vk = gateway.send_text(
        Platform::VK, 
        "", 
        "Тест ВК", 
        None
    ).await;

    let _tg = gateway.send_text(
        Platform::Telegram, 
        "", 
        "Тест ТГ", 
        None
    ).await;

    tokio::time::sleep(std::time::Duration::from_secs(100)).await;
}
