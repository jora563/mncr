//! Модуль тестов для сервера.
//! Для полноценного тестирования сервера, требуется поднять базу данных, заполнить её
//! данными, и только тогда кидать запросы на сам сервер.
use crate::config::Config;
use crate::context::CoreCtx;
use crate::http_server::admin_api::{
    IncomingNewBotAccount, IncomingNewProject, IncomingNewProjectGroup,
};
use crate::http_server::operator_api::ws_protocol as wsp;

use db::core_schema::*;
use db::test_frame::{ConfigDriver, run_test_postgres};
use futures_util::StreamExt;
use futures_util::sink::SinkExt;
use reqwest::StatusCode;
use reqwest_websocket as rwsc;
use reqwest_websocket::Upgrade;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

const AUTH: &str = "Authorization";
const ADMIN_TOKEN: &str = "Bearer \
    eyJhbGciOiJIUzUxMiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICIwOTRiMDgyMC0zZTIyLTQzMDYtODM1YS1iNTFiNjY5MmU2NzEifQ.\
    ewogICJwZXJzb25hbF9pZCI6InNvbWUtcGVyc29uYWwtaWQiLAogICJyb2xlIjoiYWRtaW4iCn0=.\
    fake-verification";
const OPERATOR_TOKEN: &str = "Bearer \
    eyJhbGciOiJIUzUxMiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICIwOTRiMDgyMC0zZTIyLTQzMDYtODM1YS1iNTFiNjY5MmU2NzEifQ.\
    ewogICJwZXJzb25hbF9pZCI6InNvbWUtcGVyc29uYWwtaWQiLAogICJyb2xlIjoib3BlcmF0b3IiCn0=.\
    fake-verification";

#[derive(Debug, Deserialize)]
struct KeycloakResponse {
    access_token: String,
}

struct ServerTestDriverAdmin;

impl ConfigDriver for ServerTestDriverAdmin {
    fn initialise() -> Self {
        Self
    }
    fn db_name_root(&self) -> Box<str> {
        "ai_omni_server_admin_test_db".into()
    }
    fn db_host(&self) -> Box<str> {
        "postgresql://aio_core:password@127.0.0.1:5432".into()
    }
}

struct ServerTestDriverOperator;

impl ConfigDriver for ServerTestDriverOperator {
    fn initialise() -> Self {
        Self
    }
    fn db_name_root(&self) -> Box<str> {
        "ai_omni_server_operator_test_db".into()
    }
    fn db_host(&self) -> Box<str> {
        "postgresql://aio_core:password@127.0.0.1:5432".into()
    }
}

#[cfg(feature = "test-aiomni-llm")]
struct ServerTestDriverLlm;

#[cfg(feature = "test-aiomni-llm")]
impl ConfigDriver for ServerTestDriverLlm {
    fn initialise() -> Self {
        Self
    }
    fn db_name_root(&self) -> Box<str> {
        "ai_omni_server_llm_test_db".into()
    }
    fn db_host(&self) -> Box<str> {
        "postgresql://aio_core:password@127.0.0.1:5432".into()
    }
}

async fn query_keycloak_token(
    ctx: &CoreCtx,
    token: &str,
    host: &str,
    client: &reqwest::Client,
) -> String {
    let auth_cfg = ctx.cfg().auth().to_owned();

    let our_realm = auth_cfg.realm().to_owned();
    let client_id = auth_cfg.client_id().to_owned();
    let client_secret = auth_cfg.client_secret().map(|x| x.to_string()).unwrap();

    // Если у нас внешний keycloak то мы у него запрашиваем токен.
    if cfg!(feature = "external-keycloak") {
        let data = [
            ("client_id", &client_id as &str),
            ("client_secret", &client_secret as &str),
            ("grant_type", "password"),
            ("username", "105127"),
            ("password", "user-1"),
        ];
        let res = client
            .post(format!(
                "http://{host}:9999/realms/{our_realm}/protocol/openid-connect/token"
            ))
            .form(&data.into_iter().collect::<HashMap<_, _>>())
            .send()
            .await
            .unwrap();
        let text = res.text().await.unwrap();
        println!("token response: {text}");
        let res: KeycloakResponse = serde_json::from_str(&text).unwrap();
        res.access_token
    } else {
        token.to_string()
    }
}

fn mock_auth_service(ctx: &CoreCtx) -> tokio::task::JoinHandle<()> {
    let auth_cfg = ctx.cfg().auth().to_owned();

    // Если у нас внешний keycloak то мы не запускаем собственный сервер.
    tokio::task::spawn(async move {
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
    })
}

