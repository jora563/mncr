//! Внутренний функционал сервера
use crate::context::{CoreCtx, OperatorData};
use crate::error::{CoreError, Result};

use actix_web::web::{Data, Payload};
use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder, Responder, get};
use actix_ws::{AggregatedMessage as AggMsg, MessageStream, Session};
use actix_ws::{CloseCode, CloseReason};
use bytes::Bytes;
use bytestring::ByteString;
use db::core_schema::DbFullMessage;
use std::sync::Arc;
use std::time::Duration;
use ws_protocol::*;

/// WS API для оператора вызывается одним методом.
///
/// После первоначального вызова и перехода на WS протокол, сообщения передаются по созданному каналу.
#[utoipa::path(
    params(("Authorization" = String, Header, description = "Bearer + JWT token")),
    responses((status = 101, body = String))
)]
#[get("/chat")]
pub(crate) async fn chat(req: HttpRequest, body: Payload) -> impl Responder {
    match inner_websocket_chat(req, body).await {
        Ok(t) => t,
        Err(e) => {
            #[cfg(test)]
            println!("Critical WS error: {e:?}");
            HttpResponseBuilder::new(e.to_status()).body(e.to_string())
        }
    }
}

/// Базовый функционал чата вебсокетов.
#[tracing::instrument(skip(body))]
async fn inner_websocket_chat(req: HttpRequest, body: Payload) -> Result<HttpResponse> {
    let ctx = req
        .app_data::<Data<Arc<CoreCtx>>>()
        .ok_or_else(|| CoreError::Other("Server data not found".into()))
        .inspect_err(|e| tracing::error!("Error on getting ctx: {e}"))?;

    let ctx = Arc::clone(ctx.as_ref());
    let (res, session, stream) = actix_ws::handle(&req, body)
        .inspect_err(|e| tracing::error!("Error on initiating ws: {e}"))?;

    // Достать полный список разрешенных проектов из запроса.

    // TODO: Fill in the runner function.
    tokio::task::spawn_local(run_ws_chat(ctx, req, session, stream));

    Ok(res)
}

/// Процесс опроса входящих сообщений, обработки, и послания обратных сообщений.
#[tracing::instrument(skip_all)]
async fn run_ws_chat(
    ctx: Arc<CoreCtx>,
    req: HttpRequest,
    mut session: Session,
    stream: MessageStream,
) -> Result<()> {
    tracing::info!("Starting WS chat for session.");
    // Чтобы не обрабатывать каждый неполный отрывок.
    let mut aggr = stream.aggregate_continuations();
    // Данные которые оператору понадобятся до конца сессии.
    // Эти данные инициируются в operator_api/ws_protocol/ws_req.rs:GetQueuedChatRequest::handle.
    let timeout = Duration::from_secs(ctx.queue().config().ping_period() as u64);
    let cloned_session = session.clone();
    let timer_task = tokio::task::spawn(ping_cycle(timeout, cloned_session));
    let mut op_data = OperatorData::new(&req, timer_task)?;

    // Трекер сообщений. Если ничего не приходит, то мы закрываем соединения.
    'ws_stream_loop: loop {
        tracing::info!("Entering loop of WS Chat with operator: {op_data:?}");
        let ending = tokio::select! {
            // Первая ветка: Приходит сообщение из чатов - его надо переслать.
            Some(msg) = op_data.get_msg() => {
                tracing::info!("Sending message to operator: {msg:?}");
                send_msg_to_operator(msg, &mut session).await
            }
            // Вторая ветка: Приходит сообщение от оператора, его надо обработать.
            msg = aggr.recv() => {
                let Some(Ok(msg)) = msg else {
                    break 'ws_stream_loop;
                };
                tracing::info!("Received message from operator, processing: {msg:?}");

                // Refresh connection age.
                op_data.tick(&ctx).await;

                match msg {
                    // Самые интересные процессы происходят в `handle_text`. Там может поменяться
                    // статус и внутренность `OperatorData`. Слидите за:
                    // - `ChatStatusChangeRequest`
                    // - `GetQueuedChatRequest`
                    AggMsg::Text(bs) => handle_text(bs, &mut op_data, &ctx, &mut session).await,
                    AggMsg::Binary(bytes) => handle_binary(bytes, &ctx, &mut session).await,
                    AggMsg::Ping(ping) => handle_ping(ping, &mut session).await,
                    AggMsg::Pong(pong) => handle_pong(pong, &mut session).await,
                    // TODO: Посмотреть правильно работает ли этот подход. Если нет то возможно
                    // его надо объединить с остальными.
                    AggMsg::Close(close) => return handle_close(close, &mut op_data, &ctx, session).await,
                }
            }
            // Если за период отключения ничего не приходит, мы закрываем шлюз.
            _ = tokio::time::sleep(ctx.ws_chats().timeout()) => {
                return handle_close(None, &mut op_data, &ctx, session).await;
            }
        };
        // Handle any resulting errors.
        match ending.into() {
            WsHandleResult::Ok => {}
            WsHandleResult::Close(r) => {
                return handle_close(Some(r), &mut op_data, &ctx, session).await;
            }
            WsHandleResult::OperatingError(e) => {
                try_send_error(e, &mut op_data, &ctx, &mut session).await?
            }
        }
    }
    Ok(())
}

