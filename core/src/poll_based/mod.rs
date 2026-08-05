//! Архитектурное решение взаимодействие с ботами через Поллинг.
//! В этом случае Core работает как клиент, посылая запросы на площадки чатов чтобы с
//! ними взаимодействовать,(забрать или послать сообщения, или создать чаты,
//! или выполнить иные взаимодействия.
use crate::context::CoreCtx;
use crate::error::Result;
use crate::messengers::ChatMessages;

use db::core_schema::DbBotAccountWithMeta;
use std::sync::Arc;
use tokio::task as tt;

/// Тут основной цыкл. Тут же должна быть
#[tracing::instrument(skip_all)]
pub(crate) async fn run_core(ctx: Arc<CoreCtx>) -> Result<()> {
    let bot_metadata = ctx
        .load_initial_platforms()
        .await
        .inspect_err(|e| tracing::error!("Error loading platforms: {e}"))?;
    let mut handles = tt::JoinSet::new();

    tracing::info!("Loaded {} bots, preparing to process.", bot_metadata.len());
    for bm in bot_metadata.into_iter() {
        handles.spawn(run_platform(ctx.clone(), bm));
    }

    // Жди пока сервер не покончит с собой
    while let Some(result) = handles.join_next().await {
        match result.map_err(Into::into) {
            Ok(Ok(_)) => tracing::info!("Core joined."),
            Ok(Err(e)) | Err(e) => tracing::error!("Core joined with critical error: {e}"),
        };
    }
    Ok(())
}

/// Cycle a single platform.
/// TODO: Install a kill signal.
#[tracing::instrument(skip_all)]
async fn run_platform(ctx: Arc<CoreCtx>, meta: DbBotAccountWithMeta) -> Result<()> {
    tracing::info!("Preparing to run platform: {}", meta.platform.platform.name);
    let meta = Arc::new(meta);
    ctx.chat().initialise(&meta).await?;
    'poll_loop: loop {
        // Достать сообщения
        let messages = match ctx.chat().get_messages(&meta).await {
            Ok(m) if m.is_empty() => {
                tracing::trace!(
                    "Empty messages received for {} on {}",
                    meta.account.external_id,
                    meta.platform.platform.name
                );
                continue 'poll_loop;
            }
            Ok(m) => m,
            Err(e) if e.is_critical() => return Err(e),
            Err(e) => {
                tracing::error!(
                    "Error for {} on {}: {e}",
                    meta.account.external_id,
                    meta.platform.platform.name
                );
                // Подождать, чтобы СПУ не съедать.
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue 'poll_loop;
            }
        };
        if let Err(e) = process_messages(ctx.clone(), meta.clone(), messages).await {
            tracing::error!("Error processing message: {e}");
        };
    }
}

/// Дело:
/// 1. проверить соответствует ли сообщение нашей логике.
/// 2. Вставить новое сообщение/чат/тикет в БД.
/// 3. Послать в ЛЛМ и получить ответ.
/// 4. Записать ответ в базу данных.
/// 5. Отослать обратно на чат мессенджера.
///
/// TODO: Расслоить если мы добываем сообщения из нескольких чатов.
#[tracing::instrument(skip_all)]
async fn process_messages(
    ctx: Arc<CoreCtx>,
    meta: Arc<DbBotAccountWithMeta>,
    chat_msgs: ChatMessages,
) -> Result<()> {
    process_messages_inner(ctx, meta, chat_msgs)
        .await
        .inspect_err(|e| tracing::error!("Error processing message: {e}"))
}

async fn process_messages_inner(
    ctx: Arc<CoreCtx>,
    meta: Arc<DbBotAccountWithMeta>,
    chat_msgs: ChatMessages,
) -> Result<()> {
    // Достать разговор из БД, осуществив нужные проверки.
    let mut data = db_ops::check_validity_get_data(&chat_msgs, &meta, ctx.db()).await?;

    // Добавить изначальное сообщение.
    let mut conjoined = String::new();
    let count = chat_msgs.len();
    for text in chat_msgs.get_text() {
        conjoined.push_str(text);
        db_ops::insert_next_user_message(text, &mut data, ctx.db()).await?;
        conjoined.push('\n');
    }

    // Послать разговор в LLM и дождаться ответа.
    let full_ticket = data.ticket.clone().get_msgs(ctx.db().get()).await?;
    tracing::info!("Validation complete for chat: {:?}", full_ticket.ticket);
    tracing::trace!("Got ticket: {full_ticket:?}");

    let llm_reply = ctx
        .llm()
        .query_llm_with_ticket(&full_ticket, conjoined, count)
        .await?;

    // сохранить ответ в БД.
    let next_message = llm_reply.extract_answer();
    tracing::warn!("Message from LLM: {next_message}");

    let msg = db_ops::insert_next_bot_message(&next_message, &meta, &mut data, ctx.db()).await?;

    // Послать ответ на чат.
    // TODO: Where do the sending params come from?
    ctx.chat()
        .send_messages(&meta, &data.chat, msg, chat_msgs)
        .await?;

    Ok(())
}

mod db_ops;
