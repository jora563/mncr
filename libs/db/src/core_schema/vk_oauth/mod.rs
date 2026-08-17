//! Сущности VK OAuth
use db_derive::CoreDbCrud;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};

use crate::core_schema::CoreDbCrud;
use crate::error::{DbError, Result};

/// OAuth данные для standalone приложения VK.
#[derive(Clone, CoreDbCrud, Debug, Deserialize, FromRow, PartialEq, Serialize)]
#[core_db_table = "vk_oauth"]
pub struct DbVkOauth {
    #[core_db_skip_insert]
    id: i64,
    /// Идентификатор платформы к которой принадлежат данные.
    pub platform_id: i64,
    /// Идентификатор проекта.
    pub project_id: i64,
    /// Идентификатор standalone приложения VK.
    pub app_id: i64,
    /// Секретный ключ приложения.
    pub secure_key: Vec<u8>,
    /// Сервисный токен.
    pub service_token: Vec<u8>,
}

/// Новые данные VK OAuth которых ещё нет в БД.
#[derive(Clone, Debug)]
pub struct DbNewVkOauth(DbVkOauth);

impl DbNewVkOauth {
    /// Создать новые данные VK OAuth до вставления в БД.
    pub fn new(
        platform_id: i64,
        project_id: i64,
        app_id: i64,
        secure_key: Vec<u8>,
        service_token: Vec<u8>,
    ) -> Self {
        Self(DbVkOauth {
            id: 0,
            platform_id,
            project_id,
            app_id,
            secure_key,
            service_token,
        })
    }

    /// Вставить новые данные VK OAuth в БД.
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbVkOauth> {
        let mut oauth = self.0;
        oauth.insert(ex).await?;
        Ok(oauth)
    }
}

impl DbVkOauth {
    /// Достать OAuth данные по идентификатору проекта.
    pub async fn get_by_project_id<E>(id: i64, ex: E) -> Result<Self>
    where
        E: for<'a> PgExecutor<'a>,
    {
        let res = Self::get_by_field("project_id", id, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbVkOauth", "project_id", id))?;
        Ok(res)
    }

    /// Достать OAuth данные по идентификатору платформы.
    pub async fn get_by_platform_id<E>(id: i64, ex: E) -> Result<Self>
    where
        E: for<'a> PgExecutor<'a>,
    {
        let res = Self::get_by_field("platform_id", id, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbVkOauth", "platform_id", id))?;
        Ok(res)
    }

    /// Достать все OAuth данные из базы данных.
    pub async fn get_all<E>(ex: E) -> Result<Vec<Self>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM vk_oauth ORDER BY project_id ASC")
            .fetch_all(ex)
            .await
            .map_err(Into::into)
    }
}

/// Состояние OAuth запроса для получения номера телефона.
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "vk_oauth_state"]
pub struct DbVkOauthState {
    #[core_db_skip_insert]
    id: i64,
    /// Уникальный state для OAuth.
    pub state: String,
    /// Внешний ID пользователя в платформе.
    pub user_ext_id: String,
    /// Идентификатор платформы.
    pub platform_id: i64,
    /// Идентификатор проекта.
    pub project_id: i64,
    /// Время создания.
    pub created_at: time::OffsetDateTime,
}

/// Новое состояние OAuth запроса.
#[derive(Clone, Debug)]
pub struct DbNewVkOauthState(DbVkOauthState);

impl DbNewVkOauthState {
    /// Создать новое состояние.
    pub fn new(state: String, user_ext_id: String, platform_id: i64, project_id: i64) -> Self {
        Self(DbVkOauthState {
            id: 0,
            state,
            user_ext_id,
            platform_id,
            project_id,
            created_at: time::UtcDateTime::now().into(),
        })
    }

    /// Вставить новое состояние в БД.
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbVkOauthState> {
        let mut state = self.0;
        state.insert(ex).await?;
        Ok(state)
    }
}

impl DbVkOauthState {
    /// Достать состояние по его значению.
    pub async fn get_by_state<'a, E: PgExecutor<'a>>(state: &str, ex: E) -> Result<Self> {
        let res = Self::get_by_field("state", state, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbVkOauthState", "state", state))?;
        Ok(res)
    }

    /// Удалить состояние по его значению.
    pub async fn delete_by_state<'a, E: PgExecutor<'a>>(state: &str, ex: E) -> Result<()> {
        sqlx::query("DELETE FROM vk_oauth_state WHERE state = $1")
            .bind(state)
            .execute(ex)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
