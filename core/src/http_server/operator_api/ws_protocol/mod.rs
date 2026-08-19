//! Внутреннее исполнение переговоров клиента с сервером по WS.
#![allow(dead_code)]
use crate::error::{CoreError, Result};

use actix_ws::{CloseCode, CloseReason};
use bytestring::ByteString;
use serde::{Deserialize, Serialize};

/// Любое текстовое сообщение из WS для нас принимает эту форму.
/// Из этого мы уже понимаем, событие это или обращение.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "data")]
pub(crate) enum WsTextMessage {
    Event(ws_event::WsEventMsg),
    Request(ws_req::WsRequestMsg),
}

impl WsTextMessage {
    /// Из сырой строки в WsMessage. По сути дела просто сериализация.
    pub(crate) fn from_text(t: ByteString) -> Result<Self> {
        serde_json::from_str(&t).map_err(Into::into)
    }
}

/// Чем закончить операцию.
/// Если операция заканчивается `Close`, то закрываем соединение.
/// Если заканчивается `OperatingError` то отправляем клиенту извещение ошибки
/// Иначе ничего не делаем.
#[derive(Debug)]
pub(super) enum WsHandleResult {
    Ok,
    Close(CloseReason),
    OperatingError(String),
}

impl From<Result<()>> for WsHandleResult {
    fn from(r: Result<()>) -> Self {
        use CoreError::*;

        if r.is_ok() {
            return Self::Ok;
        }
        match r.unwrap_err() {
            ConfigError { .. }
            | EnvVar(_)
            | DbError(db::error::DbError::ConfigParse(_))
            | DbError(db::error::DbError::MigrateSql(_))
            | Join(_)
            | Log(_)
            | TomlError(_) => Self::Close(CloseReason {
                code: CloseCode::Error,
                description: Some("Critical server error. Disconnecting.".into()),
            }),
            Io(_)
            | DbError(db::error::DbError::Io(_))
            | ChatApiDisconnected(_)
            | QueueError(_)
            | LlmError(_) => Self::Close(CloseReason {
                code: CloseCode::Error,
                description: Some("Temporary server error. Try again later.".into()),
            }),
            WSClose(_) | WSCloseError(_) | WsHandShakeError(_) => Self::Close(CloseReason {
                code: CloseCode::Protocol,
                description: Some("Error in connection.".into()),
            }),
            x => Self::OperatingError(x.to_string()),
        }
    }
}

#[cfg(test)]
mod tests;

mod ws_binary;
mod ws_event;
mod ws_req;

pub(crate) use ws_binary::*;
pub(crate) use ws_event::*;
#[cfg(test)]
pub(crate) use ws_req::*;
