//! Функционал очереди который работает как отдельный функционал.
//! Tут должен быть базовый функционал очереди:
//!
//! - Добавить тикет в очередь когда он переходит по статусу на ручную обработку.
//! - Достать следующий тикет из очереди (и промаркировать в очереди).
//! - Убрать тикет из очереди когда он уходит со статусов ручной обработки.
//! - Добавить связку оператора-тикета при начале работы с тикетом.
//! - Проверять и утверждать актуальность связки оператора-тикета.
//! - Убрать связку оператора-тикета когда работа завершена.
//! - Убрать просроченные связки оператора-тикета.
//!
//! Функционал собирается при присутствии feature-flag `intrinsic`.

use db::core_schema::{CoreDbCrud, DbTicket, DbTicketCloseStatus};
use db::queue_schema::{DbLastOperator, DbNewLastOperator};
use db::queue_schema::{DbNewQueuedTicket, DbQueuedTicket};

use crate::error::Result;
use crate::queue::*;

impl Queue {
    /// Добавить тикет в очередь когда он переходит по статусу на ручную обработку.
    /// TODO: Check if this function is practical.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `ticket: &db::core_schema::DbTicket`: Тикет который надо вставить в очередь.
    /// - `project: &str`: Наименование проекта к которому принадлежит тикет.
    /// - `vip: i64`: Уровень ВИП-ности тикета. Чем выше тем более важный. Ставите 0 если обычный тикет.
    ///
    /// __Returns:__
    /// - `Result<DbQueuedTicket>`: Запись тикета из очереди.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn insert_ticket(
        &self,
        ticket: &DbTicket,
        project: &str,
        vip: i64,
    ) -> Result<DbQueuedTicket> {
        let pool = self.db().get();
        let queued_ticket = DbNewQueuedTicket::new(ticket.pkey(), project, vip)
            .insert(pool)
            .await?;

        Ok(queued_ticket)
    }
    /// Убрать тикет из очереди когда он уходит со статусов ручной обработки.
    /// TODO: Check if this function is practical.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `ticket: &db::core_schema::DbTicket`: Тикет который надо обновить/убрать из очереди.
    ///
    /// __Returns:__
    /// - `Result<()>`: Ок если успешно, или ошибка если операция не успешна.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn update_ticket_in_queue(&self, ticket: &DbTicket) -> Result<()> {
        if matches!(ticket.close_status, DbTicketCloseStatus::EscalationOngoing) {
            return Ok(());
        }
        let mut tx = self.db().get().begin().await?;

        // Если статус больше не ручной то му удаляем запись из очереди, и её обработчика.
        #[allow(clippy::collapsible_if)]
        if let Some(queued) = DbQueuedTicket::try_get_by_id(ticket.pkey(), &mut *tx).await? {
            if let Some(ref ext) = queued.last_operator {
                DbLastOperator::delete_for_ticket(ticket.pkey(), ext, &mut *tx).await?;
                queued.delete(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Достать следующий тикет из очереди (и промаркировать в очереди).
    /// Если оператор существует в БД, берём последний его тикет если он существует.
    /// Если он не существует, ищем последний его тикет.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `operator_ext: &str`: Внешней идентификатор оператора.
    /// - `permitted_projects: &[String]`: Наименования доступных проектов для операторов.
    ///
    /// __Returns:__
    /// - `Result<Option<(DbQueuedTicket, DbLastOperator)>>`: При успешной операции `Ok`,
    ///   иначе ошибки. Если есть доступный тикеты в очереди, забирает тикет, и создаёт запись
    ///   оператора. Если нет доступного тикета, возвращает `None`.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn get_next_for_operator(
        &self,
        operator_ext: &str,
        permitted_projects: &[String],
    ) -> Result<Option<(DbQueuedTicket, DbLastOperator)>> {
        let pool = self.db().get();
        let mut tx = pool.begin().await?;

        // Если оператора нет, то ищем свободный билет, и работаем с ним.
        if let Some(queued_ticket) =
            DbQueuedTicket::get_next(operator_ext, permitted_projects, pool).await?
        {
            let operator = DbNewLastOperator::new(operator_ext, queued_ticket.pkey())
                .insert(&mut *tx)
                .await?;

            tx.commit().await?;
            return Ok(Some((queued_ticket, operator)));
        }
        Ok(None)
    }

    /// Достать последний тикет из очереди (и промаркировать в очереди) с которым этот
    /// оператор работал, если он ещё в работе этого оператора.
    /// Если оператора нет в БД, или нет открытых чатов для него, то он возвращает ниц.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `operator_ext: &str`: Внешней идентификатор оператора.
    ///
    /// __Returns:__
    /// - `Result<Option<(DbQueuedTicket, DbLastOperator)>>`: При успешной операции `Ok`,
    ///   иначе ошибки. Если есть доступный тикеты в очереди, забирает тикет, и создаёт запись
    ///   оператора. Если нет доступного тикета, возвращает `None`.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn restore_for_operator(
        &self,
        operator_ext: &str,
    ) -> Result<Option<(DbQueuedTicket, DbLastOperator)>> {
        let pool = self.db().get();
        let mut tx = pool.begin().await?;

        // Если оператора нет, то ищем свободный билет, и работаем с ним.
        let last = DbQueuedTicket::try_get_last_for_operator(operator_ext, &mut *tx).await?;

        if let Some(mut ticket) = last {
            let Some(mut op) =
                DbLastOperator::get_by_last_ticket_id(ticket.pkey(), &mut *tx).await?
            else {
                return Ok(None);
            };
            op.start_work(&mut *tx).await?;
            ticket.assign_to_operator(operator_ext, &mut *tx).await?;

            tx.commit().await?;

            return Ok(Some((ticket, op)));
        }
        Ok(None)
    }

    /// Проверить доступен ли этот тикет для оператора. Для этого у темы не должно
    /// быть других операторов, и он должен быть в очереди.
    /// - Если тикет доступен, возвращаем статус тикета.
    /// - Если тикет недоступен, возвращаем пусто.
    ///
    /// Тикет доступен если он:
    /// - Есть в очереди,
    /// - И он не привязан к другому оператору.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `operator_ext: &str`: Внешней идентификатор оператора.
    /// - `ticket_id: i64`: Внутренний идентификатор тикета
    ///
    /// __Returns:__
    /// - `Result<Option<DbQueuedTicket>>`: При успешной операции `Ok`, иначе ошибки.
    ///   Если тикет доступен оператору, возвращает тикет, иначе возвращает `None`.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn ticket_is_available_for_operator(
        &self,
        operator_ext: &str,
        ticket_id: i64,
    ) -> Result<Option<DbQueuedTicket>> {
        let pool = self.db().get();
        let queued = match DbQueuedTicket::get_by_id(ticket_id, pool).await {
            Ok(t) => t,
            Err(e) if e.is_not_found() => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        if queued.last_operator.is_some()
            && queued.last_operator.as_ref().map(|x| x as &str) != Some(operator_ext)
        {
            return Ok(None);
        }
        Ok(Some(queued))
    }

    /// Привязать тикет к определённому оператору. Это не безопасная операция, и не должна проводится
    /// без надлежащих проверок.
    ///
    /// Операция отвязывает тикет от предыдущих операторов, и привязывает к текущему.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `operator_ext: &str`: Внешней идентификатор оператора.
    /// - `mut ticket: DbQueuedTicket`: Инстанция тикета из очереди.
    ///
    /// __Returns:__
    /// - `Result<DbLastOperator>`: При успешной операции `Ok`, иначе ошибки.
    ///   Новая запись оператора.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn assign_ticket_to_operator(
        &self,
        operator_ext: &str,
        mut ticket: DbQueuedTicket,
    ) -> Result<DbLastOperator> {
        let mut tx = self.db().get().begin().await?;

        // Удалить старых операторов
        if let Some(ref ext) = ticket.last_operator {
            DbLastOperator::delete_for_ticket(ticket.pkey(), ext, &mut *tx).await?;
        }
        let operator = DbNewLastOperator::new(operator_ext, ticket.pkey())
            .insert(&mut *tx)
            .await?;

        ticket.last_operator = Some(operator_ext.to_owned());
        ticket.update(&mut *tx).await?;

        tx.commit().await?;
        Ok(operator)
    }

    /// Достать следующий тикет из очереди (и промаркировать в очереди).
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `ticket: &DbQueuedTicket`: Тикет, которому надо присвоить оператора.
    /// - `op_ext_id: &str`: Внешний идентификатор оператора.
    ///
    /// __Returns:__
    /// - `Result<DbLastOperator>`: При успешной операции `Ok`, иначе ошибки.
    ///   Если инстанция нового оператора.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn insert_operator(
        &self,
        ticket: &DbQueuedTicket,
        op_ext_id: &str,
    ) -> Result<DbLastOperator> {
        let pool = self.db().get();
        let operator = DbNewLastOperator::new(op_ext_id, ticket.pkey())
            .insert(pool)
            .await?;

        Ok(operator)
    }

    /// Проверять и утверждать актуальность связки оператора-тикета. Если связь актуальна, обновляем пинг.
    /// Если неактуально, убираем связь.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `mut operator: DbLastOperator`: Инстанция оператора.
    ///
    /// __Returns:__
    /// - `Result<Option<DbLastOperator>>`: При успешной операции `Ok`, иначе ошибки.
    ///   Если оператор слишком долго не выходил на связь, он удаляется и возвращается `None`.
    ///   Иначе возвращается оператор с обновлённом таймером.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn work_operator_ping(
        &self,
        mut operator: DbLastOperator,
    ) -> Result<Option<DbLastOperator>> {
        let elapsed = time::UtcDateTime::now() - operator.last_check_in.as_utc();

        let pool = self.db().get();
        // Если слишком долго жил, пристреливаем. Если пристреливаем то и тикет
        // в очередь возвращаем.
        // Иначе обновляем.
        if (self.config().operator_lifetime() as f64) < elapsed.as_seconds_f64() {
            let mut tx = pool.begin().await?;

            DbQueuedTicket::get_by_id(operator.last_ticket_id, &mut *tx)
                .await?
                .return_to_queue(&mut *tx)
                .await?;
            operator.delete(&mut *tx).await?;
            tx.commit().await?;

            Ok(None)
        } else {
            operator.update_check_in(pool).await?;
            Ok(Some(operator))
        }
    }
    /// Завершить работу с тикетом, не возвращая её в очередь (потом опять возьмём).
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `operator: DbLastOperator`: Инстанция оператора.
    ///
    /// __Returns:__
    /// - `Result<()>`: При успешной операции `Ok`, иначе ошибки.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn end_work_with_ticket(&self, operator: DbLastOperator) -> Result<()> {
        let pool = self.db().get();
        operator.delete(pool).await?;
        Ok(())
    }

    /// Завершить работу с тикетом, и вернуть её в очередь. Оператора забываем.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    /// - `operator: DbLastOperator`: Инстанция оператора.
    ///
    /// __Returns:__
    /// - `Result<()>`: При успешной операции `Ok`, иначе ошибки.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn end_work_return_to_queue(&self, operator: DbLastOperator) -> Result<()> {
        let pool = self.db().get();
        let mut tx = pool.begin().await?;

        DbQueuedTicket::get_by_id(operator.last_ticket_id, &mut *tx)
            .await?
            .return_to_queue(&mut *tx)
            .await?;
        operator.delete(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }
    /// Убрать просроченные связки оператора-тикета.
    /// TODO: See if this is practical.
    ///
    /// __Arguments:__
    /// - `&self`: Ссылка на сущность очереди
    ///
    /// __Returns:__
    /// - `Result<()>`: При успешной операции `Ok`, иначе ошибки.
    ///
    #[tracing::instrument(skip_all)]
    pub async fn delete_expired_operators(&self) -> Result<()> {
        let pool = self.db().get();

        DbLastOperator::delete_older(self.config().operator_lifetime() * 1000, pool).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
