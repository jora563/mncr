//! Внутренний функционал сервера
use crate::context::CoreCtx;
use crate::error::Result;

use actix_web::http::StatusCode;
use actix_web::web::{Bytes, Data};
use actix_web::{HttpResponseBuilder, Responder, get};
use std::sync::Arc;

#[get("/frontend")]
#[tracing::instrument(skip(data))]
pub(super) async fn get_frontend(data: Data<Arc<CoreCtx>>) -> impl Responder {
    tracing::info!("Get all platforms.");
    let res = get_frontend_inner(data).await;
    // We give a normal error response if we have an error.
    if let Err(err) = res {
        let status = err.to_status();
        return HttpResponseBuilder::new(status).body(err.to_string());
    }

    // If we have a good response, we use a streaming response.
    let entries = futures::stream::iter(res.unwrap());
    HttpResponseBuilder::new(StatusCode::OK).streaming(entries)
}

async fn get_frontend_inner(ctx: Data<Arc<CoreCtx>>) -> Result<Vec<Result<Bytes>>> {
    let mut output = Vec::new();

    for entry in std::fs::read_dir(&ctx.cfg().core().fe_dir)? {
        let path = entry?.path();
        let fe_file = std::fs::read_to_string(path)?;
        let bytes = actix_web::web::Bytes::from(fe_file);

        output.push(Ok(bytes));
    }
    Ok(output)
}
