//! В этом под-модуле функционал конвертации результата в HTTP ответ.

use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder};
use serde::Serialize;

use crate::error::{CoreError, Result};

pub(super) trait IntoHttpResponse: Sized {
    fn into_response(self) -> HttpResponse {
        self.into_response_with_code(StatusCode::OK)
    }

    fn into_response_with_code(self, ok_status: StatusCode) -> HttpResponse;
}

impl<T: Serialize> IntoHttpResponse for Result<T> {
    fn into_response_with_code(self, ok_status: StatusCode) -> HttpResponse {
        match self {
            Ok(t) => HttpResponseBuilder::new(ok_status).json(t),
            Err(e) => HttpResponseBuilder::new(e.to_status()).body(e.to_string()),
        }
    }
}

impl CoreError {
    /// TODO: This. Properly.
    fn to_status(&self) -> StatusCode {
        use CoreError::*;
        use actix_web::ResponseError;

        match self {
            ConfigError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            EnvVar(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Join(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Log(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TomlError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ChatLib(_) => StatusCode::INTERNAL_SERVER_ERROR,
            EmptyChat => StatusCode::OK,
            ChatApiDisconnected(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ChatValidation(_) => StatusCode::OK,
            LlmError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Http(h) => h.status_code(),
            Parse(_) => StatusCode::INTERNAL_SERVER_ERROR,
            QueueError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
