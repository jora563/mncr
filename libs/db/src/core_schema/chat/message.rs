//! Сущности сообщения
use ahash::AHashMap;
use db_derive::CoreDbCrud;
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{Acquire, FromRow, PgExecutor, PgPool, PgTransaction, Postgres};

use crate::core_schema::moma::{self, MoMa};
use crate::core_schema::{CoreDbCrud, DbBotAccount, DbChat, DbProject, DbTicket, DbUserAccount};
use crate::error::{DbError, Result};

/// Сообщение чата
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "message"]
pub struct DbMessage {
    #[core_db_skip_insert]
    id: i64,
    /// Ид учётной записи пользователя. Если послан ботом то `None`
    pub user_account_id: Option<i64>,
    /// Ид учетной записи бота. Если послан пользователям то `None`
    pub bot_account_id: Option<i64>,
    /// Тип сообщения (направление входящие или выходящие)
    pub r#type: i16,
    /// Десигнация сообщение в системе мессенджера
    pub external_id: String,
    /// Ид чата к которому принадлежит сообщение
    pub messenger_chat_id: i64,
    /// Ид тикета/темы с которой связано сообщение
    pub query_ticket_id: i64,
    /// Само сообщение
    pub content: Option<String>,
    /// Изменено ли
    pub edited: bool,
    /// Удалено ли
    pub deleted: bool,
    /// Когда создан
    #[core_db_skip_insert]
    pub created_on: PrimitiveDateTime,
}

impl DbMessage {
    /// Достать доп файлы для сообщения
    pub async fn get_files(self, ex: &PgPool) -> Result<DbFullMessage> {
        let files = sqlx::query_as::<_, DbAttachment>(
            "SELECT * FROM \"attachment\" WHERE message_id = $1 ORDER BY id ASC",
        )
        .bind(self.id)
        .fetch_all(ex)
        .await?;
        Ok(DbFullMessage {
            message: self,
            files,
        })
    }
}

#[derive(Debug)]
pub struct DbNewMessage<'a> {
    msg: DbMessage,
    user: Option<&'a DbUserAccount>,
    bot: Option<&'a DbBotAccount>,
    chat: &'a mut DbChat,
    ticket: &'a mut DbTicket,
}

impl<'a> DbNewMessage<'a> {
    fn new_inner(
        user_acc: Option<&'a DbUserAccount>,
        bot_acc: Option<&'a DbBotAccount>,
        msg_ty: i16,
        external_id: &str,
        chat: &'a mut DbChat,
        ticket: &'a mut DbTicket,
        content: Option<String>,
    ) -> Self {
        Self {
            msg: DbMessage {
                id: 0,
                user_account_id: user_acc.map(|x| x.pkey()),
                bot_account_id: bot_acc.map(|x| x.pkey()),
                r#type: msg_ty,
                external_id: external_id.to_string(),
                messenger_chat_id: chat.pkey(),
                query_ticket_id: ticket.pkey(),
                content,
                deleted: false,
                edited: false,
                created_on: PrimitiveDateTime::MIN,
            },
            user: user_acc,
            bot: bot_acc,
            chat,
            ticket,
        }
    }

    /// Новое сообщение без контента, наверно с файлом?
    /// Новое сообщение. Требует следующие проверки:
    /// 1. Пользователь, бот и чат все на той-же платформе.
    /// 2. Пользователь и бот принадлежат тому же чату.
    /// 3. Пользователь и тема/тикет доступны той же группе. (Проверяется в валидаторе)
    pub fn new_empty_user(
        user_acc: &'a DbUserAccount,
        msg_ty: i16,
        external_id: &str,
        chat: &'a mut DbChat,
        ticket: &'a mut DbTicket,
    ) -> Result<Self> {
        Self::validate_wo_db(
            user_acc.platform_id,
            &user_acc.alias,
            user_acc.pkey(),
            false,
            chat,
        )?;
        Ok(Self::new_inner(
            Some(user_acc),
            None,
            msg_ty,
            external_id,
            chat,
            ticket,
            None,
        ))
    }

    /// Новое сообщение с текстом.
    /// Новое сообщение. Требует следующие проверки:
    /// 1. Пользователь, бот и чат все на той-же платформе.
    /// 2. Пользователь и бот принадлежат тому же чату.
    /// 3. Пользователь и тема/тикет доступны той же группе. (Проверяется в валидаторе)
    pub fn new_user(
        user_acc: &'a DbUserAccount,
        msg_ty: i16,
        external_id: &str,
        chat: &'a mut DbChat,
        ticket: &'a mut DbTicket,
        c: &str,
    ) -> Result<Self> {
        Self::validate_wo_db(
            user_acc.platform_id,
            &user_acc.alias,
            user_acc.pkey(),
            false,
            chat,
        )?;
        Ok(Self::new_inner(
            Some(user_acc),
            None,
            msg_ty,
            external_id,
            chat,
            ticket,
            Some(c.to_string()),
        ))
    }

