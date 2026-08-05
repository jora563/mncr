//! Контекст всего приложения
//! Контекст как и настройки, так и основные сущности взаимодействий.
use crate::config::Config;
use crate::error::Result;
use crate::llm::LlmDriver;
use crate::messengers::ChatDriver;

use db::connect::CoreDbPool;
use db::core_schema::DbBotAccountWithMeta;

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
    /// Сущность взаимодействия с очередью
    _queue: (),
}

impl CoreCtx {
    /// Создать контекст.
    pub(crate) async fn new(cfg: Config) -> Result<Self> {
        let db = CoreDbPool::new(cfg.db()).await?;

        Ok(Self {
            db,
            chat: ChatDriver::default(),
            llm: LlmDriver::new(&cfg)?,
            cfg,
            _queue: (),
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

    /// Загрузить изначальные боты вместе с их платформа.
    /// В далнейшем эти платформы определяют к каким API чата мы обращаемся,
    /// И какие другие данные мы подгружаем из БД при обработке чатов.
    pub(crate) async fn load_initial_platforms(&self) -> Result<Vec<DbBotAccountWithMeta>> {
        let bot_meta = DbBotAccountWithMeta::get_all(self.db().get()).await?;

        Ok(bot_meta)
    }
}
