//! Модуль верификации пользователя.
//! Содержит логику извлечения данных из вложений.

use crate::models::{Attachment, UnifiedMessage};

/// Извлечь номер телефона из вложения-контакта
pub fn extract_phone(msg: &UnifiedMessage) -> Option<String> {
    msg.attachments.iter().find_map(|a| match a {
        Attachment::Contact { phone, .. } => Some(phone.clone()),
        _ => None,
    })
}

/// Извлечь имя пользователя из вложения-контакта
pub fn extract_name(msg: &UnifiedMessage) -> Option<String> {
    msg.attachments.iter().find_map(|a| match a {
        Attachment::Contact {
            first_name,
            last_name,
            ..
        } => {
            let mut name = first_name.clone();
            if let Some(last) = last_name {
                name.push(' ');
                name.push_str(last);
            }
            Some(name)
        }
        _ => None,
    })
}

/// Извлечь user_id из вложения-контакта (если контакт соответствует пользователю)
pub fn extract_contact_user_id(msg: &UnifiedMessage) -> Option<String> {
    msg.attachments.iter().find_map(|a| match a {
        Attachment::Contact { user_id, .. } => user_id.clone(),
        _ => None,
    })
}

/// Проверить, содержит ли сообщение только контакт (без осмысленного текста)
pub fn is_contact_only(msg: &UnifiedMessage) -> bool {
    let has_contact = msg
        .attachments
        .iter()
        .any(|a| matches!(a, Attachment::Contact { .. }));
    let is_media = msg.text.is_empty() || msg.text == "[Медиа]";
    has_contact && is_media
}
