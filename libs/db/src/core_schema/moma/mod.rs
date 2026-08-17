//! Модуль "Model Management". Содержит связочные сущности и правила между таблицами
//! которые имеют более сложные связи чем обычно.
//! На данный момент сущности связи относятся к таблицам:
//!
//! - DbTicket-DbChat
//! - DbProject-DbPlatform
//! - DbProject-DbBotAccount
//! - DbProject-DbUserAccount
//! - DbProject-DbUser
//!
//! Как видно, в большинстве случаев связи много-к-многим связанны именно с проектами.
use sqlx::PgExecutor;

use crate::core_schema::{DbChat, DbPlatform, DbProject, DbTicket, DbUser, DbUserAccount};
use crate::error::Result;

/// Условная связь между платформой и проектом
#[derive(Clone, Debug)]
pub struct DbProjectPlatform;

/// Условная связь между пользователем и проектом
#[derive(Clone, Debug)]
pub struct DbProjectUser;

/// Условная связь между проектом и учётной записью пользователя
#[derive(Clone, Debug)]
pub struct DbUserAccountProject;

///Условная связь между темой/тикетом и чатом
#[derive(Clone, Debug)]
pub struct DbTicketChat;

impl DbTicketChat {
    /// Достать все проекты связаны с данной платформой.
    pub async fn get_for_chat<E>(chat_id: i64, ex: E) -> Result<Vec<DbTicket>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbTicket>(
            "SELECT * FROM query_ticket
                WHERE id = ANY(SELECT query_ticket_id FROM query_ticket_chat WHERE messenger_chat_id = $1)
                ORDER BY started_on ASC",
        )
        .bind(chat_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }

    /// Достать все платформы связаны с данным проектом
    pub async fn get_for_ticket<E>(ticket_id: i64, ex: E) -> Result<Vec<DbChat>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbChat>(
            "SELECT * FROM messenger_chat
                WHERE id = ANY(SELECT messenger_chat_id FROM query_ticket_chat WHERE query_ticket_id = $1)
                ORDER BY started_on ASC",
        )
        .bind(ticket_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }
}

impl DbUserAccountProject {
    /// Достать все проекты связаны с данной платформой.
    pub async fn get_for_account<E>(account_id: i64, ex: E) -> Result<Vec<DbProject>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbProject>(
            "SELECT * FROM project
                WHERE id = ANY(SELECT project_id FROM user_account_project WHERE account_id = $1)
                ORDER BY created_on ASC",
        )
        .bind(account_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }

    /// Достать все платформы связаны с данным проектом
    pub async fn get_for_project<E>(project_id: i64, ex: E) -> Result<Vec<DbUserAccount>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbUserAccount>(
            "SELECT * FROM user_account
                WHERE id = ANY(SELECT account_id FROM user_account_project WHERE project_id = $1)
                ORDER BY id ASC",
        )
        .bind(project_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }
}

impl DbProjectUser {
    /// Достать все проекты связаны с данной платформой.
    pub async fn get_for_user<E>(user_id: i64, ex: E) -> Result<Vec<DbProject>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbProject>(
            "SELECT * FROM project
                WHERE id = ANY(SELECT project_id FROM project_user WHERE user_id = $1)
                ORDER BY created_on ASC",
        )
        .bind(user_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }

    /// Достать все платформы связаны с данным проектом
    pub async fn get_for_project<E>(project_id: i64, ex: E) -> Result<Vec<DbUser>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbUser>(
            "SELECT * FROM \"user\"
                WHERE id = ANY(SELECT user_id FROM project_user WHERE project_id = $1)
                ORDER BY id ASC",
        )
        .bind(project_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }
}

impl DbProjectPlatform {
    /// Достать все проекты связаны с данной платформой.
    pub async fn get_for_platform<E>(platform_id: i64, ex: E) -> Result<Vec<DbProject>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbProject>(
            "SELECT * FROM project
                WHERE id = ANY(SELECT project_id FROM project_platform WHERE platform_id = $1)
                ORDER BY created_on ASC",
        )
        .bind(platform_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }

    /// Достать все платформы связаны с данным проектом
    pub async fn get_for_project<E>(project_id: i64, ex: E) -> Result<Vec<DbPlatform>>
    where
        E: for<'a> PgExecutor<'a>,
    {
        sqlx::query_as::<_, DbPlatform>(
            "SELECT * FROM platform
                WHERE id = ANY(SELECT platform_id FROM project_platform WHERE project_id = $1)
                ORDER BY created_on ASC",
        )
        .bind(project_id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }
}

pub mod link_unlink;
pub use link_unlink::MoMa;

#[cfg(test)]
mod tests;
