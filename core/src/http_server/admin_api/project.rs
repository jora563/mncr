//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::*;
use crate::http_server::to_response::IntoHttpResponse;
use crate::http_server::{check_project_permission, permitted_project};

use actix_web::HttpRequest;
use actix_web::http::StatusCode;
use actix_web::web::{self, Data};
use actix_web::{Responder, delete, get, post, put};
use db::core_schema::moma::*;
use db::core_schema::*;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;
use uzor_plugin::{AsaaData, PermissionReExtract, TokenData};

#[cfg(feature = "test-aiomni-llm")]
use crate::http_server::admin_llm_api::llm_projects::LLM_PROJECTS;
#[cfg(feature = "test-aiomni-llm")]
use crate::http_server::admin_llm_api::{BlankRequest, StatusResponse};
#[cfg(feature = "test-aiomni-llm")]
use llm::messages::{CreateProjectRequest, ProjectResponse, UpdateProjectRequest};

/// Достать все проекты по идентификатору их проектной группы.
#[utoipa::path(
    responses((status = 200, body = Vec<DbFullProjectGroup>)),
    params(("Authorization" = String, Header, description = "Bearer + JWT token"))
)]
#[get("/project_group/{group_id}/projects")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_projects(
    req: HttpRequest,
    group_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Group id: {group_id}");
    get_projects_inner(req, *group_id, data.deref())
        .await
        .into_response()
}

/// Достать все проекты которые дозволены пользователю.
#[utoipa::path(
    responses((status = 200, body = Vec<DbProject>)),
    params(("Authorization" = String, Header, description = "Bearer + JWT token"))
)]
#[get("/projects")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_permitted_projects(
    req: HttpRequest,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Get all permitted projects");
    get_permitted_projects_inner(req, data.deref())
        .await
        .into_response()
}

/// Удалить проект по его идентификатору
#[utoipa::path(
    responses((status = 200, body = String)),
    params(("Authorization" = String, Header, description = "Bearer + JWT token"))
)]
#[delete("/project/{id}")]
#[tracing::instrument(skip(data))]
pub(super) async fn delete_project(
    id: web::Path<i64>,
    req: HttpRequest,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Request to delete project: {id:?}");
    delete_project_inner(*id, req, data.as_ref())
        .await
        .into_response_with_code(StatusCode::OK)
}

/// Добавить новый проект.
#[utoipa::path(
    responses((status = 200, body = DbProject)),
    params(("Authorization" = String, Header, description = "Bearer + JWT token"))
)]
#[post("/project")]
#[tracing::instrument(skip(data))]
pub(super) async fn post_new_project(
    proj: web::Json<IncomingNewProject>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming new project: {proj:?}");
    new_project_inner(proj.0, data.as_ref())
        .await
        .into_response_with_code(StatusCode::CREATED)
}

/// Обновить данные проекта.
#[utoipa::path(
    responses((status = 200, body = DbProject)),
    params(("Authorization" = String, Header, description = "Bearer + JWT token"))
)]
#[put("/project")]
pub(super) async fn post_update_project(
    req: HttpRequest,
    proj: web::Json<ApiProject>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming project for update: {proj:?}");
    update_project_inner(req, proj.0, data.as_ref())
        .await
        .into_response()
}

async fn get_projects_inner(
    req: HttpRequest,
    group_id: i64,
    ctx: &Arc<CoreCtx>,
) -> Result<DbFullProjectGroup> {
    let asaa_data = AsaaData::from_final_request(&req)?;
    let mut group = DbFullProjectGroup::get_by_id(group_id, ctx.db().get()).await?;
    group.projects = group
        .projects
        .into_iter()
        .filter(|p| permitted_project(&asaa_data, &p.project_name).is_ok())
        .collect::<Vec<_>>();

    Ok(group)
}

async fn get_permitted_projects_inner(
    req: HttpRequest,
    ctx: &Arc<CoreCtx>,
) -> Result<Vec<DbProject>> {
    let asaa_data = AsaaData::from_final_request(&req)?;
    let projects: Vec<_> = asaa_data.projects.iter().map(|p| &p.name as &str).collect();
    let projects = DbProject::get_by_names(&projects, ctx.db().get()).await?;

    Ok(projects)
}

