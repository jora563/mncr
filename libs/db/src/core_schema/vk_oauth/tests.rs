use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::vk_oauth::{DbNewVkOauth, DbNewVkOauthState, DbVkOauth, DbVkOauthState};
use crate::core_schema::{ApiId, CoreDbCrud, DbNewPlatform, DbNewProject, DbNewProjectGroup};

#[tokio::test]
async fn test_vk_oauth_insert_and_get() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Test VK Platform")
                .insert(&pool)
                .await
                .unwrap();

            let group = DbNewProjectGroup::new("Test Group")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&group, "Test Desc", "Test Project")
                .insert(&pool)
                .await
                .unwrap();

            let secure_key = b"test_secure_key".to_vec();
            let service_token = b"test_service_token".to_vec();
            let oauth = DbNewVkOauth::new(
                platform.pkey(),
                project.pkey(),
                12345,
                secure_key.clone(),
                service_token.clone(),
            )
            .insert(&pool)
            .await
            .unwrap();

            assert_eq!(oauth.platform_id, platform.pkey());
            assert_eq!(oauth.project_id, project.pkey());
            assert_eq!(oauth.app_id, 12345);
            assert_eq!(oauth.secure_key, secure_key);
            assert_eq!(oauth.service_token, service_token);

            let fetched = DbVkOauth::get_by_project_id(project.pkey(), &pool)
                .await
                .unwrap();
            assert_eq!(fetched.id, oauth.id);
            assert_eq!(fetched.app_id, 12345);

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_vk_oauth_unique_constraint() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Test VK Platform 2")
                .insert(&pool)
                .await
                .unwrap();

            let group = DbNewProjectGroup::new("Test Group 2")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&group, "Test Desc 2", "Test Project 2")
                .insert(&pool)
                .await
                .unwrap();

            let _oauth1 = DbNewVkOauth::new(
                platform.pkey(),
                project.pkey(),
                11111,
                b"key1".to_vec(),
                b"token1".to_vec(),
            )
            .insert(&pool)
            .await
            .unwrap();

            let result = DbNewVkOauth::new(
                platform.pkey(),
                project.pkey(),
                22222,
                b"key2".to_vec(),
                b"token2".to_vec(),
            )
            .insert(&pool)
            .await;

            assert!(
                result.is_err(),
                "Should fail due to unique constraint on project_id"
            );

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_vk_oauth_state_insert_and_get() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Test VK Platform 3")
                .insert(&pool)
                .await
                .unwrap();

            let group = DbNewProjectGroup::new("Test Group 3")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&group, "Test Desc 3", "Test Project 3")
                .insert(&pool)
                .await
                .unwrap();

            let state_str = "test_state_12345";
            let user_ext_id = "user_123";

            let state = DbNewVkOauthState::new(
                state_str.to_string(),
                user_ext_id.to_string(),
                platform.pkey(),
                project.pkey(),
            )
            .insert(&pool)
            .await
            .unwrap();

            assert_eq!(state.state, state_str);
            assert_eq!(state.user_ext_id, user_ext_id);
            assert_eq!(state.platform_id, platform.pkey());
            assert_eq!(state.project_id, project.pkey());

            let fetched = DbVkOauthState::get_by_state(state_str, &pool)
                .await
                .unwrap();
            assert_eq!(fetched.id, state.id);
            assert_eq!(fetched.user_ext_id, user_ext_id);

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_vk_oauth_state_delete() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Test VK Platform 4")
                .insert(&pool)
                .await
                .unwrap();

            let group = DbNewProjectGroup::new("Test Group 4")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&group, "Test Desc 4", "Test Project 4")
                .insert(&pool)
                .await
                .unwrap();

            let state_str = "test_state_to_delete";
            let _state = DbNewVkOauthState::new(
                state_str.to_string(),
                "user_456".to_string(),
                platform.pkey(),
                project.pkey(),
            )
            .insert(&pool)
            .await
            .unwrap();

            DbVkOauthState::delete_by_state(state_str, &pool)
                .await
                .unwrap();

            let result = DbVkOauthState::get_by_state(state_str, &pool).await;
            assert!(result.is_err(), "State should be deleted");

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_vk_oauth_state_unique_constraint() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Test VK Platform 5")
                .insert(&pool)
                .await
                .unwrap();

            let group = DbNewProjectGroup::new("Test Group 5")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&group, "Test Desc 5", "Test Project 5")
                .insert(&pool)
                .await
                .unwrap();

            let state_str = "duplicate_state";

            let _state1 = DbNewVkOauthState::new(
                state_str.to_string(),
                "user_1".to_string(),
                platform.pkey(),
                project.pkey(),
            )
            .insert(&pool)
            .await
            .unwrap();

            let result = DbNewVkOauthState::new(
                state_str.to_string(),
                "user_2".to_string(),
                platform.pkey(),
                project.pkey(),
            )
            .insert(&pool)
            .await;

            assert!(
                result.is_err(),
                "Should fail due to unique constraint on project_id"
            );

            Ok(())
        },
    )
    .await
}
