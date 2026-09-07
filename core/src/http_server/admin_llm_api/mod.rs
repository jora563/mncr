//! АПИ для взаимодействий с ЛЛМ сервисом.

#[derive(Debug, serde::Serialize)]
pub(super) struct BlankRequest;

/// Генерик ответ.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(super) struct StatusResponse {
    status: String,
}

pub(crate) mod llm_projects;
pub(crate) mod llm_training;

pub use llm_projects::*;
pub use llm_training::*;
