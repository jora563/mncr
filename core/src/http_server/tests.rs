//! Модуль тестов для сервера.
//! Для полноценного тестирования сервера, требуется поднять базу данных, заполнить её
//! данными, и только тогда кидать запросы на сам сервер.
use crate::config::Config;
use crate::context::CoreCtx;
use crate::http_server::admin_api::{
    IncomingNewBotAccount, IncomingNewProject, IncomingNewProjectGroup,
};

use db::core_schema::*;
use db::test_frame::{ConfigDriver, run_test_postgres};
use reqwest::StatusCode;
use std::sync::Arc;

struct ServerTestDriver;

impl ConfigDriver for ServerTestDriver {
    fn initialise() -> Self {
        Self
    }
    fn db_name_root(&self) -> Box<str> {
        "ai_omni_server_test_db".into()
    }
    fn db_host(&self) -> Box<str> {
        "postgresql://aio_core:password@127.0.0.1:5432".into()
    }
}

/// Все тесты сервера бегут в одном тесте чтобы не долбаться с настройками баз данных.
#[tokio::test]
async fn test_server_admin_api() {
    run_test_postgres::<ServerTestDriver, _>(
        "../sql/core/",
        "test/fixture/",
        "test/cleanup/",
        |_| async move {
            let config = Config::from_file("test/server-test-config.toml").unwrap();
            let ctx = Arc::new(CoreCtx::new(config).await.unwrap());

            let service = tokio::task::spawn(super::run_server(ctx.clone()));
            ///////////////////////////////////////////////////////////////////
            // Healthcheck
            let response = reqwest::get("http://localhost:8081/health_check").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let text = response.text().await.unwrap();
            assert_eq!(text, "AIOMNI Core is healthy");

            // get server configurations (todo: Mask passwords)
            let response = reqwest::get("http://localhost:8081/config").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let cfg: Config = response.json().await.unwrap();
            assert_eq!(&cfg, ctx.cfg());

            ///////////////////////////////////////////////////////////////////
            // GET Project groups
            let response = reqwest::get("http://localhost:8081/v1/admin_api/project_groups/").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let groups: Vec<DbProjectGroup> = response.json().await.unwrap();
            assert_eq!(groups.len(), 2);

            // GET projects for group 1
            let response = reqwest::get("http://localhost:8081/v1/admin_api/projects/1").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let projects2: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects2.projects.len(), 3);

            // GET projects for group 2
            let response = reqwest::get("http://localhost:8081/v1/admin_api/projects/2").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let projects2: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects2.projects.len(), 1);

            ///////////////////////////////////////////////////////////////////
            // GET projects for group 5 (doesn't exist)
            let response = reqwest::get("http://localhost:8081/v1/admin_api/projects/5").await.unwrap();
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            let text = response.text().await.unwrap();
            assert_eq!(text, "DB error: Error in Database: no rows returned by a query that expected to return at least one row");

            ///////////////////////////////////////////////////////////////////
            // Get bot accounts for project 1 (no bots)
            let response = reqwest::get("http://localhost:8081/v1/admin_api/bots/1").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bots1: Vec<DbBotAccount> = response.json().await.unwrap();
            assert!(bots1.is_empty());

            ///////////////////////////////////////////////////////////////////
            // Get bot accounts for project 2 (2 bots)
            let response = reqwest::get("http://localhost:8081/v1/admin_api/bots/2").await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bots2: Vec<DbBotAccountWithMeta> = response.json().await.unwrap();
            assert_eq!(bots2.len(), 2);

            ///////////////////////////////////////////////////////////////////
            // Get a page that doesn't exist.
            let response = reqwest::get("http://localhost:8081/v1/admin_api/awdawdawwd").await.unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            let text = response.text().await.unwrap();
            assert!(text.is_empty());

            let client = reqwest::Client::new();
            ///////////////////////////////////////////////////////////////////
            // Insert/update tests for project group
            let new_pg = IncomingNewProjectGroup::new("PG-B35T", "The Best Wonderful Group");

            let response = client.post("http://localhost:8081/v1/admin_api/project_group/new")
                .json(&new_pg)
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let mut project_group: DbProjectGroup = response.json().await .unwrap();
            assert_eq!(&project_group.external_id, "PG-B35T");
            assert_eq!(&project_group.group_name, "The Best Wonderful Group");
            assert_ne!(project_group.pkey(), 1);
            assert_ne!(project_group.pkey(), 2);

            let pkey = project_group.pkey();
            let response = reqwest::get(format!("http://localhost:8081/v1/admin_api/projects/{pkey}")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 0);

            project_group.group_name = String::from("John and John");

            let response = client.post("http://localhost:8081/v1/admin_api/project_group/update")
                .json(&project_group)
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let response = reqwest::get(format!("http://localhost:8081/v1/admin_api/projects/{pkey}")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 0);
            assert_eq!(&projects_new.group.group_name, "John and John");

            ///////////////////////////////////////////////////////////////////
            // Insert/update tests for project
            let new_proj = IncomingNewProject::new(pkey, "P-R311Y-84D", "Project Pager");

            let response = client.post("http://localhost:8081/v1/admin_api/project/new")
                .json(&new_proj)
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let mut project: DbProject = response.json().await .unwrap();
            assert_eq!(&project.external_id, "P-R311Y-84D");
            assert_eq!(&project.project_name, "Project Pager");
            assert_eq!(project.project_group_id, pkey);

            let response = reqwest::get(format!("http://localhost:8081/v1/admin_api/projects/{pkey}")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 1);
            assert_eq!(&projects_new.projects[0].project_name, "Project Pager");
            assert_eq!(&projects_new.projects[0].external_id, "P-R311Y-84D");

            project.project_name = String::from("Project Donkey");

            let response = client.post("http://localhost:8081/v1/admin_api/project/update")
                .json(&project)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);

            let response = reqwest::get(format!("http://localhost:8081/v1/admin_api/projects/{pkey}")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 1);
            assert_eq!(&projects_new.projects[0].project_name, "Project Donkey");
            assert_eq!(&projects_new.projects[0].external_id, "P-R311Y-84D");

            ///////////////////////////////////////////////////////////////////
            // Insert/update tests for bot account
            let new_bot = IncomingNewBotAccount::new(
                1,
                "L33t-80t",
                b"ajhwgdliagwbd",
            );

            let response = client.post("http://localhost:8081/v1/admin_api/bot/new")
                .json(&new_bot)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let mut new_bot: DbBotAccount = response.json().await.unwrap();
            assert_eq!(new_bot.platform_id, 1);
            assert_eq!(&new_bot.external_id, "L33t-80t");
            assert_eq!(&new_bot.token, b"ajhwgdliagwbd");

            let pkey = new_bot.pkey();
            let response = reqwest::get(format!("http://localhost:8081/v1/admin_api/bot/{pkey}")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bot: DbBotAccount = response.json().await.unwrap();
            assert_eq!(bot.platform_id, 1);
            assert_eq!(&bot.external_id, "L33t-80t");
            assert_eq!(&bot.token, b"ajhwgdliagwbd");

            new_bot.token = b"I can remember this.".to_vec();

            let response = client.post("http://localhost:8081/v1/admin_api/bot/update")
                .json(&new_bot)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);

            let response = reqwest::get(format!("http://localhost:8081/v1/admin_api/bot/{pkey}")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bot: DbBotAccount = response.json().await.unwrap();
            assert_eq!(bot.pkey(), pkey);
            assert_eq!(&bot.token, b"I can remember this.");





            service.abort();
            Ok(())
        },
    )
    .await
}
