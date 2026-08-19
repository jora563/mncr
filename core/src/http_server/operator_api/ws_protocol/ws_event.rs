//! Сообщения длв WS которые должны исходить от клиента. Условно запросы.
use crate::context::CoreCtx;
use crate::error::Result;

use actix_ws::Session;
use db::PrimitiveDateTime;
use db::core_schema::{CoreDbCrud, DbFullMessage, DbMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Любое сообщение по ВС у нас передаётся в такой структуре.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WsEventMsg {
    /// Идентификатор сообщения
    pub(crate) id: u128,
    /// Идентификатор запроса, если таков был
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_id: Option<u128>,
    /// Внутренн
    #[serde(flatten)]
    pub(crate) inner: WsEvent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
pub(crate) enum WsEvent {
    MessageSent(MessageSentEvent),
    MessageHistoryGot(MessageHistoryGotEvent),
    IncomingMessage(IncomingMessageEvent),
    QueuedChatGot(QueuedChatGotEvent),
    ConnectionStatusChanged(ConnectionStatusChangedEvent),
    ChatStatusChanged(ChatStatusChangedEvent),
    ChatRestored(ChatRestoredEvent),
    ChatByIdJoined(ChatByIdJoinedEvent),
    IFrameGot(IFrameGotEvent),
    Error(ErrorEvent),
}

impl WsEventMsg {
    /// Обработать сообщение. Пока ничего не делаем, абы не знаем цель.
    pub(crate) async fn handle(self, ctx: &Arc<CoreCtx>, session: &mut Session) -> Result<()> {
        tracing::info!("Handling WS Event.");
        let (id, req_id) = (self.id, self.request_id);
        match self.inner {
            WsEvent::MessageSent(_) => {
                tracing::warn!("Illegal (`MessageSent`) event received by server.")
            }
            WsEvent::MessageHistoryGot(_) => {
                tracing::warn!("Illegal (`MessageHistoryGot`) event received by server.")
            }
            WsEvent::IncomingMessage(_) => {
                tracing::warn!("Illegal (`IncomingMessage`) event received by server.")
            }
            WsEvent::QueuedChatGot(_) => {
                tracing::warn!("Illegal (`QueuedChatGot`) event received by server.")
            }
            WsEvent::ChatRestored(_) => {
                tracing::warn!("Illegal (`ChatRestored`) event received by server.")
            }
            WsEvent::ChatByIdJoined(_) => {
                tracing::warn!("Illegal (`ChatByIdJoinedEvent`) event received by server.")
            }
            WsEvent::IFrameGot(_) => {
                tracing::warn!("Illegal (`IFrameGot`) event received by server.")
            }
            WsEvent::ConnectionStatusChanged(x) => x.handle(id, req_id, ctx, session).await?,
            WsEvent::ChatStatusChanged(x) => x.handle(id, req_id, ctx, session).await?,
            WsEvent::Error(x) => x.handle(id, req_id, ctx, session).await?,
        };
        Ok(())
    }

    pub(crate) fn new(inner: WsEvent) -> Self {
        let request_id = None;
        let id = uuid::Uuid::now_v7().as_u128();
        Self {
            id,
            request_id,
            inner,
        }
    }

    pub(crate) fn reply(inner: WsEvent, req_id: u128) -> Self {
        let request_id = Some(req_id);
        let id = uuid::Uuid::now_v7().as_u128();
        Self {
            id,
            request_id,
            inner,
        }
    }
}

impl WsEvent {
    /// Преобразовать в сообщение и послать.
    pub(crate) async fn send_new(self, session: &mut Session) -> Result<()> {
        let msg = super::WsTextMessage::Event(WsEventMsg::new(self));
        session.text(serde_json::to_string(&msg)?).await?;
        Ok(())
    }
    /// Преобразовать в сообщение и послать.
    pub(crate) async fn send_reply(self, req_id: u128, session: &mut Session) -> Result<()> {
        let msg = super::WsTextMessage::Event(WsEventMsg::reply(self, req_id));
        session.text(serde_json::to_string(&msg)?).await?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Message {
    /// Id сообщения
    #[serde(rename = "id")]
    pub(super) id: i64,
    /// Само сообщение
    pub(super) message: String,
    /// Есть ли файл?
    #[serde(rename = "hasFile")]
    pub(super) has_file: bool,
    /// Ссылка файл(oв) если можно скачать на прямую.
    #[serde(rename = "filePath", skip_serializing_if = "Vec::is_empty")]
    pub(super) file_path: Vec<String>,
    /// Когда создано сообщение
    #[serde(rename = "dateTime")]
    pub(super) created: PrimitiveDateTime,
}

impl From<DbFullMessage> for Message {
    fn from(m: DbFullMessage) -> Self {
        Self {
            id: m.message.pkey(),
            message: m.message.content.unwrap_or_default(),
            has_file: !m.files.is_empty(),
            file_path: m.files.into_iter().map(|a| a.file_url).collect(),
            created: m.message.created_on,
        }
    }
}

impl From<DbMessage> for Message {
    fn from(m: DbMessage) -> Self {
        Self {
            id: m.pkey(),
            message: m.content.unwrap_or_default(),
            has_file: false,
            file_path: vec![],
            created: m.created_on,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MessageSentEvent(pub(super) Message);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MessageHistoryGotEvent {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(super) ticket_id: i64,
    /// массив объектов сообщений
    pub(super) messages: Vec<Message>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct IncomingMessageEvent {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(crate) ticket_id: i64,
    /// Id сообщения
    #[serde(flatten)]
    pub(crate) message: Message,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct QueuedChatGotEvent {
    /// Идентификатор темы. Если тем нет, то выдаёт `null`.
    #[serde(rename = "chatId")]
    pub(super) ticket_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ConnectionStatusChangedEvent {
    /// индикатор статуса на который поменялся
    #[serde(rename = "status")]
    pub(super) conn_status: i16,
}

impl ConnectionStatusChangedEvent {
    async fn handle(
        self,
        _id: u128,
        _req_id: Option<u128>,
        _ctx: &Arc<CoreCtx>,
        _session: &mut Session,
    ) -> Result<()> {
        //
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ChatStatusChangedEvent {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(super) ticket_id: i64,
    /// индикатор статуса на который поменялся
    #[serde(rename = "status")]
    pub(super) ticket_status: i16,
}

impl ChatStatusChangedEvent {
    async fn handle(
        self,
        _id: u128,
        _req_id: Option<u128>,
        _ctx: &Arc<CoreCtx>,
        _session: &mut Session,
    ) -> Result<()> {
        //
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ChatRestoredEvent {
    /// Идентификаторы доступных тем
    #[serde(rename = "chatId")]
    pub(super) ticket_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ChatByIdJoinedEvent {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(super) ticket_id: i64,
    /// индикатор статуса на который поменялся
    #[serde(rename = "status")]
    pub(super) ticket_status: i16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct IFrameGotEvent {
    /// Html/js/css для ФЕ
    pub(super) code: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ErrorEvent {
    pub(crate) error_text: String,
}

impl ErrorEvent {
    pub(crate) fn new(e: &str) -> Self {
        let error_text = e.to_string();
        ErrorEvent { error_text }
    }

    async fn handle(
        self,
        id: u128,
        req_id: Option<u128>,
        _ctx: &Arc<CoreCtx>,
        _session: &mut Session,
    ) -> Result<()> {
        let req_id = req_id.unwrap_or(0);
        tracing::error!(
            "Error event received by server (event id: {id}, request id: {req_id}): {self:?}"
        );
        Ok(())
    }
}
