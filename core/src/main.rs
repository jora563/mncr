#![allow(bindings_with_variant_name)]
//! The core service of AI-Omni. This service handles interaction between
//! messengers, AI, DB and the escalation queues.
use crate::config::Config;
use crate::context::CoreCtx;
use crate::error::Result;

use std::env::{VarError, var};
use tokio::runtime::Builder;

fn main() -> Result<()> {
    let path = var("AI_OMNI_CONFIG_PATH").inspect_err(|e| {
        if let VarError::NotPresent = e {
            println!("Config path variable (AI_OMNI_CONFIG_PATH) not set.");
        }
    })?;
    println!("Config path: {path}");
    let config = Config::from_file(path)?;

    Builder::new_multi_thread()
        .worker_threads(config.core().threads as usize)
        .max_blocking_threads(config.core().blocking_threads as usize)
        .enable_all()
        .on_task_spawn(set_panic_hook)
        .build()?
        .block_on(inner_main(config))
}

/// TODO: Install a kill signal.
/// TODO2: Install the logger.
async fn inner_main(config: Config) -> Result<()> {
    log::initiate_logging(config.core())?;
    tracing::info!("Hello runtime! Our config is: {config:#?}");

    let ctx = CoreCtx::new(config)
        .await
        .inspect_err(|e| tracing::error!("Failure creating core context: {e}"))?;
    tracing::info!("Db options: {:#?}", ctx.db().get().options());

    ctx.db()
        .run_up_migrations()
        .await
        .inspect_err(|e| tracing::error!("Migation failure on launch: {e}"))?;
    tracing::info!("Upwards migrations completed.");

    let poll_fut = tokio::task::spawn(poll_based::run_core(ctx));
    tracing::info!("Client-like polling core created.");

    // Этот подход позволяет нам потом добавить ешё и серверной компонент.
    let (polled,) = tokio::join!(poll_fut);
    tracing::info!("Cores joined.");

    match polled.map_err(Into::into) {
        Ok(Ok(_)) => tracing::info!("Client-like polling core result is OK"),
        Ok(Err(e)) | Err(e) => tracing::error!("Client-like polling core ended with error: {e}"),
    };
    tracing::warn!("Shutting down AI Omni Core");
    Ok(())
}

/// Установить крюк который будет сбрасывать панику в логи (а не просто печатать).
/// Так как это может нам дать очень много записей, делаем в `trace` а не в `error`.
fn set_panic_hook<'a, 'b>(meta: &'a tokio::runtime::TaskMeta<'b>) {
    let id = meta.id().to_owned();
    std::panic::set_hook(Box::new(move |info| {
        match (info.location(), info.payload_as_str()) {
            (Some(i), Some(p)) => tracing::trace!(
                "Task `{id}` panicked at '{}'-{} with: {p}",
                i.file(),
                i.line()
            ),
            (Some(i), None) => tracing::trace!(
                "Task `{id}` panicked at '{}'-{} with: unknown",
                i.file(),
                i.line()
            ),
            (None, Some(p)) => {
                tracing::trace!("Task `{id}` panicked at <unknown location> with: {p}")
            }
            _ => tracing::trace!("Task `{id}` panicked <no data available>"),
        };
    }));
}

mod config;
mod context;
mod error;
mod llm;
mod log;
mod messengers;
mod poll_based;
