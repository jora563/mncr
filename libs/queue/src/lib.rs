//! Библиотека работы очереди.
//! Очередь спроектирована чтобы могла работать и как отдельное приложение,
//! и как библиотечный модуль внутри сервера "АI Omni Core".

pub mod config;
pub mod error;
#[cfg(feature = "intrinsic")]
pub mod intrinsic;
pub mod queue;

pub use config::QueueConfig;

pub use queue::Queue;