/// Все тесты сервера бегут в одном тесте чтобы не долбаться с настройками баз данных.
/// NB: Тест обычно проворачивается с мок-токеном, но его можно также провернуть с
/// настоящим токеном полученом из настоящего keycloak. В таком случае настройки keycloak
/// в конфигурационных файлах должна соответствовать данным реальной инстанции keycloak.
/// NB: This may not work properly with external keycloak, if the roles are not set up properly
///     on the keycloak instance.
/// NB: This may not work properly with external ASAA if the projects do not match those for the user.
#[tokio::test]
async fn test_server_admin_api() {
    run_test_postgres::<ServerTestDriverAdmin, _, ()>(
        "../sql/core/",
        "test/fixture/",
        "test/cleanup/",
        |_| async move {
            let mut config = Config::from_file("test/server-admin-test-config.toml").unwrap();

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

            // Если у нас внешний keycloak то мы не запускаем собственный сервер.
            let mock_auth_services = mock_auth_service(&ctx);
            println!("Sleeping for 400ms..");
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            println!("Host name: {host}");

            let client = reqwest::Client::new();

            // Если у нас внешний keycloak то мы у него запрашиваем токен.
            let admin_token = query_keycloak_token(&ctx, ADMIN_TOKEN, host, &client).await;

            ///////////////////////////////////////////////////////////////////
            // Preliminary cleanup for aiomni-llm. We use the direct interface since the
            // project might not exist in our DB.
            #[cfg(feature = "test-aiomni-llm")]
            for i in 1..10 {
                let response = client
                    .delete(format!("http://{host}:8000/api/projects?project_id={i}"))
                    .header(AUTH, &admin_token)
                    .send()
                    .await
                    .unwrap();
                examine_response(response).await;
            }

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
                .get(format!("http://{host}:8081/v1/admin_api/project_groups"))
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

/// Используется для DRY.
#[cfg(feature = "test-aiomni-llm")]
async fn examine_response(response: reqwest::Response) -> StatusCode {
    let status = response.status();
    println!("{response:?}");
    let text = response.text().await.unwrap_or_else(|e| e.to_string());
    println!("CreateProjectrequest: {text:?}");
    status
}

#[cfg(feature = "test-aiomni-llm")]
#[tokio::test]
async fn test_server_llm_engine_api() {
    // Импорты уникальны для этой фичи
    use llm::messages::*;
    use reqwest::multipart::{Form, Part};

    let mut config = Config::from_file("test/server-llm-test-config.toml").unwrap();

    run_test_postgres::<ServerTestDriverLlm, _, ()>(
        "../sql/core/",
        "test/fixture/",
        "test/cleanup/",
        |_| async move {
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
                let _ = super::run_server(ctx2)
                    .await
                    .inspect_err(|e| println!("Server run error: {e}"));
            });
            // Wait for launch.
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;

            println!("Host name: {host}");
            let client = reqwest::Client::new();

            let token = query_keycloak_token(&ctx, ADMIN_TOKEN, host, &client).await;
            let mock_auth_services = mock_auth_service(&ctx);

            // Preliminary cleanup
            for i in 1..10 {
                let response = client
                    .delete(format!("http://{host}:8787/v1/admin_api/llm/project/{i}"))
                    .header(AUTH, &token)
                    .send()
                    .await
                    .unwrap();
                examine_response(response).await;
            }

            println!("----Preliminary Deleted----");

            ///////////////////////////////////////////////////////////////////
            // POST projects
            // See fixtures.
            let request = CreateProjectRequest {
                project_id: 1,
                project_name: "Good Project I".into(),
                system_prompt: Some("You're one of the good guys".into()),
                fallback_message: Some("Remember, we're the good guys.".into()),
            };
            let response = client
                .post(format!("http://{host}:8787/v1/admin_api/llm/projects"))
                .header(AUTH, &token)
                .json(&request)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);

            ///////////////////////////////////////////////////////////////////
            // GET project by id
            let response = client
                .get(format!("http://{host}:8787/v1/admin_api/llm/project/1"))
                .header(AUTH, &token)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);

            ///////////////////////////////////////////////////////////////////
            // GET projects
            let response = client
                .get(format!("http://{host}:8787/v1/admin_api/llm/projects"))
                .header(AUTH, &token)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);

            ///////////////////////////////////////////////////////////////////
            // PUT projects (update)
            let mut request = UpdateProjectRequest {
                project_id: 1,
                project_name: Some("Project Teapot".into()),
                system_prompt: Some("You are an assistant who does their best to help.".into()),
                fallback_message: None,
            };
            let response = client
                .put(format!("http://{host}:8787/v1/admin_api/llm/projects"))
                .header(AUTH, &token)
                .json(&request)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::FORBIDDEN);

            request.project_name = Some("Good Project I".into());
            let response = client
                .put(format!("http://{host}:8787/v1/admin_api/llm/projects"))
                .header(AUTH, &token)
                .json(&request)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);

            /////////////////////////////////////////////////////////////////
            // GET training job by id
            let response = client
                .get(format!(
                    "http://{host}:8787/v1/admin_api/llm/training/job/1"
                ))
                .header(AUTH, &token)
                .send()
                .await
                .unwrap();
            // let status = examine_response(response).await;
            let status = response.status();
            println!("{response:?}");
            let err = response.text().await.unwrap();
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(
                &err,
                "Llm interface error: Error from LLM: Training job 1 not found (code: JOB_NOT_FOUND)"
            );

            /////////////////////////////////////////////////////////////////
            // POST a lora adaptor (this adaptor is invalid)
            let file = b"I am a fake adaptor file".to_vec();
            let form = Form::new()
                    .text("project_id", "1")
                    .part("file", Part::bytes(file));
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/adaptor"
                ))
                .header(AUTH, &token)
                .multipart(form)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW

            // ТОДО: THIS TEST WILL NOT WORK UNTIL AIOMNI-LLM ITSELF IS FIXED.
            ///////////////////////////////////////////////////////////////////
            // POST QA in CSV format.
            let form = Form::new()
                    .text("project_id", "1")
                    .text("column_question", "answer")
                    .text("column_answer", "answer")
                    .part("file", Part::file("../.test-settings/llm-examples/bank_kb.csv").await.unwrap());
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/knowledge"
                ))
                .header(AUTH, &token)
                .multipart(form)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW

            ///////////////////////////////////////////////////////////////////
            // POST dataset in JSONL format.
            let form = Form::new()
                    .text("project_id", "1")
                    .part("file", Part::file("../.test-settings/llm-examples/bank_dataset.jsonl").await.unwrap());
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/dataset?project_id=1"
                ))
                .header(AUTH, &token)
                .multipart(form)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW

            ///////////////////////////////////////////////////////////////////
            // POST typical questions in a simple format.
            let file = "Как заблокировать карту?\nКак восстановить пароль?\n".as_bytes();
            let form = Form::new()
                    .text("project_id", "1")
                    .part("file", Part::bytes(file));
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/questions"
                ))
                .header(AUTH, &token)
                .multipart(form)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW

            ///////////////////////////////////////////////////////////////////
            // POST reload projects.
            let mut body = HashMap::<String, serde_json::Value>::new();
            body.insert("project_id".into(), 1.into());
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/reload"
                ))
                .header(AUTH, &token)
                .json(&body)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW

            ///////////////////////////////////////////////////////////////////
            // POST rebuild index.
            let mut body = HashMap::<String, serde_json::Value>::new();
            body.insert("project_id".into(), 1.into());
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/build_index"
                ))
                .header(AUTH, &token)
                .json(&body)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW

            ///////////////////////////////////////////////////////////////////
            // POST rebuild index.
            let mut body = HashMap::<String, serde_json::Value>::new();
            body.insert("project_id".into(), 1.into());
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/build_index"
                ))
                .header(AUTH, &token)
                .json(&body)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK); // FOR NOW


            ///////////////////////////////////////////////////////////////////
            // POST project train (probably should not be attempted in tests.)
            let request = TrainingRequest {
                project_id: 1,
                epochs: None,
                learning_rate: None,
                batch_size: None,
                lora_r: None,
                lora_alpha: None,
            };
            let response = client
                .post(format!(
                    "http://{host}:8787/v1/admin_api/llm/projects/train"
                ))
                .header(AUTH, &token)
                .json(&request)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);


            ///////////////////////////////////////////////////////////////////
            // GET training jobs for project
            let response = client
                .get(format!(
                    "http://{host}:8787/v1/admin_api/llm/training/jobs_by_project/1"
                ))
                .header(AUTH, &token)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);

            ///////////////////////////////////////////////////////////////////
            // DELETE project by id
            let response = client
                .get(format!("http://{host}:8787/v1/admin_api/llm/project/1"))
                .header(AUTH, &token)
                .send()
                .await
                .unwrap();
            let status = examine_response(response).await;
            assert_eq!(status, StatusCode::OK);

            // Закончить работу с сервисами.
            service.abort();
            mock_auth_services.abort();
            Ok(())
        },
    )
    .await
}

