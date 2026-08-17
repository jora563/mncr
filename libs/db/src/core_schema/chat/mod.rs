//! Сущности чата
use db_derive::CoreDbCrud;
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{Acquire, FromRow, PgPool, PgTransaction, Postgres};

use crate::core_schema::moma::{self, MoMa};
use crate::core_schema::{CoreDbCrud, DbBotAccount, DbPlatform, DbProject, DbUserAccount};
use crate::error::{DbError, Result};

/// Чат.
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "messenger_chat"]
pub struct DbChat {
    #[core_db_skip_insert]
    id: i64,
    /// Ид чата в системе мессенджера к которому он принадлежит.
    pub external_id: String,
    /// Ид учётной записи пользователя.
    pub user_account_id: i64,
    /// Ид учетной записи бота.
    pub bot_account_id: i64,
    /// Ид проекта с которым чат связан
    pub project_id: i64,
    /// Ид платформы к которой принадлежит чат
    pub platform_id: i64,
    /// Когда чат начат
    pub started_on: PrimitiveDateTime,
    /// Когда последний чат
    pub latest_post_on: Option<PrimitiveDateTime>,
    /// Когда чат закрыт если он закрыт
    pub closed_on: Option<PrimitiveDateTime>,
}

impl DbChat {
    #[tracing::instrument(skip_all)]
    pub async fn get_by_external_id<'a, T: sqlx::PgExecutor<'a>>(
        ext_id: &str,
        ex: T,
    ) -> Result<Self> {
        let res = Self::get_by_field("external_id", ext_id, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbChat", "external_id", ext_id))?;
        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct DbNewChat<'a> {
    chat: DbChat,
    user_account: &'a DbUserAccount,
    bot_account: &'a DbBotAccount,
    project: &'a DbProject,
    platform: &'a DbPlatform,
}

impl<'a> DbNewChat<'a> {
    pub fn new<S: std::fmt::Display>(
        external_id: S,
        user_account: &'a DbUserAccount,
        bot_account: &'a DbBotAccount,
        project: &'a DbProject,
        platform: &'a DbPlatform,
        started_on: PrimitiveDateTime,
    ) -> Self {
        Self {
            chat: DbChat {
                id: 0,
                external_id: external_id.to_string(),
                user_account_id: user_account.pkey(),
                bot_account_id: bot_account.pkey(),
                project_id: project.pkey(),
                platform_id: platform.pkey(),
                started_on,
                latest_post_on: None,
                closed_on: None,
            },
            user_account,
            bot_account,
            project,
            platform,
        }
    }

    /// Валидация билдера, после которой можно вставлять. Правило:
    /// 1. Учётная запись пользователь должна принадлежать проекту.
    /// 2. Учётная запись бота должна принадлежать проекту.
    /// 3. Учётная запись пользователя принадлежит той же платформе что чат.
    /// 4. Учётная запись бота принадлежит той же платформе что чат.
    async fn validate(self, ex: &mut PgTransaction<'a>) -> Result<DbChat> {
        if self.platform.pkey() != self.user_account.platform_id {
            return Err(DbError::IncompatibleUserChatPlatforms(
                self.user_account.external_id.to_string(),
                self.user_account.platform_id,
                self.platform.pkey(),
            ));
        }
        if self.platform.pkey() != self.bot_account.platform_id {
            return Err(DbError::IncompatibleBotChatPlatforms(
                self.bot_account.external_id.to_string(),
                self.bot_account.platform_id,
                self.platform.pkey(),
            ));
        }
        if !moma::DbUserAccountProject::exists(self.user_account, self.project, &mut **ex).await? {
            let msg = format!(
                "User account {} not part of project {}.",
                self.user_account.external_id, self.project.project_name
            );
            return Err(DbError::validation_fail("Messenger Chat", &msg));
        }
        if self.bot_account.project_id != Some(self.project.pkey()) {
            let msg = format!(
                "Bot account {} not part of project {}.",
                self.bot_account.external_id, self.project.project_name
            );
            return Err(DbError::validation_fail("Messenger Chat", &msg));
        }
        Ok(self.chat)
    }

    /// Insert the new ticket.
    pub async fn insert<'l, A>(self, ex: A) -> Result<DbChat>
    where
        A: Acquire<'l, Database = Postgres>,
    {
        let mut tr = ex.begin().await?;

        let mut chat = self.validate(&mut tr).await?;
        chat.insert(&mut *tr).await?;

        tr.commit().await?;
        Ok(chat)
    }
}

impl DbChat {
    pub async fn get_msgs(self, ex: &PgPool) -> Result<DbFullChat> {
        let messages = DbFullMessage::get_for_chat(self.id, ex).await?;

        Ok(DbFullChat {
            chat: self,
            messages,
        })
    }
}

pub struct DbFullChat {
    pub chat: DbChat,
    pub messages: Vec<DbFullMessage>,
}

impl DbFullChat {
    pub async fn get_by_id(id: i64, ex: &PgPool) -> Result<Self> {
        let chat = DbChat::get_by_id(id, ex).await?;
        let messages = DbFullMessage::get_for_chat(id, ex).await?;

        Ok(Self { chat, messages })
    }
}

pub mod message;
pub use message::{DbAttachment, DbFullMessage, DbMessage, DbNewAttachment, DbNewMessage};

#[cfg(test)]
mod tests;
