//! Этот модуль заведут очередью тикетов/тем на передачу операторам.
//! При этом, очередь которая передана операторам не исчезает из очереди,
//! Она там остаётся но в изменённым статусе. Только если статус темы меняется
//! на начальную (обработка LLM-кой) или конечную (тема закрыта), то она
//! удаляется из очереди.
//!
//! Алгоритм выборки тикетов скорее всего будет совершенствоваться.
use db_derive::CoreDbCrud;
use serde::{Deserialize, Serialize};
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{Acquire, FromRow, PgExecutor, Postgres};

use crate::core_schema::CoreDbCrud;
use crate::error::Result;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, sqlx::Type, Serialize)]
#[repr(i16)]
#[sqlx(type_name = "SMALLINT")]
pub enum DbQueuedTicketStatus {
    Queued = 0,
    InWork = 1,
}

/// Центральная сущность оператора, которая следит кто, с чем, сейчас работает.
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "queued_ticket"]
pub struct DbQueuedTicket {
    #[core_db_id]
    ticket_id: i64,
    project_name: String,
    /// Есть ли последний оператор? Если есть, новым не дают.
    pub last_operator: Option<String>,
    /// Время когда поставлено в очередь.
    added_to_queue: PrimitiveDateTime,
    /// Уровень VIP-ности. Чем выше, тем VIP-нее.
    /// Уровень випности увеличивается у всех кого не выбрали, каждый раз.
    /// Тикеты которые в работе и без операторов самые приоритетные.
    vip_level: i64,
    /// Статус
    pub ticket_status: DbQueuedTicketStatus,
}

#[derive(Clone, Debug)]
pub struct DbNewQueuedTicket(DbQueuedTicket);

impl DbNewQueuedTicket {
    pub fn new(ticket_id: i64, project_name: &str, vip_level: i64) -> Self {
        let now = time::UtcDateTime::now();
        let now = PrimitiveDateTime::new(now.date(), now.time());

        Self(DbQueuedTicket {
            ticket_id,
            project_name: project_name.to_string(),
            last_operator: None,
            added_to_queue: now,
            vip_level,
            ticket_status: DbQueuedTicketStatus::Queued,
        })
    }

    /// Вставить новый билет в очередь. Tут всё просто
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbQueuedTicket> {
        let mut insertable = self.0;

        insertable.insert(ex).await?;
        Ok(insertable)
    }
}

impl DbQueuedTicket {
    /// Достать по последнем оператором.
    pub async fn try_get_by_operator<'a, E: PgExecutor<'a>>(
        ext_id: &str,
        ex: E,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM queued_ticket WHERE last_operator = $1")
            .bind(ext_id)
            .fetch_optional(ex)
            .await
            .map_err(Into::into)
    }

    /// Достать по последнем оператором.
    pub async fn try_get_last_for_operator<'a, E: PgExecutor<'a>>(
        ext_id: &str,
        ex: E,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT *
                FROM queued_ticket
                WHERE ticket_id = (
                    SELECT last_ticket_id
                        FROM last_operator
                        WHERE ext_id = $1
                        ORDER BY last_check_in DESC
                        LIMIT 1
                )",
        )
        .bind(ext_id)
        .fetch_optional(ex)
        .await
        .map_err(Into::into)
    }

    /// Достать следующий билет из очереди. При этом, если билет попался на удочку,
    // то все остальные подвинуть вперёд.
    pub async fn get_next<'l, A>(
        operator_ext: &str,
        permitted_projects: &[String],
        ex: A,
    ) -> Result<Option<Self>>
    where
        A: Acquire<'l, Database = Postgres>,
    {
        let mut tr = ex.begin().await?;

        // Complex condition. EITHER:
        // 1. No operator.
        // 2. There is a previous operator, but they are not logged in.
        //
        // In either case we try to retrieve tickets that were previously worked by this operator
        let mut next = sqlx::query_as::<_, Self>(
            "SELECT * FROM queued_ticket q
                WHERE project_name = ANY($2) AND (
                    last_operator IS NULL
                    OR (SELECT count(*) FROM last_operator WHERE last_ticket_id = q.ticket_id) = 0
                )
                ORDER BY ticket_status, last_operator = $1, vip_level DESC, added_to_queue ASC
                LIMIT 1",
        )
        .bind(operator_ext)
        .bind(permitted_projects)
        .fetch_optional(&mut *tr)
        .await?;

        // если мы выбрали билет, то надо проставить что он в работе, и увеличить приоритет остальных.
        if let Some(ref mut next) = next {
            next.assign_to_operator(operator_ext, &mut *tr).await?;

            sqlx::query(
                "UPDATE queued_ticket SET vip_level = vip_level + 1 WHERE ticket_status = 0",
            )
            .execute(&mut *tr)
            .await?;
        }
        tr.commit().await?;

        Ok(next)
    }

    /// Отдать тикет в очереди оператору.
    pub async fn assign_to_operator<'a, E: PgExecutor<'a>>(
        &mut self,
        operator_ext: &str,
        ex: E,
    ) -> Result<()> {
        self.ticket_status = DbQueuedTicketStatus::InWork;
        self.last_operator = Some(operator_ext.to_string());
        self.update(ex).await?;

        Ok(())
    }

    /// Вернуть в очередь.
    pub async fn return_to_queue<'a, E: PgExecutor<'a>>(&mut self, ex: E) -> Result<()> {
        self.last_operator = None;
        self.update(ex).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