/// Все тесты сервера бегут в одном тесте чтобы не долбаться с настройками баз данных.
/// NB: This may not work properly with external keycloak, if the roles are not set up properly
///     on the keycloak instance.
/// NB: This may not work properly with external ASAA if the projects do not match those for the user.
#[tokio::test]
async fn test_server_operator_api() {
    run_test_postgres::<ServerTestDriverOperator, _, ()>(
        "../sql/core/",
        "test/fixture/",
        "test/cleanup/",
        |_| async move {
            let mut config = Config::from_file("test/server-operator-test-config.toml").unwrap();

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
                let _ = super::run_server(ctx2)
                    .await
                    .inspect_err(|e| println!("Server run error: {e}"));
            });
            // Wait for launch.
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;

            println!("Host name: {host}");
            let client = reqwest::Client::new();

            let token = query_keycloak_token(&ctx, OPERATOR_TOKEN, host, &client).await;
            let mock_auth_services = mock_auth_service(&ctx);

            // Try to websocket on operator_api with the right headers
            let response = client
                .get(format!("ws://{host}:8082/v1/operator_api/chat"))
                // `reqwest_websocket` works like this.
                .header("Connection", "upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header(AUTH, &token)
                .upgrade()
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

            let mut socket = response.into_websocket().await.unwrap();

            ///////////////////////////////////////////////////////////////////
            // Message history get request.
            //////////////////////////////////
            let inner = wsp::WsRequest::MessageHistoryGet(wsp::MessageHistoryGetRequest {
                ticket_id: 1,
                last_message_id: None,
                count: None,
            });
            let event = send_recv_ws_event(&mut socket, wsp::WsRequestMsg::new(1, inner)).await;

            assert_eq!(event.request_id, Some(1));
            assert!(matches!(event.inner, wsp::WsEvent::MessageHistoryGot(_)));

            ///////////////////////////////////////////////////////////////////
            // IFrame request.
            /////////////////////
            let inner = wsp::WsRequest::IFrameGet(wsp::IFrameGetRequest);
            let event = send_recv_ws_event(&mut socket, wsp::WsRequestMsg::new(12, inner)).await;

            assert_eq!(event.request_id, Some(12));
            assert!(matches!(event.inner, wsp::WsEvent::IFrameGot(_)));

            ///////////////////////////////////////////////////////////////////
            // ChatStatusChangedEvent request.
            /////////////////////
            let inner = wsp::WsRequest::ChatStatusChange(wsp::ChatStatusChangeRequest {
                ticket_id: 1,
                ticket_status: 4,
            });
            let event = send_recv_ws_event(&mut socket, wsp::WsRequestMsg::new(23, inner)).await;

            assert_eq!(event.request_id, Some(23));
            assert!(matches!(event.inner, wsp::WsEvent::ChatStatusChanged(_)));

            ///////////////////////////////////////////////////////////////////
            // ChatStatusChangedEvent request with error (ticket does not exist).
            ////////////////////////////////////////////////////////////////////////
            let inner = wsp::WsRequest::ChatStatusChange(wsp::ChatStatusChangeRequest {
                ticket_id: 99999,
                ticket_status: 4,
            });
            let event = send_recv_ws_event(&mut socket, wsp::WsRequestMsg::new(23, inner)).await;

            assert_eq!(event.request_id, None);
            assert!(matches!(event.inner, wsp::WsEvent::Error(_)));

            ///////////////////////////////////////////////////////////////////
            // MessageSendRequest request.
            /////////////////////////////////
            // NB: This test works because there are no chats connected to the messengers, so
            // nothing is sent to the chats.
            let inner = wsp::WsRequest::MessageSend(wsp::MessageSendRequest {
                ticket_id: 99999,
                message: "I like green eggs. I like green eggs and ham.".into(),
            });
            let event = send_recv_ws_event(&mut socket, wsp::WsRequestMsg::new(34, inner)).await;

            assert_eq!(event.request_id, None);
            assert!(matches!(event.inner, wsp::WsEvent::Error(_)));

            service.abort();
            mock_auth_services.abort();
            Ok(())
        },
    )
    .await
}

/// A send and receive function for websocket tests for DRY.
async fn send_recv_ws_event(soc: &mut rwsc::WebSocket, req: wsp::WsRequestMsg) -> wsp::WsEventMsg {
    let req = wsp::WsTextMessage::Request(req);
    let inner_msg = serde_json::to_string(&req).unwrap();
    println!("{inner_msg}");
    let req = rwsc::Message::Text(inner_msg);

    soc.send(req).await.unwrap();

    let event = match soc.next().await.unwrap().unwrap() {
        rwsc::Message::Text(event) => event,
        x => panic!("Expected text, got god knows what: {x:?}"),
    };
    let wsp::WsTextMessage::Event(event) = serde_json::from_str(&event).unwrap() else {
        panic!("Serde says no.");
    };
    event
}
