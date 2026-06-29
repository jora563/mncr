//! Поллинг-цикл взаимодействия с ботами
use crate::context::CoreCtx;
use crate::error::Result;
use crate::messengers::ChatMessages;
use crate::poll_based::db_ops::ValidationOutcome;

use chat::models::{ReplyMarkup, TelegramKeyboard};
use db::core_schema::DbBotAccountWithMeta;
use std::sync::Arc;
use tokio::task as tt;

#[tracing::instrument(skip_all)]
pub(crate) async fn run_core(ctx: CoreCtx) -> Result<()> {
    let bot_metadata = ctx
        .load_initial_platforms()
        .await
        .inspect_err(|e| tracing::error!("Error loading platforms: {e}"))?;

    let mut handles = tt::JoinSet::new();
    let ctx = Arc::new(ctx);

    tracing::info!("Loaded {} bots, preparing to process.", bot_metadata.len());
    for bm in bot_metadata {
        handles.spawn(run_platform(ctx.clone(), bm));
    }

    while let Some(result) = handles.join_next().await {
        match result.map_err(Into::into) {
            Ok(Ok(_)) => tracing::info!("Core joined."),
            Ok(Err(e)) | Err(e) => tracing::error!("Core joined with critical error: {e}"),
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn run_platform(ctx: Arc<CoreCtx>, meta: DbBotAccountWithMeta) -> Result<()> {
    tracing::info!("Preparing to run platform: {}", meta.platform.platform.name);
    let meta = Arc::new(meta);
    ctx.chat().initialise(&meta).await?;

    loop {
        let messages = match ctx.chat().get_messages(&meta).await {
            Ok(m) if m.is_empty() => continue,
            Ok(m) => m,
            Err(e) if e.is_critical() => return Err(e),
            Err(e) => {
                tracing::error!("Error for {}: {e}", meta.account.external_id);
                continue;
            }
        };

        if let Err(e) = process_messages(ctx.clone(), meta.clone(), messages).await {
            tracing::error!("Error processing message: {e}");
        }
    }
}

#[tracing::instrument(skip_all)]
async fn process_messages(
    ctx: Arc<CoreCtx>,
    meta: Arc<DbBotAccountWithMeta>,
    chat_msgs: ChatMessages,
) -> Result<()> {
    let outcome = db_ops::check_validity_get_data(&chat_msgs, &meta, ctx.db()).await?;

    let mut data = match outcome {
        ValidationOutcome::Ok(data) => data,
        ValidationOutcome::NeedPhoneVerification { chat_ext_id, .. } => {
            return request_phone_verification(&ctx, &meta, &chat_ext_id, &chat_msgs).await;
        }
    };

    if chat_msgs.is_contact_only() {
        return handle_contact_verification(&ctx, &meta, &mut data, &chat_msgs).await;
    }

    process_regular_message(&ctx, &meta, &mut data, &chat_msgs).await
}

async fn request_phone_verification(
    ctx: &CoreCtx,
    meta: &Arc<DbBotAccountWithMeta>,
    chat_ext_id: &str,
    chat_msgs: &ChatMessages,
) -> Result<()> {
    tracing::info!("User from chat {} needs phone verification", chat_ext_id);

    let reply_markup = ReplyMarkup::Telegram(TelegramKeyboard::request_contact());

    ctx.chat()
        .send(
            meta,
            chat_ext_id,
            "Для продолжения работы, пожалуйста, поделитесь своим номером телефона.",
            chat_msgs.last_msg_external_id(),
            Some(reply_markup),
        )
        .await
}

async fn handle_contact_verification(
    ctx: &CoreCtx,
    meta: &Arc<DbBotAccountWithMeta>,
    data: &mut db_ops::StandardData,
    chat_msgs: &ChatMessages,
) -> Result<()> {
    tracing::info!("User verified via contact");

    let text = chat_msgs.texts().last().unwrap_or("[Медиа]");
    db_ops::insert_next_user_message(text, data, ctx.db()).await?;

    let reply_markup = ReplyMarkup::Telegram(TelegramKeyboard::remove());

    ctx.chat()
        .send(
            meta,
            &data.chat.external_id,
            "Спасибо! Ваш номер телефона успешно сохранён.",
            chat_msgs.last_msg_external_id(),
            Some(reply_markup),
        )
        .await
}

async fn process_regular_message(
    ctx: &CoreCtx,
    meta: &Arc<DbBotAccountWithMeta>,
    data: &mut db_ops::StandardData,
    chat_msgs: &ChatMessages,
) -> Result<()> {
    let mut conjoined = String::new();
    for text in chat_msgs.texts() {
        conjoined.push_str(text);
        conjoined.push('\n');
        db_ops::insert_next_user_message(text, data, ctx.db()).await?;
    }

    let full_ticket = data.ticket.clone().get_msgs(ctx.db().get()).await?;
    let llm_reply = ctx
        .llm()
        .query_llm_with_ticket(&full_ticket, conjoined, chat_msgs.len())
        .await?;

    let next_message = llm_reply.extract_answer();
    tracing::warn!("Message from LLM: {next_message}");

    let msg = db_ops::insert_next_bot_message(&next_message, meta, data, ctx.db()).await?;

    ctx.chat()
        .send_messages(meta, &data.chat, msg, chat_msgs.clone())
        .await
}

mod db_ops;
