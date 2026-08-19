//! Библиотека для plugina проверки токенов.
//!
//! Плугин [`UzorPlugin`] это классический middleware для actix сервера.
//!
//! ```ignore
//! App::new().wrap(UzorPlugin::new(config)).service(my_service)
//! ```
//! При получение запросы, [`UzorPlugin`] проверяет наличие и валидность токена
//! keycloak в одном из двух режимов.
//! 1. Послать запрос на Keycloak сервер через токен через REST Api
//!    protocols/openid-connect/token/introspect)
//! 2. Проверить хеш хедера и контента токена против той что есть в токена.
//!
//! При успешной проверке токена библиотека экстрагирует поле `personal_id` и посылает
//! её на ASAA сервер по REST Api (v1/employee/whats?personalId={id}), чтобы запросить
//! разрешенные проекты.
//!
//! Эти данные передаются уже в обработчики запросов для правильной фильтрации выданной информации.
//!
//! Дополнительно есть две простые middleware функции которые проверяют роли для API оператора
//! и администратора.
//! - admin_gate
//! - operator_gate
//!
//! Они используются следующим образом:
//! ```ignore
//! App::new().wrap(from_fn(operator_gate))
//!
//!

use actix_web::Error;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::middleware::Next;
use futures::future::Ready;
use std::pin::Pin;
use std::rc::Rc;

/// Стык для взаимодействия с системами узора. В основном для яутентификации.
#[derive(Default)]
pub struct UzorPlugin {
    config: config::UzorPluginConfig,
}

impl UzorPlugin {
    pub fn new(config: &dyn config::GetUzorPluginConfig) -> Self {
        let config = config.get_config().clone();
        Self { config }
    }
}

pub struct UzorPluginMiddleware<S> {
    service: Rc<S>,
    config: config::UzorPluginConfig,
}

// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for UzorPlugin
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = UzorPluginMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let config = self.config.clone();
        let service = Rc::new(service);
        futures::future::ready(Ok(UzorPluginMiddleware { service, config }))
    }
}

impl<S, B> Service<ServiceRequest> for UzorPluginMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    #[tracing::instrument(skip_all)]
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let (req, payload) = req.into_parts();
        let service = self.service.clone();
        let config = self.config.clone();

        Box::pin(async move {
            let client = reqwest::Client::new();

            let token_data = if config.keycloak.use_server_based_check {
                client::check_token_via_keycloak_server(&req, &config, &client)
                    .await
                    .inspect_err(|e| tracing::error!("{e}"))?
            } else {
                client::check_token_via_sha512(&req, &config)?
            };

            let extra_data = client::get_asaa_data(&token_data, &config, &client).await?;

            let mut service_req = ServiceRequest::from_parts(req, payload);
            client::fill_request(&mut service_req, token_data, extra_data)?;

            let res = service.call(service_req).await?;
            Ok(res)
        })
    }
}

async fn inner_gate(
    req: ServiceRequest,
    role: &str,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    use client::PermissionReExtract;

    let token = client::TokenData::from_final_request(req.request())?;

    if token.role != role {
        return Err(error::UzorPluginError::role(role, &token.role).into());
    }
    next.call(req).await
}

/// Утилита чтобы только операторы проходили через эту проверку.
pub async fn operator_gate(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    inner_gate(req, "operator", next).await
}

/// Утилита чтобы только администраторы проходили через эту проверку.
pub async fn admin_gate(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    inner_gate(req, "admin", next).await
}

mod client;
pub mod config;
pub mod error;
#[cfg(feature = "mock_server")]
pub mod mock_server;

pub use client::{AsaaData, AsaaProject, PermissionReExtract, TokenData};
