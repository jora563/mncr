//! Конфигурация очереди.
use crate::error::Result;

use db::connect::{CoreDbPool, CoreDbSettings};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Конфигурация очереди. Пока что она упрощённая.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QueueConfig {
    /// Конфигурация БД
    db: CoreDbSettings,
    /// Как часто посылать пинг.
    ping_period_s: u16,
    /// После какого промежутка удалять старые записи операторов из БД
    operator_lifetime_s: u16,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            db: CoreDbSettings::default(),
            ping_period_s: 60,
            operator_lifetime_s: 360,
        }
    }
}

impl QueueConfig {
    /// Получить настройку очереди из файла
    pub fn from_file<T: AsRef<Path>>(path: T) -> Result<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::from_str(&file).map_err(Into::into)
    }

    /// Подсоединить очередь к базе данных.
    pub async fn connect(&self) -> Result<CoreDbPool> {
        CoreDbPool::new(&self.db).await.map_err(Into::into)
    }

    /// Достать конфигурацию базы данных.
    pub fn db(&self) -> &CoreDbSettings {
        &self.db
    }

    /// Достать период пинга.
    pub fn ping_period(&self) -> u32 {
        self.ping_period_s as u32
    }

    /// Достать период жизни оператора
    pub fn operator_lifetime(&self) -> u32 {
        self.operator_lifetime_s as u32
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse() {
        let root_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let cfg_path = PathBuf::from(root_dir).join("tests/test_queue_cfg.toml");
        let cfg = QueueConfig::from_file(cfg_path).unwrap();

        assert_eq!(cfg.ping_period(), 30);
        assert_eq!(cfg.operator_lifetime(), 600);
        assert_eq!(cfg.db().db_name(), "intrinsic_queue_test_db_0");
        assert_eq!(cfg.db().db_host(), "localhost");
        assert_eq!(cfg.db().migrations_home(), "sql/queue/");
        assert!(cfg.db().fixtures_dir().is_none());
    }
}
