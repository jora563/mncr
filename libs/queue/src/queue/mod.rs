//! Модуль сущности очереди.
//!
//! При этом основной функционал разбросан по модулям.
//! - [`crate::intrinsic`] это функционал который работает когда очередь работает как встроенный модуль.
//! - `crate::worker` это функционал который работает когда очередь работает как отдельный сервис
//!   внутри сервиса.
//! - `crate::stand_alone` это функционал который работает когда очередь работает как отдельный сервис

use crate::config::QueueConfig;
use crate::error::Result;

use db::connect::CoreDbPool;
use std::path::Path;

/// Структура очереди.
#[derive(Debug)]
pub struct Queue {
    /// Конфигурация очереди.
    config: QueueConfig,
    /// Подсоединения к БД очереди.
    pool: CoreDbPool,
    /// Время последнего обновления.
    last_accessed: std::time::Instant,
}

impl Queue {
    /// Создать очередь из файла конфигураций
    pub async fn from_file<T: AsRef<Path>>(path: T) -> Result<Self> {
        let cfg = QueueConfig::from_file(path)?;
        Self::from_cfg(&cfg).await
    }

    /// Создать очередь из готовых конфигураций.
    pub async fn from_cfg(config: &QueueConfig) -> Result<Self> {
        let pool = config.connect().await?;
        let last_accessed = std::time::Instant::now();

        Ok(Self {
            config: config.clone(),
            pool,
            last_accessed,
        })
    }

    pub fn db(&self) -> &CoreDbPool {
        &self.pool
    }

    pub fn config(&self) -> &QueueConfig {
        &self.config
    }

    pub fn last_accessed(&self) -> std::time::Instant {
        self.last_accessed
    }

    /// Проставить последний вход на данный момент.
    #[allow(dead_code)]
    pub(crate) fn update_accessed(&mut self) {
        self.last_accessed = std::time::Instant::now();
    }
}
