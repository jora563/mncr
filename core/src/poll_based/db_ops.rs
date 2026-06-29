//! Операции связанные с бд.
use db::connect::CoreDbPool;
use db::core_schema::moma::*;
use db::core_schema::*;
use db::error::DbError;

use crate::error::{CoreError, Result};
use crate::messengers::ChatMessages;

/// Результат валидации. В зависимости от результата мы или:
/// 1. Прошли, пользователь новый; его надо внести в реестр и создать чат, и тему.
/// 2. Прошли, пользователь есть, чат новый; надо создать чат и тему.
/// 3. Прошли, пользователь и чат есть - возможно надо создать тему.
/// 4. Не прошли — нужна верификация по телефону.
///
/// Данные валидации, которыми мы можем в последствии воспользоваться
/// в дальнейших операциях с этим чатом.
#[derive(Debug)]
pub(super) struct StandardData {
    pub(super) user_account: DbUserAccount,
    user: DbUser,
    bot_account: DbBotAccount,
    project: DbProject,
    pub(super) ticket: DbTicket,
    pub(super) chat: DbChat,
}

/// Исход валидации: либо успех с данными, либо запрос на верификацию телефона.
#[derive(Debug)]
pub(super) enum ValidationOutcome {
    Ok(StandardData),
    /// Пользователь не найден, и в сообщении нет контакта. Нужно попросить отправить контакт.
    NeedPhoneVerification {
        chat_ext_id: String,
        user_ext_id: String,
    },
}

/// Проверить валидность сообщений которые приходят (мы не доверяем пользователю АПИ, и
/// поэтому проверяем всё.)
#[tracing::instrument(skip_all)]
pub(super) async fn check_validity_get_data(
    messages: &ChatMessages,
    meta: &DbBotAccountWithMeta,
    db: &CoreDbPool,
) -> Result<ValidationOutcome> {
    let pool = db.get();
    let user_acc_external_id = messages.get_user_external_id();
    let chat_ext_id = messages.get_chat_external_id();
    let maybe_user_phone = messages.get_phone();
    let user_name = messages.get_user_name();

    // Мы берем бота и проект из метаданных которые нам и так доступны.
    let ba = &meta.account;
    let proj = &meta.project;
    let platform = &meta.platform.platform;

    // если учётной записи пользователя нет, то можно создать.
    let ua = match DbUserAccount::get_by_external_id(user_acc_external_id, pool).await {
        Ok(user) => Some(user),
        Err(DbError::NotFound { .. }) => None,
        Err(e) => return Err(e.into()),
    };

    let user = if let Some(ref ua) = ua {
        Some(DbUser::get_by_id(ua.user_id, pool).await?)
    } else if let Some(ref num) = maybe_user_phone {
        DbUser::try_get_by_phone(num, pool).await?
    } else {
        // Пользователя нет в БД, и мы не знаем его телефон (нет контакта в сообщении).
        // Возвращаем исход "нужна верификация".
        return Ok(ValidationOutcome::NeedPhoneVerification {
            chat_ext_id: chat_ext_id.to_string(),
            user_ext_id: user_acc_external_id.to_string(),
        });
    };

    // Если нет пользователя, создаём запись.
    let user = if let Some(user) = user {
        user
    } else {
        let user_phone = maybe_user_phone.as_ref().expect("Exists");
        let designation = &user_name;
        tracing::info!("Creating new user with phone: {}, name: {}", user_phone, designation);
        DbNewUser::new(user_phone, designation).insert(pool).await?
    };
    if !DbProjectUser::exists(proj, &user, pool).await? {
        DbProjectUser::link(proj, &user, pool).await?;
    }

    // Если нет учётки, создаём запись.
    let user_account = if let Some(ua) = ua {
        ua
    } else {
        tracing::info!("Creating new user account for user_id: {}", user.pkey());
        DbNewUserAccount::new(
            &user,
            platform,
            user_acc_external_id,
            &user_name,
        )
        .insert(pool)
        .await?
    };
    if !DbUserAccountProject::exists(&user_account, proj, pool).await? {
        DbUserAccountProject::link(&user_account, proj, pool).await?;
    }

    // Если нет чата, создаём запись.
    let chat = match DbChat::get_by_external_id(chat_ext_id, pool).await {
        Ok(chat) => chat,
        Err(DbError::NotFound { .. }) => {
            DbNewChat::new(chat_ext_id, &user_account, ba, proj, platform, {
                let now = time::UtcDateTime::now();
                db::PrimitiveDateTime::new(now.date(), now.time())
            })
            .insert(pool)
            .await?
        }
        Err(e) => return Err(e.into()),
    };

    if user_account.pkey() != chat.user_account_id {
        let msg = format!(
            "User account ({}) does not match chat user_account {}",
            user_account.pkey(),
            chat.user_account_id
        );
        return Err(CoreError::ChatValidation(msg));
    } else if ba.pkey() != chat.bot_account_id {
        let msg = format!(
            "Bot account {} does not match chat {}",
            ba.pkey(),
            chat.bot_account_id
        );
        return Err(CoreError::ChatValidation(msg));
    }

    // Достать или в худшем случае создать тикет/тему.
    // Смотрим только на открытые тикеты/темы
    let ticket = if let Some(ticket) = DbTicketChat::get_for_chat(chat.pkey(), db.get())
        .await?
        .into_iter()
        .rfind(|x| x.closed_on.is_none() && ba.ticket_not_expired(x))
    {
        Some(ticket)
    } else if let Some(external_ticket_id) = messages.get_ticket_number() {
        Some(DbTicket::get_by_ticket_no(external_ticket_id, db.get()).await?)
    } else {
        None
    };

    let ticket = match ticket {
        Some(ticket) if ba.ticket_not_expired(&ticket) => ticket,
        _ => create_ticket(&chat, messages, db).await?,
    };

    // Проверка старых заявок. Если заявка старая, создаём новую.

    if !DbTicketChat::exists(&ticket, &chat, pool).await? {
        DbTicketChat::link(&ticket, &chat, pool).await?;
    }

    Ok(ValidationOutcome::Ok(StandardData {
        user_account,
        user,
        bot_account: ba.to_owned(),
        project: proj.to_owned(),
        ticket,
        chat,
    }))
}

