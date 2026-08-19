//! Модуль схемы БД Queue.
//!
//! Тут сущности и связанные с ними запросы. Функционал описанный в этом модуле
//! должен соблюдать бизнес логику Queue.
//!
//! База данных Queue необязательно должна быть независимой. Особенно из за того что
//! у неё на данной момент всего две таблицы, и данные в них долго не существуют, она
//! теоретический может существовать внутри БД AI Omni Core. При этом назначение
//! этого БД чуть отличается, и для масштабирования принято решение держать
//! его отдельно от основного БД.

pub mod last_operator;
pub mod queued_ticket;

pub use last_operator::{DbLastOperator, DbNewLastOperator};
pub use queued_ticket::{DbNewQueuedTicket, DbQueuedTicket};

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
            "queue_db".into()
        }
        fn db_host(&self) -> Box<str> {
            "postgresql://aio_core:password@127.0.0.1:5432".into()
        }
    }
}
