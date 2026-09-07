//! Под-модуль АПИ тренировки модели ЛЛМ.
use actix_web::web::{self, Data};
use actix_web::{HttpRequest, Responder};
use actix_web::{get, post};
use llm::messages::{JobStatusResponse, TrainingResponse};
use std::sync::Arc;

use super::BlankRequest;
use crate::context::CoreCtx;
use crate::error::Result;
use crate::http_server::check_project_permission;
use crate::http_server::to_response::IntoHttpResponse;

// Константы путей запросов на сервис ЛЛМ.
const LLM_TRAINING_RESUME: &str = "/api/training/resume";
const LLM_TRAINING_JOBS: &str = "/api/training/jobs";
const LLM_TRAINING_JOBS_BY_PROJECT: &str = "/api/training/jobs/by-project";

/// Запрос на продолжение обучения адаптера.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = String))
)]
#[post("/training/resume/{job_uuid}")]
pub(super) async fn post_training_resume(
    job_uuid: web::Path<String>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/training/resume");
    let path = format!("{LLM_TRAINING_RESUME}/{job_uuid}");
    data.llm()
        .raw()
        .post::<_, TrainingResponse>(BlankRequest, &path)
        .await
        .map_err(Into::into)
        .into_response()
}

#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = TrainingResponse))
)]
#[get("/training/job/{job_uuid}")]
pub(super) async fn get_training_job(
    job_uuid: web::Path<String>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/training/job");
    let path = format!("{LLM_TRAINING_JOBS}/{job_uuid}");
    data.llm()
        .raw()
        .get::<_, JobStatusResponse>(BlankRequest, &path)
        .await
        .map_err(Into::into)
        .into_response()
}

#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = TrainingResponse))
)]
#[get("/training/jobs_by_project/{project_id}")]
pub(super) async fn get_training_jobs_by_project(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/training/jobs_by_project");
    get_training_job_by_project_inner(req, project_id, data)
        .await
        .into_response()
}

async fn get_training_job_by_project_inner(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> Result<Vec<TrainingResponse>> {
    // Гет данные проекта чтобы проверить есть ли доступ.
    let path = format!("{LLM_TRAINING_JOBS_BY_PROJECT}/{project_id}");
    check_project_permission(req, *project_id, &data).await?;

    data.llm()
        .raw()
        .get(BlankRequest, &path)
        .await
        .map_err(Into::into)
}
