//! Под-модуль работы с проектами ЛЛМ сервиса.
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::text::Text;
use actix_web::web::{self, Data};
use actix_web::{HttpRequest, Responder};
use actix_web::{delete, get, post, put};
use llm::messages::*;
use reqwest::Method;
use reqwest::multipart::{Form, Part};
use std::collections::HashMap;
use std::sync::Arc;
use uzor_plugin::{AsaaData, PermissionReExtract};

use super::{BlankRequest, StatusResponse};
use crate::context::CoreCtx;
use crate::error::*;
use crate::http_server::to_response::IntoHttpResponse;
use crate::http_server::{check_project_permission, permitted_project};

// Константы путей запросов на сервис ЛЛМ.
pub(crate) const LLM_PROJECTS: &str = "/api/projects";
pub(crate) const LLM_PROJECT: &str = "/api/project";
const LLM_PROJECTS_ADAPTER: &str = "/api/projects/adapter";
const LLM_PROJECTS_KNOWLEDGE: &str = "/api/projects/knowledge";
const LLM_PROJECTS_DATABASE: &str = "/api/projects/dataset";
const LLM_PROJECTS_QUESTIONS: &str = "/api/projects/typical-questions";
const LLM_PROJECTS_TRAIN: &str = "/api/projects/train";
const LLM_PROJECTS_BUILD_INDEX: &str = "/api/projects/build-index";
const LLM_PROJECTS_RELOAD: &str = "/api/projects/reload";

/// ⚠️ Не пользоваться этим методом так как возникнут расхождения с AIOmni Core - он существует
///  чисто для тестирования запросов. Вместо него следует пользоваться
///  [POST /v1/admin_api/project](#post-project).
///
/// Удалить проект в базе данных AIOmni LLM. Идентификатор тот же что и в базе данных AIOmni Core.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = String))
)]
#[delete("/project/{project_id}")]
pub(super) async fn delete_project(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    delete_project_inner(req, project_id, data)
        .await
        .into_response()
}

/// Достать проект из базы данных AIOmni LLM по его идентификатору.
///  Идентификатор тот же что и в базе данных AIOmni Core.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = HashMap<i64, ProjectResponse>))
)]
#[get("/projects")]
pub(super) async fn get_projects(req: HttpRequest, data: Data<Arc<CoreCtx>>) -> impl Responder {
    tracing::info!("llm/get_projects");
    get_projects_inner(req, data).await.into_response()
}

/// ⚠️ Не пользоваться этим методом так как возникнут расхождения с AIOmni Core - он существует
/// чисто для тестирования запросов. Вместо него следует пользоваться
/// [POST /v1/admin_api/project](#post-project).
///
/// Создать проект в базе данных AIOmni LLM. Идентификатор тот же что и в базе данных AIOmni Core.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = ProjectResponse))
)]
#[post("/projects")]
pub(super) async fn post_project(
    req: HttpRequest,
    project: web::Json<CreateProjectRequest>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/post_project");
    post_project_inner(req, project, data).await.into_response()
}

/// ⚠️ Не пользоваться этим методом так как возникнут расхождения с AIOmni Core - он существует
///  чисто для тестирования запросов. Вместо него следует пользоваться
/// [PUT /v1/admin_api/project](#put-project).
///
/// Обновить проект в базе данных AIOmni LLM. Идентификатор тот же что и в базе данных AIOmni Core.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = ProjectResponse))
)]
#[put("/projects")]
pub(super) async fn put_project(
    req: HttpRequest,
    project: web::Json<UpdateProjectRequest>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/put_project");
    put_project_inner(req, project, data).await.into_response()
}

/// Этот АПИ не следует использовать сам по себе. Функциональность исполняется
/// внутри `GET /v1/admin_api/project/{id}`
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = ProjectResponse))
)]
#[get("/project/{project_id}")]
pub(super) async fn get_project(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    #[cfg(test)]
    println!("Requesting project from LLM: {project_id}");
    tracing::info!("Requesting project from LLM: {project_id}");

    get_project_inner(req, project_id, data)
        .await
        .into_response()
}

#[derive(Debug, MultipartForm, utoipa::ToSchema)]
struct ProjectAdaptorForm {
    #[schema(value_type = i64)]
    project_id: Text<i64>,
    #[schema(value_type = String, format = Binary, content_media_type = "application/octet-stream")]
    file: TempFile,
}

#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    request_body(content = ProjectAdaptorForm, content_type = "multipart/form-data"),
    responses((status = 200, body = UploadResponse))
)]
#[post("/projects/adaptor")]
pub(super) async fn post_projects_adaptor(
    req: HttpRequest,
    form: MultipartForm<ProjectAdaptorForm>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Adaptor project id: {:?}", form.project_id);
    post_projects_adaptor_inner(req, form, data)
        .await
        .into_response()
}

