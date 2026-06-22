//! Модуль тестирования связанных структур.
use sqlx::PgPool;

use super::*;
use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::*;
use crate::test_frame::run_test_postgres;

const MIG: &str = "tests/sql/postgres/";
const FIX: &str = "../../sql/core/";
const CLEAN: &str = "tests/sql/postgres/drop_core";

#[tokio::test]
async fn test_ticket_chat() {
    run_test_postgres::<TestCfg, _>(MIG, FIX, CLEAN, |pool| async move {
        let mut s = full_setup(&pool).await;
        // Add tickets.
        setup_tickets_annex(&mut s, &pool).await;
        setup_chats_annex(&mut s, &pool).await;

        let tickets = DbTicketChat::get_for_chat(s.chats[0].pkey(), &pool)
            .await
            .unwrap();
        let chats = DbTicketChat::get_for_ticket(s.tickets[0].pkey(), &pool)
            .await
            .unwrap();
        assert!(chats.is_empty());
        assert!(tickets.is_empty());

        DbTicketChat::link(&s.tickets[0], &s.chats[0], &pool)
            .await
            .unwrap();
        DbTicketChat::link(&s.tickets[1], &s.chats[1], &pool)
            .await
            .unwrap();
        DbTicketChat::link(&s.tickets[1], &s.chats[2], &pool)
            .await
            .unwrap();
        DbTicketChat::link(&s.tickets[2], &s.chats[3], &pool)
            .await
            .unwrap();

        let tickets = DbTicketChat::get_for_chat(s.chats[0].pkey(), &pool)
            .await
            .unwrap();
        let chats = DbTicketChat::get_for_ticket(s.tickets[0].pkey(), &pool)
            .await
            .unwrap();

        let exists = DbTicketChat::exists(&s.tickets[0], &s.chats[0], &pool)
            .await
            .unwrap();

        assert!(exists);
        assert_eq!(chats.len(), 1);
        assert_eq!(tickets.len(), 1);
        assert_eq!(chats[0], s.chats[0]);
        assert_eq!(tickets[0], s.tickets[0]);

        DbTicketChat::un_link(&s.tickets[0], &s.chats[0], &pool)
            .await
            .unwrap();

        let tickets = DbTicketChat::get_for_chat(s.chats[0].pkey(), &pool)
            .await
            .unwrap();
        let chats = DbTicketChat::get_for_ticket(s.tickets[0].pkey(), &pool)
            .await
            .unwrap();
        assert!(chats.is_empty());
        assert!(tickets.is_empty());

        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_bot_account_project() {
    run_test_postgres::<TestCfg, _>(MIG, FIX, CLEAN, |pool| async move {
        let s = full_setup(&pool).await;

        let project = DbBotAccountProject::get_for_account(s.bots[0].pkey(), &pool)
            .await
            .unwrap();
        let account =
            DbBotAccountProject::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();
        assert!(account.is_empty());
        assert!(project.is_empty());

        DbBotAccountProject::link(&s.bots[0], &s.project_group.projects[0], &pool)
            .await
            .unwrap();
        DbBotAccountProject::link(&s.bots[1], &s.project_group.projects[1], &pool)
            .await
            .unwrap();
        DbBotAccountProject::link(&s.bots[1], &s.project_group.projects[2], &pool)
            .await
            .unwrap();
        DbBotAccountProject::link(&s.bots[2], &s.project_group.projects[3], &pool)
            .await
            .unwrap();

        let project = DbBotAccountProject::get_for_account(s.bots[0].pkey(), &pool)
            .await
            .unwrap();
        let account =
            DbBotAccountProject::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();

        let exists = DbBotAccountProject::exists(&s.bots[0], &s.project_group.projects[0], &pool)
            .await
            .unwrap();

        assert!(exists);
        assert_eq!(account.len(), 1);
        assert_eq!(project.len(), 1);
        assert_eq!(account[0], s.bots[0]);
        assert_eq!(project[0], s.project_group.projects[0]);

        DbBotAccountProject::un_link(&s.bots[0], &s.project_group.projects[0], &pool)
            .await
            .unwrap();

        let project = DbBotAccountProject::get_for_account(s.bots[0].pkey(), &pool)
            .await
            .unwrap();
        let account =
            DbBotAccountProject::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();
        assert!(account.is_empty());
        assert!(project.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_user_account_project() {
    run_test_postgres::<TestCfg, _>(MIG, FIX, CLEAN, |pool| async move {
        let s = full_setup(&pool).await;

        let project = DbUserAccountProject::get_for_account(s.user_accounts[0].pkey(), &pool)
            .await
            .unwrap();
        let account =
            DbUserAccountProject::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();
        assert!(account.is_empty());
        assert!(project.is_empty());

        DbUserAccountProject::link(&s.user_accounts[0], &s.project_group.projects[0], &pool)
            .await
            .unwrap();
        DbUserAccountProject::link(&s.user_accounts[1], &s.project_group.projects[1], &pool)
            .await
            .unwrap();
        DbUserAccountProject::link(&s.user_accounts[1], &s.project_group.projects[2], &pool)
            .await
            .unwrap();
        DbUserAccountProject::link(&s.user_accounts[2], &s.project_group.projects[3], &pool)
            .await
            .unwrap();

        let project = DbUserAccountProject::get_for_account(s.user_accounts[0].pkey(), &pool)
            .await
            .unwrap();
        let account =
            DbUserAccountProject::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();

        let exists =
            DbUserAccountProject::exists(&s.user_accounts[0], &s.project_group.projects[0], &pool)
                .await
                .unwrap();

        assert!(exists);
        assert_eq!(account.len(), 1);
        assert_eq!(project.len(), 1);
        assert_eq!(account[0], s.user_accounts[0]);
        assert_eq!(project[0], s.project_group.projects[0]);

        DbUserAccountProject::un_link(&s.user_accounts[0], &s.project_group.projects[0], &pool)
            .await
            .unwrap();

        let project = DbUserAccountProject::get_for_account(s.user_accounts[0].pkey(), &pool)
            .await
            .unwrap();
        let account =
            DbUserAccountProject::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();
        assert!(account.is_empty());
        assert!(project.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_project_user() {
    run_test_postgres::<TestCfg, _>(MIG, FIX, CLEAN, |pool| async move {
        let mut s = full_setup(&pool).await;
        setup_chats_annex(&mut s, &pool).await;

        let project = DbProjectUser::get_for_user(s.users[0].pkey(), &pool)
            .await
            .unwrap();
        let account = DbProjectUser::get_for_project(s.project_group.projects[0].pkey(), &pool)
            .await
            .unwrap();
        assert!(account.is_empty());
        assert!(project.is_empty());

        DbProjectUser::link(&s.project_group.projects[0], &s.users[0], &pool)
            .await
            .unwrap();
        DbProjectUser::link(&s.project_group.projects[1], &s.users[1], &pool)
            .await
            .unwrap();
        DbProjectUser::link(&s.project_group.projects[2], &s.users[2], &pool)
            .await
            .unwrap();
        DbProjectUser::link(&s.project_group.projects[3], &s.users[3], &pool)
            .await
            .unwrap();

        let project = DbProjectUser::get_for_user(s.users[0].pkey(), &pool)
            .await
            .unwrap();
        let account = DbProjectUser::get_for_project(s.project_group.projects[0].pkey(), &pool)
            .await
            .unwrap();

        let exists = DbProjectUser::exists(&s.project_group.projects[0], &s.users[0], &pool)
            .await
            .unwrap();

        assert!(exists);
        assert_eq!(account.len(), 1);
        assert_eq!(project.len(), 1);
        assert_eq!(account[0], s.users[0]);
        assert_eq!(project[0], s.project_group.projects[0]);

        DbProjectUser::un_link(&s.project_group.projects[0], &s.users[0], &pool)
            .await
            .unwrap();

        let project = DbProjectUser::get_for_user(s.users[0].pkey(), &pool)
            .await
            .unwrap();
        let account = DbProjectUser::get_for_project(s.project_group.projects[0].pkey(), &pool)
            .await
            .unwrap();
        assert!(account.is_empty());
        assert!(project.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_project_platform() {
    run_test_postgres::<TestCfg, _>(MIG, FIX, CLEAN, |pool| async move {
        let mut s = full_setup(&pool).await;
        setup_chats_annex(&mut s, &pool).await;

        let project = DbProjectPlatform::get_for_platform(s.platforms[0].pkey(), &pool)
            .await
            .unwrap();
        let platform =
            DbProjectPlatform::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();
        assert!(platform.is_empty());
        assert!(project.is_empty());

        DbProjectPlatform::link(&s.project_group.projects[0], &s.platforms[0], &pool)
            .await
            .unwrap();
        DbProjectPlatform::link(&s.project_group.projects[1], &s.platforms[1], &pool)
            .await
            .unwrap();
        DbProjectPlatform::link(&s.project_group.projects[2], &s.platforms[2], &pool)
            .await
            .unwrap();
        DbProjectPlatform::link(&s.project_group.projects[3], &s.platforms[3], &pool)
            .await
            .unwrap();

        let project = DbProjectPlatform::get_for_platform(s.platforms[0].pkey(), &pool)
            .await
            .unwrap();
        let platform =
            DbProjectPlatform::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();

        let exists =
            DbProjectPlatform::exists(&s.project_group.projects[0], &s.platforms[0], &pool)
                .await
                .unwrap();
        assert!(exists);
        assert_eq!(platform.len(), 1);
        assert_eq!(project.len(), 1);
        assert_eq!(platform[0], s.platforms[0]);
        assert_eq!(project[0], s.project_group.projects[0]);

        DbProjectPlatform::un_link(&s.project_group.projects[0], &s.platforms[0], &pool)
            .await
            .unwrap();

        let project = DbProjectPlatform::get_for_platform(s.platforms[0].pkey(), &pool)
            .await
            .unwrap();
        let platform =
            DbProjectPlatform::get_for_project(s.project_group.projects[0].pkey(), &pool)
                .await
                .unwrap();
        assert!(platform.is_empty());
        assert!(project.is_empty());
        Ok(())
    })
    .await
}

/// Сущность для упрощённой сборки изначальных данных.
#[derive(Debug)]
struct Setup {
    project_group: DbFullProjectGroup,
    platforms: Vec<DbPlatform>,
    users: Vec<DbUser>,
    user_accounts: Vec<DbUserAccount>,
    bots: Vec<DbBotAccount>,
    chats: Vec<DbChat>,
    tickets: Vec<DbTicket>,
}

async fn setup_tickets_annex(s: &mut Setup, pool: &PgPool) {
    DbProjectUser::link(&s.project_group.projects[0], &s.users[0], pool)
        .await
        .unwrap();
    DbProjectUser::link(&s.project_group.projects[0], &s.users[1], pool)
        .await
        .unwrap();
    DbProjectUser::link(&s.project_group.projects[1], &s.users[2], pool)
        .await
        .unwrap();

    let started = time::macros::datetime!(2024-01-01 00:02);
    let topic1 = "Сломалась сенокосилка и унитаз";
    let topic2 = "Помогите найти сенокосилку которая не пугает котиков.";
    let topic3 = "Отвалилось колесо, надо его заменить.";

    let ticket1 = DbNewTicket::new(&s.users[0], &s.project_group.projects[0], topic1, started);
    let ticket2 = DbNewTicket::new(&s.users[1], &s.project_group.projects[0], topic2, started);
    let ticket3 = DbNewTicket::new(&s.users[2], &s.project_group.projects[1], topic3, started);

    let ticket1 = ticket1.insert(pool).await.unwrap();
    let ticket2 = ticket2.insert(pool).await.unwrap();
    let ticket3 = ticket3.insert(pool).await.unwrap();

    s.tickets = vec![ticket1, ticket2, ticket3];
}

async fn setup_chats_annex(s: &mut Setup, pool: &PgPool) {
    let project1 = &s.project_group.projects[0];
    let project2 = &s.project_group.projects[1];
    let project3 = &s.project_group.projects[2];
    let project4 = &s.project_group.projects[3];

    let platform1 = &s.platforms[0];
    let platform2 = &s.platforms[1];
    let platform3 = &s.platforms[2];

    let started = time::macros::datetime!(2024-01-01 00:02);
    let chat_id = "XYZ-1000";
    let chat1 = DbNewChat::new(
        chat_id,
        &s.user_accounts[0],
        &s.bots[0],
        project1,
        platform1,
        started,
    );
    moma::DbUserAccountProject::link(&s.user_accounts[0], project1, pool)
        .await
        .unwrap();
    moma::DbBotAccountProject::link(&s.bots[0], project1, pool)
        .await
        .unwrap();

    let chat_id = "XYZ-1001";
    let chat2 = DbNewChat::new(
        chat_id,
        &s.user_accounts[2],
        &s.bots[0],
        project1,
        platform1,
        started,
    );
    moma::DbUserAccountProject::link(&s.user_accounts[2], project1, pool)
        .await
        .unwrap();

    let chat_id = "XYZ-1002";
    let chat3 = DbNewChat::new(
        chat_id,
        &s.user_accounts[3],
        &s.bots[2],
        project2,
        platform2,
        started,
    );
    moma::DbUserAccountProject::link(&s.user_accounts[3], project2, pool)
        .await
        .unwrap();
    moma::DbBotAccountProject::link(&s.bots[2], project2, pool)
        .await
        .unwrap();

    let chat_id = "XYZ-1003";
    let chat4 = DbNewChat::new(
        chat_id,
        &s.user_accounts[6],
        &s.bots[4],
        project2,
        platform3,
        started,
    );
    moma::DbUserAccountProject::link(&s.user_accounts[6], project2, pool)
        .await
        .unwrap();
    moma::DbBotAccountProject::link(&s.bots[4], project2, pool)
        .await
        .unwrap();

    let chat_id = "XYZ-1004";
    let chat5 = DbNewChat::new(
        chat_id,
        &s.user_accounts[9],
        &s.bots[1],
        project3,
        platform1,
        started,
    );
    moma::DbUserAccountProject::link(&s.user_accounts[9], project3, pool)
        .await
        .unwrap();
    moma::DbBotAccountProject::link(&s.bots[1], project3, pool)
        .await
        .unwrap();

    let chat_id = "XYZ-1005";
    let chat6 = DbNewChat::new(
        chat_id,
        &s.user_accounts[9],
        &s.bots[1],
        project4,
        platform1,
        started,
    );
    moma::DbUserAccountProject::link(&s.user_accounts[9], project4, pool)
        .await
        .unwrap();
    moma::DbBotAccountProject::link(&s.bots[1], project4, pool)
        .await
        .unwrap();

    let chat1 = chat1.insert(pool).await.unwrap();
    let chat2 = chat2.insert(pool).await.unwrap();
    let chat3 = chat3.insert(pool).await.unwrap();
    let chat4 = chat4.insert(pool).await.unwrap();
    let chat5 = chat5.insert(pool).await.unwrap();
    let chat6 = chat6.insert(pool).await.unwrap();
    s.chats = vec![chat1, chat2, chat3, chat4, chat5, chat6];
}

/// Единое создание схемы для тестирования
async fn full_setup(pool: &PgPool) -> Setup {
    let project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
        .insert(pool)
        .await
        .unwrap();
    let pg_id = project_group.pkey();

    let pg = DbProjectGroup::get_by_id(pg_id, pool).await.unwrap();

    DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
        .insert(pool)
        .await
        .unwrap();

    DbNewProject::new(&project_group, "AKUWDHWA-8692", "The Biggest Spam")
        .insert(pool)
        .await
        .unwrap();

    DbNewProject::new(&project_group, "AKUWDHWA-8712", "The AI Spam")
        .insert(pool)
        .await
        .unwrap();

    DbNewProject::new(&project_group, "AKUWDHWA-8735", "Rise of the Machine God")
        .insert(pool)
        .await
        .unwrap();

    let full_project_group = pg.get_projects(pool).await.unwrap();

    let platform1 = DbNewPlatform::new(ApiId::Vk, "Rangergram")
        .insert(pool)
        .await
        .unwrap();
    let platform2 = DbNewPlatform::new(ApiId::Telegram, "X")
        .insert(pool)
        .await
        .unwrap();
    let platform3 = DbNewPlatform::new(ApiId::Telegram, "Y")
        .insert(pool)
        .await
        .unwrap();
    let platform4 = DbNewPlatform::new(ApiId::Max, "Spandex Chat")
        .insert(pool)
        .await
        .unwrap();
    let platform5 = DbNewPlatform::new(ApiId::Max, "Mandex Chat")
        .insert(pool)
        .await
        .unwrap();

    let user1 = DbNewUser::new("+79451001122", "The Red Ranger");
    let user2 = DbNewUser::new("+79452001133", "The White Ranger");
    let user3 = DbNewUser::new("+79453001144", "The Blue Ranger");
    let user4 = DbNewUser::new("+79454001155", "The Green Ranger");
    let user5 = DbNewUser::new("+79455001166", "The Black Ranger");

    let user1 = user1.insert(pool).await.unwrap();
    let user2 = user2.insert(pool).await.unwrap();
    let user3 = user3.insert(pool).await.unwrap();
    let user4 = user4.insert(pool).await.unwrap();
    let user5 = user5.insert(pool).await.unwrap();

    let user1_account1 = DbNewUserAccount::new(&user1, &platform1, "PWRR-001", "Red");
    let user1_account2 = DbNewUserAccount::new(&user1, &platform2, "X-001", "Red");
    let user2_account1 = DbNewUserAccount::new(&user2, &platform1, "PWRR-002", "White");
    let user2_account2 = DbNewUserAccount::new(&user2, &platform2, "X-002", "White");
    let user3_account1 = DbNewUserAccount::new(&user3, &platform1, "PWRR-003", "Blue");
    let user3_account2 = DbNewUserAccount::new(&user3, &platform2, "X-003", "Blue");
    let user3_account3 = DbNewUserAccount::new(&user3, &platform3, "Y-001", "Blue");
    let user3_account4 = DbNewUserAccount::new(&user3, &platform4, "Sp-001", "Blue++");
    let user4_account1 = DbNewUserAccount::new(&user4, &platform1, "PWRR-004", "Grren");
    let user5_account1 = DbNewUserAccount::new(&user5, &platform1, "PWRR-005", "Black");

    let user1_account1 = user1_account1.insert(pool).await.unwrap();
    let user1_account2 = user1_account2.insert(pool).await.unwrap();
    let user2_account1 = user2_account1.insert(pool).await.unwrap();
    let user2_account2 = user2_account2.insert(pool).await.unwrap();
    let user3_account1 = user3_account1.insert(pool).await.unwrap();
    let user3_account2 = user3_account2.insert(pool).await.unwrap();
    let user3_account3 = user3_account3.insert(pool).await.unwrap();
    let user3_account4 = user3_account4.insert(pool).await.unwrap();
    let user4_account1 = user4_account1.insert(pool).await.unwrap();
    let user5_account1 = user5_account1.insert(pool).await.unwrap();

    let bot_account1 = DbNewBotAccount::new(&platform1, "RB1-001", b"password".to_vec());
    let bot_account2 = DbNewBotAccount::new(&platform1, "RB1-002", b"password".to_vec());
    let bot_account3 = DbNewBotAccount::new(&platform2, "RB2-001", b"password".to_vec());
    let bot_account4 = DbNewBotAccount::new(&platform2, "RB2-002", b"password".to_vec());
    let bot_account5 = DbNewBotAccount::new(&platform3, "RB3-001", b"password".to_vec());
    let bot_account6 = DbNewBotAccount::new(&platform3, "RB3-002", b"password".to_vec());
    let bot_account7 = DbNewBotAccount::new(&platform4, "RB4-001", b"password".to_vec());
    let bot_account8 = DbNewBotAccount::new(&platform4, "RB4-002", b"password".to_vec());
    let bot_account9 = DbNewBotAccount::new(&platform5, "RB5-001", b"password".to_vec());

    let bot_account1 = bot_account1.insert(pool).await.unwrap();
    let bot_account2 = bot_account2.insert(pool).await.unwrap();
    let bot_account3 = bot_account3.insert(pool).await.unwrap();
    let bot_account4 = bot_account4.insert(pool).await.unwrap();
    let bot_account5 = bot_account5.insert(pool).await.unwrap();
    let bot_account6 = bot_account6.insert(pool).await.unwrap();
    let bot_account7 = bot_account7.insert(pool).await.unwrap();
    let bot_account8 = bot_account8.insert(pool).await.unwrap();
    let bot_account9 = bot_account9.insert(pool).await.unwrap();

    let bots = vec![
        bot_account1,
        bot_account2,
        bot_account3,
        bot_account4,
        bot_account5,
        bot_account6,
        bot_account7,
        bot_account8,
        bot_account9,
    ];
    let user_accounts = vec![
        user1_account1,
        user1_account2,
        user2_account1,
        user2_account2,
        user3_account1,
        user3_account2,
        user3_account3,
        user3_account4,
        user4_account1,
        user5_account1,
    ];
    let platforms = vec![
        DbPlatform::get_by_id(platform1.pkey(), pool).await.unwrap(),
        DbPlatform::get_by_id(platform2.pkey(), pool).await.unwrap(),
        DbPlatform::get_by_id(platform3.pkey(), pool).await.unwrap(),
        DbPlatform::get_by_id(platform4.pkey(), pool).await.unwrap(),
        DbPlatform::get_by_id(platform5.pkey(), pool).await.unwrap(),
    ];

    Setup {
        project_group: full_project_group,
        platforms,
        users: vec![user1, user2, user3, user4, user5],
        user_accounts,
        bots,
        chats: vec![],
        tickets: vec![],
    }
}
