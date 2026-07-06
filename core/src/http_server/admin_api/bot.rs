//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::Result;
use crate::http_server::to_response::IntoHttpResponse;

use actix_web::web::{self, Data};
use actix_web::{Responder, get, post};
use db::core_schema::*;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;

#[get("/bots/{project_id}")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_bots_for_project(
    project_id: web::Path<i64>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("project id: {project_id}");
    get_bots_for_project_inner(*project_id, data.deref())
        .await
        .into_response()
}
#[get("/bot/{bot_id}")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_bot(bot_id: web::Path<i64>, data: Data<Arc<CoreCtx>>) -> impl Responder {
    tracing::info!("bot id: {bot_id}");
    DbBotAccount::get_by_id(*bot_id, data.db().get())
        .await
        .map_err(Into::into)
        .into_response()
}

#[post("/bot/new")]
#[tracing::instrument(skip(data))]
pub(super) async fn post_new_bot_account(
    bot: web::Json<IncomingNewBotAccount>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming new bot account: {bot:?}");
    post_new_bot_account_inner(bot.0, data.as_ref())
        .await
        .into_response()
}

#[post("/bot/update")]
pub(super) async fn post_update_bot_account(
    bot: web::Json<DbBotAccount>,
    data: Data<Arc<CoreCtx>>,
) -> impl Responder {
    tracing::info!("Incoming bot account for update: {bot:?}");
    post_update_bot_account_inner(bot.0, data.as_ref())
        .await
        .into_response()
}

async fn get_bots_for_project_inner(
    project_id: i64,
    ctx: &Arc<CoreCtx>,
) -> Result<Vec<DbBotAccountWithMeta>> {
    DbBotAccountWithMeta::get_for_project(project_id, ctx.db().get())
        .await
        .map_err(Into::into)
}

/// Обновить новый бот в БД.
async fn post_update_bot_account_inner(bot: DbBotAccount, ctx: &Arc<CoreCtx>) -> Result<()> {
    let pool = ctx.db().get();
    bot.update(pool).await.map_err(Into::into)
}

/// Добавить новый бот в БД.
async fn post_new_bot_account_inner(
    new_bot: IncomingNewBotAccount,
    ctx: &Arc<CoreCtx>,
) -> Result<DbBotAccount> {
    let pool = ctx.db().get();
    let platform = DbPlatform::get_by_id(new_bot.platform_id, pool)
        .await
        .inspect_err(|e| {
            tracing::error!(
                "Platform with id \"{}\"for Bot cannot be retrieved: {e}.",
                new_bot.platform_id
            );
        })?;
    new_bot
        .into_new(platform)
        .insert(pool)
        .await
        .map_err(Into::into)
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct IncomingNewBotAccount {
    platform_id: i64,
    external_id: String,
    token: Vec<u8>,
    expiry_h: Option<i64>,
}

impl IncomingNewBotAccount {
    /// Функция чисто для тестирования
    #[cfg(test)]
    pub(crate) fn new(platform_id: i64, external_id: &str, token: &[u8]) -> Self {
        Self {
            platform_id,
            external_id: external_id.to_string(),
            token: token.to_vec(),
            expiry_h: None,
        }
    }

    fn into_new(self, plat: DbPlatform) -> DbNewBotAccount {
        DbNewBotAccount::new(&plat, self.external_id, self.expiry_h, self.token)
    }
}
