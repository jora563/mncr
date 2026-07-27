//! Сущности пользователя
use db_derive::CoreDbCrud;
use sqlx::{FromRow, PgExecutor, PgPool};

use crate::core_schema::{CoreDbCrud, DbPlatform};
use crate::error::{DbError, Result};

/// Статус учётной записи пользователя.
#[derive(Clone, Copy, Debug, PartialEq, sqlx::Type)]
#[repr(i16)]
#[sqlx(type_name = "SMALLINT")]
pub enum DbAccountStatus {
    /// Новая запись
    New = 0,
    /// Запись проходит проверку. Мы ей не пользуемся.
    /// Переходит в любой статус но не [DbAccountStatus::New]
    VerificationInProgress = 1,
    /// Проверка записи провалена. Mы ей не пользуемся.
    /// От сюда удаляется, наверно.
    VerificationFailed = 2,
    /// Запись прошла проверку. Мы ей пользуемся.
    /// От сюда может идти в [DbAccountStatus::Blacklisted] или [DbAccountStatus::Deleted].
    Verified = 3,
    /// Запись заблокирована. Мы ей не пользуемся.
    /// От сюда может идти в [DbAccountStatus::Verified] или [DbAccountStatus::Deleted].
    BlackListed = 4,
    /// Конечный статус. Мы ей не пользуемся.
    /// Нужен если пользователь удалил учётную запись, но нам до сих пор нужны исторические
    /// сообщения которые с ней связаны.
    Deleted = 5,
}

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

    /// Достать по идентификатору учётной записи.
    #[tracing::instrument(skip_all)]
    pub async fn get_by_account_id<'a, E: PgExecutor<'a>>(id: i64, ex: E) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT *
             FROM \"user\"
             WHERE id = (SELECT user_id FROM user_account WHERE id = $1)",
        )
        .bind(id)
        .fetch_one(ex)
        .await
        .map_err(Into::into)
    }

    /// Достать по номеру телефона
    pub async fn try_get_by_phone<'a, T: sqlx::PgExecutor<'a>>(
        phone: &str,
        ex: T,
    ) -> Result<Option<Self>> {
        Ok(Self::get_by_field("phone", phone, ex).await?.pop())
    }

    /// Обновить номер телефона пользователя.
    pub async fn update_phone<'a, E: PgExecutor<'a>>(&self, new_phone: &str, ex: E) -> Result<()> {
        sqlx::query("UPDATE \"user\" SET phone = $1 WHERE id = $2")
            .bind(new_phone)
            .bind(self.id)
            .execute(ex)
            .await?;
        Ok(())
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
    /// Статус верификации учётной записи. См. [DbAccountStatus]
    pub account_status: DbAccountStatus,
}

impl DbUserAccount {
    #[tracing::instrument(skip_all)]
    pub async fn get_by_external_id<'a, T: sqlx::PgExecutor<'a>>(
        ext_id: &str,
        ex: T,
    ) -> Result<Self> {
        let res = Self::get_by_field("external_id", ext_id, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbUserAccount", "external_id", ext_id))?;
        Ok(res)
    }

    /// Обновить идентификатор пользователя для учётной записи.
    pub async fn update_user_id<'a, E: PgExecutor<'a>>(
        &self,
        new_user_id: i64,
        ex: E,
    ) -> Result<()> {
        sqlx::query("UPDATE user_account SET user_id = $1 WHERE id = $2")
            .bind(new_user_id)
            .bind(self.id)
            .execute(ex)
            .await?;
        Ok(())
    }
}

/// Новая учётная запись пользователя в платформе, которой ещё нет в БД.
#[derive(Clone, Debug)]
pub struct DbNewUserAccount(DbUserAccount);

impl DbNewUserAccount {
    /// Новая учётная запись со статусом "Новый"
    pub fn new(user: &DbUser, platform: &DbPlatform, external_id: &str, alias: &str) -> Self {
        Self::new_with_status(user, platform, external_id, alias, DbAccountStatus::New)
    }

    /// Новая учётная запись с особым статусом
    pub fn new_with_status(
        user: &DbUser,
        platform: &DbPlatform,
        external_id: &str,
        alias: &str,
        account_status: DbAccountStatus,
    ) -> Self {
        Self(DbUserAccount {
            id: 0,
            platform_id: platform.pkey(),
            user_id: user.pkey(),
            external_id: external_id.to_string(),
            alias: alias.to_string(),
            account_status,
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