    /// Новое сообщение с текстом.
    /// Новое сообщение. Требует следующие проверки:
    /// 1. Пользователь, бот и чат все на той-же платформе.
    /// 2. Пользователь и бот принадлежат тому же чату.
    /// 3. Пользователь и тема/тикет доступны той же группе. (Проверяется в валидаторе)
    pub fn new_bot(
        bot_acc: &'a DbBotAccount,
        msg_ty: i16,
        external_id: &str,
        chat: &'a mut DbChat,
        ticket: &'a mut DbTicket,
        c: &str,
    ) -> Result<Self> {
        Self::validate_wo_db(
            bot_acc.platform_id,
            &bot_acc.external_id,
            bot_acc.pkey(),
            true,
            chat,
        )?;
        Ok(Self::new_inner(
            None,
            Some(bot_acc),
            msg_ty,
            external_id,
            chat,
            ticket,
            Some(c.to_string()),
        ))
    }
    fn validate_wo_db(
        user_platform_id: i64,
        user_designation: &str,
        user_id: i64,
        user_is_bot: bool,
        chat: &DbChat,
    ) -> Result<()> {
        if user_platform_id != chat.platform_id && user_is_bot {
            Err(DbError::IncompatibleBotChatPlatforms(
                user_designation.to_string(),
                user_platform_id,
                chat.platform_id,
            ))
        } else if user_platform_id != chat.platform_id {
            Err(DbError::IncompatibleUserChatPlatforms(
                user_designation.to_string(),
                user_platform_id,
                chat.platform_id,
            ))
        } else if user_id != chat.bot_account_id && user_is_bot {
            Err(DbError::AlienBot(user_designation.to_string(), chat.pkey()))
        } else if user_id != chat.user_account_id && !user_is_bot {
            Err(DbError::AlienUser(
                user_designation.to_string(),
                chat.pkey(),
            ))
        } else {
            Ok(())
        }
    }

    /// Валидация билдера, после которой можно вставлять. Данные правила пройдены заранее:
    /// - Пользователь, бот и чат все на той-же платформе.
    /// - Пользователь и бот принадлежат тому же чату.
    ///
    /// Данные правила проходятся здесь:
    /// 1. Учётная запись бота доступна проектной группе тикета/темы.
    /// 2. Учётная запись пользователя доступна проектной группе тикета/темы.
    /// 3. Чат относится к тем которые включены в тикет.
    async fn validate(
        self,
        ex: &mut PgTransaction<'a>,
    ) -> Result<(DbMessage, &'a mut DbChat, &'a mut DbTicket)> {
        let project = DbProject::get_by_id(self.ticket.project_id, &mut **ex)
            .await
            .unwrap();
        if let Some(user) = self.user
            && !moma::DbUserAccountProject::exists(user, &project, &mut **ex).await?
        {
            let msg = format!(
                "User account {} not part of project {}.",
                user.external_id, project.project_name
            );
            return Err(DbError::validation_fail("Chat Message", &msg));
        }
        if let Some(bot) = self.bot
            && bot.project_id != Some(project.pkey())
        {
            let msg = format!(
                "Bot account {} not part of project {}.",
                bot.external_id, project.project_name
            );
            return Err(DbError::validation_fail("Chat Message", &msg));
        }
        if !moma::DbTicketChat::exists(self.ticket, self.chat, &mut **ex).await? {
            let msg = format!(
                "Chat {} not part of Ticket {}.",
                self.chat.pkey(),
                self.ticket.user_ticket_number
            );
            return Err(DbError::validation_fail("Chat Message", &msg));
        }
        Ok((self.msg, self.chat, self.ticket))
    }

    /// Insert the new ticket.
    pub async fn insert<'l, A>(self, ex: A) -> Result<DbMessage>
    where
        A: Acquire<'l, Database = Postgres>,
    {
        let mut tr = ex.begin().await?;

        let (mut msg, chat, ticket) = self.validate(&mut tr).await?;
        msg.insert(&mut *tr).await?;

        chat.latest_post_on = Some(msg.created_on);
        ticket.latest_post_on = Some(msg.created_on);

        chat.update(&mut *tr).await?;
        ticket.update(&mut *tr).await?;

        tr.commit().await?;
        Ok(msg)
    }
}

