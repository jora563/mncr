//! Сущности бота
use db_derive::CoreDbCrud;
use sqlx::{FromRow, PgExecutor, PgPool};

use crate::core_schema::{CoreDbCrud, DbPlatform};
use crate::error::Result;

/// Сущность проекта
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "bot_account"]
pub struct DbBotAccount {
    #[core_db_skip_insert]
    id: i64,
    /// Ид. платформы к которой учетная запись бота принадлежит.
    pub platform_id: i64,
    /// Наименование проекта
    pub external_id: String,
    /// Токен авторизации в учётную запись бота.
    pub token: Vec<u8>, // TODO: Более безопасный тип.
}

#[derive(Clone, Debug)]
pub struct DbNewBotAccount(DbBotAccount);

impl DbNewBotAccount {
    /// Создать новую учётную запись бота до вставления в БД.
    pub fn new(platform: &DbPlatform, external_id: &str, token: Vec<u8>) -> Self {
        Self(DbBotAccount {
            id: 0,
            platform_id: platform.pkey(),
            external_id: external_id.to_string(),
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
}

/// Описания бота
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
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
    pub async fn get_by_external_id(ext_id: &str, ex: &PgPool) -> Result<Self> {
        let bot_account = sqlx::query_as::<_, DbBotAccount>(
            "SELECT * FROM bot_account WHERE external_id = $1 ORDER BY id ASC",
        )
        .bind(ext_id)
        .fetch_one(ex)
        .await?;

        bot_account.get_bots(ex).await
    }
}

#[cfg(test)]
mod tests;
