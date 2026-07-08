//! Модуль схемы БД Ai-Omni Core.
//!
//! Тут сущности и связанные с ними запросы. Функционал описанный в этом модуле
//! должен соблюдать бизнес логику Ai-Omni Core.

pub mod bot;
pub mod chat;
/// "model_management"
pub mod moma;
pub mod platform;
pub mod project;
pub mod ticket;
pub mod user;
pub mod vk_oauth;

pub use bot::{
    DbBot, DbBotAccount, DbBotAccountWithMeta, DbFullBotAccount, DbNewBot, DbNewBotAccount,
};
pub use chat::{
    DbAttachment, DbChat, DbFullChat, DbFullMessage, DbMessage, DbNewAttachment, DbNewChat,
    DbNewMessage,
};
pub use platform::{
    ApiId, DbFullPlatform, DbNewPlatform, DbNewPlatformMirror, DbPlatform, DbPlatformMirror,
};
pub use project::{DbFullProjectGroup, DbNewProject, DbNewProjectGroup, DbProject, DbProjectGroup};
pub use ticket::{DbFullTicket, DbNewTicket, DbTicket};
pub use user::{DbFullUser, DbNewUser, DbNewUserAccount, DbUser, DbUserAccount};
pub use vk_oauth::{DbNewVkOauth, DbNewVkOauthState, DbVkOauth, DbVkOauthState};

pub trait CoreDbCrud {
    fn pkey(&self) -> i64;
}

/// Модуль для тест-конфигурации для `run_test_postgres`.
#[cfg(test)]
mod test_cfg {
    use crate::test_frame::ConfigDriver;
    pub(super) struct TestCfg;

    impl ConfigDriver for TestCfg {
        fn initialise() -> Self {
            Self
        }
        fn db_name_root(&self) -> Box<str> {
            "ai_omni_db".into()
        }
        fn db_host(&self) -> Box<str> {
            "postgresql://aio_core:password@127.0.0.1:5432".into()
        }
    }
}
