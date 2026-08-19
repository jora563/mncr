//! Сущности слежки за операторами и тикетами с которыми они работают.
//! Также работает как таймер для переподключения оператора при отпавшем соединении.
use db_derive::CoreDbCrud;
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{Acquire, FromRow, PgExecutor, Postgres};

use crate::core_schema::CoreDbCrud;
use crate::error::{DbError, Result};

/// Центральная сущность оператора, которая следит кто, с чем, сейчас работает.
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "last_operator"]
pub struct DbLastOperator {
    #[core_db_skip_insert]
    id: i64,
    /// Внешней ИД оператора. Один оператор может обрабатывать несколько тем.
    pub ext_id: String,
    /// ИД темы. Тема должна в таблице быть уникальной.
    pub last_ticket_id: i64,
    /// Когда данный оператор начал работать с этим тикетом.
    pub work_started: PrimitiveDateTime,
    /// Когда была последняя активность оператора.
    pub last_check_in: PrimitiveDateTime,
    /// Присоединён ли сейчас оператор?
    /// (Возможен вариант не ошибкоустойчивой реализации)
    pub in_work: bool,
}

#[derive(Clone, Debug)]
pub struct DbNewLastOperator(DbLastOperator);

impl DbNewLastOperator {
    pub fn new(ext_id: &str, last_ticket_id: i64) -> Self {
        let now = time::UtcDateTime::now();
        let now = PrimitiveDateTime::new(now.date(), now.time());

        Self(DbLastOperator {
            id: 0,
            ext_id: ext_id.to_string(),
            last_ticket_id,
            work_started: now,
            last_check_in: now,
            in_work: true,
        })
    }

    /// Проверить валидность вставки. Главное правило, это что тикет может обрабатываться
    /// только одним оператором.
    async fn validate<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbLastOperator> {
        let exists = sqlx::query_as::<_, (bool,)>(
            "SELECT count(*) > 0 FROM last_operator WHERE last_ticket_id = $1",
        )
        .bind(self.0.last_ticket_id)
        .fetch_one(ex)
        .await?;

        if exists.0 {
            let msg = format!(
                "Ticket with \"id\" {} is already assigned.",
                self.0.last_ticket_id
            );
            return Err(DbError::validation_fail("Last Operator", &msg));
        }
        Ok(self.0)
    }

    /// Вставить "последнего оператора".
    pub async fn insert<'l, A>(self, ex: A) -> Result<DbLastOperator>
    where
        A: Acquire<'l, Database = Postgres>,
    {
        let mut tr = ex.begin().await?;
        let mut insertable = self.validate(&mut *tr).await?;

        insertable.insert(&mut *tr).await?;
        tr.commit().await?;

        Ok(insertable)
    }
}

impl DbLastOperator {
    /// Достать по тикету
    pub async fn get_by_last_ticket_id<'a, E: PgExecutor<'a>>(
        ticket_id: i64,
        ex: E,
    ) -> Result<Option<Self>> {
        Self::get_by_field("last_ticket_id", ticket_id, ex)
            .await
            .map(|mut x| x.pop())
    }

    /// Достать по внешнем ИД.
    pub async fn try_get_by_ext_id<'a, E: PgExecutor<'a>>(
        ext_id: &str,
        ex: E,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM last_operator WHERE ext_id = $1")
            .bind(ext_id)
            .fetch_optional(ex)
            .await
            .map_err(Into::into)
    }

    /// Обновить `last_check_in` на данный момент время.
    pub async fn update_check_in<'a, E: PgExecutor<'a>>(&mut self, ex: E) -> Result<()> {
        let now = time::UtcDateTime::now();

        self.last_check_in = PrimitiveDateTime::new(now.date(), now.time());
        self.update(ex).await
    }

    /// Начать работу и обновить `last_check_in` на данный момент время.
    pub async fn start_work<'a, E: PgExecutor<'a>>(&mut self, ex: E) -> Result<()> {
        let now = time::UtcDateTime::now();

        self.last_check_in = PrimitiveDateTime::new(now.date(), now.time());
        self.in_work = true;
        self.update(ex).await
    }

    /// Закончить работу и обновить `last_check_in` на данный момент время.
    pub async fn end_work<'a, E: PgExecutor<'a>>(&mut self, ex: E) -> Result<()> {
        let now = time::UtcDateTime::now();

        self.last_check_in = PrimitiveDateTime::new(now.date(), now.time());
        self.in_work = false;
        self.update(ex).await
    }

    /// Удалить последнего оператора если он просрочен.
    /// При этом также снимается ссылка с записей в `queued_ticket`.
    /// TODO: See if this is practical.
    pub async fn delete_older<'l, A>(age_ms: u32, ex: A) -> Result<()>
    where
        A: Acquire<'l, Database = Postgres>,
    {
        let age_ms = time::SignedDuration::milliseconds(age_ms as i64);
        let now = time::UtcDateTime::now();
        let cutoff = PrimitiveDateTime::new(now.date(), now.time()).saturating_sub(age_ms);

        let mut tx = ex.begin().await?;

        let ids = sqlx::query_as::<_, (String,)>(
            "SELECT ext_id FROM last_operator WHERE last_check_in < $1",
        )
        .bind(cutoff)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|x| x.0)
        .collect::<Vec<_>>();

        sqlx::query("UPDATE queued_ticket SET last_operator = NULL WHERE last_operator = ANY($1)")
            .bind(&ids)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM last_operator WHERE last_check_in < $1")
            .bind(cutoff)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    /// Delete the operator(s) working on a given ticket from the queue system.
    pub async fn delete_for_ticket<'a, E: PgExecutor<'a>>(
        ticket_id: i64,
        ext_id: &str,
        ex: E,
    ) -> Result<()> {
        sqlx::query("DELETE FROM last_operator WHERE ext_id = $1 AND ticket_id = $2")
            .bind(ext_id)
            .bind(ticket_id)
            .execute(ex)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