/// Обработчик текстовых сообщений.
#[tracing::instrument(skip_all)]
async fn handle_text(
    bs: ByteString,
    op_data: &mut OperatorData,
    ctx: &Arc<CoreCtx>,
    s: &mut Session,
) -> Result<()> {
    tracing::info!("Received text message through WS protocol.");

    match WsTextMessage::from_text(bs)? {
        WsTextMessage::Request(r) => r.handle(op_data, ctx, s).await?,
        WsTextMessage::Event(e) => e.handle(ctx, s).await?,
    };
    Ok(())
}

/// Обработчик бинарных сообщений.
/// ТОДО: Есть ли у нас бинарные сообщения.
#[tracing::instrument(skip_all)]
async fn handle_binary(bytes: Bytes, ctx: &Arc<CoreCtx>, s: &mut Session) -> Result<()> {
    // Декодировать бинарное сообщение.
    let bin_item = WsRawBinItem::from_bytes(bytes)?.into_item()?;

    // Обработать бинарное сообщение.
    match bin_item {
        WsBinItem::File(file) => file.handle(ctx, s).await?,
    }

    tracing::info!("Received binary message through WS protocol.");
    Ok(())
}

/// Обработчик текстовых сообщений.
#[tracing::instrument(skip_all)]
async fn handle_ping(bytes: Bytes, s: &mut Session) -> Result<()> {
    tracing::info!("Received binary message through WS protocol.");
    s.pong(&bytes).await?;
    Ok(())
}

/// Обработчик текстовых сообщений.
#[tracing::instrument(skip_all)]
async fn handle_pong(_: Bytes, _s: &mut Session) -> Result<()> {
    tracing::info!("Received PONG through WS protocol.");
    Ok(())
}

/// Обработчик штатного закрытия ВС.
#[tracing::instrument(skip_all)]
async fn handle_close(
    closing: Option<CloseReason>,
    op_data: &mut OperatorData,
    ctx: &Arc<CoreCtx>,
    s: Session,
) -> Result<()> {
    #[cfg(test)]
    println!("Received close message through WS protocol: {closing:?}.");
    tracing::info!("Received close message through WS protocol: {closing:?}.");

    let reason = match closing {
        Some(closing) => closing,
        None => CloseReason {
            code: CloseCode::Normal,
            description: Some("Unknown WS peer closure".to_string()),
        },
    };
    ctx.ws_chats().purge_chat_if_held(op_data).await;
    op_data.end_work_with_ticket(ctx.queue()).await?;
    s.close(Some(reason)).await.map_err(Into::into)
}

/// Послать ошибку в ответ на запрос. Если нельзя послать, то у нас проблемы и надо заканчивать
/// с этими шуточками.
#[tracing::instrument(skip_all)]
async fn try_send_error(
    error_text: String,
    op_data: &mut OperatorData,
    ctx: &Arc<CoreCtx>,
    s: &mut Session,
) -> Result<()> {
    #[cfg(test)]
    println!("Handling error: {error_text}");
    tracing::info!("Handling error: {error_text}.");
    let event = WsEvent::Error(ErrorEvent { error_text });
    let res = event.send_new(s).await;

    if let Err(ref e) = res {
        ctx.ws_chats().purge_chat_if_held(op_data).await;
        op_data.end_work_with_ticket(ctx.queue()).await?;
        tracing::error!("Cannot send error to operator: {e}");
    }
    res
}

/// Переслать сообщение на оператора как событие
#[tracing::instrument(skip_all)]
async fn send_msg_to_operator(msg: DbFullMessage, s: &mut Session) -> Result<()> {
    let ticket_id = msg.message.query_ticket_id;
    let message = msg.into();

    let event = WsEvent::IncomingMessage(IncomingMessageEvent { ticket_id, message });
    event.send_new(s).await
}

#[tracing::instrument(skip_all)]
async fn ping_cycle(timeout: Duration, mut session: Session) -> Result<()> {
    loop {
        tokio::time::sleep(timeout).await;
        session.ping(b"AI Omni Core ping.").await?;
    }
}

pub(super) mod ws_protocol;
