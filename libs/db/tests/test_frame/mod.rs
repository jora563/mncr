use db::test_frame::config::ConfigDriver;

#[derive(Debug)]
pub(crate) struct TestConfig {
    pub(crate) name_root: Box<str>,
    pub(crate) address_root: Box<str>,
}

#[derive(Debug)]
pub(crate) struct PostgresTestConfig(TestConfig);

impl ConfigDriver for PostgresTestConfig {
    fn initialise() -> Self {
        Self(TestConfig {
            name_root: "testdb".into(),
            address_root: "postgresql://aio_core:password@127.0.0.1:5432".into(),
        })
    }

    fn db_name_root(&self) -> Box<str> {
        self.0.name_root.clone()
    }
    fn db_host(&self) -> Box<str> {
        self.0.address_root.clone()
    }
}
mod tests_pg;
