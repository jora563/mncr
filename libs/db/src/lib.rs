/// Конфигурации БД
pub mod connect;
/// Основная схема Ai-Omni Core
pub mod core_schema;
/// Модуль ошибок
pub mod error;
/// Копия <https://bitbucket.telecontact.ru/projects/TELECONTACT/repos/telecontact-rust-libs/>
pub mod test_frame;

/// TODO: Может нужна библиотека реэкспорта
pub use sqlx::types::time::PrimitiveDateTime;
