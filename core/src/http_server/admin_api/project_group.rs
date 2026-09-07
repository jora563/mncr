//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::{CoreError, Result};
use crate::http_server::permitted_projects;
use crate::http_server::to_response::IntoHttpResponse;

use actix_web::HttpRequest;
use actix_web::http::StatusCode;
use actix_web::web::{self, Data};
use actix_web::{Responder, delete, get, post, put};
use db::core_schema::*;
use serde::Deserialize;
use std::sync::Arc;
use uzor_plugin::{AsaaData, PermissionReExtract};

/// Достать все проектные группы. Работает как справочник.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = Vec<DbProjectGroup>)))]
#[get("/project_groups")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_project_groups(data: Data<Arc<CoreCtx>>) -> impl Responder {
    tracing::info!("Fetching all project groups.");
    get_project_groups_inner(data.as_ref())
        .await
        .into_response()
}

#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = String)))]
#[delete("/project_group/{id}")]
#[tracing::instrument(skip(data))]
pub(super) async fn delete_project_group(
    id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming request to delete project group: {id:?}");
    delete_project_group_inner(*id, data.as_ref())
        .await
        .into_response_with_code(StatusCode::OK)
}

/// Добавить новую группу проектов.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = DbProjectGroup)))]
#[post("/project_group")]
#[tracing::instrument(skip(data))]
pub(super) async fn post_new_project_group(
    proj_group: web::Json<IncomingNewProjectGroup>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming new project: {proj_group:?}");
    post_new_project_group_inner(proj_group.0, data.as_ref())
        .await
        .into_response_with_code(StatusCode::CREATED)
}

/// Обновить данные группы проектов.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 200, body = String)))]
#[put("/project_group")]
pub(super) async fn post_update_project_group(
    req: HttpRequest,
    proj_group: web::Json<DbProjectGroup>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming project group for update: {proj_group:?}");

    post_update_project_group_inner(req, proj_group.0, data.as_ref())
        .await
        .into_response()
}
/// Достать все группы проектов в БД.
async fn get_project_groups_inner(ctx: &Arc<CoreCtx>) -> Result<Vec<DbProjectGroup>> {
    DbProjectGroup::get_all(ctx.db().get())
        .await
        .map_err(Into::into)
}

/// Добавить новую группу проектов в БД.
async fn post_new_project_group_inner(
    new_bot: IncomingNewProjectGroup,
    ctx: &Arc<CoreCtx>,
) -> Result<DbProjectGroup> {
    let pool = ctx.db().get();
    new_bot.into_new().insert(pool).await.map_err(Into::into)
}

/// Обновит группу проектов в БД.
async fn delete_project_group_inner(id: i64, ctx: &Arc<CoreCtx>) -> Result<()> {
    let pool = ctx.db().get();
    // Проверь доступ к проекту.
    let group = DbFullProjectGroup::get_by_id(id, pool).await?;
    if !group.projects.is_empty() {
        return Err(CoreError::NoAccess(
            "deleting project group",
            group.group.group_name.to_string(),
        ));
    }
    group.group.delete(pool).await.map_err(Into::into)
}

/// Обновит группу проектов в БД.
async fn post_update_project_group_inner(
    req: HttpRequest,
    proj_group: DbProjectGroup,
    ctx: &Arc<CoreCtx>,
) -> Result<()> {
    let pool = ctx.db().get();

    let asaa_data = AsaaData::from_final_request(&req)?;
    // Проверь доступ к проекту.
    let group = DbFullProjectGroup::get_by_id(proj_group.pkey(), pool).await?;
    permitted_projects(asaa_data, &group.projects)?;

    proj_group.update(pool).await.map_err(Into::into)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct IncomingNewProjectGroup {
    name: String,
}

impl IncomingNewProjectGroup {
    #[cfg(test)]
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    fn into_new(self) -> DbNewProjectGroup {
        DbNewProjectGroup::new(self.name)
    }
}
