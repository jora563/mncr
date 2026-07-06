//! Модуль создания и разрыва связи.
use sqlx::PgExecutor;

use super::*;
use crate::core_schema::{
    CoreDbCrud, DbBotAccount, DbChat, DbPlatform, DbProject, DbTicket, DbUser, DbUserAccount,
};
use crate::error::Result;

#[allow(async_fn_in_trait)]
pub trait MoMa {
    type TypeA: CoreDbCrud;
    const FIELD_A: &'static str;
    type TypeB: CoreDbCrud;
    const FIELD_B: &'static str;
    const TABLE: &'static str;

    /// Link two many-to-many features.
    async fn link<E>(a: &Self::TypeA, b: &Self::TypeB, ex: E) -> Result<()>
    where
        E: for<'a> PgExecutor<'a>,
    {
        let query = format!(
            "INSERT INTO {link_table}({field_a}, {field_b}) VALUES($1, $2)",
            link_table = Self::TABLE,
            field_a = Self::FIELD_A,
            field_b = Self::FIELD_B,
        );
        sqlx::query(sqlx::AssertSqlSafe(&query as &str))
            .bind(a.pkey())
            .bind(b.pkey())
            .execute(ex)
            .await?;
        Ok(())
    }

    /// Unlink two many_to_many features
    async fn un_link<E>(a: &Self::TypeA, b: &Self::TypeB, ex: E) -> Result<()>
    where
        E: for<'a> PgExecutor<'a>,
    {
        let query = format!(
            "DELETE FROM {link_table}
                WHERE {field_a} = $1 AND {field_b} = $2",
            link_table = Self::TABLE,
            field_a = Self::FIELD_A,
            field_b = Self::FIELD_B,
        );
        sqlx::query(sqlx::AssertSqlSafe(&query as &str))
            .bind(a.pkey())
            .bind(b.pkey())
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn exists<'a, E>(a: &Self::TypeA, b: &Self::TypeB, ex: E) -> Result<bool>
    where
        E: PgExecutor<'a>,
    {
        let query = format!(
            "SELECT count(*) > 0 FROM {link_table}
                WHERE {field_a} = $1 AND {field_b} = $2",
            link_table = Self::TABLE,
            field_a = Self::FIELD_A,
            field_b = Self::FIELD_B,
        );
        let exists = sqlx::query_as::<_, (bool,)>(sqlx::AssertSqlSafe(&query as &str))
            .bind(a.pkey())
            .bind(b.pkey())
            .fetch_one(ex)
            .await?
            .0;
        Ok(exists)
    }
}

impl MoMa for DbProjectPlatform {
    type TypeA = DbProject;
    const FIELD_A: &'static str = "project_id";
    type TypeB = DbPlatform;
    const FIELD_B: &'static str = "platform_id";
    const TABLE: &'static str = "project_platform";
}

impl MoMa for DbProjectUser {
    type TypeA = DbProject;
    const FIELD_A: &'static str = "project_id";
    type TypeB = DbUser;
    const FIELD_B: &'static str = "user_id";
    const TABLE: &'static str = "project_user";
}

impl MoMa for DbUserAccountProject {
    type TypeA = DbUserAccount;
    const FIELD_A: &'static str = "account_id";
    type TypeB = DbProject;
    const FIELD_B: &'static str = "project_id";
    const TABLE: &'static str = "user_account_project";
}

impl MoMa for DbBotAccountProject {
    type TypeA = DbBotAccount;
    const FIELD_A: &'static str = "account_id";
    type TypeB = DbProject;
    const FIELD_B: &'static str = "project_id";
    const TABLE: &'static str = "bot_account_project";
}

impl MoMa for DbTicketChat {
    type TypeA = DbTicket;
    const FIELD_A: &'static str = "query_ticket_id";
    type TypeB = DbChat;
    const FIELD_B: &'static str = "messenger_chat_id";
    const TABLE: &'static str = "query_ticket_chat";
}
