use chat::messengers::telegram::TelegramMessenger;
use chat::messengers::vk::VkMessenger;
use chat::messengers::Messenger;
use chat::models::SendMessageRequest;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // --- TELEGRAM ---
    let tg_messenger = TelegramMessenger::new();
    let _ = tg_messenger.ensure_polling_mode().await;

    let tg_handle = tokio::spawn(async move {
        let mut tg_offset: i64 = 0;
        println!("[TG] Запуск polling задачи...");
        
        loop {
            match tg_messenger.fetch_messages(tg_offset).await {
                Ok((messages, new_offset)) => {
                    for msg in messages {
                        msg.print();
                        
                        let reply_request = SendMessageRequest {
                            chat_id: msg.chat_id.clone(),
                            text: format!("Эхо (TG): {}", msg.text),
                            reply_to_message_id: msg.message_id,
                        };
                        
                        println!("[TG] Отправка ответа...");
                        match tg_messenger.send_message(&reply_request).await {
                            Ok(_) => println!("[TG] Ответ успешно отправлен"),
                            Err(e) => eprintln!("[TG] Ошибка отправки: {}", e),
                        }
                    }
                    if new_offset > tg_offset {
                        tg_offset = new_offset;
                    }
                }
                Err(e) => {
                    eprintln!("[TG FATAL] {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    // --- VK ---
    let vk_messenger = VkMessenger::new();
    let vk_handle = tokio::spawn(async move {
        let mut vk_offset: i64 = 0;
        println!("[VK] Запуск polling задачи...");
        
        loop {
            match vk_messenger.fetch_messages(vk_offset).await {
                Ok((messages, new_offset)) => {
                    for msg in messages {
                        msg.print();
                        
                        let reply_request = SendMessageRequest {
                            chat_id: msg.chat_id.clone(),
                            text: format!("Эхо (VK): {}", msg.text),
                            reply_to_message_id: None, 
                        };
                        
                        println!("[VK] Отправка ответа...");
                        match vk_messenger.send_message(&reply_request).await {
                            Ok(_) => println!("[VK] Ответ успешно отправлен"),
                            Err(e) => eprintln!("[VK] Ошибка отправки: {}", e),
                        }
                    }
                    if new_offset > vk_offset {
                        vk_offset = new_offset;
                    }
                }
                Err(e) => {
                    eprintln!("[VK FATAL] {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    let _ = tokio::join!(tg_handle, vk_handle);
}
