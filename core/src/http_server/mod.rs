//! Тут сервер. Для начала мы его используем для health-check и дистанционной
//! настройки конфигураций и проектов.
use crate::Config;
use crate::context::CoreCtx;
use crate::error::*;

use actix_web::http::StatusCode;
use actix_web::middleware::NormalizePath;
use actix_web::web::{self, Data};
use actix_web::{App, HttpResponse, HttpResponseBuilder, HttpServer, Responder, get};
use db::core_schema::DbProject;
use std::ops::Deref;
use std::sync::Arc;

/// This is stupid, but needed fo testing.
fn create_app(
    ctx: Arc<CoreCtx>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let cfg = ctx.cfg().clone();
    App::new()
        .app_data(Data::new(ctx))
        .wrap(NormalizePath::trim())
        .configure(|c| configure_server(c, cfg))
}

/// Основная функция сервера.
/// TODO: HTTPS/TLS чтобы не передавать чувствительные данные.
pub(crate) async fn run_server(ctx: Arc<CoreCtx>) -> Result<()> {
    let port = ctx.cfg().core().server_port;
    let worker_count = ctx.cfg().core().server_worker_count as usize;
    let max_blocking = ctx.cfg().core().server_max_blocking_threads as usize;

    let server = HttpServer::new(move || create_app(ctx.clone()))
        .workers(worker_count)
        .worker_max_blocking_threads(max_blocking);
    #[cfg(test)]
    {
        println!("Binding server port.");
    }
    let server2 = server.bind(("0.0.0.0", port))?;
    #[cfg(test)]
    {
        println!("Running server.");
    }

    server2.run().await?;
    Ok(())
}

fn configure_server(cfg: &mut web::ServiceConfig, server_cfg: Config) {
    use uzor_plugin::UzorPlugin;

    cfg.service(health)
        .service(get_config)
        .service(vk_callback::vk_callback)
        .service(
            web::scope("/v1")
                .wrap(UzorPlugin::new(&server_cfg))
                .service(
                    web::scope("/admin_api")
                        .wrap(actix_web::middleware::from_fn(uzor_plugin::admin_gate))
                        .service(admin_api::delete_bot)
                        .service(admin_api::delete_project)
                        .service(admin_api::delete_project_group)
                        .service(admin_api::get_bot)
                        .service(admin_api::get_bots_for_project)
                        .service(admin_api::get_frontend)
                        .service(admin_api::get_permitted_projects)
                        .service(admin_api::get_platforms)
                        .service(admin_api::get_projects)
                        .service(admin_api::get_project_groups)
                        .service(admin_api::post_new_bot_account)
                        .service(admin_api::post_new_project)
                        .service(admin_api::post_new_project_group)
                        .service(admin_api::post_update_bot_account)
                        .service(admin_api::post_update_project)
                        .service(admin_api::post_update_project_group),
                ),
        );
}

#[get("/health")]
#[tracing::instrument]
async fn health() -> impl Responder {
    HttpResponse::with_body(StatusCode::OK, "AIOMNI Core is healthy")
}

#[get("/config")]
#[tracing::instrument(skip_all)]
async fn get_config(data: Data<Arc<CoreCtx>>) -> impl Responder {
    let ctx: &Arc<CoreCtx> = data.deref();
    HttpResponseBuilder::new(StatusCode::OK).json(ctx.cfg().clone())
}

fn permitted_project(asaa_data: &uzor_plugin::AsaaData, proj_name: &str) -> Result<()> {
    if !asaa_data.projects.iter().any(|p| p.name == proj_name) {
        Err(CoreError::NoAccess("Project", proj_name.to_string()))
    } else {
        Ok(())
    }
}

fn permitted_projects(asaa_data: uzor_plugin::AsaaData, projects: &[DbProject]) -> Result<()> {
    // TODO: Sometimes an object does not belong to any project.
    if projects.is_empty() {
        return Ok(());
    }
    for name in projects.iter().map(|x| &x.project_name) {
        if permitted_project(&asaa_data, name).is_ok() {
            return Ok(());
        }
    }
    Err(CoreError::NoAccess("Project", "Unknown".to_string()))
}

mod admin_api;
mod to_response;
mod vk_callback;

#[cfg(test)]
mod tests;
