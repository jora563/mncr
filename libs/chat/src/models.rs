use serde::{Deserialize, Serialize};

/// Перечисление поддерживаемых платформ (мессенджеров).
/// Используется для маршрутизации сообщений и определения типа бота.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Platform {
    Telegram,
    VK,
    Max,
}

/// Вложения в сообщении (медиафайлы, контакты и т.д.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Attachment {
    /// Контакт пользователя.
    Contact {
        /// Номер телефона из контакта.
        phone: String,
        /// Имя из контакта.
        first_name: String,
        /// Фамилия из контакта (если есть).
        last_name: Option<String>,
        /// Внешний ID пользователя в мессенджере, к которому привязан контакт.
        /// Используется для проверки того, что пользователь поделился именно
        /// своим собственным контактом, а не чужим.
        user_id: Option<String>,
    },
    /// Фотография.
    Photo {
        /// Идентификатор файла на стороне мессенджера.
        file_id: String,
        /// Прямая ссылка на файл (если доступна).
        #[serde(skip_serializing_if = "Option::is_none")]
        file_url: Option<String>,
        /// Размер файла в байтах.
        #[serde(skip_serializing_if = "Option::is_none")]
        file_size: Option<i64>,
    },
    /// Документ или произвольный файл.
    Document {
        /// Идентификатор файла на стороне мессенджера.
        file_id: String,
        /// Прямая ссылка на файл (если доступна).
        #[serde(skip_serializing_if = "Option::is_none")]
        file_url: Option<String>,
        /// Размер файла в байтах.
        #[serde(skip_serializing_if = "Option::is_none")]
        file_size: Option<i64>,
        /// Оригинальное имя файла.
        #[serde(skip_serializing_if = "Option::is_none")]
        file_name: Option<String>,
    },
}

/// Унифицированное представление входящего сообщения от любой платформы.
/// Позволяет бизнес-логике работать с сообщениями независимо от источника.
#[derive(Debug, Serialize, Clone)]
pub struct UnifiedMessage {
    /// Платформа, с которой пришло сообщение.
    pub platform: Platform,
    /// Внешний идентификатор пользователя (отправителя) в мессенджере.
    pub user_id: String,
    /// Внешний идентификатор чата.
    pub chat_id: String,
    /// Текстовое содержимое сообщения (или "[Медиа]", если текста нет).
    pub text: String,
    /// Временная метка получения сообщения (Unix timestamp).
    pub timestamp: u64,
    /// Внешний идентификатор сообщения (для ответов и привязки к тикетам).
    pub message_id: Option<String>,
    /// Список вложений (медиа, контакты).
    pub attachments: Vec<Attachment>,
}

impl UnifiedMessage {
    /// Выводит сообщение в консоль в формате JSON для отладки.
    pub fn print(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Ошибка сериализации UnifiedMessage: {}", e),
        }
    }
}

/// Разметка для ответа бота (клавиатуры, инлайн-кнопки).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReplyMarkup {
    /// Разметка, специфичная для Telegram.
    Telegram(TelegramKeyboard),
}

/// Конфигурация клавиатуры для Telegram.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelegramKeyboard {
    /// Показать клавиатуру с заданными кнопками.
    Show {
        /// Двумерный массив кнопок (строки и столбцы).
        keyboard: Vec<Vec<TelegramKeyboardButton>>,
        /// Изменить размер клавиатуры под размер экрана.
        #[serde(skip_serializing_if = "Option::is_none")]
        resize_keyboard: Option<bool>,
        /// Скрыть клавиатуру после первого нажатия.
        #[serde(skip_serializing_if = "Option::is_none")]
        one_time_keyboard: Option<bool>,
    },
    /// Скрыть или удалить текущую клавиатуру.
    Remove { remove_keyboard: bool },
}

/// Отдельная кнопка клавиатуры Telegram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramKeyboardButton {
    /// Текст, отображаемый на кнопке.
    pub text: String,
    /// Если true, при нажатии бот получит контакт пользователя
    /// (требует подтверждения от пользователя в клиенте Telegram).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_contact: Option<bool>,
}

impl TelegramKeyboard {
    /// Создаёт разметку с одной кнопкой для запроса контакта пользователя.
    pub fn request_contact() -> Self {
        Self::Show {
            keyboard: vec![vec![TelegramKeyboardButton {
                text: "📱 Поделиться номером телефона".to_string(),
                request_contact: Some(true),
            }]],
            resize_keyboard: Some(true),
            one_time_keyboard: Some(true),
        }
    }

    /// Создаёт разметку для удаления клавиатуры.
    pub fn remove() -> Self {
        Self::Remove {
            remove_keyboard: true,
        }
    }
}

/// Запрос на отправку сообщения через шлюз мессенджеров (MessengerGateway).
#[derive(Debug, Clone)]
pub struct SendMessageRequest {
    /// Внешний идентификатор чата, куда отправляется сообщение.
    pub chat_id: String,
    /// Текст сообщения.
    pub text: String,
    /// Идентификатор сообщения, на которое дается ответ (для цитирования/reply).
    pub reply_to_message_id: Option<String>,
    /// Дополнительная разметка (например, клавиатура).
    pub reply_markup: Option<ReplyMarkup>,
}
