use chat::messengers::telegram::TelegramMessenger;
use chat::messengers::vk::VkMessenger;
use chat::messengers::Messenger;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // --- TELEGRAM ---
    let tg_messenger = TelegramMessenger::new();

    let tg_handle = tokio::spawn(async move {
        let mut tg_offset: i64 = 0;
        println!("[TG] Polling...");
        
        loop {
            match tg_messenger.fetch_messages(tg_offset).await {
                Ok((messages, new_offset)) => {
                    if !messages.is_empty() {
                        for msg in messages {
                            msg.print();
                        }
                    }
                    
                    if new_offset > tg_offset {
                        tg_offset = new_offset;
                    }
                }
                Err(e) => {
                    eprintln!("[TG] {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    // --- VK ---
    let vk_messenger = VkMessenger::new();
    let vk_handle = tokio::spawn(async move {
        let mut vk_offset: i64 = 0;
        println!("[VK] Polling...");
        
        loop {
            match vk_messenger.fetch_messages(vk_offset).await {
                Ok((messages, new_offset)) => {
                    if !messages.is_empty() {
                        for msg in messages {
                            msg.print();
                        }
                    }
                    if new_offset > vk_offset {
                        vk_offset = new_offset;
                    }
                }
                Err(e) => {
                    eprintln!("[VK] {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    let _ = tokio::join!(tg_handle, vk_handle);
}
