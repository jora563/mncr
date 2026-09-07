//! Тут сервер. Для начала мы его используем для health-check и дистанционной
//! настройки конфигураций и проектов.
use crate::Config;
use crate::context::CoreCtx;
use crate::error::*;

use actix_web::http::StatusCode;
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::web::Data;
use actix_web::{App, HttpRequest, HttpResponseBuilder, HttpServer, Responder, get};
use db::core_schema::DbProject;
use std::ops::Deref;
use std::sync::Arc;
use uzor_plugin::{AsaaData, PermissionReExtract};

use utoipa::openapi::{Info, OpenApi, Paths};
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::{Config as SwaggerCfg, SwaggerUi};

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
        .wrap(NormalizePath::new(TrailingSlash::MergeOnly))
        .into_utoipa_app()
        .app_data(Data::new(ctx))
        .configure(|c| configure_server(c, cfg))
        .openapi_service(|api| {
            let cfg = SwaggerCfg::from("/api.json").with_credentials(true);
            let mut final_api = OpenApi::new(Info::new("AI OMNI Core API", "0.1.0"), Paths::new());

            final_api.merge(api);

            SwaggerUi::new("/docs/{_:.*}")
                .config(cfg)
                .url("/api.json", final_api)
        })
        .into_app()
}

/// Основная функция сервера.
/// TODO: HTTPS/TLS чтобы не передавать чувствительные данные.
#[tracing::instrument(skip_all)]
pub(crate) async fn run_server(ctx: Arc<CoreCtx>) -> Result<()> {
    let port = ctx.cfg().core().server_port;
    let worker_count = ctx.cfg().core().server_worker_count as usize;
    let max_blocking = ctx.cfg().core().server_max_blocking_threads as usize;

    let server = HttpServer::new(move || create_app(ctx.clone()))
        .workers(worker_count)
        .worker_max_blocking_threads(max_blocking);

    #[cfg(test)]
    println!("Binding server port.");
    tracing::info!("Binding server port.");

    let server2 = server.bind(("0.0.0.0", port))?;

    #[cfg(test)]
    println!("Server started.");
    tracing::info!("Server started.");

    server2.run().await?;
    Ok(())
}

fn configure_server(cfg: &mut utoipa_actix_web::service_config::ServiceConfig, server_cfg: Config) {
    use uzor_plugin::UzorPlugin;

    cfg.service(health)
        .service(get_config)
        .service(vk_callback::vk_callback)
        .service(
            utoipa_actix_web::scope("/v1")
                .wrap(UzorPlugin::new(&server_cfg))
                .service(
                    utoipa_actix_web::scope("/admin_api")
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
                        .service(admin_api::post_update_project_group)
                        .service(
                            utoipa_actix_web::scope("/llm")
                                .service(admin_llm_api::delete_project)
                                .service(admin_llm_api::get_project)
                                .service(admin_llm_api::get_projects)
                                .service(admin_llm_api::get_training_job)
                                .service(admin_llm_api::get_training_jobs_by_project)
                                .service(admin_llm_api::post_project)
                                .service(admin_llm_api::post_projects_adaptor)
                                .service(admin_llm_api::post_projects_build_index)
                                .service(admin_llm_api::post_projects_dataset)
                                .service(admin_llm_api::post_projects_knowledge)
                                .service(admin_llm_api::post_projects_questions)
                                .service(admin_llm_api::post_projects_reload)
                                .service(admin_llm_api::post_training_resume)
                                .service(admin_llm_api::post_projects_train)
                                .service(admin_llm_api::put_project),
                        ),
                )
                .service(
                    utoipa_actix_web::scope("/operator_api")
                        .wrap(actix_web::middleware::from_fn(uzor_plugin::operator_gate))
                        .service(operator_api::chat),
                ),
        );
}

#[utoipa::path(responses((status = 200, body = String)))]
#[get("/health")]
#[tracing::instrument]
async fn health() -> impl Responder {
    HttpResponseBuilder::new(StatusCode::OK).body("AIOMNI Core is healthy")
}

#[utoipa::path]
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

/// Утилита
async fn check_project_permission(
    req: HttpRequest,
    project_id: i64,
    data: &Arc<CoreCtx>,
) -> Result<(DbProject, AsaaData)> {
    let asaa_data = AsaaData::from_final_request(&req)?;
    let project = DbProject::get_by_id(project_id, data.db().get()).await?;

    permitted_project(&asaa_data, &project.project_name)?;
    Ok((project, asaa_data))
}

mod admin_api;
mod admin_llm_api;
mod operator_api;
mod to_response;
mod vk_callback;

#[cfg(test)]
mod tests;