/// Описание файла прикреплённого к сообщению
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "attachment "]
pub struct DbAttachment {
    #[core_db_skip_insert]
    id: i64,
    /// Id сообщения к которому принадлежи
    pub message_id: i64,
    /// Тип: Файл или образ
    pub r#type: i16,
    /// Десигнация в системе мессенджера
    pub external_id: String,
    /// УРЛ по которому достать файл в платформе
    /// TODO: Is this enough?
    pub file_url: String,
    /// Теоретический размер файла.
    pub file_size: i64,
    /// Когда создан
    #[core_db_skip_insert]
    pub created_on: PrimitiveDateTime,
}

/// Новый файл о которым бд ещё не знает.
#[derive(Clone, Debug)]
pub struct DbNewAttachment(DbAttachment);

impl DbNewAttachment {
    pub fn new<T: std::fmt::Display>(
        msg: &DbMessage,
        tpe: i16,
        external_id: &str,
        url: T,
        size: i64,
    ) -> Self {
        Self(DbAttachment {
            id: 0,
            message_id: msg.id,
            r#type: tpe,
            external_id: external_id.to_string(),
            file_url: url.to_string(),
            file_size: size,
            created_on: PrimitiveDateTime::MIN,
        })
    }

    /// Вставить новый файл
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbAttachment> {
        let mut att = self.0;
        att.insert(ex).await?;
        Ok(att)
    }
}

/// Чат и его сообщения
#[derive(Clone, Debug, PartialEq)]
pub struct DbFullMessage {
    /// Платформа
    pub message: DbMessage,
    /// Список адресов
    pub files: Vec<DbAttachment>,
}

impl DbFullMessage {
    /// Достать все полные сообщения из какого-то чата.
    pub async fn get_for_chat(q_id: i64, ex: &PgPool) -> Result<Vec<DbFullMessage>> {
        let messages = sqlx::query_as::<_, DbMessage>(
            "SELECT * FROM \"message\" WHERE messenger_chat_id=$1 ORDER BY created_on ASC",
        )
        .bind(q_id)
        .fetch_all(ex)
        .await?;
        let attachments = sqlx::query_as::<_, DbAttachment>(
            "SELECT * FROM \"attachment\" WHERE message_id = ANY(SELECT id FROM message WHERE messenger_chat_id=$1)"
        )
            .bind(q_id)
            .fetch_all(ex)
            .await?;
        Ok(Self::from_many(messages, attachments))
    }

    /// Достать все полные сообщения по какой-то теме.
    pub async fn get_for_ticket(q_id: i64, ex: &PgPool) -> Result<Vec<DbFullMessage>> {
        Self::get_history(q_id, None, None, ex).await
    }

    /// Достать историю сообщений начиная с определенного сообщения. Также ограничить по числу.
    pub async fn get_history(
        ticket_id: i64,
        last_message_id: Option<i64>,
        count: Option<u32>,
        ex: &PgPool,
    ) -> Result<Vec<DbFullMessage>> {
        // Create extra clauses.
        let date_clause = match last_message_id {
            Some(_) => " AND id > $2",
            None => "",
        };
        let count_clause = match count {
            Some(_) => " LIMIT $3",
            None => "",
        };

        // Create main query.
        let mut query = sqlx::query_as::<_, DbMessage>(sqlx::AssertSqlSafe(format!(
            "SELECT * FROM \"message\"
                WHERE query_ticket_id=$1{date_clause}
                ORDER BY created_on ASC{count_clause}",
        )))
        .bind(ticket_id);

        // Bind extra variables.
        if let Some(id) = last_message_id {
            query = query.bind(id);
        };
        if let Some(count) = count {
            query = query.bind(count as i64);
        };

        let messages = query.fetch_all(ex).await?;
        // We get more attachments than we need, but `from_many` should sort them correctly.
        let attachments = sqlx::query_as::<_, DbAttachment>(
            "SELECT * FROM \"attachment\" WHERE message_id = ANY(SELECT id FROM message WHERE query_ticket_id=$1)"
        )
            .bind(ticket_id)
            .fetch_all(ex)
            .await?;
        Ok(Self::from_many(messages, attachments))
    }

    /// Предполагаем что материал правильный
    fn from_many(msg: Vec<DbMessage>, att: Vec<DbAttachment>) -> Vec<Self> {
        let mut att_map = AHashMap::new();
        att.into_iter().for_each(|a| {
            att_map.entry(a.message_id).or_insert_with(Vec::new).push(a);
        });
        msg.into_iter()
            .map(|m| DbFullMessage {
                files: att_map.remove(&m.id).unwrap_or_default(),
                message: m,
            })
            .collect()
    }
}
