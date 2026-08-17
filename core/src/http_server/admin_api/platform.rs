//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::http_server::to_response::IntoHttpResponse;

use actix_web::web::Data;
use actix_web::{Responder, get};
use db::core_schema::*;
use std::ops::Deref;
use std::sync::Arc;

#[get("/platforms")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_platforms(data: Data<Arc<CoreCtx>>) -> impl Responder {
    tracing::info!("Get all platforms.");

    DbFullPlatform::get_all(data.deref().db().get())
        .await
        .map_err(Into::into)
        .into_response()
}
