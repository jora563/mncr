//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::Result;
use crate::http_server::to_response::IntoHttpResponse;

use actix_web::http::StatusCode;
use actix_web::web::{self, Data};
use actix_web::{Responder, get, post, put};
use db::core_schema::*;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;

#[get("/project_group/{group_id}/projects")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_projects(
    group_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Group id: {group_id}");
    get_projects_inner(*group_id, data.deref())
        .await
        .into_response()
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
    proj: web::Json<DbProject>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming project for update: {proj:?}");
    post_update_project_inner(proj.0, data.as_ref())
        .await
        .into_response()
}

async fn get_projects_inner(group_id: i64, ctx: &Arc<CoreCtx>) -> Result<DbFullProjectGroup> {
    DbFullProjectGroup::get_by_id(group_id, ctx.db().get())
        .await
        .map_err(Into::into)
}

/// Обновить проект в БД.
async fn post_update_project_inner(proj: DbProject, ctx: &Arc<CoreCtx>) -> Result<()> {
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
