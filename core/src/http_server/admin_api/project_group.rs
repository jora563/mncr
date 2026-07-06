//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::Result;
use crate::http_server::to_response::IntoHttpResponse;

use actix_web::web::{self, Data};
use actix_web::{Responder, get, post};
use db::core_schema::*;
use serde::Deserialize;
use std::sync::Arc;

#[get("/project_groups")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_project_groups(data: Data<Arc<CoreCtx>>) -> impl Responder {
    tracing::info!("Fetching all project groups.");
    get_project_groups_inner(data.as_ref())
        .await
        .into_response()
}

#[post("/project_group/new")]
#[tracing::instrument(skip(data))]
pub(super) async fn post_new_project_group(
    proj_group: web::Json<IncomingNewProjectGroup>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming new project: {proj_group:?}");
    post_new_project_group_inner(proj_group.0, data.as_ref())
        .await
        .into_response()
}

#[post("/project_group/update")]
pub(super) async fn post_update_project_group(
    proj_group: web::Json<DbProjectGroup>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming project group for update: {proj_group:?}");
    post_update_project_group_inner(proj_group.0, data.as_ref())
        .await
        .into_response()
}
/// Достать все группы проектов в БД.
async fn get_project_groups_inner(ctx: &Arc<CoreCtx>) -> Result<Vec<DbProjectGroup>> {
    let pool = ctx.db().get();
    DbProjectGroup::get_all(pool).await.map_err(Into::into)
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
async fn post_update_project_group_inner(
    proj_group: DbProjectGroup,
    ctx: &Arc<CoreCtx>,
) -> Result<()> {
    let pool = ctx.db().get();
    proj_group.update(pool).await.map_err(Into::into)
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct IncomingNewProjectGroup {
    external_id: String,
    name: String,
}

impl IncomingNewProjectGroup {
    #[cfg(test)]
    pub(crate) fn new(external_id: &str, name: &str) -> Self {
        Self {
            external_id: external_id.to_string(),
            name: name.to_string(),
        }
    }

    fn into_new(self) -> DbNewProjectGroup {
        DbNewProjectGroup::new(self.external_id, self.name)
    }
}
