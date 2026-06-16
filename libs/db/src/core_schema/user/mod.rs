//! Сущности пользователя
use db_derive::CoreDbCrud;
use sqlx::{FromRow, PgExecutor, PgPool};

use crate::core_schema::{CoreDbCrud, DbPlatform};
use crate::error::Result;

/// Центральная сущность пользователя
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "\"user\""]
pub struct DbUser {
    #[core_db_skip_insert]
    id: i64,
    /// Номер телефона пользователя: Главный его идентификатор!
    pub phone: String,
    /// Само-десигнация пользователя
    pub designation: String,
}

/// Новый пользователь в платформе, которой ещё нет в БД.
#[derive(Clone, Debug)]
pub struct DbNewUser(DbUser);

impl DbNewUser {
    pub fn new(phone: &str, designation: &str) -> Self {
        Self(DbUser {
            id: 0,
            phone: phone.to_string(),
            designation: designation.to_string(),
        })
    }

    /// Вставление нового пользователя.
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbUser> {
        let mut user = self.0;
        user.insert(ex).await?;
        Ok(user)
    }
}

impl DbUser {
    /// Добыть учётные записи пользователя
    pub async fn get_accounts(self, ex: &PgPool) -> Result<DbFullUser> {
        let accounts = sqlx::query_as::<_, DbUserAccount>(
            "SELECT * FROM user_account WHERE user_id = $1 ORDER BY id ASC",
        )
        .bind(self.id)
        .fetch_all(ex)
        .await?;

        Ok(DbFullUser {
            user: self,
            accounts,
        })
    }
}

/// Учётная запись пользователя в платформе
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "user_account"]
pub struct DbUserAccount {
    #[core_db_skip_insert]
    id: i64,
    /// Ид платформы к которой привязана запись
    pub platform_id: i64,
    /// Ид пользователя к которому привязана запись.
    pub user_id: i64,
    /// Идентификатор пользователя внутри платформы к которой он привязан
    pub external_id: String,
    /// Наименования поль
    pub alias: String,
}

/// Новая учётная запись пользователя в платформе, которой ещё нет в БД.
#[derive(Clone, Debug)]
pub struct DbNewUserAccount(DbUserAccount);

impl DbNewUserAccount {
    pub fn new(user: &DbUser, platform: &DbPlatform, external_id: &str, alias: &str) -> Self {
        Self(DbUserAccount {
            id: 0,
            platform_id: platform.pkey(),
            user_id: user.pkey(),
            external_id: external_id.to_string(),
            alias: alias.to_string(),
        })
    }

    /// Вставить учётную запись.
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbUserAccount> {
        let mut user = self.0;
        user.insert(ex).await?;
        Ok(user)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DbFullUser {
    pub user: DbUser,
    pub accounts: Vec<DbUserAccount>,
}

impl DbFullUser {
    /// Достать пользователя с учётными записями по пользователю.
    pub async fn get_by_id(id: i64, pool: &PgPool) -> Result<Self> {
        DbUser::get_by_id(id, pool).await?.get_accounts(pool).await
    }
}

#[cfg(test)]
mod tests;
