use super::*;
use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::{ApiId, DbNewPlatform};
use crate::error::DbError;

#[tokio::test]
async fn test_bot_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();

            let mut bot_account =
                DbNewBotAccount::new(&platform, "RB-890123", b"password".to_vec())
                    .insert(&pool)
                    .await
                    .unwrap();
            let ba_id = bot_account.pkey();

            let ba = DbBotAccount::get_by_id(ba_id, &pool).await.unwrap();
            assert_eq!(bot_account.id, ba.id);
            assert_eq!(bot_account.platform_id, ba.platform_id);
            assert_eq!(bot_account.external_id, ba.external_id);
            assert_eq!(bot_account.token, ba.token);
            bot_account.token = b"AWLiduhw89l!".to_vec();

            bot_account.update(&pool).await.unwrap();
            let ba2 = DbBotAccount::get_by_id(ba_id, &pool).await.unwrap();
            assert_eq!(ba2, bot_account);

            bot_account.delete(&pool).await.unwrap();
            let err = DbBotAccount::get_by_id(ba_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_bot_account_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();

            let bot_account = DbNewBotAccount::new(&platform, "RB-890123", b"password".to_vec())
                .insert(&pool)
                .await
                .unwrap();

            let mut bot = DbNewBot::new(&bot_account, "The Fake Ranger")
                .insert(&pool)
                .await
                .unwrap();
            let b_id = bot.pkey();

            let b = DbBot::get_by_id(b_id, &pool).await.unwrap();
            assert_eq!(b.id, bot.id);
            assert_eq!(b.bot_account_id, bot_account.id);
            assert_eq!(b.designation, bot.designation);
            bot.designation = "The Real Ranger".to_string();

            bot.update(&pool).await.unwrap();
            let b2 = DbBot::get_by_id(b_id, &pool).await.unwrap();
            assert_eq!(b2, bot);

            bot.delete(&pool).await.unwrap();
            let err = DbBot::get_by_id(b_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_full_bot() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();

            let bot_account = DbNewBotAccount::new(&platform, "RB-890123", b"password".to_vec())
                .insert(&pool)
                .await
                .unwrap();

            DbNewBotAccount::new(&platform, "RB-890999", b"password2".to_vec())
                .insert(&pool)
                .await
                .unwrap();

            let bot1 = DbNewBot::new(&bot_account, "The Fake Ranger")
                .insert(&pool)
                .await
                .unwrap();
            let bot2 = DbNewBot::new(&bot_account, "The Fake Mega-man")
                .insert(&pool)
                .await
                .unwrap();
            let bot3 = DbNewBot::new(&bot_account, "The Fake Godzilla")
                .insert(&pool)
                .await
                .unwrap();
            let bot4 = DbNewBot::new(&bot_account, "The Fake King Kong")
                .insert(&pool)
                .await
                .unwrap();

            let bot_account = DbBotAccount::get_by_id(bot_account.id, &pool)
                .await
                .unwrap();
            let full1 = DbFullBotAccount::get_by_id(bot_account.id, &pool)
                .await
                .unwrap();
            let full15 = DbFullBotAccount::get_by_external_id(&bot_account.external_id, &pool)
                .await
                .unwrap();

            assert_eq!(full1.account, bot_account);

            let full2 = bot_account.get_bots(&pool).await.unwrap();
            assert_eq!(full1, full15);
            assert_eq!(full1, full2);

            assert_eq!(full1.bots.len(), 4);
            assert_eq!(full1.bots[0].id, bot1.id);
            assert_eq!(full1.bots[0].designation, bot1.designation);
            assert_eq!(full1.bots[1].id, bot2.id);
            assert_eq!(full1.bots[1].designation, bot2.designation);
            assert_eq!(full1.bots[2].id, bot3.id);
            assert_eq!(full1.bots[2].designation, bot3.designation);
            assert_eq!(full1.bots[3].id, bot4.id);
            assert_eq!(full1.bots[3].designation, bot4.designation);

            Ok(())
        },
    )
    .await
}
