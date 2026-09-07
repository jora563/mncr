//! Архитектурное решение взаимодействие с ботами через Поллинг.
//! В этом случае Core работает как клиент, посылая запросы на площадки чатов чтобы с
//! ними взаимодействовать,(забрать или послать сообщения, или создать чаты,
//! или выполнить иные взаимодействия.
use crate::context::CoreCtx;
use crate::error::Result;
use crate::llm::LlmReply;
use crate::messengers::ChatMessages;
use crate::poll_based::db_ops::ValidationOutcome;

use chat::models::{ReplyMarkup, TelegramKeyboard};
use db::core_schema::{ApiId, CoreDbCrud, DbBotAccountWithMeta, DbTicketCloseStatus};
use std::sync::Arc;
use tokio::task as tt;

/// Базовый URL для VK OAuth.
const VK_OAUTH_URL: &str = "https://oauth.vk.com";

#[tracing::instrument(skip_all)]
pub(crate) async fn run_core(ctx: Arc<CoreCtx>) -> Result<()> {
    let bot_metadata = ctx
        .load_initial_platforms()
        .await
        .inspect_err(|e| tracing::error!("Error loading platforms: {e}"))?;

    let mut handles = tt::JoinSet::new();

    tracing::info!("Loaded {} bots, preparing to process.", bot_metadata.len());
    for bm in bot_metadata {
        handles.spawn(run_platform(ctx.clone(), bm));
    }

    // Жди пока сервер не покончит с собой
    while let Some(result) = handles.join_next().await {
        match result.map_err(Into::into) {
            Ok(Ok(_)) => tracing::info!("Core joined."),
            Ok(Err(e)) | Err(e) => tracing::error!("Core joined with critical error: {e}"),
        }
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

    loop {
        let messages = match ctx.chat().get_messages(&meta).await {
            Ok(m) if m.is_empty() => {
                tracing::trace!(
                    "Empty messages received for {} on {}",
                    meta.account.external_id,
                    meta.platform.platform.name
                );
                continue;
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
                continue;
            }
        };

        if let Err(e) = process_messages(ctx.clone(), meta.clone(), messages).await {
            tracing::error!("Error processing message: {e}");
        }
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
    let outcome = db_ops::check_validity_get_data(&chat_msgs, &meta, ctx.db()).await?;

    let mut data = match outcome {
        ValidationOutcome::Ok(data) => *data,
        ValidationOutcome::NeedPhoneVerification { chat_ext_id } => {
            return request_phone_verification(&ctx, &meta, &chat_ext_id, &chat_msgs).await;
        }
        ValidationOutcome::InvalidContact { chat_ext_id } => {
            return handle_invalid_contact(&ctx, &meta, &chat_ext_id, &chat_msgs).await;
        }
    };

    if chat_msgs.is_contact() {
        return handle_contact_verification(&ctx, &meta, &mut data, &chat_msgs).await;
    }

    process_regular_message(&ctx, &meta, &mut data, chat_msgs).await
}

#[tracing::instrument(skip_all)]
async fn request_phone_verification(
    ctx: &CoreCtx,
    meta: &Arc<DbBotAccountWithMeta>,
    chat_ext_id: &str,
    chat_msgs: &ChatMessages,
) -> Result<()> {
    tracing::info!("User from chat {} needs phone verification", chat_ext_id);

    let platform = &meta.platform.platform;
    let project_id = meta.project.pkey();

    match platform.api_id {
        ApiId::Vk => {
            // Для VK: генерируем ссылку на OAuth

            let pool = ctx.db().get();
            let oauth = db::core_schema::DbVkOauth::get_by_project_id(project_id, pool).await?;

            // Генерируем уникальный state используя tokio task id
            let task_id = tokio::task::id();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let state = format!("vk_oauth_{}_{}", task_id, timestamp);

            // Сохраняем state в БД
            let new_state = db::core_schema::DbNewVkOauthState::new(
                state.clone(),
                chat_ext_id.to_string(),
                platform.pkey(),
                project_id,
            );
            new_state.insert(pool).await?;

            // Получаем redirect_uri из конфигурации
            let redirect_uri = &ctx.cfg().core().vk_redirect_uri;
            let url = format!(
                "{}/authorize?client_id={}&redirect_uri={}&scope=phone&response_type=code&state={}",
                VK_OAUTH_URL, oauth.app_id, redirect_uri, state
            );

            ctx.chat()
                .send(
                    meta,
                    chat_ext_id,
                    &format!(
                        "Для подтверждения номера телефона перейдите по ссылке: {}",
                        url
                    ),
                    chat_msgs.last_msg_external_id(),
                    None,
                )
                .await
        }
        ApiId::Telegram => {
            // Для Telegram: используем стандартный запрос контакта
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
        x => panic!("Platform {x} is not handled"),
    }
}

#[tracing::instrument(skip_all)]
async fn handle_invalid_contact(
    ctx: &CoreCtx,
    meta: &Arc<DbBotAccountWithMeta>,
    chat_ext_id: &str,
    chat_msgs: &ChatMessages,
) -> Result<()> {
    tracing::warn!("User sent invalid contact (not their own)");

    let reply_markup = ReplyMarkup::Telegram(TelegramKeyboard::request_contact());

    ctx.chat()
        .send(
            meta,
            chat_ext_id,
            "Пожалуйста, поделитесь своим собственным номером телефона, а не чужим контактом.",
            chat_msgs.last_msg_external_id(),
            Some(reply_markup),
        )
        .await
}

#[tracing::instrument(skip_all)]
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
    chat_msgs: ChatMessages,
) -> Result<()> {
    let mut conjoined = String::new();
    for text in chat_msgs.texts() {
        conjoined.push_str(text);
        conjoined.push('\n');
        db_ops::insert_next_user_message(text, data, ctx.db()).await?;
    }

    // Послать разговор в LLM и дождаться ответа.
    let mut full_ticket = data.ticket.clone().get_msgs(ctx.db().get()).await?;
    tracing::info!("Validation complete for chat: {:?}", full_ticket.ticket);
    tracing::trace!("Got ticket: {full_ticket:?}");

    // Если у нас тикет на статусе работы с оператором, то мы всё пересылаем оператору, иначе
    // мы отсылаем всё это дело боту.
    let close_status = data.ticket.close_status;
    if matches!(close_status, DbTicketCloseStatus::EscalationOngoing) {
        let ws_chats = ctx.ws_chats();
        for message in full_ticket.messages.into_iter() {
            ws_chats
                .try_send(full_ticket.ticket.pkey(), message)
                .await?;
        }
        return Ok(());
    };

    let llm_reply = ctx
        .llm()
        .query_llm_with_ticket(&full_ticket, conjoined, chat_msgs.len())
        .await?;

    // Если анализ показывает что надо эскалировать, передаём данные чата в очередь
    // что дает очереди доступ к его данным.
    // ТОDO: A more thorough work-over of the ticket status.
    if escalation_required(&llm_reply) {
        full_ticket.ticket.close_status = DbTicketCloseStatus::EscalationOngoing;
        ctx.queue()
            .insert_ticket(&full_ticket.ticket, &meta.project.project_name, 0)
            .await?;
    }

    // сохранить ответ в БД.
    let next_message = llm_reply.extract_answer();
    tracing::warn!("Message from LLM: {next_message}");

    let msg = db_ops::insert_next_bot_message(&next_message, meta, data, ctx.db()).await?;

    // Послать ответ на чат.
    // TODO: Where do the sending params come from?
    ctx.chat()
        .send_messages(meta, &data.chat, msg, chat_msgs)
        .await
}

/// Эта функция анализирует ответ нейросетки, и решает, нужно ли эскалировать.
fn escalation_required(llm_reply: &LlmReply) -> bool {
    llm_reply.0.forward_to_operator
}

mod db_ops;
