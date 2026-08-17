//! Суб-модуль ошибок
use actix_web::HttpResponse;
use actix_web::body::BoxBody;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, UzorPluginError>;

#[derive(Debug, Error)]
pub enum UzorPluginError {
    #[error("Bad header: {0}")]
    BadHeader(#[from] actix_web::http::header::ToStrError),
    #[error("Bad output header: {0}")]
    BadHeaderOut(#[from] actix_web::http::header::InvalidHeaderValue),
    #[error("Cannot decode from Base64: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON Deserialize error: {0}")]
    JsonDe(#[from] serde_json::Error),
    #[error("Error Verifying JWT: {0}")]
    JwtVerify(#[from] josekit::JoseError),
    #[error("Keycloak Token has expired.")]
    KeycloakTokenExpired,
    #[error("Invalid Keycloak Token presented.")]
    KeycloakTokenInvalid,
    #[error("No Keycloak Token provided.")]
    KeycloakTokenMissing,
    #[error("Keycloak signature method not supported: {0}.")]
    KeycloakUnsupportedSign(String),
    #[error("Error from Keycloak: {0}")]
    KeycloakGeneral(String),
    #[error("Request missing header: {0}")]
    MissingHeader(&'static str),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Incorrect Role. Required {0}, got {1}")]
    Role(String, String),
    #[error("Serialization error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Other Error: {0}")]
    Other(String),
}

impl UzorPluginError {
    pub fn role<A: Into<String>>(desired: A, got: A) -> Self {
        Self::Role(desired.into(), got.into())
    }
}

impl ResponseError for UzorPluginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let status = self.status_code();
        let body = BoxBody::new(self.to_string());
        HttpResponse::with_body(status, body)
    }
}
