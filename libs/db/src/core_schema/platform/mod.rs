//! Сущности платформы
use db_derive::CoreDbCrud;
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{FromRow, PgExecutor, PgPool};

use crate::core_schema::CoreDbCrud;
use crate::error::Result;

#[derive(Clone, Copy, Debug, PartialEq, sqlx::Type)]
#[repr(i16)]
#[sqlx(type_name = "SMALLINT")]
pub enum ApiId {
    Vk = 1,
    Telegram = 2,
    Max = 3,
    // TODO: Для начала хватит.
}

/// Платформа, или её инстанция.
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "platform"]
pub struct DbPlatform {
    #[core_db_skip_insert]
    id: i64,
    pub api_id: ApiId,
    /// Наименование платформы.
    pub name: String,
    #[core_db_skip_insert]
    pub created_on: PrimitiveDateTime,
    pub altered_on: Option<PrimitiveDateTime>,
}

/// Новая платформа которой еЩё нет в БД.
#[derive(Clone, Debug)]
pub struct DbNewPlatform(DbPlatform);

impl DbNewPlatform {
    pub fn new<I: Into<ApiId>>(api_id: I, name: &str) -> Self {
        Self(DbPlatform {
            id: 0,
            api_id: api_id.into(),
            name: name.to_string(),
            created_on: PrimitiveDateTime::MIN,
            altered_on: None,
        })
    }

    /// Вставить новую платформу
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbPlatform> {
        let mut p = self.0;
        p.insert(ex).await?;
        Ok(p)
    }
}

impl DbPlatform {
    pub async fn get_mirrors(self, ex: &PgPool) -> Result<DbFullPlatform> {
        Ok(DbFullPlatform {
            mirrors: DbPlatformMirror::get_by_platform_id(self.id, ex).await?,
            platform: self,
        })
    }
}

/// Адреса платформы
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "platform_mirror"]
pub struct DbPlatformMirror {
    #[core_db_id]
    platform_id: i64,
    pub url: String,
    pub note: String,
}

/// Новая платформа которой еЩё нет в БД.
#[derive(Clone, Debug)]
pub struct DbNewPlatformMirror(DbPlatformMirror);

impl DbNewPlatformMirror {
    pub fn new<S: std::fmt::Display>(platform: &DbPlatform, url: S, note: &str) -> Self {
        Self(DbPlatformMirror {
            platform_id: platform.id,
            url: url.to_string(),
            note: note.to_string(),
        })
    }

    /// Вставить новую платформу
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbPlatformMirror> {
        let mut p = self.0;
        p.insert(ex).await?;
        Ok(p)
    }
}

impl DbPlatformMirror {
    pub async fn get_by_platform_id<E>(id: i64, ex: E) -> Result<Vec<Self>>
    where
        E: for<'a> sqlx::PgExecutor<'a>,
    {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM platform_mirror WHERE platform_id = $1 ORDER BY url ASC",
        )
        .bind(id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }
}

/// Платформа с адресами
#[derive(Clone, Debug, PartialEq)]
pub struct DbFullPlatform {
    /// Платформа
    pub platform: DbPlatform,
    /// Список адресов
    pub mirrors: Vec<DbPlatformMirror>,
}

impl DbFullPlatform {
    pub async fn get_by_id(id: i64, ex: &PgPool) -> Result<Self> {
        Ok(DbFullPlatform {
            platform: DbPlatform::get_by_id(id, ex).await?,
            mirrors: DbPlatformMirror::get_by_platform_id(id, ex).await?,
        })
    }
}

#[cfg(test)]
mod tests;
