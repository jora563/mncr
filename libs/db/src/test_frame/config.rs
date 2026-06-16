//! This module contains the trait for creating a setting reader that then
//! feeds the base address and DB name to the [`crate::test_frame::db_list::DbList`].

/// Способ настроить конфигурацию наименования и хост тестового БД.
pub trait ConfigDriver {
    fn initialise() -> Self;
    fn db_name_root(&self) -> Box<str>;
    fn db_host(&self) -> Box<str>;
}
