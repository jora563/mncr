//! This submodule contains the code that keeps tracks of and creates databases
//! for testing purposes.
//! Тут копия кода для создания и отслеживания тестовых ДБ. Для тестов этого кода см.:
//! <https://bitbucket.telecontact.ru/projects/TELECONTACT/repos/telecontact-rust-libs/browse/sqlx-db-pool/src/db_list.rs>

use std::marker::PhantomData;

use sqlx::migrate::MigrateDatabase;
use sqlx::{Any, Postgres};
use sqlx::{Database, Pool};

use super::Result;

/// A structure for keeping track of databases.
#[derive(Debug)]
pub struct DbList<T: Database + MigrateDatabase> {
    address_root: Box<str>,
    name_root: Box<str>,
    // This is the idx of the next database to use for tests.
    next_idx: Option<usize>,
    dbs: Vec<DbRef>,
    // This is the next number to use in a name. It is never decremented.
    // Thus we cannot have more than [`usize::MAX`] tests per run.
    n: usize,
    _ph: PhantomData<T>,
}

impl<T: Database + MigrateDatabase> DbList<T> {
    /// This is a pseudo drop implementation.
    pub async fn drop(self) {
        for db in self.dbs {
            let _ = <T as MigrateDatabase>::drop_database(&db.name).await;
        }
    }
}

/// A structure that keeps a DB name and says whether or not it is
/// occupied or not.
#[derive(Debug, Clone)]
pub struct DbRef {
    pub(crate) name: Box<str>,
    in_use: bool,
}

impl DbRef {
    fn make_ref<T: Database>(&self) -> BorrowedDbRef<T> {
        let db_ref = self.clone();
        let _ph = PhantomData {};
        BorrowedDbRef { db_ref, _ph }
    }
}

///A reference to a DbRef which frees the DbRef when used.
#[derive(Debug)]
pub(crate) struct BorrowedDbRef<T: Database> {
    pub(crate) db_ref: DbRef,
    _ph: PhantomData<T>,
}

pub(crate) trait NameDb {
    fn name_db(&self) -> Result<Box<str>> {
        Ok("".into())
    }
}

impl<T: Database + MigrateDatabase> DbList<T> {
    fn get_name_params(&self) -> (usize, &str, &str) {
        (self.n, &self.name_root, &self.address_root)
    }
}

impl NameDb for DbList<Any> {}

impl NameDb for DbList<Postgres> {
    fn name_db(&self) -> Result<Box<str>> {
        let (n, name_root, address_root) = self.get_name_params();

        Ok(format!("{address_root}/{name_root}_{n}").into_boxed_str())
    }
}

#[allow(private_bounds)]
impl<T: Database + MigrateDatabase> DbList<T>
where
    DbList<T>: NameDb,
{
    /// Creates a new instance of [`DbList`]. This does not create any databases
    /// or check the validity of paths, but simply creates an empty instance.
    /// The address root and name root should be obtained using the [`NameConfig`] trait.
    pub(crate) fn new(address_root: &str, name_root: &str) -> Self {
        Self {
            address_root: address_root.to_owned().into_boxed_str(),
            name_root: name_root.to_owned().into_boxed_str(),
            next_idx: None,
            dbs: Vec::with_capacity(16),
            n: 0,
            _ph: PhantomData {},
        }
    }

    fn add_ref(&mut self, name: Box<str>) -> BorrowedDbRef<T> {
        let in_use = true;
        let db_ref = DbRef { name, in_use };
        let useable_ref = db_ref.make_ref();

        self.dbs.push(db_ref);
        self.next_idx = Some(self.n);
        self.n += 1;

        useable_ref
    }
    /// Spawn a new database using the existing naming convention.
    async fn spawn(&mut self) -> Result<BorrowedDbRef<T>> {
        let name = self.name_db()?;
        if !<T as MigrateDatabase>::database_exists(&name).await? {
            <T as MigrateDatabase>::create_database(&name).await?;
        }
        Ok(self.add_ref(name))
    }
    /// Mark a given database as free. NB: We first check that the DB
    /// exists in our list.
    pub(crate) async fn free(&mut self, name: &str) -> Result<()> {
        if let Some(n) = self.dbs.iter().position(|x| *x.name == *name) {
            self.dbs[n].in_use = false;
            self.next_idx = Some(n);
        } else {
            let msg = format!("Database {name} is not listed in manager");
            return Err(std::io::Error::other(msg).into());
        }
        Ok(())
    }
    /// Delete a given database. NB: We first check that the DB exists in our list.
    #[allow(dead_code)]
    async fn delete(&mut self, name: &str) -> Result<()> {
        // Find the DB we wish to delete.
        if let Some(n) = self.dbs.iter().position(|x| *x.name == *name) {
            // Delete it.
            self.dbs.remove(n);
            // Find if we have a free DB to use.
            self.next_idx = self.dbs.iter().position(|x| !x.in_use);
        } else {
            let msg = format!("Database {name} is not listed in manager");
            return Err(std::io::Error::other(msg).into());
        }
        if <T as MigrateDatabase>::database_exists(name).await? {
            <T as MigrateDatabase>::drop_database(name).await?;
        }
        Ok(())
    }

    /// This function gets the connection pool for the first free database pool in
    /// existence. If no database pool exists, then we create a database and connect
    /// to it.
    ///
    /// Currently there is no limit on the number of databases that this function
    /// creates.
    pub(crate) async fn get_pool(&mut self) -> Result<(Pool<T>, BorrowedDbRef<T>)> {
        // First we try to raise an existing DB.
        if let Some(i) = self.next_idx {
            println!("Next proposed DBS: [{i}]={:?}", self.dbs[i]);
            let pool = Pool::<T>::connect(&self.dbs[i].name).await?;
            let db_ref = self.dbs[i].make_ref();
            // If we are programming defensively, we should also check for this state,
            // but in fact the selection of the position below guarantees that we will
            // not select this number again.
            self.dbs[i].in_use = true;
            self.next_idx = self.dbs.iter().position(|x| !x.in_use);
            return Ok((pool, db_ref));
        }
        // IF there is no existing BD marked as free, we create a new one.
        let db_ref = self.spawn().await?;
        let pool = Pool::<T>::connect(&db_ref.db_ref.name).await?;
        self.next_idx = None;
        Ok((pool, db_ref))
    }
}
