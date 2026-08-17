//! Moк сервер который должен "проверять" токен и отдавать "Ок", а также список проектов.
use crate::client::{AsaaData, AsaaProject, KeyCloakIntrospectResponse};
use crate::config::UzorPluginConfig;
use crate::error::*;

use actix_web::http::StatusCode;
use actix_web::middleware::NormalizePath;
use actix_web::{App, HttpResponseBuilder, HttpServer, Responder, post, web};

/// This is stupid, so is actix.
fn create_asaa() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().wrap(NormalizePath::trim()).configure(cfg_asaa)
}

/// This is stupid, so is actix.
fn create_cloak() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().wrap(NormalizePath::trim()).configure(cfg_cloak)
}

/// This is stupid, so is actix.
fn create_both() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().wrap(NormalizePath::trim()).configure(cfg_joint)
}

fn cfg_joint(cfg: &mut web::ServiceConfig) {
    cfg.service(mock_asaa).service(mock_cloak);
}

fn cfg_asaa(cfg: &mut web::ServiceConfig) {
    cfg.service(mock_asaa);
}

fn cfg_cloak(cfg: &mut web::ServiceConfig) {
    cfg.service(mock_cloak);
}

fn ports(cfg: &UzorPluginConfig) -> Result<(u16, u16)> {
    let asaa_port = reqwest::Url::parse(&cfg.asaa_home)
        .map_err(|x| UzorPluginError::Other(x.to_string()))
        .inspect(|x| println!("{x:?}"))?
        .port_or_known_default()
        .unwrap_or(80);
    let keycloak_port = reqwest::Url::parse(&cfg.keycloak.home)
        .map_err(|x| UzorPluginError::Other(x.to_string()))
        .inspect(|x| println!("{x:?}"))?
        .port_or_known_default()
        .unwrap_or(80);
    Ok((asaa_port, keycloak_port))
}

/// Запуск мок сервера keycloak/asaa. Этот функционал можно применить в тестах
/// где нужны иметь аутентификацию.
pub async fn run_mock_asaa_server(cfg: &UzorPluginConfig) -> Result<()> {
    let (asaa_port, keycloak_port) = ports(cfg)?;

    println!("Asaa port: {asaa_port}, Keycloak port: {keycloak_port}");
    if asaa_port == keycloak_port {
        return Err(UzorPluginError::Other(
            "ASAA and keycloak ports must be different".into(),
        ));
    }
    let server = HttpServer::new(create_asaa)
        .workers(1)
        .worker_max_blocking_threads(1)
        .bind(("0.0.0.0", asaa_port))
        .inspect_err(|e| println!("Cannot run asaa server: {e}"))?;
    // We use join since it alters the return type to make it `send`.
    tokio::join!(server.run()).0?;
    Ok(())
}

/// Запуск мок сервера keycloak/asaa. Этот функционал можно применить в тестах
/// где нужны иметь аутентификацию.
pub async fn run_mock_keycloak_server(cfg: &UzorPluginConfig) -> Result<()> {
    let (asaa_port, keycloak_port) = ports(cfg)?;

    println!("Asaa port: {asaa_port}, Keycloak port: {keycloak_port}");
    if asaa_port == keycloak_port {
        return Err(UzorPluginError::Other(
            "ASAA and keycloak ports must be different".into(),
        ));
    }
    let server = HttpServer::new(create_cloak)
        .workers(1)
        .worker_max_blocking_threads(1)
        .bind(("0.0.0.0", keycloak_port))
        .inspect_err(|e| println!("Cannot run keycloak server: {e}"))?;
    println!("Mock keycloak server launched.");
    // We use join since it alters the return type to make it `send`.
    tokio::join!(server.run()).0?;
    Ok(())
}

/// Запуск мок сервера keycloak/asaa. Этот функционал можно применить в тестах
/// где нужны иметь аутентификацию.
pub async fn run_mock_auth_servers(cfg: &UzorPluginConfig) -> Result<()> {
    let (asaa_port, keycloak_port) = ports(cfg)?;

    println!("Asaa port: {asaa_port}, Keycloak port: {keycloak_port}");
    if asaa_port == keycloak_port {
        let joint_server = HttpServer::new(create_both)
            .workers(1)
            .worker_max_blocking_threads(1)
            .bind(("0.0.0.0", asaa_port))
            .inspect_err(|e| println!("Cannot run joint server: {e}"))?;
        println!("Mock authentication servers launched");

        joint_server.run().await?;
    } else {
        let asaa_server = HttpServer::new(create_asaa)
            .workers(1)
            .worker_max_blocking_threads(1)
            .bind(("0.0.0.0", asaa_port))
            .inspect_err(|e| println!("Cannot run asaa server: {e}"))?;

        let keycloak_server = HttpServer::new(create_cloak)
            .workers(1)
            .worker_max_blocking_threads(1)
            .bind(("0.0.0.0", keycloak_port))
            .inspect_err(|e| println!("Cannot run keycloak server: {e}"))?;
        println!("Mock authentication servers launched");

        let (a, b) = tokio::join!(asaa_server.run(), keycloak_server.run(),);
        a?;
        b?;
    }
    Ok(())
}

/// Всё разрешено (никто не будет настоящии токены создавать в юнит тестах).
#[post("/realms/{realm}/protocol/openid-connect/token/introspect")]
#[tracing::instrument]
async fn mock_cloak() -> impl Responder {
    let (active, error) = (true, None);
    let data = KeyCloakIntrospectResponse { active, error };
    println!("Mock keycloak response: {data:?}");
    HttpResponseBuilder::new(StatusCode::OK).json(data)
}

/// Отдаём проекты которые могут быть зашиты в фикстуры БД.
#[post("/v1/employee/whats")]
#[tracing::instrument]
async fn mock_asaa() -> impl Responder {
    println!("Request in mock asaa");
    let projects = [
        "Good Project I",
        "Good Project II",
        "Good Project III",
        "Bad Project",
        "Project Pager",
        "Project Donkey",
    ]
    .into_iter()
    .map(Into::into)
    .collect::<Vec<_>>();
    let data = AsaaData {
        projects,
        ..Default::default()
    };
    println!("Mock Asaa response: {data:?}");
    HttpResponseBuilder::new(StatusCode::OK).json(data)
}

impl From<&str> for AsaaProject {
    fn from(s: &str) -> Self {
        let (name, client_digital_code) = (s.to_string(), 0);
        Self {
            name,
            client_digital_code,
        }
    }
}
