use serde::{Deserialize, Serialize};

use super::*;

/// Ответ модели на запрос /chat. Возвращается после обработки запроса.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatResponse {
    /// Флаг перевода на оператора.
    /// - `true` — запрос передан оператору (ответ не содержит полезной информации).
    /// - `false` — ответ сгенерирован AI.
    pub forward_to_operator: bool,
    /// Сгенерированный ответ модели или сообщение о переводе оператору.
    pub reply: String,
    /// Источник ответа:
    /// - "ai" — ответ сгенерирован моделью.
    /// - "operator" — принято решение перевести оператору (например, из-за
    ///   неуверенности или отсутствия контекста).
    pub source: String,
    /// Фрагменты из knowledge-базы, использованные для генерации (опционально, для отладки).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context_used: Vec<String>,
}

/// Полная информация о проекте.
///
/// Возвращается в ответ на GET /api/projects, GET /api/projects/?project_id=...
/// и после создания/обновления.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ProjectResponse {
    /// ЗАЧЕМ??
    pub id: i64,
    /// Числовой идентификатор проекта (заданный при создании).
    pub project_id: i64,
    /// Название проекта
    #[serde(rename = "name")]
    pub project_name: String,
    /// Путь к папке с LoRA-адаптером для этого проекта (если загружен).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_path: Option<String>,
    /// Путь к папке с knowledge-базой (CSV, FAISS индекс) для этого проекта.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_base_path: Option<String>,
    /// Путь к датасету обучения (JSONL) для этого проекта.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_path: Option<String>,
    /// Системный промпт для модели.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Сообщение при переводе оператору.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_message: Option<String>,
    /// Путь к файлу с типичными вопросами для построения центроида (классификатор темы).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typical_questions_path: Option<String>,
    /// Дата и время создания проекта (ISO 8601).
    pub created_at: String,
    /// Дата и время последнего обновления проекта (ISO 8601).
    pub updated_at: String,
}

/// Статус задачи обучения.
///
/// Возвращается на GET /api/training/jobs/{job_id}.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct TrainingResponse {
    /// Идентификатор задачи.
    pub job_id: String,
    /// Статус задачи. Всегда "started" при успешном запуске.
    pub status: String,
}

/// Статус задачи обучения.
///
/// Возвращается на GET /api/training/jobs/{job_id}.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct JobStatusResponse {
    /// Идентификатор задачи.
    pub job_id: String,
    /// ID проекта, для которого запущено обучение.
    pub project_id: i64,
    /// Текущий статус задачи:
    /// - "pending" — задача создана, ожидает выполнения.
    /// - "running" — обучение выполняется.
    /// - "completed" — обучение завершено успешно.
    /// - "failed" — обучение завершилось с ошибкой.
    /// - "interrupted" — прервано (например, из-за перезапуска сервера).
    pub status: String,
    /// Прогресс обучения (от 0.0 до 1.0).
    pub progress: f64,
    /// Текстовые логи обучения (последние строки или полный вывод).
    pub logs: String,
    /// Путь к сохранённому адаптеру, если обучение завершено успешно. Иначе None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_path: Option<String>,
}

/// Ответ на GET /health.
///
/// Используется для проверки состояния сервиса.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub service_status: String,
    /// Список project_id всех проектов, загруженных в системе.
    pub projects: Vec<i64>,
}

/// Общий ответ для операций без тела.
///
/// Используется, когда нужно вернуть только статус и опциональное сообщение.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct StatusResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub operation_status: String,
    /// Дополнительное сообщение (опционально).
    pub message: Option<String>,
}

/// Общий ответ для операций уплоад
///
/// Используется, когда нужно вернуть только статус и путь файла/адаптера
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UploadResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub operation_status: String,
    /// Дополнительное сообщение (опционально).
    pub path: String,
}

/// Общий ответ для операций загрузки знаний.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct KnowledgeResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub operation_status: String,
    /// Число добавленных записей.
    pub entries: i64,
    /// Путь индекса
    pub index_path: String,
}

/// Общий ответ для операций загрузки вопросов.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct QuestionsResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub operation_status: String,
    /// Число добавленных вопросов
    pub questions_count: i64,
}

/// Общий ответ для операций построить индекс.
///
/// Используется, когда нужно вернуть только статус и путь индекса.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct BuildIndexResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub operation_status: String,
    /// Дополнительное сообщение (опционально).
    pub index_path: String,
}

/// Общий ответ для операций перезагрузки проекта.
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ReloadProjectResponse {
    /// Статус операции (например, "ok", "reloaded", "deleted").
    #[serde(rename = "status")]
    pub operation_status: String,
    /// Дополнительное сообщение (опционально).
    pub message: Option<String>,
}

/// Стандартный ответ с ошибкой.
#[derive(Clone, Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(default)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}
