//! Сущности тикета/темы
use db_derive::CoreDbCrud;
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{Acquire, FromRow, PgExecutor, PgPool, Postgres};

use crate::core_schema::moma::{DbProjectUser, MoMa};
use crate::core_schema::{CoreDbCrud, DbFullMessage, DbProject, DbUser};
use crate::error::{DbError, Result};

/// Тема/Тикет
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "query_ticket"]
pub struct DbTicket {
    #[core_db_skip_insert]
    id: i64,
    /// Человеко-читабельный номер тикета/темы для пользователя
    pub user_ticket_number: i32,
    /// Ид пользователя.
    pub user_id: i64,
    /// Ид проекта с которым тема связан
    pub project_id: i64,
    /// Статус темы (в основном с каким результатом закрыта)
    pub close_status: i16,
    /// текст темы
    pub topic: String,
    /// Когда тема начата
    pub started_on: PrimitiveDateTime,
    /// Когда последнее сообщение
    pub latest_post_on: Option<PrimitiveDateTime>,
    /// Когда чат закрыт если он закрыт
    pub closed_on: Option<PrimitiveDateTime>,
}

/// Билдер для темы тикета.
#[derive(Debug, Clone, PartialEq)]
pub struct DbNewTicket<'a, 'b> {
    user: &'a DbUser,
    project: &'b DbProject,
    ticket: DbTicket,
}

impl<'a, 'b> DbNewTicket<'a, 'b> {
    pub fn new(
        ticket_no: i32,
        user: &'a DbUser,
        project: &'b DbProject,
        topic: &str,
        started_on: PrimitiveDateTime,
    ) -> Self {
        let ticket = DbTicket {
            id: 0,
            user_ticket_number: ticket_no,
            user_id: user.pkey(),
            project_id: project.pkey(),
            close_status: 0,
            topic: topic.to_string(),
            started_on,
            latest_post_on: None,
            closed_on: None,
        };
        Self {
            ticket,
            user,
            project,
        }
    }

    /// Валидация билдера, после которой можно вставлять. Правило:
    /// 1. Пользователь должен принадлежать проекту.
    async fn validate<'l, E: PgExecutor<'l>>(self, ex: E) -> Result<DbTicket> {
        if !DbProjectUser::exists(self.project, self.user, ex).await? {
            let msg = format!(
                "User {} not part of project {}.",
                self.user.designation, self.project.project_name
            );
            return Err(DbError::validation_fail("Ticket", &msg));
        }
        Ok(self.ticket)
    }

    /// Insert the new ticket.
    pub async fn insert<'l, A>(self, ex: A) -> Result<DbTicket>
    where
        A: Acquire<'l, Database = Postgres>,
    {
        let mut tr = ex.begin().await?;

        let mut ticket = self.validate(&mut *tr).await?;
        ticket.insert(&mut *tr).await?;

        tr.commit().await?;
        Ok(ticket)
    }
}

impl DbTicket {
    pub async fn get_msgs(self, ex: &PgPool) -> Result<DbFullTicket> {
        let messages = DbFullMessage::get_for_ticket(self.id, ex).await?;

        Ok(DbFullTicket {
            ticket: self,
            messages,
        })
    }
}

pub struct DbFullTicket {
    pub ticket: DbTicket,
    pub messages: Vec<DbFullMessage>,
}

impl DbFullTicket {
    pub async fn get_by_id(id: i64, ex: &PgPool) -> Result<Self> {
        let ticket = DbTicket::get_by_id(id, ex).await?;
        let messages = DbFullMessage::get_for_ticket(id, ex).await?;

        Ok(Self { ticket, messages })
    }
}

#[cfg(test)]
mod tests;