#[tracing::instrument(skip_all)]
pub(super) async fn insert_next_bot_message(
    text: &str,
    meta: &DbBotAccountWithMeta,
    data: &mut StandardData,
    db: &CoreDbPool,
) -> Result<DbFullMessage> {
    const FAKE_MSG_EXT_ID: &str = "";
    let chat_meta = &mut data.chat;
    let ticket = &mut data.ticket;
    let bot = &meta.account;
    // TODO: External id probably doesn't need to be added.
    let message = DbNewMessage::new_bot(bot, 1, FAKE_MSG_EXT_ID, chat_meta, ticket, text)
        .inspect_err(|e| tracing::error!("BOT message could not be inserted: {e}"))?
        .insert(db.get())
        .await
        .inspect_err(|e| tracing::error!("BOT message could not be inserted: {e}"))?;

    message.get_files(db.get()).await.map_err(Into::into)
}

#[tracing::instrument(skip_all)]
pub(super) async fn insert_next_user_message(
    text: &str,
    data: &mut StandardData,
    db: &CoreDbPool,
) -> Result<DbFullMessage> {
    const FAKE_MSG_EXT_ID: &str = "";
    let chat_meta = &mut data.chat;
    let ticket = &mut data.ticket;
    let user = &data.user_account;

    // TODO: External id probably doesn't need to be added.
    let message = DbNewMessage::new_user(user, 1, FAKE_MSG_EXT_ID, chat_meta, ticket, text)
        .inspect_err(|e| tracing::error!("USER message could not be inserted: {e}"))?
        .insert(db.get())
        .await
        .inspect_err(|e| tracing::error!("USER message could not be inserted: {e}"))?;

    message.get_files(db.get()).await.map_err(Into::into)
}

/// TODO: Decide whether the ticket number is created locally or remotely. Probably locally.
/// For now we use a mock.
/// TODO2: Decide how to determine the topic of a project.
#[tracing::instrument(skip_all)]
async fn create_ticket(
    chat: &DbChat,
    messages: &ChatMessages,
    db: &CoreDbPool,
) -> Result<DbTicket> {
    let Some(msg_text) = messages.get_text().last() else {
        return Err(CoreError::EmptyChat);
    };
    let user = DbUser::get_by_account_id(chat.user_account_id, db.get()).await?;
    let project = DbProject::get_by_id(chat.project_id, db.get()).await?;

    let now = time::UtcDateTime::now();
    let time = db::PrimitiveDateTime::new(now.date(), now.time());

    let ticket = DbNewTicket::new(&user, &project, msg_text, time);

    ticket.insert(db.get()).await.map_err(Into::into)
}