#[derive(Debug, MultipartForm, utoipa::ToSchema)]
struct PostKnowledgeMultiForm {
    #[schema(value_type = i64)]
    project_id: Text<i64>,
    #[schema(value_type = String)]
    column_question: Option<Text<String>>,
    #[schema(value_type = String)]
    column_answer: Option<Text<String>>,
    #[schema(value_type = String, format = Binary, content_media_type = "application/octet-stream")]
    file: TempFile,
}

/// Добавить список вопросов и ответов для обучения модели. Файл передаётся в СSV формате.
/// У файла два столбца. В первом вопросы, во втором соответствующие ответы.
/// В первом ряду наименование столбцов, которое должно соответствовать полям
/// заданными в полях форме `column_question` и `column_answer`.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    request_body(content = PostKnowledgeForm, content_type = "multipart/form-data"),
    responses((status = 200, body = KnowledgeResponse))
)]
#[post("/projects/knowledge")]
pub(super) async fn post_projects_knowledge(
    req: HttpRequest,
    form: MultipartForm<PostKnowledgeMultiForm>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/post_projects_knowledge");
    post_projects_knowledge_inner(req, form, data)
        .await
        .into_response()
}

type PostDatasetMultiForm = ProjectAdaptorForm;

/// Добавить набор данных для обучения модели в формате JSONL.
/// В каждой строке JSONL, сущность Messages соответствует формату OpenAI.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    request_body(content = ProjectAdaptorForm, content_type = "multipart/form-data"),
    responses((status = 200, body = UploadResponse))
)]
#[post("/projects/dataset")]
pub(super) async fn post_projects_dataset(
    req: HttpRequest,
    form: MultipartForm<PostDatasetMultiForm>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/post_projects_dataset");
    post_projects_dataset_inner(req, form, data)
        .await
        .into_response()
}

type PostQuestionMultiForm = ProjectAdaptorForm;

/// Добавить типичных вопросов для модели.
/// Вопросы передаются в текст формате. Каждый вопрос на новой строке.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    request_body(content = ProjectAdaptorForm, content_type = "multipart/form-data"),
    responses((status = 200, body = QuestionsResponse))
)]
#[post("/projects/questions")]
pub(super) async fn post_projects_questions(
    req: HttpRequest,
    form: MultipartForm<PostQuestionMultiForm>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    post_projects_questions_inner(req, form, data)
        .await
        .into_response()
}

/// Начать обучение ЛОРА адаптера для определённого проекта.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = TrainingResponse))
)]
#[post("/projects/train")]
pub(super) async fn post_projects_train(
    req: HttpRequest,
    train: web::Json<TrainingRequest>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    post_projects_train_inner(req, train, data)
        .await
        .into_response()
}

#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = BuildIndexResponse))
)]
#[post("/projects/build_index")]
pub(super) async fn post_projects_build_index(
    req: HttpRequest,
    map: web::Json<BuildIndexRequest>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("llm/post_projects_build_index");
    post_projects_build_index_inner(req, map, data)
        .await
        .into_response()
}

/// Перезагрузка проекта
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = ReloadProjectResponse))
)]
#[post("/projects/reload")]
pub(super) async fn post_projects_reload(
    req: HttpRequest,
    map: web::Json<ReloadProjectRequest>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    post_projects_reload_inner(req, map, data)
        .await
        .into_response()
}

async fn delete_project_inner(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> Result<StatusResponse> {
    // Гет данные проекта чтобы проверить есть ли доступ.
    let path = format!("{LLM_PROJECTS}?project_id={project_id}");
    check_project_permission(req, *project_id, &data).await?;

    data.llm()
        .raw()
        .delete(BlankRequest, &path)
        .await
        .map_err(Into::into)
}

async fn get_project_inner(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> Result<ProjectResponse> {
    // Гет данные проекта чтобы проверить есть ли доступ.
    let path = format!("{LLM_PROJECT}?project_id={project_id}");
    check_project_permission(req, *project_id, &data).await?;
    let r = ReloadProjectRequest {
        project_id: *project_id,
    };
    data.llm().raw().get(r, &path).await.map_err(Into::into)
}

async fn post_project_inner(
    req: HttpRequest,
    project: web::Json<CreateProjectRequest>,
    data: Data<Arc<CoreCtx>>,
) -> Result<ProjectResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    let asaa_data = AsaaData::from_final_request(&req)?;
    permitted_project(&asaa_data, &project.0.project_name)?;

    check_project_permission(req, project.0.project_id, &data).await?;

    data.llm()
        .raw()
        .post(project.0, LLM_PROJECTS)
        .await
        .map_err(Into::into)
}

async fn put_project_inner(
    req: HttpRequest,
    project: web::Json<UpdateProjectRequest>,
    data: Data<Arc<CoreCtx>>,
) -> Result<ProjectResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    if let Some(name) = project.project_name.as_ref() {
        let asaa_data = AsaaData::from_final_request(&req)?;
        permitted_project(&asaa_data, name)?;
    }
    check_project_permission(req, project.0.project_id, &data).await?;

    data.llm()
        .raw()
        .put(project.0, LLM_PROJECTS)
        .await
        .inspect_err(|e| {
            #[cfg(test)]
            println!("Error in `put_project_inner`: {e}");
            tracing::error!("Error in `put_project_inner`: {e}");
        })
        .map_err(Into::into)
}

