//! Модуль активации и настройки логирования.
use crate::config::{CoreSettings, TracingLevel};
use crate::error::Result;

use std::path::PathBuf;
use tracing_appender::rolling;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;

impl From<TracingLevel> for Option<tracing::Level> {
    fn from(x: TracingLevel) -> Self {
        match x {
            TracingLevel::Off => None,
            TracingLevel::Error => Some(tracing::Level::ERROR),
            TracingLevel::Warn => Some(tracing::Level::WARN),
            TracingLevel::Info => Some(tracing::Level::INFO),
            TracingLevel::Debug => Some(tracing::Level::DEBUG),
            TracingLevel::Trace => Some(tracing::Level::TRACE),
        }
    }
}

pub(crate) fn initiate_logging(cfg: &CoreSettings) -> Result<()> {
    use TracingLevel::*;

    let Some(level) = Option::<tracing::Level>::from(cfg.log_level) else {
        return Ok(());
    };

    let layer = fmt::Layer::new();
    let registry = tracing_subscriber::registry()
        .with(layer)
        .with(LevelFilter::from_level(level));
    // Имеем и то и то чтобы в конфиг. файле можно было не зачеркивать строку
    // с дирекотрией где хранятся логи.
    let dir = if let Some(ref dir) = cfg.log_dir
        && cfg.record_logs
    {
        PathBuf::from(dir)
    } else {
        tracing::subscriber::set_global_default(registry)?;
        return Ok(());
    };
    let layer = fmt::Layer::new().json();
    let files =
        rolling::hourly(dir.clone().join("error"), "").with_max_level(tracing::Level::ERROR);

    if matches!(cfg.log_level, Error) {
        tracing::subscriber::set_global_default(registry.with(layer.with_writer(files)))?;
        return Ok(());
    }
    // Usually this is bad practice.
    let files = files
        .and(rolling::hourly(dir.clone().join("warn"), "").with_max_level(tracing::Level::WARN));
    if matches!(cfg.log_level, Warn) {
        tracing::subscriber::set_global_default(registry.with(layer.with_writer(files)))?;
        return Ok(());
    }
    let files = files
        .and(rolling::hourly(dir.clone().join("info"), "").with_max_level(tracing::Level::INFO));
    if matches!(cfg.log_level, Info) {
        tracing::subscriber::set_global_default(registry.with(layer.with_writer(files)))?;
        return Ok(());
    }
    let files = files
        .and(rolling::hourly(dir.clone().join("debug"), "").with_max_level(tracing::Level::DEBUG));
    if matches!(cfg.log_level, Debug) {
        tracing::subscriber::set_global_default(registry.with(layer.with_writer(files)))?;
        return Ok(());
    }
    let files = files
        .and(rolling::hourly(dir.clone().join("trace"), "").with_max_level(tracing::Level::TRACE));

    tracing::subscriber::set_global_default(registry.with(layer.with_writer(files)))?;
    Ok(())
}
