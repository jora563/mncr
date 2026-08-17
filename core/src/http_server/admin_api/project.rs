//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::Result;
use crate::http_server::permitted_project;
use crate::http_server::to_response::IntoHttpResponse;

use actix_web::HttpRequest;
use actix_web::http::StatusCode;
use actix_web::web::{self, Data};
use actix_web::{Responder, delete, get, post, put};
use db::core_schema::*;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;
use uzor_plugin::{AsaaData, PermissionReExtract, TokenData};

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

#[post("/project")]
#[tracing::instrument(skip(data))]
pub(super) async fn post_new_project(
    proj: web::Json<IncomingNewProject>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming new project: {proj:?}");
    post_new_project_inner(proj.0, data.as_ref())
        .await
        .into_response_with_code(StatusCode::CREATED)
}

#[put("/project")]
pub(super) async fn post_update_project(
    req: HttpRequest,
    proj: web::Json<DbProject>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming project for update: {proj:?}");
    post_update_project_inner(req, proj.0, data.as_ref())
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

async fn delete_project_inner(id: i64, req: HttpRequest, ctx: &Arc<CoreCtx>) -> Result<()> {
    let asaa_data = AsaaData::from_final_request(&req)?;
    let project = DbProject::get_by_id(id, ctx.db().get()).await?;

    permitted_project(&asaa_data, &project.project_name)?;

    project.delete(ctx.db().get()).await.map_err(Into::into)
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

/// Обновить проект в БД.
async fn post_update_project_inner(
    req: HttpRequest,
    mut proj: DbProject,
    ctx: &Arc<CoreCtx>,
) -> Result<()> {
    // Проверить аутентификацию
    let asaa_data = AsaaData::from_final_request(&req)?;
    permitted_project(&asaa_data, &proj.project_name)?;

    // Добавить того кто его изменил.
    proj.altered_by = TokenData::from_final_request(&req)?.personal_id;

    let pool = ctx.db().get();
    proj.update(pool).await.map_err(Into::into)
}

/// Добавить новый проект в БД.
async fn post_new_project_inner(
    new_proj: IncomingNewProject,
    ctx: &Arc<CoreCtx>,
) -> Result<DbProject> {
    let pool = ctx.db().get();
    let group = DbProjectGroup::get_by_id(new_proj.group_id, pool)
        .await
        .inspect_err(|e| {
            tracing::error!(
                "Group with id \"{}\"for project cannot be retrieved: {e}.",
                new_proj.group_id
            );
        })?;
    new_proj
        .into_new(group)
        .insert(pool)
        .await
        .map_err(Into::into)
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct IncomingNewProject {
    group_id: i64,
    external_id: String,
    name: String,
}

impl IncomingNewProject {
    #[cfg(test)]
    pub(crate) fn new(group_id: i64, external_id: &str, name: &str) -> Self {
        Self {
            group_id,
            external_id: external_id.to_string(),
            name: name.to_string(),
        }
    }

    fn into_new(self, group: DbProjectGroup) -> DbNewProject {
        DbNewProject::new(&group, self.external_id, self.name)
    }
}
