//! Операции связанные с бд
use db::connect::CoreDbPool;
use db::core_schema::moma::*;
use db::core_schema::*;
use db::error::DbError;

use crate::error::{CoreError, Result};
use crate::messengers::ChatMessages;

#[derive(Debug)]
pub(super) struct StandardData {
    pub(super) user_account: DbUserAccount,
    pub(super) ticket: DbTicket,
    pub(super) chat: DbChat,
}

#[derive(Debug)]
pub(super) enum ValidationOutcome {
    Ok(StandardData),
    NeedPhoneVerification {
        chat_ext_id: String,
    },
}

#[tracing::instrument(skip_all)]
pub(super) async fn check_validity_get_data(
    messages: &ChatMessages,
    meta: &DbBotAccountWithMeta,
    db: &CoreDbPool,
) -> Result<ValidationOutcome> {
    let pool = db.get();
    let user_acc_external_id = messages.user_external_id();
    let chat_ext_id = messages.chat_external_id();
    let phone = messages.phone();
    let user_name = messages.user_name();

    let ba = &meta.account;
    let proj = &meta.project;
    let platform = &meta.platform.platform;

    // Проверяем существование учётной записи
    let ua = match DbUserAccount::get_by_external_id(user_acc_external_id, pool).await {
        Ok(user) => Some(user),
        Err(DbError::NotFound { .. }) => None,
        Err(e) => return Err(e.into()),
    };

    // Получаем или создаём пользователя
    let user = if let Some(ref ua) = ua {
        Some(DbUser::get_by_id(ua.user_id, pool).await?)
    } else if let Some(ref num) = phone {
        DbUser::try_get_by_phone(num, pool).await?
    } else {
        return Ok(ValidationOutcome::NeedPhoneVerification {
            chat_ext_id: chat_ext_id.to_string(),
        });
    };

    let user = match user {
        Some(user) => user,
        None => {
            let phone = phone.as_ref().expect("Phone exists");
            tracing::info!("Creating new user: phone={}, name={}", phone, user_name);
            DbNewUser::new(phone, &user_name).insert(pool).await?
        }
    };

    if !DbProjectUser::exists(proj, &user, pool).await? {
        DbProjectUser::link(proj, &user, pool).await?;
    }

    // Создаём учётную запись если нужно
    let user_account = match ua {
        Some(ua) => ua,
        None => {
            tracing::info!("Creating new user account for user_id={}", user.pkey());
            DbNewUserAccount::new(&user, platform, user_acc_external_id, &user_name)
                .insert(pool)
                .await?
        }
    };

    if !DbUserAccountProject::exists(&user_account, proj, pool).await? {
        DbUserAccountProject::link(&user_account, proj, pool).await?;
    }

    // Создаём чат если нужно
    let chat = match DbChat::get_by_external_id(chat_ext_id, pool).await {
        Ok(chat) => chat,
        Err(DbError::NotFound { .. }) => {
            let now = time::UtcDateTime::now();
            let time = db::PrimitiveDateTime::new(now.date(), now.time());
            DbNewChat::new(chat_ext_id, &user_account, ba, proj, platform, time)
                .insert(pool)
                .await?
        }
        Err(e) => return Err(e.into()),
    };

    // Валидация связей
    if user_account.pkey() != chat.user_account_id {
        return Err(CoreError::ChatValidation(format!(
            "User account mismatch: {} != {}",
            user_account.pkey(),
            chat.user_account_id
        )));
    }
    if ba.pkey() != chat.bot_account_id {
        return Err(CoreError::ChatValidation(format!(
            "Bot account mismatch: {} != {}",
            ba.pkey(),
            chat.bot_account_id
        )));
    }

    // Получаем или создаём тикет
    let ticket = find_or_create_ticket(&chat, messages, ba, db).await?;

    if !DbTicketChat::exists(&ticket, &chat, pool).await? {
        DbTicketChat::link(&ticket, &chat, pool).await?;
    }

    Ok(ValidationOutcome::Ok(StandardData {
        user_account,
        ticket,
        chat,
    }))
}

async fn find_or_create_ticket(
    chat: &DbChat,
    messages: &ChatMessages,
    ba: &DbBotAccount,
    db: &CoreDbPool,
) -> Result<DbTicket> {
    // Ищем открытый тикет
    let existing = DbTicketChat::get_for_chat(chat.pkey(), db.get())
        .await?
        .into_iter()
        .rfind(|x| x.closed_on.is_none() && ba.ticket_not_expired(x));

    if let Some(ticket) = existing {
        return Ok(ticket);
    }

    // Создаём новый тикет
    let msg_text = messages.texts().last().ok_or(CoreError::EmptyChat)?;
    let user = DbUser::get_by_account_id(chat.user_account_id, db.get()).await?;
    let project = DbProject::get_by_id(chat.project_id, db.get()).await?;

    let now = time::UtcDateTime::now();
    let time = db::PrimitiveDateTime::new(now.date(), now.time());

    DbNewTicket::new(&user, &project, msg_text, time)
        .insert(db.get())
        .await
        .map_err(Into::into)
}

#[tracing::instrument(skip_all)]
pub(super) async fn insert_next_bot_message(
    text: &str,
    meta: &DbBotAccountWithMeta,
    data: &mut StandardData,
    db: &CoreDbPool,
) -> Result<DbFullMessage> {
    const FAKE_MSG_EXT_ID: &str = "";
    let bot = &meta.account;

    DbNewMessage::new_bot(bot, 1, FAKE_MSG_EXT_ID, &mut data.chat, &mut data.ticket, text)?
        .insert(db.get())
        .await?
        .get_files(db.get())
        .await
        .map_err(Into::into)
}

#[tracing::instrument(skip_all)]
pub(super) async fn insert_next_user_message(
    text: &str,
    data: &mut StandardData,
    db: &CoreDbPool,
) -> Result<DbFullMessage> {
    const FAKE_MSG_EXT_ID: &str = "";
    let user = &data.user_account;

    DbNewMessage::new_user(user, 1, FAKE_MSG_EXT_ID, &mut data.chat, &mut data.ticket, text)?
        .insert(db.get())
        .await?
        .get_files(db.get())
        .await
        .map_err(Into::into)
}
