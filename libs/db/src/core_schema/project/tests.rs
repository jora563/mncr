use super::*;
use crate::core_schema::test_cfg::TestCfg;
use crate::error::DbError;

#[tokio::test]
async fn test_project_group_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
                .insert(&pool)
                .await
                .unwrap();
            let id = project_group.pkey();

            let mut pg = DbProjectGroup::get_by_id(id, &pool).await.unwrap();

            assert_eq!(pg.id, id);
            assert_eq!(project_group.id, id);
            assert_eq!(pg.external_id, project_group.external_id);
            assert_eq!(pg.group_name, project_group.group_name);
            assert_eq!(pg.created_on, project_group.created_on);
            assert_eq!(pg.altered_on, project_group.altered_on);

            pg.group_name = "Clowncorp".to_string();
            pg.update(&pool).await.unwrap();

            let pg2 = DbProjectGroup::get_by_id(id, &pool).await.unwrap();
            assert_eq!(pg, pg2);

            pg.delete(&pool).await.unwrap();
            let err = DbProjectGroup::get_by_id(id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_project_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
                .insert(&pool)
                .await
                .unwrap();
            let pg_id = project_group.pkey();

            let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();
            let id = project.pkey();

            let mut p = DbProject::get_by_id(id, &pool).await.unwrap();

            assert_eq!(p.id, id);
            assert_eq!(p.project_group_id, project.project_group_id);
            assert_eq!(p.external_id, project.external_id);
            assert_eq!(p.project_name, project.project_name);
            assert_eq!(p.created_on, project.created_on);
            assert_eq!(p.altered_on, project.altered_on);

            p.project_name = "The Biggest Spam".to_string();
            p.update(&pool).await.unwrap();

            let p2 = DbProject::get_by_id(id, &pool).await.unwrap();
            assert_eq!(p, p2);

            p.delete(&pool).await.unwrap();
            let err = DbProject::get_by_id(id, &pool).await.unwrap_err();

            DbProjectGroup::delete_by_id(pg_id, &pool).await.unwrap();
            let err2 = DbProjectGroup::get_by_id(pg_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            assert!(matches!(err2, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_full_project_group() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
                .insert(&pool)
                .await
                .unwrap();
            let pg_id = project_group.pkey();

            let pg = DbProjectGroup::get_by_id(pg_id, &pool).await.unwrap();

            let p1 = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();

            let p2 = DbNewProject::new(&project_group, "AKUWDHWA-8692", "The Biggest Spam")
                .insert(&pool)
                .await
                .unwrap();

            let p3 = DbNewProject::new(&project_group, "AKUWDHWA-8712", "The AI Spam")
                .insert(&pool)
                .await
                .unwrap();

            let p4 = DbNewProject::new(&project_group, "AKUWDHWA-8735", "Rise of the Machine God")
                .insert(&pool)
                .await
                .unwrap();

            let fg = pg.clone().get_projects(&pool).await.unwrap();
            let fg2 = DbFullProjectGroup::get_by_id(pg_id, &pool).await.unwrap();
            assert_eq!(fg, fg2);

            assert_eq!(fg.group, pg);
            assert_eq!(fg.projects.len(), 4);
            assert_eq!(fg.projects[0].project_name, p1.project_name);
            assert_eq!(fg.projects[1].project_name, p2.project_name);
            assert_eq!(fg.projects[2].project_name, p3.project_name);
            assert_eq!(fg.projects[3].project_name, p4.project_name);
            Ok(())
        },
    )
    .await
}
