use super::*;
use crate::core_schema::moma::{DbBotAccountProject, DbProjectPlatform, DbProjectUser, MoMa};
use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::{
    ApiId, DbNewPlatform, DbNewProject, DbNewProjectGroup, DbNewTicket, DbNewUser,
};
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

            let ba2 = DbBotAccount::get_by_external_id("RB-890123", &pool)
                .await
                .unwrap();
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

#[tokio::test]
async fn test_get_bots_for_platforms() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Wagongram")
                .insert(&pool)
                .await
                .unwrap();
            let platform2 = DbNewPlatform::new(ApiId::Telegram, "Kv")
                .insert(&pool)
                .await
                .unwrap();
            let platform3 = DbNewPlatform::new(ApiId::Max, "Min")
                .insert(&pool)
                .await
                .unwrap();

            let platforms = [platform, platform2, platform3];

            let mut full_platforms = Vec::new();
            for p in platforms.iter() {
                DbNewBotAccount::new(p, &format!("{}-1", p.name), b"password".to_vec())
                    .insert(&pool)
                    .await
                    .unwrap();

                DbNewBotAccount::new(p, &format!("{}-2", p.name), b"password2".to_vec())
                    .insert(&pool)
                    .await
                    .unwrap();
                full_platforms.push(p.clone().get_mirrors(&pool).await.unwrap());
            }

            let bots =
                crate::core_schema::DbFullBotAccount::get_for_platforms(&full_platforms, &pool)
                    .await
                    .unwrap();

            assert_eq!(bots.len(), 3);

            let bots1 = bots.get(&platforms[0].pkey()).unwrap();
            let bots2 = bots.get(&platforms[1].pkey()).unwrap();
            let bots3 = bots.get(&platforms[2].pkey()).unwrap();

            assert_eq!(bots1.len(), 2);
            assert_eq!(bots2.len(), 2);
            assert_eq!(bots3.len(), 2);

            assert_eq!(bots1[0].account.platform_id, platforms[0].pkey());
            assert_eq!(bots1[1].account.platform_id, platforms[0].pkey());
            assert_eq!(bots2[0].account.platform_id, platforms[1].pkey());
            assert_eq!(bots2[1].account.platform_id, platforms[1].pkey());
            assert_eq!(bots3[0].account.platform_id, platforms[2].pkey());
            assert_eq!(bots3[1].account.platform_id, platforms[2].pkey());

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_get_bots_with_meta() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Wagongram")
                .insert(&pool)
                .await
                .unwrap();
            let platform2 = DbNewPlatform::new(ApiId::Telegram, "Kv")
                .insert(&pool)
                .await
                .unwrap();
            let platform3 = DbNewPlatform::new(ApiId::Max, "Min")
                .insert(&pool)
                .await
                .unwrap();

            let group = DbNewProjectGroup::new("G-1", "Gruppa")
                .insert(&pool)
                .await
                .unwrap();

            let project1 = DbNewProject::new(&group, "PG-1", "The Spam")
                .insert(&pool)
                .await
                .unwrap();
            let project2 = DbNewProject::new(&group, "PG-2", "The Spam2")
                .insert(&pool)
                .await
                .unwrap();
            let _ = DbNewProject::new(&group, "PG-3", "The Spam3")
                .insert(&pool)
                .await
                .unwrap();

            let platforms = [platform, platform2, platform3];

            let mut bot_accounts = Vec::new();
            for p in platforms.iter() {
                let b1 = DbNewBotAccount::new(p, &format!("{}-1", p.name), b"password".to_vec())
                    .insert(&pool)
                    .await
                    .unwrap();

                let b2 = DbNewBotAccount::new(p, &format!("{}-2", p.name), b"password2".to_vec())
                    .insert(&pool)
                    .await
                    .unwrap();

                DbBotAccountProject::link(&b1, &project1, &pool)
                    .await
                    .unwrap();
                DbBotAccountProject::link(&b2, &project2, &pool)
                    .await
                    .unwrap();
                DbProjectPlatform::link(&project1, p, &pool).await.unwrap();
                DbProjectPlatform::link(&project2, p, &pool).await.unwrap();
                bot_accounts.push(b1);
                bot_accounts.push(b2);
            }
            let bot_meta = DbBotAccountWithMeta::get_all(&pool).await.unwrap();

            assert_eq!(bot_meta.len(), 6);

            assert_eq!(bot_meta[0].account, bot_accounts[0]);
            assert_eq!(bot_meta[0].platform.platform, platforms[0]);
            assert_eq!(bot_meta[0].project, project1);
            assert_eq!(bot_meta[1].account, bot_accounts[1]);
            assert_eq!(bot_meta[1].platform.platform, platforms[0]);
            assert_eq!(bot_meta[1].project, project2);
            assert_eq!(bot_meta[2].account, bot_accounts[2]);
            assert_eq!(bot_meta[2].platform.platform, platforms[1]);
            assert_eq!(bot_meta[2].project, project1);
            assert_eq!(bot_meta[3].account, bot_accounts[3]);
            assert_eq!(bot_meta[3].platform.platform, platforms[1]);
            assert_eq!(bot_meta[3].project, project2);
            assert_eq!(bot_meta[4].account, bot_accounts[4]);
            assert_eq!(bot_meta[4].platform.platform, platforms[2]);
            assert_eq!(bot_meta[4].project, project1);
            assert_eq!(bot_meta[5].account, bot_accounts[5]);
            assert_eq!(bot_meta[5].platform.platform, platforms[2]);
            assert_eq!(bot_meta[5].project, project2);

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_get_bots_ticket_expiry() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let user = DbNewUser::new("+79451234567", "The Red Ranger")
                .insert(&pool)
                .await
                .unwrap();

            let platform = DbNewPlatform::new(ApiId::Vk, "Wagongram")
                .insert(&pool)
                .await
                .unwrap();
            let group = DbNewProjectGroup::new("G-1", "Gruppa")
                .insert(&pool)
                .await
                .unwrap();
            let project1 = DbNewProject::new(&group, "PG-1", "The Spam")
                .insert(&pool)
                .await
                .unwrap();

            let mut b1 = DbNewBotAccount::new(&platform, "Super bot", b"password".to_vec())
                .insert(&pool)
                .await
                .unwrap();
            DbBotAccountProject::link(&b1, &project1, &pool)
                .await
                .unwrap();
            DbProjectPlatform::link(&project1, &platform, &pool)
                .await
                .unwrap();
            DbProjectUser::link(&project1, &user, &pool).await.unwrap();

            let started = time::UtcDateTime::now();
            let date = started.date();
            let time = started.time();

            let started = time::PrimitiveDateTime::new(date, time)
                .checked_sub(time::Duration::hours(5))
                .unwrap();
            let latest = time::PrimitiveDateTime::new(date, time)
                .checked_sub(time::Duration::hours(3))
                .unwrap();

            let mut ticket = DbNewTicket::new(&user, &project1, "This.", started)
                .insert(&pool)
                .await
                .unwrap();

            assert!(b1.ticket_not_expired(&ticket));

            b1.expiry_time_hours = Some(4);
            assert!(!b1.ticket_not_expired(&ticket));

            ticket.latest_post_on = Some(latest);
            assert!(b1.ticket_not_expired(&ticket));

            Ok(())
        },
    )
    .await
}
