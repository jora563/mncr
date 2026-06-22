//! Под-модуль соединений к БД.
use sqlx::ConnectOptions;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPool};
use std::path::{Path, PathBuf};

use super::config::CoreDbSettings;
use crate::error::Result;

/// Сущность имеет основной поол, и поол без логгинга для тех запросов
/// где очень много биндов, и логгинг сильно замедляет запрос.
#[derive(Debug)]
pub struct CoreDbPool {
    main_pool: PgPool,
    no_log_pool: Option<PgPool>,
    /// Настройки сохраняем. Вдруг перезапустить прийдётся.
    settings: CoreDbSettings,
}

impl CoreDbPool {
    /// Достать главный пул соединений.
    /// Этот пул позволяет `sqlx` логировать запросы.
    ///
    /// eg:
    /// ``` ignore
    /// let pool = get_initial_core_pool().await?;
    /// let ret = sqlx::query("SELECT * from user").execute(pool.get()).await?;
    /// // If we want to forbid logging we would:
    /// // let ret = sqlx::query("SELECT * from user").execute(pool.get_no_log()).await?;
    /// ```
    ///
    /// Если нужно запретить логирования, берите [`Self::get_no_log`].
    pub fn get(&self) -> &PgPool {
        &self.main_pool
    }

    /// Достать пул который не позволяет логировать запросы. Выдаёт `None` когда он не настроен.
    /// Скорее всего не очень полезный функционал, так как если надо без логов
    /// Но есть только соединения только с логами, то от запроса скорее всего
    /// отказаться нельзя.
    ///
    /// Если не надо запрещать логирования используете [`Self::get`]
    pub fn try_get_no_log(&self) -> Option<&PgPool> {
        self.no_log_pool.as_ref()
    }

    /// Достать пул который не позволяет логировать запросы НО если его нет, достать хоть какой.
    /// Такой подход позволяет администратору сервиса самому решить нужно ему позволять логировать
    /// соединения или нет.
    ///
    /// Если не надо запрещать логирования используете [`Self::get`]
    pub fn get_no_log(&self) -> &PgPool {
        self.try_get_no_log().unwrap_or(&self.main_pool)
    }

    /// Достать настройки.
    pub fn settings(&self) -> &CoreDbSettings {
        &self.settings
    }

    /// Создать из настроек
    #[tracing::instrument]
    pub async fn new(settings: &CoreDbSettings) -> Result<Self> {
        let main_options = settings.get_main_options()?;
        let no_log_options = settings.get_no_log_options();

        let url = settings.connections_string();
        let main_pool = main_options.connect(&url).await?;
        let no_log_pool = match no_log_options {
            Some(x) => {
                let silent_pool = x.connect(&url).await?;
                let silent_opt = PgConnectOptions::new().disable_statement_logging();
                silent_pool.set_connect_options(silent_opt);
                Some(silent_pool)
            }
            None => None,
        };

        Ok(Self {
            main_pool,
            no_log_pool,
            settings: settings.clone(),
        })
    }

    /// Загрузить с файла настроек (скорее всего так и будет).
    #[tracing::instrument]
    pub async fn load<T: AsRef<Path> + std::fmt::Debug>(cfg_path: T) -> Result<Self> {
        let settings = CoreDbSettings::from_file(cfg_path)?;
        Self::new(&settings).await
    }

    /// Провести миграций вверх.
    #[tracing::instrument]
    pub async fn run_up_migrations(&self) -> Result<()> {
        let path = PathBuf::from(self.settings.migrations_home()).join("up");
        Migrator::new(path).await?.run(self.get()).await?;
        Ok(())
    }

    /// Откатить миграций.
    #[tracing::instrument]
    pub async fn run_down_migrations(&self) -> Result<()> {
        let path = PathBuf::from(self.settings.migrations_home()).join("down");
        // Run down to version 0. It will run all of them.
        Migrator::new(path).await?.undo(self.get(), 0).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_frame::{ConfigDriver, run_test_postgres};

    use std::path::PathBuf;
    use std::time::Duration;

    struct TestCfg;

    impl ConfigDriver for TestCfg {
        fn initialise() -> Self {
            Self
        }
        fn db_name_root(&self) -> Box<str> {
            "ai_omni_db".into()
        }
        fn db_host(&self) -> Box<str> {
            "postgresql://aio_core:password@127.0.0.1:5432".into()
        }
    }

    fn default_cfg_path() -> PathBuf {
        let root_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        PathBuf::from(root_dir)
            .join("tests")
            .join("core_db_settings.toml")
    }

    // НД: Если не работает, проверьте что пользователь и БД из
    // `db/tests/core_db_settings.toml` существуют.
    #[tokio::test]
    async fn test_load() {
        // Используем тестфрэйм чтобы гарантировать существование БД.
        run_test_postgres::<TestCfg, _>(
            "../../sql/core/",
            "../../sql/core/",
            "tests/sql/postgres/drop_core",
            |_| async move {
                let cfg_path = default_cfg_path();
                let core_pool = CoreDbPool::load(cfg_path).await.unwrap();

                let pool = core_pool.get();
                let opts = pool.options();
                assert_eq!(opts.get_max_connections(), 18);
                assert_eq!(opts.get_min_connections(), 4);
                assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(30));
                assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(3600)));
                assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(60)));

                let pool = core_pool.get_no_log();
                let opts = pool.options();
                assert_eq!(opts.get_max_connections(), 2);
                assert_eq!(opts.get_min_connections(), 2);
                assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(30));
                assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(3600)));
                assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(60)));
                Ok(())
            },
        )
        .await
    }

    /// Тест проверяет пробегают-ли вообще миграции, и падает ли мигратор.
    /// НБ: Надо чтобы ДБ "ai_omni_test_db_0" существовало.
    #[tokio::test]
    async fn test_migrator() {
        // Используем тестфрэйм чтобы гарантировать существование БД.
        run_test_postgres::<TestCfg, _>(
            "../../sql/core/",
            "../../sql/core/",
            "tests/sql/postgres/drop_core",
            |_| async move {
                let cfg_path = default_cfg_path();
                let core_pool = CoreDbPool::load(cfg_path).await.unwrap();

                core_pool.run_up_migrations().await.unwrap();
                core_pool.run_down_migrations().await.unwrap();

                Ok(())
            },
        )
        .await
    }
}
