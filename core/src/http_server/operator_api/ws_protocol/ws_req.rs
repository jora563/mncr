//! Сообщения длв WS которые должны исходить от клиента. Условно запросы.
use super::ws_event as wse;
use crate::context::{CoreCtx, OperatorData};
use crate::error::Result;

use actix_ws::Session;
use ahash::AHashMap;
use db::core_schema::moma::DbTicketChat;
use db::core_schema::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Любое сообщение по ВС у нас передаётся в такой структуре.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WsRequestMsg {
    /// Идентификатор сообщения
    pub(crate) id: u128,
    /// Внутренность
    #[serde(flatten)]
    pub(crate) inner: WsRequest,
}

#[cfg(test)]
impl WsRequestMsg {
    pub(crate) fn new(id: u128, inner: WsRequest) -> Self {
        Self { id, inner }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
pub(crate) enum WsRequest {
    MessageSend(MessageSendRequest),
    MessageHistoryGet(MessageHistoryGetRequest),
    FileGet(FileGetRequest),
    GetQueuedChat(GetQueuedChatRequest),
    ConnectionStatusChange(ConnectionStatusChangeRequest),
    ChatStatusChange(ChatStatusChangeRequest),
    ChatRestore(ChatRestoreRequest),
    ChatByIdJoin(ChatByIdJoinRequest),
    IFrameGet(IFrameGetRequest),
}

impl WsRequestMsg {
    /// Обработать сообщение. Пока ничего не делаем, абы не знаем цель.
    pub(crate) async fn handle(
        self,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        tracing::info!("Handling WS Request.");
        let id = self.id;
        match self.inner {
            WsRequest::MessageSend(x) => x.handle(id, op_data, ctx, session).await?,
            WsRequest::MessageHistoryGet(x) => x.handle(id, op_data, ctx, session).await?,
            WsRequest::FileGet(x) => x.handle(id, ctx, session).await?,
            WsRequest::GetQueuedChat(x) => x.handle(id, op_data, ctx, session).await?,
            WsRequest::ChatRestore(x) => x.handle(id, op_data, ctx, session).await?,
            WsRequest::ConnectionStatusChange(x) => x.handle(id, ctx, session).await?,
            WsRequest::ChatStatusChange(x) => x.handle(id, op_data, ctx, session).await?,
            WsRequest::ChatByIdJoin(x) => x.handle(id, op_data, ctx, session).await?,
            WsRequest::IFrameGet(x) => x.handle(id, ctx, session).await?,
        };
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MessageSendRequest {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(crate) ticket_id: i64,
    /// Само сообщение
    pub(crate) message: String,
}

impl MessageSendRequest {
    /// Обработать приход сообщения от клиента.
    /// 1. Достать чат к которой сообщение принадлежит.
    /// 2. Достать тему.
    /// 3. Достать все чаты по теме.
    /// 4. Послать на все чаты.
    /// 5. Сохранить б БД.
    /// 6. Послать уведомление на клиент.
    async fn handle(
        self,
        req_id: u128,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        let pool = ctx.db().get();
        let chat_driver = ctx.chat();

        op_data.ticket_permitted(self.ticket_id, ctx).await?;

        // Get relevant data.
        let mut ticket = DbTicket::get_by_id(self.ticket_id, pool).await?;
        let chats = DbTicketChat::get_for_ticket(self.ticket_id, pool).await?;

        let chat_ids = chats.iter().map(|x| x.pkey()).collect::<Vec<_>>();
        let mut bots = DbBotAccountWithMeta::get_by_ids(&chat_ids, pool)
            .await?
            .into_iter()
            .map(|x| (x.account.pkey(), x))
            .collect::<AHashMap<_, _>>();

        let chat_data = chats
            .into_iter()
            .flat_map(|ch| bots.remove(&ch.bot_account_id).map(|meta| (ch, meta)));
        // Послать сообщение в каждый релевантный чат.
        for (mut chat, platform) in chat_data {
            let msg = DbNewMessage::new_bot(
                &platform.account,
                1,
                "",
                &mut chat,
                &mut ticket,
                &self.message,
            )?
            .insert(pool)
            .await?;
            chat_driver
                .send_message_from_operator(&self.message, &chat, &platform)
                .await?;
            let event = wse::WsEvent::MessageSent(wse::MessageSentEvent(msg.into()));
            event.send_reply(req_id, session).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MessageHistoryGetRequest {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(crate) ticket_id: i64,
    /// Если добирать не все сообщения, то ид последнего которое НЕ надо добыть.
    #[serde(rename = "messageId")]
    pub(crate) last_message_id: Option<i64>,
    #[serde(rename = "size")]
    pub(crate) count: Option<u32>,
}

impl MessageHistoryGetRequest {
    /// Обработать запрос на историю сообщений.
    /// 1. Делается запрос БД на сообщения которые ассоциируются с тикетом с `id` = `ticket_id`,
    ///    и имеют дату отправки после сообщение с `id` = `last_message_id`.
    /// 2. Отправить эти сообщения.
    async fn handle(
        self,
        req_id: u128,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        let Self {
            ticket_id,
            last_message_id,
            count,
        } = self;

        op_data.ticket_permitted(self.ticket_id, ctx).await?;

        let messages =
            DbFullMessage::get_history(ticket_id, last_message_id, count, ctx.db().get())
                .await?
                .into_iter()
                .map(wse::Message::from)
                .collect();

        let msg = wse::WsEvent::MessageHistoryGot(wse::MessageHistoryGotEvent {
            ticket_id,
            messages,
        });
        msg.send_reply(req_id, session).await
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct FileGetRequest {
    /// Идентификатор сообщения, для которого добыть файл.
    #[serde(rename = "messageId")]
    pub(super) message_id: i64,
}

impl FileGetRequest {
    async fn handle(self, _id: u128, _ctx: &Arc<CoreCtx>, _session: &mut Session) -> Result<()> {
        //
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct GetQueuedChatRequest {
    /// набор тэгов для составления очереди приоритетов, содержит либо теги в виде строки,
    /// либо под-массивы с те ми же тэгами, под-массив указывает на связку ИЛИ между тегами
    /// внутри него.
    pub(super) tags: Vec<String>,
}

impl GetQueuedChatRequest {
    /// Обработать запрос: есть ли чаты которые можно достать из очереди:
    /// 1. Смотрим в БД и достаём ту которые в очереди первые.
    /// 2. Отдаём Ид.
    async fn handle(
        self,
        req_id: u128,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        let queue_answer = ctx
            .queue()
            .get_next_for_operator(op_data.external_id(), op_data.permitted_projects())
            .await?;

        let ticket_id = queue_answer.as_ref().map(|(x, _)| x.pkey());
        let event = wse::WsEvent::QueuedChatGot(wse::QueuedChatGotEvent { ticket_id });

        // Если есть тикет, добавляем в механизм передачи.
        if let Some((_ticket, operator)) = queue_answer {
            ctx.ws_chats().add_chat(operator, op_data).await?;
        }

        event.send_reply(req_id, session).await
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ChatRestoreRequest;

impl ChatRestoreRequest {
    async fn handle(
        self,
        req_id: u128,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        let queue_answer = ctx
            .queue()
            .restore_for_operator(op_data.external_id())
            .await?;

        let ticket_id = queue_answer.as_ref().map(|(x, _)| x.pkey());
        let event = wse::WsEvent::ChatRestored(wse::ChatRestoredEvent { ticket_id });

        // Если есть тикет, то мы его берём и возвращаем.
        if let Some((_ticket, operator)) = queue_answer {
            ctx.ws_chats().add_chat(operator, op_data).await?;
        }

        event.send_reply(req_id, session).await
    }
}

/// TODO: Понять нужно ли это, так как пока что чаты автоматический не раздаются.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ConnectionStatusChangeRequest {
    /// индикатор статуса на который меняем, нужен, для прерывания
    /// автоматической раздачи чатов по получению.
    #[serde(rename = "status")]
    pub(super) conn_status: i16,
}

impl ConnectionStatusChangeRequest {
    async fn handle(self, _id: u128, _ctx: &Arc<CoreCtx>, _session: &mut Session) -> Result<()> {
        //
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ChatStatusChangeRequest {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(crate) ticket_id: i64,
    /// индикатор статуса на который меняем
    #[serde(rename = "status")]
    pub(crate) ticket_status: i16,
}

impl ChatStatusChangeRequest {
    #[tracing::instrument(skip(ctx, session))]
    async fn handle(
        self,
        req_id: u128,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        op_data.ticket_permitted(self.ticket_id, ctx).await?;

        let pool = ctx.db().get();
        let new_status = DbTicketCloseStatus::from_i16(self.ticket_status)?;
        let mut ticket = DbTicket::get_by_id(self.ticket_id, pool).await?;

        ticket.close_status = new_status;
        ticket.update(pool).await?;

        ctx.queue().update_ticket_in_queue(&ticket).await?;

        // Если эскаляция окончена, убираем тикет из очереди, и снимаем его с оператора.
        if !matches!(ticket.close_status, DbTicketCloseStatus::EscalationOngoing) {
            ctx.ws_chats().purge_chat_if_held(op_data).await;
            op_data.end_work_with_ticket(ctx.queue()).await?;
        }

        let msg = wse::WsEvent::ChatStatusChanged(wse::ChatStatusChangedEvent {
            ticket_id: self.ticket_id,
            ticket_status: self.ticket_status,
        });
        msg.send_reply(req_id, session).await
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ChatByIdJoinRequest {
    /// Идентификатор темы
    #[serde(rename = "chatId")]
    pub(super) ticket_id: i64,
}

impl ChatByIdJoinRequest {
    /// Войти в чат:
    /// 1. Если сущности чата существуют в очереди без оператора, или существует канал но не оператор,
    ///    мы создаём канал и оператор, и его создаём и отдаём данному оператору.
    /// 2. Если сущность чата существует с оператором и канал есть, то мы оставляем его в покое
    ///    и выписываем ошибку.
    /// 3. Если сущности нет в очереди вообще (т.е. она на AI статусе ил закрыта) то мы также выдаём ошибку.
    async fn handle(
        self,
        req_id: u128,
        op_data: &mut OperatorData,
        ctx: &Arc<CoreCtx>,
        session: &mut Session,
    ) -> Result<()> {
        op_data.ticket_permitted(self.ticket_id, ctx).await?;

        let has_chat = ctx.ws_chats().has_chat(self.ticket_id).await;
        let ticket_is_queued = ctx
            .queue()
            .ticket_is_available_for_operator(op_data.external_id(), self.ticket_id)
            .await?;

        // Если тикет не доступен оператору мы возвращаем ошибку.
        if ticket_is_queued.is_none() || has_chat {
            tracing::warn!(
                "{} is not available for {}.",
                self.ticket_id,
                op_data.external_id()
            );

            let err = wse::WsEvent::Error(wse::ErrorEvent::new("Topic not available."));
            return err.send_reply(req_id, session).await;
        }
        // Not idiomatic, but expedient.
        let ticket = ticket_is_queued.expect("Checked above");

        let msg = wse::WsEvent::ChatByIdJoined(wse::ChatByIdJoinedEvent {
            ticket_id: ticket.pkey(),
            ticket_status: ticket.ticket_status as i16,
        });
        // Передать тикет этому оператору.
        let operator = ctx
            .queue()
            .assign_ticket_to_operator(op_data.external_id(), ticket)
            .await?;

        // Заменить операторя.
        ctx.ws_chats().add_chat(operator, op_data).await?;
        msg.send_reply(req_id, session).await
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct IFrameGetRequest;

impl IFrameGetRequest {
    /// Достать код IFrame-а чтобы его можно было передать на обработку.
    ///
    /// - Псевдо функцию. Пока мы не знаем какое будет у неё наполнение.
    ///   Будет ли заполнение из БД? Будет ли конфиг со путем к файлу кода IFrame? Будет ли ссылка?
    async fn retrieve_iframe(_ctx: &Arc<CoreCtx>) -> Result<String> {
        Ok(r#"<!DOCTYPE html><html lang="en">I</html>"#.to_string())
    }

    /// Обработать запрос на IFRAME.
    /// 1. Достать IFrame откуда-то.
    /// 2. Послать оператору.
    async fn handle(self, req_id: u128, ctx: &Arc<CoreCtx>, session: &mut Session) -> Result<()> {
        let code = Self::retrieve_iframe(ctx).await?;
        let msg = wse::WsEvent::IFrameGot(wse::IFrameGotEvent { code });

        msg.send_reply(req_id, session).await
    }
}
