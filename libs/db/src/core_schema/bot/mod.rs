//! Сущности бота
use ahash::AHashMap;
use db_derive::CoreDbCrud;
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{FromRow, PgExecutor, PgPool};

use crate::core_schema::{CoreDbCrud, DbFullPlatform, DbPlatform, DbPlatformMirror, DbTicket};
use crate::error::{DbError, Result};

/// Сущность проекта
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq, Deserialize, Serialize)]
#[core_db_table = "bot_account"]
pub struct DbBotAccount {
    #[core_db_skip_insert]
    id: i64,
    /// Ид. платформы к которой учетная запись бота принадлежит.
    pub platform_id: i64,
    /// Наименование проекта
    pub external_id: String,
    /// Время закрытия заявки для этого определённого бота
    /// (возможно надо чтобы было для проекта)
    pub expiry_time_hours: Option<i64>,
    /// Токен авторизации в учётную запись бота.
    pub token: Vec<u8>, // TODO: Более безопасный тип.
}

#[derive(Clone, Debug)]
pub struct DbNewBotAccount(DbBotAccount);

impl DbNewBotAccount {
    /// Создать новую учётную запись бота до вставления в БД.
    pub fn new<T: Into<String>>(
        platform: &DbPlatform,
        external_id: T,
        ex: Option<i64>,
        token: Vec<u8>,
    ) -> Self {
        Self(DbBotAccount {
            id: 0,
            platform_id: platform.pkey(),
            external_id: external_id.into(),
            expiry_time_hours: ex,
            token,
        })
    }
    /// Вставить новую учётную запись бота
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbBotAccount> {
        let mut bot = self.0;
        bot.insert(ex).await?;
        Ok(bot)
    }
}

impl DbBotAccount {
    pub(super) fn from_tuple(row: (i64, i64, String, Option<i64>, Vec<u8>)) -> Self {
        Self {
            id: row.0,
            platform_id: row.1,
            external_id: row.2,
            expiry_time_hours: row.3,
            token: row.4,
        }
    }