async fn get_projects_inner(
    req: HttpRequest,
    data: Data<Arc<CoreCtx>>,
) -> Result<HashMap<String, ProjectResponse>> {
    let asaa_data = AsaaData::from_final_request(&req)?;

    let res: HashMap<String, ProjectResponse> =
        data.llm().raw().get(BlankRequest, LLM_PROJECTS).await?;

    let res = res
        .into_iter()
        .filter(|(_, x)| permitted_project(&asaa_data, &x.project_name).is_ok())
        .collect();

    Ok(res)
}

async fn post_projects_reload_inner(
    req: HttpRequest,
    map: web::Json<ReloadProjectRequest>,
    data: Data<Arc<CoreCtx>>,
) -> Result<ReloadProjectResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    check_project_permission(req, map.project_id, &data).await?;

    data.llm()
        .raw()
        .post(map.0, LLM_PROJECTS_RELOAD)
        .await
        .map_err(Into::into)
}

async fn post_projects_build_index_inner(
    req: HttpRequest,
    map: web::Json<BuildIndexRequest>,
    data: Data<Arc<CoreCtx>>,
) -> Result<BuildIndexResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    check_project_permission(req, map.project_id, &data).await?;

    data.llm()
        .raw()
        .post(map.0, LLM_PROJECTS_BUILD_INDEX)
        .await
        .map_err(Into::into)
}

async fn post_projects_adaptor_inner(
    req: HttpRequest,
    form: MultipartForm<ProjectAdaptorForm>,
    data: Data<Arc<CoreCtx>>,
) -> Result<UploadResponse> {
    tracing::info!("post_projects_adaptor_inner");
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    check_project_permission(req, *form.project_id, &data).await?;

    let form = Form::new()
        .text("project_id", form.project_id.to_string())
        .part("file", Part::file(form.file.file.path()).await?);

    data.llm()
        .raw()
        .send_payload(form, LLM_PROJECTS_ADAPTER, Method::POST)
        .await
        .map_err(Into::into)
}

async fn post_projects_questions_inner(
    req: HttpRequest,
    form: MultipartForm<PostQuestionMultiForm>,
    data: Data<Arc<CoreCtx>>,
) -> Result<QuestionsResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    check_project_permission(req, *form.project_id, &data).await?;

    let form = Form::new()
        .text("project_id", form.project_id.to_string())
        .part("file", Part::file(form.file.file.path()).await?);

    data.llm()
        .raw()
        .send_payload(form, LLM_PROJECTS_QUESTIONS, Method::POST)
        .await
        .map_err(Into::into)
}

async fn post_projects_dataset_inner(
    req: HttpRequest,
    form: MultipartForm<PostDatasetMultiForm>,
    data: Data<Arc<CoreCtx>>,
) -> Result<UploadResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    check_project_permission(req, *form.project_id, &data).await?;

    let form = Form::new()
        .text("project_id", form.project_id.to_string())
        .part("file", Part::file(form.file.file.path()).await?);

    data.llm()
        .raw()
        .send_payload(form, LLM_PROJECTS_DATABASE, Method::POST)
        .await
        .map_err(Into::into)
}

async fn post_projects_knowledge_inner(
    req: HttpRequest,
    form: MultipartForm<PostKnowledgeMultiForm>,
    data: Data<Arc<CoreCtx>>,
) -> Result<KnowledgeResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    tracing::info!("In post_projects_knowledge_inner");
    check_project_permission(req, *form.project_id, &data).await?;

    let column_answer = form
        .column_answer
        .as_ref()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "answers".to_string());
    let column_question = form
        .column_question
        .as_ref()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "questions".to_string());
    let form = Form::new()
        .text("project_id", form.project_id.to_string())
        .text("column_question", column_question)
        .text("column_answer", column_answer)
        .part("file", Part::file(form.file.file.path()).await?);

    data.llm()
        .raw()
        .send_payload(form, LLM_PROJECTS_KNOWLEDGE, Method::POST)
        .await
        .map_err(Into::into)
}

async fn post_projects_train_inner(
    req: HttpRequest,
    train: web::Json<TrainingRequest>,
    data: Data<Arc<CoreCtx>>,
) -> Result<TrainingResponse> {
    // Проверяем данные два раза, так как наименование и идентификатор оба должны
    // подходить.
    check_project_permission(req, train.0.project_id, &data).await?;

    data.llm()
        .raw()
        .post(train.0, LLM_PROJECTS_TRAIN)
        .await
        .map_err(Into::into)
}
