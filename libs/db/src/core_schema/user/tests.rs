use super::*;
use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::{ApiId, DbNewPlatform};
use crate::error::DbError;

#[tokio::test]
async fn test_user_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let user = DbNewUser::new("+7-549-11-22-345", "The Red Power Ranger");
            let mut user = user.insert(&pool).await.unwrap();

            let u = DbUser::get_by_id(user.id, &pool).await.unwrap();
            assert_eq!(user.id, u.id);
            assert_eq!(user.phone, u.phone);
            assert_eq!(user.designation, u.designation);
            user.phone = "+7-945-11-22-345".to_string();

            user.update(&pool).await.unwrap();
            let u2 = DbUser::get_by_id(user.id, &pool).await.unwrap();
            assert_eq!(u2, user);

            let u2 = DbUser::try_get_by_phone("+7-945-11-22-346", &pool)
                .await
                .unwrap();
            assert!(u2.is_none());

            let u2 = DbUser::try_get_by_phone("+7-945-11-22-345", &pool)
                .await
                .unwrap();
            assert_eq!(u2.unwrap(), user);

            user.delete(&pool).await.unwrap();
            let err = DbUser::get_by_id(user.id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_user_account_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            // NB: Нужен user и Platform чтобы создать учётку.
            let user = DbNewUser::new("+7-549-11-22-345", "The Red Power Ranger");
            let user = user.insert(&pool).await.unwrap();

            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();

            let account = DbNewUserAccount::new(&user, &platform, "RPR-001", "Red");
            let mut account = account.insert(&pool).await.unwrap();

            let ua = DbUserAccount::get_by_id(account.id, &pool).await.unwrap();
            assert_eq!(ua.id, account.id);
            assert_eq!(ua.user_id, user.pkey());
            assert_eq!(ua.platform_id, platform.pkey());
            assert_eq!(ua.external_id, account.external_id);
            assert_eq!(ua.alias, account.alias);
            account.alias = "Red Ranger".to_string();

            account.update(&pool).await.unwrap();
            let ua2 = DbUserAccount::get_by_id(account.id, &pool).await.unwrap();
            assert_eq!(ua2, account);

            let ua2 = DbUserAccount::get_by_external_id("RPR-001", &pool)
                .await
                .unwrap();
            assert_eq!(ua2, account);

            account.delete(&pool).await.unwrap();
            let err = DbUserAccount::get_by_id(account.id, &pool)
                .await
                .unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_user_full_account() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            // NB: Нужен user и Platform чтобы создать учётку.
            let user = DbNewUser::new("+7-549-11-22-345", "The Red Power Ranger");
            let user = user.insert(&pool).await.unwrap();

            let platform1 = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();

            let platform2 = DbNewPlatform::new(ApiId::Vk, "Hamstergram")
                .insert(&pool)
                .await
                .unwrap();

            let platform3 = DbNewPlatform::new(ApiId::Telegram, "XYZ")
                .insert(&pool)
                .await
                .unwrap();

            let account = DbNewUserAccount::new(&user, &platform1, "RPR-001", "Red");
            let account = account.insert(&pool).await.unwrap();

            let account2 = DbNewUserAccount::new(&user, &platform2, "H-002", "Not red at all");
            let account2 = account2.insert(&pool).await.unwrap();

            let account3 = DbNewUserAccount::new(&user, &platform3, "XYZ-002", "Secretly Red");
            let account3 = account3.insert(&pool).await.unwrap();

            let ret_user = DbUser::get_by_id(user.id, &pool).await.unwrap();
            let full1 = DbFullUser::get_by_id(user.id, &pool).await.unwrap();
            let full2 = DbFullUser::get_by_id(user.id, &pool).await.unwrap();

            assert_eq!(full1, full2);
            assert_eq!(full1.user, ret_user);
            assert_eq!(full1.accounts.len(), 3);
            assert_eq!(full1.accounts[0].id, account.id);
            assert_eq!(full1.accounts[0].alias, account.alias);
            assert_eq!(full1.accounts[1].id, account2.id);
            assert_eq!(full1.accounts[1].alias, account2.alias);
            assert_eq!(full1.accounts[2].id, account3.id);
            assert_eq!(full1.accounts[2].alias, account3.alias);
            Ok(())
        },
    )
    .await
}
