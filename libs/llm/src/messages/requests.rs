use serde::{Deserialize, Serialize};

use super::*;

/// Запрос на генерацию ответа от модели.
///
/// Отправляется на эндпоинт POST /chat.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ChatRequest {
    /// Идентификатор проекта, для которого генерируется ответ.
    pub project_id: i64,
    /// Текущее сообщение пользователя. Максимальная длина: 4096 символов.
    pub message: String,
    /// Список предыдущих сообщений в диалоге (до 10–15 сообщений).
    /// Если не передан, используется только текущее сообщение.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<HistoryMessage>,
    /// Идентификатор сессии (если история хранится на стороне сервера).
    ///  Приоритет ниже, чем history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl ChatRequest {
    pub fn with_proj(project_id: i64) -> Self {
        Self {
            project_id,
            ..Default::default()
        }
    }

    pub fn add_assistant_history(&mut self, content: &str) {
        self.add_history("assistant", content);
    }

    pub fn add_user_history(&mut self, content: &str) {
        self.add_history("user", content);
    }

    fn add_history(&mut self, role: &str, content: &str) {
        self.history.push(HistoryMessage {
            content: content.to_owned(),
            role: role.to_owned(),
        });
    }
}

/// Запрос на создание нового проекта.
///
/// Отправляется на эндпоинт POST /api/projects.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateProjectRequest {
    /// Уникальный числовой идентификатор проекта (должен быть уникальным).
    pub project_id: i64,
    /// Название проекта (например, "Банк", "Магазин").
    #[serde(rename = "name")]
    pub project_name: String,
    /// Системный промпт, задающий роль и стиль поведения модели.
    /// По умолчанию: "Ты — полезный ассистент.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Сообщение, которое будет показано клиенту при переводе оператору.
    /// По умолчанию: "К сожалению, я не могу ответить на этот вопрос. Ваш запрос передан оператору.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_message: Option<String>,
}

/// Запрос на перезагрузку проекта
///
/// Отправляется на эндпоинт POST /api/projects/reload
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ReloadProjectRequest {
    /// Уникальный числовой идентификатор проекта (должен быть уникальным).
    pub project_id: i64,
}

/// Запрос на перезагрузку индекса
///
/// Отправляется на эндпоинт POST /api/projects/build-index
pub type BuildIndexRequest = ReloadProjectRequest;

/// Запрос на обновление существующего проекта.
///
/// Отправляется на эндпоинт PUT /api/projects.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateProjectRequest {
    /// Уникальный числовой идентификатор проекта (должен быть уникальным).
    pub project_id: i64,
    /// Название проекта (например, "Банк", "Магазин").
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Системный промпт, задающий роль и стиль поведения модели.
    /// По умолчанию: "Ты — полезный ассистент.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Сообщение, которое будет показано клиенту при переводе оператору.
    /// По умолчанию: "К сожалению, я не могу ответить на этот вопрос. Ваш запрос передан оператору.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_message: Option<String>,
}

/// Запрос на запуск асинхронного обучения LoRA-адаптера.
///
/// Отправляется на эндпоинт POST /api/projects/train.
/// Все поля, кроме project_id, опциональны и переопределяют значения по умолчанию из Config.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TrainingRequest {
    /// Идентификатор проекта, для которого запускается обучение.
    pub project_id: i64,
    /// Количество эпох обучения. По умолчанию: 3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<i64>,
    /// Скорость обучения. По умолчанию: 2e-4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate: Option<f64>,
    /// Размер батча. Рекомендуется 1 для экономии памяти на CPU.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<i64>,
    /// Ранг LoRA. Меньшее значение (4–8) экономит память. По умолчанию: 8.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lora_r: Option<i64>,
    /// Коэффициент масштабирования LoRA. Обычно в 2–4 раза больше r. По умолчанию: 16.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lora_alpha: Option<i64>,
}

/// Часть запроса на PostAdaptor (POST /api/projects/adapter)
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct PostAdapterForm {
    /// Идентификатор проекта, для которого запускается обучение.
    pub project_id: i64,
}

/// Alias так как они одинаковы. (POST /api/projects/dataset)
pub type PostDatasetForm = PostAdapterForm;
/// Alias так как они одинаковы. (POST /api/projects/typical-questions)
pub type PostQuestionForm = PostAdapterForm;

/// Часть запроса на Post Knowledge. (POST /api/projects/knowledge)
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct PostKnowledgeForm {
    /// Идентификатор проекта, для которого запускается обучение.
    pub project_id: i64,
    /// Вопрос
    pub column_question: String,
    /// Ответ
    pub column_answer: String,
}
