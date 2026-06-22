//! Модуль сущностей и функционала проекта.
use db_derive::CoreDbCrud;
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{FromRow, PgExecutor, PgPool};

use crate::core_schema::CoreDbCrud;
use crate::error::{DbError, Result};

/// Сущность проекта
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "project"]
pub struct DbProject {
    #[core_db_skip_insert]
    id: i64,
    /// Ид. группы к который принадлежит
    pub project_group_id: i64,
    /// Внешняя десигнация в системе заказчика
    pub external_id: String,
    /// Наименование проекта
    pub project_name: String,
    #[core_db_skip_insert]
    pub created_on: PrimitiveDateTime,
    pub altered_on: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug)]
pub struct DbNewProject(DbProject);

impl DbNewProject {
    pub fn new(group: &DbProjectGroup, external_id: &str, name: &str) -> Self {
        Self(DbProject {
            id: 0,
            project_group_id: group.id,
            external_id: external_id.to_string(),
            project_name: name.to_string(),
            created_on: PrimitiveDateTime::MIN,
            altered_on: None,
        })
    }

    /// Вставить новый проект
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbProject> {
        let mut p = self.0;
        p.insert(ex).await?;
        Ok(p)
    }
}

impl DbProject {
    pub(super) fn from_tuple(
        row: (
            i64,
            i64,
            String,
            String,
            PrimitiveDateTime,
            Option<PrimitiveDateTime>,
        ),
    ) -> Self {
        Self {
            id: row.0,
            project_group_id: row.1,
            external_id: row.2,
            project_name: row.3,
            created_on: row.4,
            altered_on: row.5,
        }
    }

    pub async fn get_by_group_id<E>(id: i64, ex: E) -> Result<Vec<Self>>
    where
        E: for<'a> sqlx::PgExecutor<'a>,
    {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM project WHERE project_group_id = $1 ORDER BY created_on ASC",
        )
        .bind(id)
        .fetch_all(ex)
        .await
        .map_err(Into::into)
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_by_external_id<'a, T: sqlx::PgExecutor<'a>>(
        ext_id: &str,
        ex: T,
    ) -> Result<Self> {
        let res = Self::get_by_field("external_id", ext_id, ex)
            .await?
            .pop()
            .ok_or_else(|| DbError::not_found("DbProject", "external_id", ext_id))?;
        Ok(res)
    }
}

/// Сущность группы к которой принадлежит проект
#[derive(Clone, CoreDbCrud, Debug, FromRow, PartialEq)]
#[core_db_table = "project_group"]
pub struct DbProjectGroup {
    #[core_db_skip_insert]
    id: i64,
    /// Внешняя десигнация в системе заказчика
    pub external_id: String,
    /// Наименование группы
    pub group_name: String,
    #[core_db_skip_insert]
    pub created_on: PrimitiveDateTime,
    pub altered_on: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug)]
pub struct DbNewProjectGroup(DbProjectGroup);

impl DbNewProjectGroup {
    pub fn new(external: &str, name: &str) -> Self {
        Self(DbProjectGroup {
            id: 0,
            external_id: external.to_string(),
            group_name: name.to_string(),
            created_on: PrimitiveDateTime::MIN,
            altered_on: None,
        })
    }

    /// Вставить новую проектную группу
    pub async fn insert<'a, E: PgExecutor<'a>>(self, ex: E) -> Result<DbProjectGroup> {
        let mut pg = self.0;
        pg.insert(ex).await?;
        Ok(pg)
    }
}

impl DbProjectGroup {
    pub async fn get_projects(self, ex: &PgPool) -> Result<DbFullProjectGroup> {
        Ok(DbFullProjectGroup {
            projects: DbProject::get_by_group_id(self.id, ex).await?,
            group: self,
        })
    }
}

/// Сущность группы со всеми её проектами.
#[derive(Clone, Debug, PartialEq)]
pub struct DbFullProjectGroup {
    pub group: DbProjectGroup,
    pub projects: Vec<DbProject>,
}

impl DbFullProjectGroup {
    pub async fn get_by_id(id: i64, ex: &PgPool) -> Result<Self> {
        Ok(Self {
            group: DbProjectGroup::get_by_id(id, ex).await?,
            projects: DbProject::get_by_group_id(id, ex).await?,
        })
    }
}

#[cfg(test)]
mod tests;
