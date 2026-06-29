//! Модуль верификации пользователя.
//! Содержит логику проверки номера телефона и извлечения его из вложений.
//! Вся логика определения телефона из контакта находится здесь (в libs/chat).

use crate::models::{Attachment, UnifiedMessage};

/// Извлечь номер телефона из вложений сообщения (из контакта).
/// Если в сообщении есть контакт, возвращаем номер телефона из него.
pub fn extract_phone_from_message(msg: &UnifiedMessage) -> Option<String> {
    for attachment in &msg.attachments {
        if let Attachment::Contact { phone, .. } = attachment {
            return Some(phone.clone());
        }
    }
    None
}

/// Проверить, содержит ли сообщение вложение с контактом.
pub fn has_contact_attachment(msg: &UnifiedMessage) -> bool {
    msg.attachments
        .iter()
        .any(|a| matches!(a, Attachment::Contact { .. }))
}
