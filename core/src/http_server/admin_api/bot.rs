//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::Result;
use crate::http_server::to_response::IntoHttpResponse;
use crate::http_server::{permitted_project, permitted_projects};

use actix_web::HttpRequest;
use actix_web::http::StatusCode;
use actix_web::web::{self, Data};
use actix_web::{Responder, delete, get, post, put};
use db::core_schema::*;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;
use uzor_plugin::{AsaaData, PermissionReExtract};

#[get("/project/{project_id}/bots")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_bots_for_project(
    req: HttpRequest,
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("project id: {project_id}");
    get_bots_for_project_inner(req, *project_id, data.deref())
        .await
        .into_response()
}

#[delete("/bot/{bot_id}")]
#[tracing::instrument(skip(data))]
pub(super) async fn delete_bot(
    req: HttpRequest,
    bot_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("bot id: {bot_id}");
    delete_bot_inner(req, *bot_id, data)
        .await
        .into_response_with_code(StatusCode::OK)
}

#[get("/bot/{bot_id}")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_bot(
    req: HttpRequest,
    bot_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("bot id: {bot_id}");
    get_bot_inner(req, bot_id, data).await.into_response()
}

#[post("/bot")]
#[tracing::instrument(skip(data))]
pub(super) async fn post_new_bot_account(
    req: HttpRequest,
    bot: web::Json<IncomingNewBotAccount>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming new bot account: {bot:?}");
    post_new_bot_account_inner(req, bot.0, data.as_ref())
        .await
        .into_response_with_code(StatusCode::CREATED)
}

#[put("/bot")]
pub(super) async fn post_update_bot_account(
    req: HttpRequest,
    bot: web::Json<DbBotAccount>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming bot account for update: {bot:?}");
    post_update_bot_account_inner(req, bot.0, data.as_ref())
        .await
        .into_response()
}

async fn get_bots_for_project_inner(
    req: HttpRequest,
    project_id: i64,
    ctx: &Arc<CoreCtx>,
) -> Result<Vec<DbBotAccountWithMeta>> {
    // Проверь доступ к проекту.
    let asaa_data = AsaaData::from_final_request(&req)?;
    let project = DbProject::get_by_id(project_id, ctx.db().get()).await?;
    permitted_project(&asaa_data, &project.project_name)?;

    // Достать боты.
    DbBotAccountWithMeta::get_for_project(project_id, ctx.db().get())
        .await
        .map_err(Into::into)
}

#[tracing::instrument(skip(data))]
async fn get_bot_inner(
    req: HttpRequest,
    bot_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> Result<DbBotAccount> {
    let pool = data.db().get();
    let asaa_data = AsaaData::from_final_request(&req)?;
    let bot = DbBotAccount::get_by_id(*bot_id, pool).await?;

    // Проверить разрешение.
    let project = DbProject::get_by_id(bot.pkey(), pool).await?;
    permitted_project(&asaa_data, &project.project_name)?;

    Ok(bot)
}

#[tracing::instrument(skip(data))]
async fn delete_bot_inner(req: HttpRequest, bot_id: i64, data: Data<Arc<CoreCtx>>) -> Result<()> {
    let pool = data.db().get();
    let asaa_data = AsaaData::from_final_request(&req)?;
    let bot = DbBotAccount::get_by_id(bot_id, pool).await?;

    // Проверить разрешение.
    let project = DbProject::get_by_id(bot.pkey(), pool).await?;
    permitted_project(&asaa_data, &project.project_name)?;

    bot.delete(pool).await.map_err(Into::into)
}

/// Обновить новый бот в БД.
async fn post_update_bot_account_inner(
    req: HttpRequest,
    bot: DbBotAccount,
    ctx: &Arc<CoreCtx>,
) -> Result<()> {
    let pool = ctx.db().get();
    let asaa_data = AsaaData::from_final_request(&req)?;

    // Проверить разрешение.
    let project = DbProject::get_by_id(bot.pkey(), pool).await?;
    permitted_project(&asaa_data, &project.project_name)?;

    bot.update(pool).await.map_err(Into::into)
}

/// Добавить новый бот в БД.
async fn post_new_bot_account_inner(
    req: HttpRequest,
    new_bot: IncomingNewBotAccount,
    ctx: &Arc<CoreCtx>,
) -> Result<DbBotAccount> {
    let pool = ctx.db().get();
    let asaa_data = AsaaData::from_final_request(&req)?;

    let platform = DbPlatform::get_by_id(new_bot.platform_id, pool)
        .await
        .inspect_err(|e| {
            tracing::error!(
                "Platform with id \"{}\"for Bot cannot be retrieved: {e}.",
                new_bot.platform_id
            );
        })?;

    // Проверь доступ к проекту.
    let projects = moma::DbProjectPlatform::get_for_platform(platform.pkey(), pool).await?;

    let project = new_bot
        .project_id
        .and_then(|k| projects.iter().find(|p| p.pkey() == k));
    permitted_projects(asaa_data, &projects)?;

    new_bot
        .into_new(platform, project)
        .insert(pool)
        .await
        .map_err(Into::into)
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct IncomingNewBotAccount {
    platform_id: i64,
    project_id: Option<i64>,
    external_id: String,
    token: Vec<u8>,
    expiry_h: Option<i64>,
}

impl IncomingNewBotAccount {
    /// Функция чисто для тестирования
    #[cfg(test)]
    pub(crate) fn new(
        platform_id: i64,
        project_id: Option<i64>,
        external_id: &str,
        token: &[u8],
    ) -> Self {
        Self {
            platform_id,
            project_id,
            external_id: external_id.to_string(),
            token: token.to_vec(),
            expiry_h: None,
        }
    }

    fn into_new(self, plat: DbPlatform, proj: Option<&DbProject>) -> DbNewBotAccount {
        DbNewBotAccount::new(&plat, proj, self.external_id, self.expiry_h, self.token)
    }
}