async fn delete_project_inner(id: i64, req: HttpRequest, ctx: &Arc<CoreCtx>) -> Result<()> {
    let asaa_data = AsaaData::from_final_request(&req)?;
    let project = DbProject::get_by_id(id, ctx.db().get()).await?;

    permitted_project(&asaa_data, &project.project_name)?;

    // Начать локальный процесс удаления.
    let pool = ctx.db().get();
    let mut tr = pool.begin().await?;
    let links = DbProjectPlatform::get_for_project(id, &mut *tr).await?;
    // Удалить связи, и удалить проект. Если у проекта есть боты и пользователи, то
    // их ПОКА что надо удалять вручную.
    for platform in links.into_iter() {
        DbProjectPlatform::un_link(&project, &platform, &mut *tr).await?;
    }
    project.delete(&mut *tr).await?;
    // Удалить проект из ЛЛМ БД.
    #[cfg(feature = "test-aiomni-llm")]
    {
        let llm_path = format!("{LLM_PROJECTS}?project_id={id}");
        // If project did not exist on the LLM, we do not consider that an error and let
        // it slide.
        match ctx
            .llm()
            .raw()
            .delete::<_, StatusResponse>(BlankRequest, &llm_path)
            .await
        {
            Ok(_) => {}
            Err(llm::error::LlmError::AiOmniLlm(d)) if &d.code == "PROJECT_NOT_FOUND" => {}
            Err(e) => return Err(e.into()),
        };
    }
    tr.commit().await?;
    Ok(())
}

/// Обновить проект в БД.
async fn update_project_inner(
    req: HttpRequest,
    mut proj: ApiProject,
    ctx: &Arc<CoreCtx>,
) -> Result<()> {
    // Проверить аутентификацию
    let db_proj = &mut proj.db_project;
    // Изначальная проверка по старому проекту.
    let token_data = TokenData::from_final_request(&req)?;
    let (_, asaa_data) = check_project_permission(req, db_proj.pkey(), ctx).await?;
    // Проверка что наименование проекта не переходит на запрещённое.
    permitted_project(&asaa_data, &db_proj.project_name)?;

    // Добавить того кто его изменил.
    db_proj.altered_by = token_data.personal_id;

    let pool = ctx.db().get();
    let mut tr = pool.begin().await?;
    db_proj.update(&mut *tr).await?;

    #[cfg(feature = "test-aiomni-llm")]
    {
        let req = UpdateProjectRequest {
            project_id: db_proj.pkey(),
            project_name: Some(db_proj.project_name.to_owned()),
            system_prompt: proj.system_prompt,
            fallback_message: proj.fallback_message,
        };
        ctx.llm()
            .raw()
            .put::<_, ProjectResponse>(req, LLM_PROJECTS)
            .await?;
    }
    tr.commit().await?;
    Ok(())
}

/// Добавить новый проект в БД.
async fn new_project_inner(new_proj: IncomingNewProject, ctx: &Arc<CoreCtx>) -> Result<DbProject> {
    println!("IN `post_new_project_inner`");
    let pool = ctx.db().get();
    let group = DbProjectGroup::get_by_id(new_proj.group_id, pool)
        .await
        .inspect_err(|e| {
            tracing::error!(
                "Group with id \"{}\"for project cannot be retrieved: {e}.",
                new_proj.group_id
            );
        })?;

    let pool = ctx.db().get();
    let mut tr = pool.begin().await?;
    let proj = new_proj.to_new(group).insert(&mut *tr).await?;

    #[cfg(feature = "test-aiomni-llm")]
    {
        println!("Preparing to post new project to LLM");
        let req = CreateProjectRequest {
            project_id: proj.pkey(),
            project_name: proj.project_name.to_owned(),
            system_prompt: new_proj.system_prompt,
            fallback_message: new_proj.fallback_message,
        };
        ctx.llm()
            .raw()
            .post::<_, ProjectResponse>(req, LLM_PROJECTS)
            .await
            .inspect_err(|e| println!("Err in post llm project: {e}"))?;
    }
    tr.commit().await?;
    Ok(proj)
}

/// Общая сущность ввода для обновления проекта в Core и LLM.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub struct ApiProject {
    /// Сам проект
    #[serde(flatten)]
    pub db_project: DbProject,
    /// Системный промпт, задающий роль и стиль поведения модели.
    /// По умолчанию: "Ты — полезный ассистент.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub system_prompt: Option<String>,
    /// Сообщение, которое будет показано клиенту при переводе оператору.
    /// По умолчанию: "К сожалению, я не могу ответить на этот вопрос. Ваш запрос передан оператору.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub fallback_message: Option<String>,
}

/// Общая сущность создания для обновления проекта в Core и LLM.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct IncomingNewProject {
    group_id: i64,
    external_id: String,
    name: String,
    /// Системный промпт, задающий роль и стиль поведения модели.
    /// По умолчанию: "Ты — полезный ассистент.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub system_prompt: Option<String>,
    /// Сообщение, которое будет показано клиенту при переводе оператору.
    /// По умолчанию: "К сожалению, я не могу ответить на этот вопрос. Ваш запрос передан оператору.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub fallback_message: Option<String>,
}

impl IncomingNewProject {
    #[cfg(test)]
    pub(crate) fn new(group_id: i64, external_id: &str, name: &str) -> Self {
        Self {
            group_id,
            external_id: external_id.to_string(),
            name: name.to_string(),
            system_prompt: None,
            fallback_message: None,
        }
    }

    fn to_new(&self, group: DbProjectGroup) -> DbNewProject {
        DbNewProject::new(&group, &self.external_id, &self.name)
    }
}
