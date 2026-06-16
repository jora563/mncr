//! Под-модуль для конфигурации БД Postgres и для системы которая её обслуживает.
//! Тут не только обёртка для `sqlx::PgPoolOptions`, но и дополнительная
//! настройка.
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::path::Path;
use std::time::Duration;

use crate::error::{DbError, Result};

/// Настройки БД, которые хранятся в ТОМЛ файле, и используются
/// для создания соединения с БД.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct CoreDbSettings {
    max_connections: u32,
    min_connections: u32,
    max_no_log_connections: u32,
    acquire_timeout_s: u16,
    max_lifetime_s: u16,
    idle_timeout_s: u16,
    db_host_url: String,
    db_name: String,
    migrations_home: String,
    user: String,
    pw: String,
}

/// Дефаулт в ручную чтобы неуказанные значение в конфиге имели какой-то смысл.
/// Урлы и адреса конечно не помогут, а числа может быть будут иметь пользу.
impl Default for CoreDbSettings {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            max_no_log_connections: 0,
            acquire_timeout_s: 30,
            max_lifetime_s: 3600,
            idle_timeout_s: 60,
            db_host_url: "localhost".to_string(),
            db_name: "ai_omni_db".to_string(),
            migrations_home: "sql/core/".to_string(),
            user: "root".to_string(),
            pw: "password".to_string(),
        }
    }
}

impl CoreDbSettings {
    pub(super) fn from_file<T: AsRef<Path>>(path: T) -> Result<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::from_str(&file).map_err(Into::into)
    }

    /// TODO: Perhaps change the realization to allow there to be zero
    /// connections with logs.
    pub(super) fn get_main_options(&self) -> Result<PgPoolOptions> {
        if self.max_connections == 0 {
            Err(DbError::MaxConnectionZero)
        } else if self.max_connections <= self.max_no_log_connections {
            Err(DbError::MaxNoLogConnectionHigh)
        } else {
            Ok(self.get_options(self.max_connections - self.max_no_log_connections))
        }
    }

    pub(super) fn get_no_log_options(&self) -> Option<PgPoolOptions> {
        if self.max_no_log_connections == 0 {
            None
        } else {
            Some(self.get_options(self.max_no_log_connections))
        }
    }

    fn get_options(&self, max_conn: u32) -> PgPoolOptions {
        PgPoolOptions::new()
            .max_connections(max_conn)
            .min_connections(std::cmp::min(self.min_connections, max_conn))
            .acquire_timeout(Duration::from_secs(self.acquire_timeout_s.into()))
            .max_lifetime(Duration::from_secs(self.max_lifetime_s.into()))
            .idle_timeout(Duration::from_secs(self.idle_timeout_s.into()))
    }

    pub(super) fn connections_string(&self) -> String {
        format!(
            "postgresql://{user}:{pw}@{url}/{db_name}",
            user = self.user,
            pw = self.pw,
            url = self.db_host_url,
            db_name = self.db_name,
        )
    }

    /// Достать наименование БД
    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    /// Достать хост БД
    pub fn db_host(&self) -> &str {
        &self.db_host_url
    }

    /// Достать хост БД
    pub fn migrations_home(&self) -> &str {
        &self.migrations_home
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_cfg() -> CoreDbSettings {
        let root_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let prompts_addr = PathBuf::from(root_dir)
            .join("tests")
            .join("core_db_settings.toml");
        CoreDbSettings::from_file(prompts_addr).unwrap()
    }

    #[test]
    fn test_cfg_from_file() {
        let cfg = get_test_cfg();

        let exp = CoreDbSettings {
            max_connections: 20,
            min_connections: 4,
            max_no_log_connections: 2,
            acquire_timeout_s: 30,
            max_lifetime_s: 3600,
            idle_timeout_s: 60,
            db_host_url: "localhost".to_string(),
            db_name: "ai_omni_test_db_0".to_string(),
            migrations_home: "../../sql/core/".to_string(),
            user: "root".to_string(),
            pw: "password".to_string(),
        };
        assert_eq!(cfg, exp);
    }

    #[test]
    fn test_connection_string() {
        let cfg = get_test_cfg();
        let conn_str = cfg.connections_string();
        let exp = "postgresql://root:password@localhost/ai_omni_test_db_0";
        assert_eq!(conn_str, exp);
    }

    #[test]
    fn cfg_to_options() {
        let cfg = get_test_cfg();
        let opts = cfg.get_main_options().unwrap();

        assert_eq!(opts.get_max_connections(), 18);
        assert_eq!(opts.get_min_connections(), 4);
        assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(30));
        assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(3600)));
        assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn cfg_to_no_log_options() {
        let cfg = get_test_cfg();
        let opts = cfg.get_no_log_options().unwrap();

        assert_eq!(opts.get_max_connections(), 2);
        assert_eq!(opts.get_min_connections(), 2);
        assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(30));
        assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(3600)));
        assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(60)));
    }
}
