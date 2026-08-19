//! This module contains the test frame which is then used with the database list.
// use sqlx_migrator;
use super::config::ConfigDriver;
use super::db_list::{DbList, NameDb};
use super::{DB_LIST, DbLock, Result};

use ahash::AHashMap;
use sqlx::AssertSqlSafe;
use sqlx::migrate::{Migrate, MigrateDatabase, Migrator};
use sqlx::{Acquire, Connection, Database, Executor, Pool, Postgres};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A simple function for running fixtures which should be reset after each test.
/// The reset is done by means of a cleanup script which the user must define
/// themselves.
///___
///
/// Функция для запуска фикстур которые надо пере-играть каждый раз когда тест
/// запускается. Перезапуск осуществлён прокруткой SQL скрипта перезапуска.
async fn run_fixtures<D: Database>(
    mig_dir: &str,
    fix_dir: &str,
    conn: &mut <D as Database>::Connection,
) -> Result<()>
where
    for<'a> <<&'a Pool<D> as Acquire<'a>>::Connection as Deref>::Target: Migrate,
    for<'a> &'a mut <D as Database>::Connection: Executor<'a>,
{
    let main_f = PathBuf::from(mig_dir).join("fixtures");
    let main_fixtures = std::fs::read_dir(&main_f).inspect_err(|e| {
        println!("Error reading main fixtures from '{main_f:?}': {e}");
    })?;
    let extra_fixtures = std::fs::read_dir(PathBuf::from(fix_dir)).inspect_err(|e| {
        println!("Error reading extra fixtures from '{fix_dir:?}': {e}");
    })?;

    for e in main_fixtures.chain(extra_fixtures) {
        let e = e?;
        let path = e.path();
        // We ignore non-sql files in the fixtures directory.
        if path.extension() != Some(std::ffi::OsStr::new("sql")) {
            continue;
        };
        let fixture = std::fs::read_to_string(&path).inspect_err(|e| {
            println!("Error reading fixture file from '{path:?}': {e}");
        })?;
        sqlx::raw_sql(AssertSqlSafe(fixture))
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// This function runs the tests against a pool of databases.
/// The configuration of the databases is set via the [`crate::test_frame::config::ConfigDriver`] which is
/// constructed internally.
///
/// The frame run the migrations in the "/migration_dir/up/" folder, then runs the test, and
/// finally runs the cleanup script from the "/cleanup_dir/cleanup.sql" file.
/// ___
///
/// Функция (test-frame) которая проигрывает тесты против набора БД.
/// Конфигурация БД идёт через [`crate::test_frame::config::ConfigDriver`] который внутренне
/// Создаётся (для каждой новой настройки своя новая структура).
///
/// Сначала проводятся миграции из "{migration_dir}/up/" директории, потом
/// сами тесты, потом скрипт очистки из "{cleanup_dir}/cleanup.sql" файла.
pub async fn run_test_postgres<C, F, T>(
    mig_dir: &str,
    extra_fixtures_dir: &str,
    cleanup_dir: &str,
    test_fn: impl FnOnce(Pool<Postgres>) -> F + Send + 'static,
) -> T
where
    C: ConfigDriver,
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    use std::error::Error;

    let f = std::time::Instant::now();
    let r = run_test_inner::<C, Postgres, F, T>(
        &DB_LIST,
        mig_dir,
        extra_fixtures_dir,
        cleanup_dir,
        test_fn,
    );
    let r = match r.await {
        Err(e) => panic!("Test failed: {e}: {:?}", e.source()),
        Ok(r) => r,
    };
    println!("Whole test: {:?}", f.elapsed());
    r
}

/// NB: Broken things outside of the test permanently lock the database for the rest
///     of the test series. This is a feature, not a bug.
async fn run_test_inner<C, D, F, T>(
    list: &DbLock<D>,
    migrations_dir: &str,
    extra_fixtures_dir: &str,
    cleanup_dir: &str,
    test_fn: impl FnOnce(Pool<D>) -> F + Send + 'static,
) -> Result<T>
where
    C: ConfigDriver,
    D: Database + MigrateDatabase,
    F: Future<Output = Result<T>> + Send + 'static,
    for<'a> <<&'a Pool<D> as Acquire<'a>>::Connection as Deref>::Target: Migrate,
    for<'a> &'a mut <D as Database>::Connection: Executor<'a>,
    DbList<D>: NameDb,
    T: Send + Sync + 'static,
{
    let test_config = C::initialise();
    let name = test_config.db_name_root();
    let address = test_config.db_host();

    // If different root names are used, we create a new database list for each one,
    // else we end up getting tangled in names.
    let db_list = list.get_or_init(|| {
        let mut map = AHashMap::new();
        map.insert(name.clone(), DbList::new(&address, &name));
        Arc::new(Mutex::new(map))
    });
    let (pool, db_ref) = db_list
        .lock()
        .await
        .entry(name.clone())
        .or_insert_with(|| DbList::new(&address, &name))
        .get_pool()
        .await?;
    println!("DBREF={db_ref:?}");
    let _ = db_list;

    // Run the migrations. NB: Migrations should not be rerun for existing DBs.
    let m_dir = PathBuf::from(migrations_dir).join("up");
    let mut m = Migrator::new(m_dir.clone()).await?;
    m.set_ignore_missing(true); // TODO: find out where this bug comes from.

    let e = std::time::Instant::now();
    if let Err(e) = m.run(&pool).await {
        println!("Migration error for {}", db_ref.db_ref.name);
        return Err(e.into());
    }
    println!("Migration time: {:?}", e.elapsed());

    // Run the fixtures (they do not use a migrator, since they should be rerun.)

    // Detatch a connection for cleanup after the test.
    let mut cleanup_conn = pool.acquire().await?;
    cleanup_conn.close_on_drop();
    let mut cleanup_conn = cleanup_conn.detach();
    run_fixtures::<D>(migrations_dir, extra_fixtures_dir, &mut cleanup_conn).await?;
    // Spawn and run the test
    let e = std::time::Instant::now();
    let handle = tokio::task::spawn(async move { test_fn(pool).await });

    // Collect the results.
    let mut output = None;
    let (success, msg) = match handle.await {
        // If the test panics we should get here.
        Err(e) => (false, format!("Panic in test: {e}")),
        // If we have a result in the test we should end up here.
        Ok(Err(e)) => (false, format!("Error in test: {e}")),
        // If the test is executed successfully then we end up here.
        Ok(Ok(x)) => {
            output = Some(x);
            (true, "Test success".to_string())
        }
    };
    println!("TEST RESULT {:?}: {msg}", e.elapsed());
    // Get the cleanup script.
    let cleanup_addr = PathBuf::from(cleanup_dir).join("cleanup.sql");
    let cleanup_script = std::fs::read_to_string(&cleanup_addr).inspect_err(|e| {
        println!("Error reading cleanup script from '{cleanup_addr:?}': {e}");
    })?;
    // Execute the cleanup script. Make sure the change is committed before the
    // next test is allowed to begin.
    let e = std::time::Instant::now();
    let mut tx = cleanup_conn.begin().await?;
    sqlx::raw_sql(AssertSqlSafe(cleanup_script))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    println!("Cleanup: {:?}", e.elapsed());

    // mark resources as free
    let db_list = list
        .get()
        .ok_or_else(|| std::io::Error::other("This is initialised earlier in this fn"))?;
    let db_name = db_ref.db_ref.name.to_string();
    db_list
        .lock()
        .await
        .get_mut(&name)
        .expect("Impossible lock")
        .free(&db_ref.db_ref.name)
        .await?;
    // Display result.
    if !success {
        let msg = format!("TEST FAILED: {msg}\n    ON: {db_name}");
        return Err(std::io::Error::other(msg).into());
    }
    Ok(output.expect("Guaranteed by previous checks."))
}
