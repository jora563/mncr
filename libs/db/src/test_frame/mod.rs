use std::sync::Arc;
use std::sync::OnceLock;

use tokio::sync::Mutex;

use self::db_list::DbList;

/// TODO: Забыть когда `OnceLock::get_mut_or_init` войдёт в stable.
type DbLock<D> = OnceLock<Arc<Mutex<DbList<D>>>>;

/// Глобальная переменная чтобы отслеживать временные тестовые БД
static DB_LIST: DbLock<sqlx::Postgres> = OnceLock::new();

type Result<T> = std::result::Result<T, sqlx::Error>;

pub mod config;
pub mod db_list;
pub mod frame;

pub use config::ConfigDriver;
pub use frame::run_test_postgres;
