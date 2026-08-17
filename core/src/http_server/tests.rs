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
use futures::StreamExt;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

const AUTH: &str = "Authorization";
const ADMIN_TOKEN: &str = "Bearer \
    eyJhbGciOiJIUzUxMiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICIwOTRiMDgyMC0zZTIyLTQzMDYtODM1YS1iNTFiNjY5MmU2NzEifQ.\
    ewogICJwZXJzb25hbF9pZCI6InNvbWUtcGVyc29uYWwtaWQiLAogICJyb2xlIjoiYWRtaW4iCn0=.\
    fake-verification";
const _OPERATOR_TOKEN: &str = "Bearer \
    eyJhbGciOiJIUzUxMiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICIwOTRiMDgyMC0zZTIyLTQzMDYtODM1YS1iNTFiNjY5MmU2NzEifQ.\
    ewogICJwZXJzb25hbF9pZCI6InNvbWUtcGVyc29uYWwtaWQiLAogICJyb2xlIjoib3BlcmF0b3IiCn0=.\
    fake-verification";

#[derive(Debug, Deserialize)]
struct KeycloakResponse {
    access_token: String,
}

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
/// NB: Тест обычно проворачивается с мок-токеном, но его можно также провернуть с
/// настоящим токеном полученом из настоящего keycloak. В таком случае настройки keycloak
/// в конфигурационных файлах должна соответствовать данным реальной инстанции keycloak.
#[tokio::test]
async fn test_server_admin_api() {
    run_test_postgres::<ServerTestDriver, _>(
        "../sql/core/",
        "test/fixture/",
        "test/cleanup/",
        |_| async move {
            let mut config = Config::from_file("test/server-test-config.toml").unwrap();


            // Jenkins docker in docker builds do not like the words "localhost"
            let host = if std::env::var("AIOMNI_JENKINS_BUILD").is_ok() {
                println!("Building on Jenkins... ");
                config.auth_mut().set_asaa_home("http://127.0.0.1:9090");
                config.auth_mut().set_keycloak_home("http://127.0.0.1:9090");
                "127.0.0.1"
            } else {
                "localhost"
            };


            let ctx = Arc::new(CoreCtx::new(config).await.unwrap());
            let ctx2 = ctx.clone();

            let service = tokio::task::spawn(async move {
                let _ = super::run_server(ctx2).await.inspect_err(|e| println!("Server run error: {e}"));
            });

            let auth_cfg = ctx.cfg().auth().to_owned();
            let our_realm = auth_cfg.realm().to_owned();
            let client_id = auth_cfg.client_id().to_owned();
            let client_secret = auth_cfg.client_secret().map(|x| x.to_string()).unwrap();

            // Если у нас внешний keycloak то мы не запускаем собственный сервер.
            let mock_auth_services = tokio::task::spawn(async move {
                if cfg!(feature = "external-asaa") && cfg!(feature = "external-keycloak") {
                // Do nothing if everything is external.
                } else if cfg!(feature = "external-asaa") {
                // If only asaa is external, run keycloak internally
                    uzor_plugin::mock_server::run_mock_keycloak_server(&auth_cfg)
                        .await
                        .inspect_err(|e| println!("Server run error: {e:?}"))
                        .unwrap();
                } else if cfg!(feature = "external-keycloak") {
                    // if only keycloak is internal, run asaa internally
                    uzor_plugin::mock_server::run_mock_asaa_server(&auth_cfg)
                        .await
                        .inspect_err(|e| println!("Server run error: {e:?}"))
                        .unwrap();
                } else {
                    // If nothing is external, run everything internally.
                    uzor_plugin::mock_server::run_mock_auth_servers(&auth_cfg)
                        .await
                        .inspect_err(|e| println!("Server run error: {e:?}"))
                        .unwrap();
                };
            });
            println!("Sleeping for 400ms..");
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            println!("Host name: {host}");

            let client = reqwest::Client::new();

            // Если у нас внешний keycloak то мы у него запрашиваем токен.
            let admin_token = if cfg!(feature = "external-keycloak") {
                let data = [
                    ("client_id", &client_id as &str),
                    ("client_secret", &client_secret as &str),
                    ("grant_type", "password"),
                    ("username", "105127"),
                    ("password", "user-1"),
                ];
                let res = client
                    .post(format!("http://{host}:9999/realms/{our_realm}/protocol/openid-connect/token"))
                    .form(&data.into_iter().collect::<HashMap<_, _>>())
                    .send()
                    .await
                    .unwrap();
                let text = res.text().await.unwrap();
                println!("token response: {text}");
                let res: KeycloakResponse  = serde_json::from_str(&text).unwrap();
                res.access_token
            } else {
                ADMIN_TOKEN.to_string()
            };
            ///////////////////////////////////////////////////////////////////
            // Healthcheck
            let response = client
                .get(format!("http://{host}:8081/health"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let text = response.text().await.unwrap();
            assert_eq!(text, "AIOMNI Core is healthy");

            // get server configurations (todo: Mask passwords)
            let response = client
                .get(format!("http://{host}:8081/config"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let cfg: Config = response.json().await.unwrap();
            assert_eq!(&cfg, ctx.cfg());

            ///////////////////////////////////////////////////////////////////
            // GET Project groups
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_groups/"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let groups: Vec<DbProjectGroup> = response.json().await.unwrap();
            assert_eq!(groups.len(), 2);

            // GET projects for group 1
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/1/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects2: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects2.projects.len(), 3);

            // GET projects for group 2
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/2/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects2: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects2.projects.len(), 1);

            ///////////////////////////////////////////////////////////////////
            // GET projects for group 5 (doesn't exist)
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/5/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR, "{}", response.text().await.unwrap());
            let text = response.text().await.unwrap();
            assert_eq!(text, "DB error: Error in Database: no rows returned by a query that expected to return at least one row");

            ///////////////////////////////////////////////////////////////////
            // Get bot accounts for project 1 (no bots)
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project/1/bots"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let bots1: Vec<DbBotAccount> = response.json().await.unwrap();
            assert!(bots1.is_empty());

            ///////////////////////////////////////////////////////////////////
            // Get bot accounts for project 2 (2 bots)
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project/2/bots"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let bots2: Vec<DbBotAccountWithMeta> = response.json().await.unwrap();
            assert_eq!(bots2.len(), 2);

            ///////////////////////////////////////////////////////////////////
            // Get a page that doesn't exist.
            let response = client
            .get(format!("http://{host}:8081/v1/admin_api/awdawdawwd")).header(AUTH, &admin_token).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{}", response.text().await.unwrap());
            let text = response.text().await.unwrap();
            assert!(text.is_empty());

            ///////////////////////////////////////////////////////////////////
            // Insert/update tests for project group
            let new_pg = IncomingNewProjectGroup::new("The Best Wonderful Group");

            let response = client
                .post(format!("http://{host}:8081/v1/admin_api/project_group"))
                .header(AUTH, &admin_token)
                .json(&new_pg)
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::CREATED, "{}", response.text().await.unwrap());
            let mut project_group: DbProjectGroup = response.json().await .unwrap();
            assert_eq!(&project_group.group_name, "The Best Wonderful Group");
            assert_ne!(project_group.pkey(), 1);
            assert_ne!(project_group.pkey(), 2);

            let pkey = project_group.pkey();
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/{pkey}/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 0);

            project_group.group_name = String::from("John and John");

            let response = client
                .put(format!("http://{host}:8081/v1/admin_api/project_group"))
                .header(AUTH, &admin_token)
                .json(&project_group)
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/{pkey}/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 0);
            assert_eq!(&projects_new.group.group_name, "John and John");

            ///////////////////////////////////////////////////////////////////
            // Insert/update tests for project
            let new_proj = IncomingNewProject::new(pkey, "P-R311Y-84D", "Project Pager");

            let response = client
                .post(format!("http://{host}:8081/v1/admin_api/project"))
                .header(AUTH, &admin_token)
                .json(&new_proj)
                .send()
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::CREATED, "{}", response.text().await.unwrap());
            let mut project: DbProject = response.json().await .unwrap();
            assert_eq!(&project.external_id, "P-R311Y-84D");
            assert_eq!(&project.project_name, "Project Pager");
            assert_eq!(project.project_group_id, pkey);

            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/{pkey}/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 1);
            assert_eq!(&projects_new.projects[0].project_name, "Project Pager");
            assert_eq!(&projects_new.projects[0].external_id, "P-R311Y-84D");

            project.project_name = String::from("Project Donkey");

            let response = client
                .put(format!("http://{host}:8081/v1/admin_api/project"))
                .header(AUTH, &admin_token)
                .json(&project)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());

            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/{pkey}/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects_new: DbFullProjectGroup = response.json().await.unwrap();
            assert_eq!(projects_new.projects.len(), 1);
            assert_eq!(&projects_new.projects[0].project_name, "Project Donkey");
            assert_eq!(&projects_new.projects[0].external_id, "P-R311Y-84D");

            ///////////////////////////////////////////////////////////////////
            // Insert/update tests for bot account
            let new_bot = IncomingNewBotAccount::new(
                1,
                None,
                "L33t-80t",
                b"ajhwgdliagwbd",
            );

            let response = client
                .post(format!("http://{host}:8081/v1/admin_api/bot"))
                .header(AUTH, &admin_token)
                .json(&new_bot)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::CREATED, "{}", response.text().await.unwrap());
            let mut new_bot: DbBotAccount = response.json().await.unwrap();
            assert_eq!(new_bot.platform_id, 1);
            assert_eq!(&new_bot.external_id, "L33t-80t");
            assert_eq!(&new_bot.token, b"ajhwgdliagwbd");

            let bot_pkey = new_bot.pkey();
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/bot/{bot_pkey}"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let bot: DbBotAccount = response.json().await.unwrap();
            assert_eq!(bot.platform_id, 1);
            assert_eq!(&bot.external_id, "L33t-80t");
            assert_eq!(&bot.token, b"ajhwgdliagwbd");

            new_bot.token = b"I can remember this.".to_vec();

            let response = client
                .put(format!("http://{host}:8081/v1/admin_api/bot"))
                .header(AUTH, &admin_token)
                .json(&new_bot)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());

            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/bot/{pkey}"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let bot: DbBotAccount = response.json().await.unwrap();
            assert_eq!(bot.pkey(), pkey);
            assert_eq!(&bot.token, b"I can remember this.");

            ///////////////////////////////////////////////////////////////////////////////////////
            // Get all permitted projects
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/projects"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let projects: Vec<DbProject> = response.json().await.unwrap();
            assert_eq!(projects.len(), 5);

            ///////////////////////////////////////////////////////////////////////////////////////
            // Get platforms
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/platforms"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let platforms: Vec<DbFullPlatform> = response.json().await.unwrap();
            assert_eq!(platforms.len(), 3);

            ///////////////////////////////////////////////////////////////////////////////////////
            // Delete test for bot.
            let response = client
                .delete(format!("http://{host}:8081/v1/admin_api/bot/{bot_pkey}"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());

            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/bot/{bot_pkey}"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR, "{}", response.text().await.unwrap());


            ///////////////////////////////////////////////////////////////////////////////////////
            // Delete test for project.
            client
                .delete(format!("http://{host}:8081/v1/admin_api/bot/1"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            client
                .delete(format!("http://{host}:8081/v1/admin_api/bot/2"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            client
                .delete(format!("http://{host}:8081/v1/admin_api/bot/3"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            client
                .delete(format!("http://{host}:8081/v1/admin_api/bot/4"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();


            let response = client
                .delete(format!("http://{host}:8081/v1/admin_api/project/4"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project/4"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{}", response.text().await.unwrap());

            ///////////////////////////////////////////////////////////////////////////////////////
            // Delete test for project group.
            let response = client
                .delete(format!("http://{host}:8081/v1/admin_api/project_group/1"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            // We cannot delete a group wioth projects.
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{}", response.text().await.unwrap());

            let response = client
                .delete(format!("http://{host}:8081/v1/admin_api/project_group/2"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            // We cannot delete a group wioth projects.
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());

            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/project_group/2"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{}", response.text().await.unwrap());

            ///////////////////////////////////////////////////////////////////////////////////////
            // Get frontend.
            let response = client
                .get(format!("http://{host}:8081/v1/admin_api/frontend"))
                .header(AUTH, &admin_token)
                .send()
                .await
                .unwrap();
            // We cannot delete a group wioth projects.
            assert_eq!(response.status(), StatusCode::OK, "{}", response.text().await.unwrap());
            let b: Vec<String> = response
                .bytes_stream()
                .map(|x| {
                    let x = x.unwrap_or_default()[..].to_vec();
                    String::from_utf8(x).unwrap()
                })
                .collect::<Vec<String>>().await;

            assert_eq!(b.len(), 1);
            assert_eq!(b[0], r#"<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="utf-8">
</head>

<body>HELLO WORLD</body>

</html>"#);

            service.abort();
            mock_auth_services.abort();
            Ok(())
        },
    )
    .await
}
