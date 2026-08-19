//! Модуль структуры чатов с операторами.
//! Передача данных основана на темах, а не на операторах, так как
//! Во многих кол-центрах, оператор не как не связан с темой.
use crate::CoreCtx;
use crate::config::CoreSettings;
use crate::error::{CoreError, Result};

use actix_web::HttpRequest;
use ahash::AHashMap;
use db::core_schema::{DbFullMessage, DbProject, DbTicket};
use db::queue_schema::DbLastOperator;
use queue::Queue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use uzor_plugin::{AsaaData, PermissionReExtract, TokenData};

/// Структура хранения кананлов связи с чатами.
#[derive(Debug, Default)]
pub(crate) struct WsChats {
    /// Карта чатов которые на данный момент обслуживаются.
    /// Ключ: Идишник темы.
    /// Значение: Отправляющая сторона канала связи.
    chats: Arc<RwLock<AHashMap<i64, Sender<DbFullMessage>>>>,
    /// Таймаут: Через этот период, соединение отключается.
    timeout: Duration,
}

impl WsChats {
    /// Новая инстанция.
    pub(crate) fn new(cfg: &CoreSettings) -> Self {
        Self {
            chats: Arc::new(RwLock::new(AHashMap::new())),
            timeout: Duration::from_secs(cfg.operator_idle_timeout_s as u64),
        }
    }

    /// Достать таймаут соединения.
    pub(crate) fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Послать сообщение на WS чат живому оператору.
    /// Если не найден оператор, результат `Ok(false)`, если найдёт то `Ok(true)`,
    /// если ошибка канала то `Err(_)`
    pub(crate) async fn try_send(&self, theme: i64, message: DbFullMessage) -> Result<bool> {
        if let Some(ch) = self.chats.read().await.get(&theme) {
            ch.send(message).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Добавить канал в набор чатов.
    pub(crate) async fn add_chat(
        &self,
        queue_data: DbLastOperator,
        op_data: &mut OperatorData,
    ) -> Result<()> {
        let theme = queue_data.last_ticket_id;
        if self.chats.read().await.contains_key(&theme) {
            return Err(CoreError::TicketInUse(theme));
        }
        let (tx, rx) = channel(100);

        self.chats.write().await.insert(theme, tx);

        op_data.inner = Some(OperatorDataInner {
            ticket_id: theme,
            queue_data,
            rx,
        });
        Ok(())
    }

    /// Проверить, есть ли тема в передатчике тем.
    pub(crate) async fn has_chat(&self, topic_id: i64) -> bool {
        self.chats.read().await.contains_key(&topic_id)
    }

    /// Remove the chat if the operator data holds the ticket.
    pub(crate) async fn purge_chat_if_held(&self, op_data: &OperatorData) {
        if let Some(ref inner) = op_data.inner {
            self.chats.write().await.remove(&inner.ticket_id);
        }
    }
}

/// Данные которые живут в соединение оператора с сервером.
/// Эти данные заполняются когда оператор получает номер чата для обработки.
#[derive(Debug)]
struct OperatorDataInner {
    /// Номер тикета
    ticket_id: i64,
    queue_data: DbLastOperator,
    /// Канал связи по которым приходят сообщения.
    rx: Receiver<DbFullMessage>,
}

/// Данные чата со стороны оператора.
/// Они создаются когда создается соединение, но заполняются когда оператор
/// берёт чат на обработку.
#[derive(Debug)]
pub(crate) struct OperatorData {
    /// Внешней ид оператора.
    external_id: String,
    permitted_projects: Vec<String>,
    inner: Option<OperatorDataInner>,
    _ping_handle: tokio::task::JoinHandle<Result<()>>,
}

impl OperatorData {
    pub(crate) fn new(
        req: &HttpRequest,
        _ping_handle: tokio::task::JoinHandle<Result<()>>,
    ) -> Result<Self> {
        let external_id = TokenData::from_final_request(req)?.personal_id;
        let permitted_projects = AsaaData::projects(req)?;

        Ok(Self {
            inner: None,
            permitted_projects,
            external_id,
            _ping_handle,
        })
    }

    pub(crate) async fn end_work_with_ticket(&mut self, queue: &Queue) -> Result<()> {
        if let Some(inner) = self.inner.take() {
            queue.end_work_with_ticket(inner.queue_data).await?;
        }
        Ok(())
    }

    /// Посмотреть, есть ли для оператора сообщение.
    pub(crate) async fn get_msg(&mut self) -> Option<DbFullMessage> {
        match self.inner {
            None => None,
            // TODO: Do we need to work the None from `recv`?
            Some(ref mut inner) => inner.rx.recv().await,
        }
    }

    /// Достать ссылку на разрешенные проекты.
    pub(crate) fn permitted_projects(&self) -> &[String] {
        &self.permitted_projects
    }

    /// Проверка что оператору разрешается работать с данным тикетом.
    /// Если проект не входит в список разрешенных то выдаём ошибку.
    pub(crate) async fn ticket_permitted(&self, ticket_id: i64, ctx: &Arc<CoreCtx>) -> Result<()> {
        let pool = ctx.db().get();
        let ticket = DbTicket::get_by_id(ticket_id, pool).await?;
        let project = DbProject::get_by_id(ticket.project_id, pool).await?;

        if self.permitted_projects.contains(&project.project_name) {
            Ok(())
        } else {
            Err(CoreError::NoAccess(
                "Ticket",
                ticket.user_ticket_number.to_string(),
            ))
        }
    }

    /// Обновить внутренний таймер оператора.
    /// если он истёк, то мы теряем тикет.
    pub(crate) async fn tick(&mut self, ctx: &Arc<CoreCtx>) {
        let Some(OperatorDataInner {
            ticket_id,
            queue_data,
            rx,
        }) = self.inner.take()
        else {
            return;
        };
        let old_data = queue_data.clone();
        let new_data = ctx
            .queue()
            .work_operator_ping(queue_data)
            .await
            .unwrap_or(Some(old_data));

        self.inner = new_data.map(|queue_data| OperatorDataInner {
            ticket_id,
            queue_data,
            rx,
        });
    }

    /// Достать внешней ИД оператора
    pub(crate) fn external_id(&self) -> &str {
        &self.external_id
    }
}
