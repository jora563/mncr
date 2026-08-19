//! Контекст всего приложения
//! Контекст как и настройки, так и основные сущности взаимодействий.
use crate::config::Config;
use crate::error::Result;
use crate::llm::LlmDriver;
use crate::messengers::ChatDriver;

use db::connect::CoreDbPool;
use db::core_schema::DbBotAccountWithMeta;
use queue::Queue;

/// Контекст
/// ТОДО: Нужен ли нам `Arc<RwLock<T>>` или даже `ArcSwap`?
///       И нужно ли внешне или для каждого поля?
/// ТОДО: Возможно что контекст не нужен, но это мало вероятно.
#[derive(Debug)]
pub(crate) struct CoreCtx {
    /// Общии настройки чтобы можно было пересоздать то что надо,
    /// когда нада
    cfg: Config,
    /// Сущьность взаимодействий с БД
    /// ТОДО: Решить, стоит ли обарачивать в "DbDriver"
    db: CoreDbPool,
    /// Сущьность взаимодействия с чатами
    chat: ChatDriver,
    /// Сущность взаимодействия с LLM
    llm: LlmDriver<llm_client::openai::OpenAiRequest>,
    /// Карта соединений с операторами, для того чтобы отправлять им сообщения.
    ws_chats: ws_chats::WsChats,
    /// Сущность взаимодействия с очередью
    queue: Queue,
}

impl CoreCtx {
    /// Создать контекст.
    pub(crate) async fn new(cfg: Config) -> Result<Self> {
        let db = CoreDbPool::new(cfg.db()).await?;
        let queue = Queue::from_cfg(cfg.queue()).await?;
        let ws_chats = ws_chats::WsChats::new(cfg.core());

        Ok(Self {
            db,
            chat: ChatDriver::default(),
            llm: LlmDriver::new(&cfg)?,
            cfg,
            ws_chats,
            queue,
        })
    }
    pub(crate) fn cfg(&self) -> &Config {
        &self.cfg
    }
    pub(crate) fn db(&self) -> &CoreDbPool {
        &self.db
    }
    pub(crate) fn chat(&self) -> &ChatDriver {
        &self.chat
    }
    pub(crate) fn llm(&self) -> &LlmDriver<llm_client::openai::OpenAiRequest> {
        &self.llm
    }
    pub(crate) fn queue(&self) -> &Queue {
        &self.queue
    }
    pub(crate) fn ws_chats(&self) -> &WsChats {
        &self.ws_chats
    }

    /// Загрузить изначальные боты вместе с их платформа.
    /// В далнейшем эти платформы определяют к каким API чата мы обращаемся,
    /// И какие другие данные мы подгружаем из БД при обработке чатов.
    pub(crate) async fn load_initial_platforms(&self) -> Result<Vec<DbBotAccountWithMeta>> {
        let bot_meta = DbBotAccountWithMeta::get_all(self.db().get()).await?;

        Ok(bot_meta)
    }
}

mod ws_chats;

pub(crate) use ws_chats::*;