    /// Достань ботов связанные с определённой учётной записью.
    pub async fn get_bots(self, ex: &PgPool) -> Result<DbFullBotAccount> {
        let bots = sqlx::query_as::<_, DbBot>(
            "SELECT * FROM bot WHERE bot_account_id = $1 ORDER BY id ASC",
        )
        .bind(self.id)
        .fetch_all(ex)
        .await?;
        Ok(DbFullBotAccount {
            account: self,
            bots,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_by_external_id<'a, T: sqlx::PgExecutor<'a>>(
        ext_id: &str,
        ex: T,
    ) -> Result<Self> {
        let res = Self::get_by_field("external_id", ext_id, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbBotAccount", "external_id", ext_id))?;
        Ok(res)
    }

    /// Eсли последние сообщение больше чем X часов назад, то мы просрочились.
    /// Если сообщений нет, то судим по полю `started_on`
    /// Если у бота нет срока для тем/тикетов, то тикет живёт вечно.
    #[tracing::instrument(skip_all)]
    pub fn ticket_not_expired(&self, ticket: &DbTicket) -> bool {
        let ticket_last_used = ticket.latest_post_on.unwrap_or(ticket.started_on);
        let h = match self.expiry_time_hours {
            Some(h) => h,
            None => return true,
        };
        // Если у нас None то человечества давно нет...
        let Some(expiry_time) = ticket_last_used.checked_add(time::Duration::hours(h)) else {
            return false;
        };
        expiry_time.as_utc() > time::UtcDateTime::now()
    }
}

/// Описания бота
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq, Serialize)]
#[core_db_table = "bot"]
pub struct DbBot {
    #[core_db_skip_insert]
    id: i64,
    /// Ид учётный записи бота.
    pub bot_account_id: i64,
    /// Не знаю что, наименование наверно
    pub designation: String,
}

#[derive(Clone, Debug)]
pub struct DbNewBot(DbBot);

impl DbNewBot {
    /// Новый бот которого ещё нет в БД.
    pub fn new(account: &DbBotAccount, designation: &str) -> Self {
        Self(DbBot {
            id: 0,
            bot_account_id: account.id,
            designation: designation.to_string(),
        })
    }
    /// Вставить новый бот
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbBot> {
        let mut bot = self.0;
        bot.insert(ex).await?;
        Ok(bot)
    }
}

/// Учётная запись бота с описанием бота.
#[derive(Clone, Debug, PartialEq)]
pub struct DbFullBotAccount {
    pub account: DbBotAccount,
    pub bots: Vec<DbBot>,
}

impl DbFullBotAccount {
    /// Достать учётную запись бота и все связанные боты по основному ИД.
    pub async fn get_by_id(id: i64, ex: &PgPool) -> Result<Self> {
        DbBotAccount::get_by_id(id, ex).await?.get_bots(ex).await
    }

    /// Достать учётную запись бота и все связанные боты ои внешнему Ид.
    #[tracing::instrument(skip_all)]
    pub async fn get_by_external_id(ext_id: &str, ex: &PgPool) -> Result<Self> {
        let bot_account = sqlx::query_as::<_, DbBotAccount>(
            "SELECT * FROM bot_account WHERE external_id = $1 ORDER BY id ASC",
        )
        .bind(ext_id)
        .fetch_one(ex)
        .await?;

        bot_account.get_bots(ex).await
    }

    /// Достать боты для списка платформ.
    /// Возвращает полные записи ботов сортированны по платформе.
    /// TODO: Optimise before production.
    pub async fn get_for_platforms(
        platforms: &[DbFullPlatform],
        ex: &PgPool,
    ) -> Result<AHashMap<i64, Vec<DbFullBotAccount>>> {
        let ids = platforms
            .iter()
            .map(|x| x.platform.pkey())
            .collect::<Vec<_>>();

        let mut accounts = sqlx::query_as::<_, DbBotAccount>(
            "SELECT * FROM bot_account WHERE platform_id = ANY($1) ORDER BY platform_id ASC",
        )
        .bind(&ids)
        .fetch_all(ex)
        .await?;

        let mut bots = sqlx::query_as::<_, DbBot>(
            "SELECT * FROM bot
                WHERE bot_account_id = ANY(
                    SELECT id FROM bot_account WHERE platform_id = ANY($1))
                ORDER BY bot_account_id ASC",
        )
        .bind(&ids)
        .fetch_all(ex)
        .await?;

        let ret = ids
            .into_iter()
            .map(|i| {
                let full_accounts = accounts
                    .extract_if(std::ops::RangeFull, |ac| ac.platform_id == i)
                    .map(|account| {
                        let bots = bots
                            .extract_if(std::ops::RangeFull, |b| b.bot_account_id == account.id)
                            .collect::<Vec<DbBot>>();

                        DbFullBotAccount { account, bots }
                    })
                    .collect::<Vec<DbFullBotAccount>>();

                (i, full_accounts)
            })
            .collect::<AHashMap<_, _>>();

        Ok(ret)
    }
}

use crate::core_schema::DbProject;

/// Сущность Учётной записи бота с платформой которая ему принадлежит.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DbBotAccountWithMeta {
    pub account: DbBotAccount,
    pub platform: DbFullPlatform,
    pub project: DbProject,
}

type MetaRet = (
    i64,
    i64,
    String,
    Option<i64>,
    Vec<u8>,
    i64,
    crate::core_schema::ApiId,
    String,
    PrimitiveDateTime,
    Option<PrimitiveDateTime>,
    i64,
    i64,
    String,
    String,
    PrimitiveDateTime,
    Option<PrimitiveDateTime>,
);

impl DbBotAccountWithMeta {
    /// TODO: Do we need to dedup here? To be decided based on schema.
    pub async fn get_all(ex: &PgPool) -> Result<Vec<Self>> {
        let results = sqlx::query_as::<_, MetaRet>(
            "SELECT b.*, pl.*, proj.* FROM bot_account b
                INNER JOIN platform pl ON b.platform_id = pl.id
                INNER JOIN bot_account_project bap ON bap.account_id = b.id
                INNER JOIN project proj ON proj.id = bap.project_id
                ORDER BY b.id ASC, pl.id",
        )
        .fetch_all(ex);

        let mirrors = sqlx::query_as::<_, DbPlatformMirror>(
            "SELECT * FROM platform_mirror ORDER BY platform_id DESC",
        )
        .fetch_all(ex);

        let (results, mirrors) = tokio::join!(results, mirrors);
        let ret = Self::sort_results(results?, mirrors?);

        Ok(ret)
    }

    /// TODO: Do we need to dedup here? To be decided based on schema.
    pub async fn get_for_project(proj_id: i64, ex: &PgPool) -> Result<Vec<Self>> {
        let results = sqlx::query_as::<_, MetaRet>(
            "SELECT b.*, pl.*, proj.* FROM bot_account b
                INNER JOIN platform pl ON b.platform_id = pl.id
                INNER JOIN bot_account_project bap ON bap.account_id = b.id
                INNER JOIN project proj ON proj.id = bap.project_id AND proj.id = $1
                ORDER BY b.id ASC, pl.id",
        )
        .bind(proj_id)
        .fetch_all(ex);

        let mirrors = sqlx::query_as::<_, DbPlatformMirror>(
            "SELECT * FROM platform_mirror ORDER BY platform_id DESC",
        )
        .fetch_all(ex);

        let (results, mirrors) = tokio::join!(results, mirrors);
        let ret = Self::sort_results(results?, mirrors?);

        Ok(ret)
    }

    fn sort_results(res: Vec<MetaRet>, mirrors: Vec<DbPlatformMirror>) -> Vec<Self> {
        let mut ret = Vec::with_capacity(res.len());
        for (a1, a2, a3, a4, a5, pl1, pl2, pl3, pl4, pl5, p1, p2, p3, p4, p5, p6) in res {
            let platform = DbPlatform::from_tuple((pl1, pl2, pl3, pl4, pl5));
            // Since platforms are repeated, we will clone mirrors.
            let mirrors = mirrors
                .iter()
                .filter(|m| platform.pkey() == m.pkey())
                .cloned()
                .collect::<Vec<DbPlatformMirror>>();
            ret.push(Self {
                account: DbBotAccount::from_tuple((a1, a2, a3, a4, a5)),
                platform: DbFullPlatform { platform, mirrors },
                project: DbProject::from_tuple((p1, p2, p3, p4, p5, p6)),
            });
        }
        ret
    }
}

#[cfg(test)]
mod tests;
