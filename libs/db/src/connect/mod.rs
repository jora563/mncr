//! Модуль соединений к БД Postgres и её настроек.
pub mod config;
pub mod pool;

pub use config::CoreDbSettings;
pub use pool::CoreDbPool;
