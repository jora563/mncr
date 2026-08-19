use super::message::*;
use super::*;
use crate::core_schema::moma::{self, MoMa};
use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::*;
use crate::error::DbError;

#[tokio::test]
async fn test_validate_chat() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
            "tests/sql/postgres/",
            "../../sql/core/",
            "tests/sql/postgres/drop_core",
            |pool| async move {
                let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                    .insert(&pool)
                    .await
                    .unwrap();
                let platform2 = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                    .insert(&pool)
                    .await
                    .unwrap();
                let user = DbNewUser::new("+79451234567", "The Red Ranger")
                    .insert(&pool)
                    .await
                    .unwrap();

                let project_group = DbNewProjectGroup::new("Telecorp")
                    .insert(&pool)
                    .await
                    .unwrap();

                let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                    .insert(&pool)
                    .await
                    .unwrap();

                let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
                let bot_account =
                    DbNewBotAccount::new(&platform, None, "RB-890123",None  , b"password".to_vec());

                let user_account = user_account.insert(&pool).await.unwrap();
                let mut bot_account = bot_account.insert(&pool).await.unwrap();

                let project2 =
                    DbNewProject::new(&project_group, "AKUWDHWA-8692", "The Biggest Spam")
                        .insert(&pool)
                        .await
                        .unwrap();

                moma::DbUserAccountProject::link(&user_account, &project, &pool)
                    .await
                    .unwrap();

                let started = time::macros::datetime!(2024-01-01 00:02);
                let chat_id = "XYZ-1000";
                // Валидация пользователь проект
                let chat_err =
                    DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform, started)
                        .insert(&pool)
                        .await
                        .unwrap_err();
                let chat_err2 =
                    DbNewChat::new(chat_id, &user_account, &bot_account, &project2, &platform, started)
                        .insert(&pool)
                        .await
                        .unwrap_err();
                let chat_err3 =
                    DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform2, started)
                        .insert(&pool)
                        .await
                        .unwrap_err();
                assert_eq!(
                    &chat_err.to_string(),
                    "Cannot validate Messenger Chat: Bot account RB-890123 not part of project The Big Spam."
                );
                assert_eq!(
                    &chat_err2.to_string(),
                    "Cannot validate Messenger Chat: User account PWRR-001 not part of project The Biggest Spam."
                );
                assert_eq!(
                    &chat_err3.to_string(),
                    "User account (PWRR-001) and chat platform incompatible (1 vs 2)."
                );

                bot_account.project_id = Some(project.pkey());
                bot_account.update(&pool).await.unwrap();

                // Валидация чат проект/чат платформа
                let chat_err4 =
                    DbNewChat::new(chat_id, &user_account, &bot_account, &project2, &platform, started)
                        .insert(&pool)
                        .await
                        .unwrap_err();
                let chat_err5 =
                    DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform2, started)
                        .insert(&pool)
                        .await
                        .unwrap_err();
                assert_eq!(
                    &chat_err4.to_string(),
                    "Cannot validate Messenger Chat: User account PWRR-001 not part of project The Biggest Spam."
                );
                assert_eq!(
                    &chat_err5.to_string(),
                    "User account (PWRR-001) and chat platform incompatible (1 vs 2)."
                );

                DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform, started)
                    .insert(&pool)
                    .await
                    .unwrap();
                Ok(())
            },
        )
        .await
}

#[tokio::test]
async fn test_chat_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();
            let user = DbNewUser::new("+79451234567", "The Red Ranger")
                .insert(&pool)
                .await
                .unwrap();

            let project_group = DbNewProjectGroup::new("Telecorp")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();

            let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
            let bot_account = DbNewBotAccount::new(
                &platform,
                Some(&project),
                "RB-890123",
                None,
                b"password".to_vec(),
            );

            let user_account = user_account.insert(&pool).await.unwrap();
            let bot_account = bot_account.insert(&pool).await.unwrap();

            moma::DbUserAccountProject::link(&user_account, &project, &pool)
                .await
                .unwrap();

            let started = time::macros::datetime!(2024-01-01 00:02);
            let chat_id = "XYZ-1000";
            let mut chat = DbNewChat::new(
                chat_id,
                &user_account,
                &bot_account,
                &project,
                &platform,
                started,
            )
            .insert(&pool)
            .await
            .unwrap();
            let ch_id = chat.pkey();

            let ch = DbChat::get_by_id(ch_id, &pool).await.unwrap();

            assert_eq!(ch.id, chat.id);
            assert_eq!(ch.user_account_id, chat.user_account_id);
            assert_eq!(ch.bot_account_id, chat.bot_account_id);
            assert_eq!(ch.project_id, chat.project_id);
            assert_eq!(ch.platform_id, chat.platform_id);
            assert_eq!(ch.latest_post_on, chat.latest_post_on);
            assert_eq!(ch.closed_on, chat.closed_on);
            assert_eq!(ch.started_on, chat.started_on);

            chat.latest_post_on = Some(time::macros::datetime!(2024-01-01 00:03));
            chat.update(&pool).await.unwrap();

            let ch2 = DbChat::get_by_id(ch_id, &pool).await.unwrap();
            assert_eq!(chat, ch2);

            chat.delete(&pool).await.unwrap();
            let err = DbChat::get_by_id(ch_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_full_chat() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();
            let user = DbNewUser::new("+79451234567", "The Red Ranger")
                .insert(&pool)
                .await
                .unwrap();

            let project_group = DbNewProjectGroup::new("Telecorp")
                .insert(&pool)
                .await
                .unwrap();

            let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();

            let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
            let bot_account = DbNewBotAccount::new(
                &platform,
                Some(&project),
                "RB-890123",
                None,
                b"password".to_vec(),
            );

            let user_account = user_account.insert(&pool).await.unwrap();
            let bot_account = bot_account.insert(&pool).await.unwrap();

            moma::DbUserAccountProject::link(&user_account, &project, &pool)
                .await
                .unwrap();

            let started = time::macros::datetime!(2024-01-01 00:02);
            let chat_id = "XYZ-1000";
            let mut chat = DbNewChat::new(
                chat_id,
                &user_account,
                &bot_account,
                &project,
                &platform,
                started,
            )
            .insert(&pool)
            .await
            .unwrap();

            let ch1 = DbChat::get_by_id(chat.pkey(), &pool).await.unwrap();
            let ch2 = DbChat::get_by_external_id(&chat.external_id, &pool)
                .await
                .unwrap();
            assert_eq!(ch1, chat);
            assert_eq!(ch2, chat);

            moma::DbProjectUser::link(&project, &user, &pool)
                .await
                .unwrap();
            let started = time::macros::datetime!(2024-01-01 00:02);
            let topic = "Сломалась сенокосилка и унитаз";
            let ticket = DbNewTicket::new(&user, &project, topic, started);
            let mut ticket = ticket.insert(&pool).await.unwrap();

            moma::DbTicketChat::link(&ticket, &chat, &pool)
                .await
                .unwrap();

            let content = "Help me, help me!";
            let msg1 = DbNewMessage::new_user(
                &user_account,
                1,
                "PWRR-001/M-0970",
                &mut chat,
                &mut ticket,
                content,
            )
            .unwrap()
            .insert(&pool)
            .await
            .unwrap();

            let att1 = DbNewAttachment::new(
                &msg1,
                1,
                "PWRR-001/F-0970",
                "http://rangergram.com/files/f-0970",
                9979,
            )
            .insert(&pool)
            .await
            .unwrap();

            let content = "My lawnmower broke.";
            let msg2 = DbNewMessage::new_user(
                &user_account,
                1,
                "PWRR-001/M-0970",
                &mut chat,
                &mut ticket,
                content,
            )
            .unwrap()
            .insert(&pool)
            .await
            .unwrap();

            let att2 = DbNewAttachment::new(
                &msg2,
                1,
                "PWRR-001/F-0971",
                "http://rangergram.com/files/f-0970",
                9979,
            )
            .insert(&pool)
            .await
            .unwrap();

            let content = "It's the New Year and my lawnmower is broken!";
            let msg3 = DbNewMessage::new_user(
                &user_account,
                1,
                "PWRR-001/M-0970",
                &mut chat,
                &mut ticket,
                content,
            )
            .unwrap()
            .insert(&pool)
            .await
            .unwrap();

            let att3 = DbNewAttachment::new(
                &msg3,
                1,
                "PWRR-001/F-0972",
                "http://rangergram.com/files/f-0970",
                9979,
            )
            .insert(&pool)
            .await
            .unwrap();

            let full = chat.clone().get_msgs(&pool).await.unwrap();

            assert_eq!(full.chat.id, chat.id);
            assert_eq!(full.chat.user_account_id, chat.user_account_id);
            assert_eq!(full.chat.bot_account_id, chat.bot_account_id);
            assert_eq!(full.chat.project_id, chat.project_id);
            assert_eq!(full.chat.platform_id, chat.platform_id);
            assert_eq!(full.chat.latest_post_on, chat.latest_post_on);
            assert_eq!(full.chat.closed_on, chat.closed_on);
            assert_eq!(full.chat.started_on, chat.started_on);

            assert_eq!(full.messages.len(), 3);

            assert_eq!(full.messages[0].message.content, msg1.content);
            assert_eq!(full.messages[0].message.pkey(), msg1.pkey());
            assert_eq!(full.messages[1].message.content, msg2.content);
            assert_eq!(full.messages[1].message.pkey(), msg2.pkey());
            assert_eq!(full.messages[2].message.content, msg3.content);
            assert_eq!(full.messages[2].message.pkey(), msg3.pkey());

            assert_eq!(full.messages[0].files[0].external_id, att1.external_id);
            assert_eq!(full.messages[0].files[0].pkey(), att1.pkey());
            assert_eq!(full.messages[1].files[0].external_id, att2.external_id);
            assert_eq!(full.messages[1].files[0].pkey(), att2.pkey());
            assert_eq!(full.messages[2].files[0].external_id, att3.external_id);
            assert_eq!(full.messages[2].files[0].pkey(), att3.pkey());

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_message_validate() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
            "tests/sql/postgres/",
            "../../sql/core/",
            "tests/sql/postgres/drop_core",
            |pool| async move {
                let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                    .insert(&pool)
                    .await
                    .unwrap();
                let platform2 = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                    .insert(&pool)
                    .await
                    .unwrap();
                let user = DbNewUser::new("+79451234567", "The Red Ranger")
                    .insert(&pool)
                    .await
                    .unwrap();

                let project_group = DbNewProjectGroup::new("Telecorp")
                    .insert(&pool)
                    .await
                    .unwrap();

                let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                    .insert(&pool)
                    .await
                    .unwrap();

                let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
                let user_account2 = DbNewUserAccount::new(&user, &platform, "PWRR-002", "Red2");
                let user_account3 = DbNewUserAccount::new(&user, &platform2, "PWRR-003", "Red3");
                let bot_account =
                    DbNewBotAccount::new(&platform, Some(&project), "RB-890123",None , b"password".to_vec());
                let bot_account2 =
                    DbNewBotAccount::new(&platform, None, "RB-890124",None , b"password".to_vec());
                let bot_account3 =
                    DbNewBotAccount::new(&platform2, None, "RB-890125",None , b"password".to_vec());

                let user_account = user_account.insert(&pool).await.unwrap();
                let user_account2 = user_account2.insert(&pool).await.unwrap();
                let user_account3 = user_account3.insert(&pool).await.unwrap();
                let mut bot_account = bot_account.insert(&pool).await.unwrap();
                let bot_account2 = bot_account2.insert(&pool).await.unwrap();
                let bot_account3 = bot_account3.insert(&pool).await.unwrap();

                let project2 = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                    .insert(&pool)
                    .await
                    .unwrap();

                moma::DbProjectUser::link(&project, &user, &pool)
                    .await
                    .unwrap();
                let started = time::macros::datetime!(2024-01-01 00:02);
                let topic = "Сломалась сенокосилка и унитаз";
                let ticket = DbNewTicket::new(&user, &project, topic, started);
                let ticket2 = DbNewTicket::new(&user, &project2, topic, started);
                let mut ticket = ticket.insert(&pool).await.unwrap();
                let ticket_err = ticket2.insert(&pool).await.unwrap_err();

                // Мы не можем создать плохой тикет! :)
                assert_eq!(
                    ticket_err.to_string(),
                    "Cannot validate Ticket: User The Red Ranger not part of project The Big Spam."
                );


                moma::DbUserAccountProject::link(&user_account, &project, &pool)
                    .await
                    .unwrap();

                let started = time::macros::datetime!(2024-01-01 00:02);
                let chat_id = "XYZ-1000";
                let mut chat =
                    DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform, started)
                        .insert(&pool)
                        .await
                        .unwrap();

                moma::DbTicketChat::link(&ticket, &chat, &pool)
                    .await
                    .unwrap();

                let content = "Help me, help me! My lawnmower broke!";
                let err = DbNewMessage::new_user(
                    &user_account2,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap_err();
                let err2 = DbNewMessage::new_bot(
                    &bot_account2,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap_err();
                let err3 = DbNewMessage::new_user(
                    &user_account3,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap_err();
                let err4 = DbNewMessage::new_bot(
                    &bot_account3,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap_err();

                assert_eq!(
                    err.to_string(),
                    "User Red2 does not belong to chat with id 1"
                );
                assert_eq!(
                    err2.to_string(),
                    "Bot RB-890124 does not belong to chat with id 1"
                );
                assert_eq!(
                    err3.to_string(),
                    "User account (Red3) and chat platform incompatible (2 vs 1)."
                );
                assert_eq!(
                    err4.to_string(),
                    "Bot account (RB-890125) and chat platform incompatible (2 vs 1)."
                );

                DbNewMessage::new_user(
                    &user_account,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap()
                .insert(&pool)
                .await
                .unwrap();

                DbNewMessage::new_bot(&bot_account, 1, "PWRR-001/M-0970", &mut chat, &mut ticket, content)
                    .unwrap()
                    .insert(&pool)
                    .await
                    .unwrap();

                // Unlink and hence test project moma.
                moma::DbUserAccountProject::un_link(&user_account, &project, &pool)
                    .await
                    .unwrap();
                bot_account.project_id = None;
                bot_account.update(&pool).await.unwrap();

                let err5 = DbNewMessage::new_user(
                    &user_account,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap()
                .insert(&pool)
                .await
                .unwrap_err();

                let err6 = DbNewMessage::new_bot(
                    &bot_account,
                    1,
                    "PWRR-001/M-0970",
                    &mut chat,
                    &mut ticket,
                    content,
                )
                .unwrap()
                .insert(&pool)
                .await
                .unwrap_err();

                assert_eq!(
                    err5.to_string(),
                    "Cannot validate Chat Message: User account PWRR-001 not part of project The Big Spam."
                );
                assert_eq!(
                    err6.to_string(),
                    "Cannot validate Chat Message: Bot account RB-890123 not part of project The Big Spam."
                );

                Ok(())
            },
        )
        .await
}

#[tokio::test]
async fn test_message_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
            "tests/sql/postgres/",
            "../../sql/core/",
            "tests/sql/postgres/drop_core",
            |pool| async move {
                let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram").insert(&pool).await.unwrap();
                let user = DbNewUser::new("+79451234567", "The Red Ranger").insert(&pool).await.unwrap();

                let project_group = DbNewProjectGroup::new("Telecorp")
                    .insert(&pool)
                    .await
                    .unwrap();

                let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam").insert(&pool).await.unwrap();


                let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
                let bot_account =
                    DbNewBotAccount::new(&platform, Some(&project), "RB-890123",None , b"password".to_vec());

                let user_account = user_account.insert(&pool).await.unwrap();
                let bot_account = bot_account.insert(&pool).await.unwrap();
                moma::DbProjectUser::link(&project, &user, &pool).await.unwrap();
                let started = time::macros::datetime!(2024-01-01 00:02);
                let topic = "Сломалась сенокосилка и унитаз";
                let ticket = DbNewTicket::new(&user, &project, topic, started);
                let mut ticket = ticket.insert(&pool).await.unwrap();

                moma::DbUserAccountProject::link(&user_account, &project, &pool).await.unwrap();

                let started = time::macros::datetime!(2024-01-01 00:02);
                let chat_id = "XYZ-1000";
                let mut chat = DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform, started)
                    .insert(&pool)
                    .await
                    .unwrap();

                moma::DbTicketChat::link(&ticket, &chat, &pool).await.unwrap();

                let content = "Help me, help me! My lawnmower broke. It's the New Year and my lawnmower is broken!";
                let msg = DbNewMessage::new_user(&user_account, 1, "PWRR-001/M-0970", &mut chat, &mut ticket, content).unwrap()
                .insert(&pool).await.unwrap();

                let att = DbNewAttachment::new(&msg, 1, "PWRR-001/F-0970", "http://rangergram.com/files/f-0970", 9979)
                    .insert(&pool).await.unwrap();

                let mut msg_got = DbMessage::get_by_id(msg.pkey(), &pool).await.unwrap();
                let mut att_got = DbAttachment::get_by_id(msg.pkey(), &pool).await.unwrap();

                assert_eq!(msg_got.pkey(), msg.pkey());
                assert_eq!(msg_got.user_account_id, msg.user_account_id);
                assert_eq!(msg_got.bot_account_id, msg.bot_account_id);
                assert_eq!(msg_got.r#type, msg.r#type);
                assert_eq!(msg_got.external_id, msg.external_id);
                assert_eq!(msg_got.messenger_chat_id, msg.messenger_chat_id);
                assert_eq!(msg_got.query_ticket_id, msg.query_ticket_id);
                assert_eq!(msg_got.content, msg.content);
                assert_eq!(msg_got.edited, msg.edited);
                assert_eq!(msg_got.deleted, msg.deleted);
                assert_eq!(att_got.pkey(), att.pkey());
                assert_eq!(att_got.message_id, att.message_id);
                assert_eq!(att_got.r#type, att.r#type);
                assert_eq!(att_got.external_id, att.external_id);
                assert_eq!(att_got.file_url, att.file_url);
                assert_eq!(att_got.file_size, att.file_size);

                att_got.file_size = 10_003;
                msg_got.content = "My lawnmower has broken down. It happened shortly after New Year's.".to_string().into();

                att_got.update(&pool).await.unwrap();
                msg_got.update(&pool).await.unwrap();

                let msg_got2 = DbMessage::get_by_id(msg.pkey(), &pool).await.unwrap();
                let att_got2 = DbAttachment::get_by_id(msg.pkey(), &pool).await.unwrap();

                assert_eq!(msg_got2, msg_got);
                assert_eq!(att_got2, att_got);

                att.delete(&pool).await.unwrap();
                msg.delete(&pool).await.unwrap();

                let err = DbMessage::get_by_id(msg.pkey(), &pool).await.unwrap_err();
                let err2 = DbAttachment::get_by_id(msg.pkey(), &pool).await.unwrap_err();

                assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
                assert!(matches!(err2, DbError::RawSql(sqlx::Error::RowNotFound)));
                Ok(())
            },
        )
        .await
}

#[tokio::test]
async fn test_full_message_single_and_many() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "tests/sql/postgres/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
                let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram").insert(&pool).await.unwrap();
                let user = DbNewUser::new("+79451234567", "The Red Ranger").insert(&pool).await.unwrap();

                let project_group = DbNewProjectGroup::new("Telecorp")
                    .insert(&pool)
                    .await
                    .unwrap();
                let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam").insert(&pool).await.unwrap();


                let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
                let bot_account =
                    DbNewBotAccount::new(&platform, Some(&project), "RB-890123",None , b"password".to_vec());

                let user_account = user_account.insert(&pool).await.unwrap();
                let bot_account = bot_account.insert(&pool).await.unwrap();
                moma::DbProjectUser::link(&project, &user, &pool).await.unwrap();
                let started = time::macros::datetime!(2024-01-01 00:02);
                let topic = "Сломалась сенокосилка и унитаз";
                let ticket = DbNewTicket::new(&user, &project, topic, started);
                let mut ticket = ticket.insert(&pool).await.unwrap();

                moma::DbUserAccountProject::link(&user_account, &project, &pool).await.unwrap();

                let started = time::macros::datetime!(2024-01-01 00:02);
                let chat_id = "XYZ-1000";
                let mut chat = DbNewChat::new(chat_id, &user_account, &bot_account, &project, &platform, started)
                    .insert(&pool)
                    .await
                    .unwrap();

                moma::DbTicketChat::link(&ticket, &chat, &pool).await.unwrap();

                let content = "Help me, help me! My lawnmower broke. It's the New Year and my lawnmower is broken!";
                let msg = DbNewMessage::new_user(&user_account, 1, "PWRR-001/M-0970", &mut chat, &mut ticket, content).unwrap()
                .insert(&pool).await.unwrap();
                let msg2 = DbNewMessage::new_user(&user_account, 1, "PWRR-001/M-0971", &mut chat, &mut ticket, content).unwrap()
                .insert(&pool).await.unwrap();
                let msg3 = DbNewMessage::new_user(&user_account, 1, "PWRR-001/M-0972", &mut chat, &mut ticket, content).unwrap()
                .insert(&pool).await.unwrap();
                let msg4 = DbNewMessage::new_user(&user_account, 1, "PWRR-001/M-0972", &mut chat, &mut ticket, content).unwrap()
                .insert(&pool).await.unwrap();

                let att = DbNewAttachment::new(&msg, 1, "PWRR-001/F-0970", "http://rangergram.com/files/f-0970", 9979)
                    .insert(&pool).await.unwrap();
                let att2 = DbNewAttachment::new(&msg2, 1, "PWRR-001/F-0971", "http://rangergram.com/files/f-0971", 9979)
                    .insert(&pool).await.unwrap();
                let att3 = DbNewAttachment::new(&msg, 1, "PWRR-001/F-0972", "http://rangergram.com/files/f-0972", 9979)
                    .insert(&pool).await.unwrap();
                let att4 = DbNewAttachment::new(&msg2, 1, "PWRR-001/F-0973", "http://rangergram.com/files/f-0973", 9979)
                    .insert(&pool).await.unwrap();
                let att5 = DbNewAttachment::new(&msg, 1, "PWRR-001/F-0974", "http://rangergram.com/files/f-0974", 9979)
                    .insert(&pool).await.unwrap();
                let att6 = DbNewAttachment::new(&msg3, 1, "PWRR-001/F-0975", "http://rangergram.com/files/f-0975", 9979)
                    .insert(&pool).await.unwrap();
                let att7 = DbNewAttachment::new(&msg3, 1, "PWRR-001/F-0976", "http://rangergram.com/files/f-0976", 9979)
                    .insert(&pool).await.unwrap();
                let att8 = DbNewAttachment::new(&msg3, 1, "PWRR-001/F-0977", "http://rangergram.com/files/f-0977", 9979)
                    .insert(&pool).await.unwrap();
                let att9 = DbNewAttachment::new(&msg3, 1, "PWRR-001/F-0978", "http://rangergram.com/files/f-0978", 9979)
                    .insert(&pool).await.unwrap();

                let full_msg = msg.clone().get_files(&pool).await.unwrap();
                let full_msg2 = msg2.clone().get_files(&pool).await.unwrap();
                let full_msg3 = msg3.clone().get_files(&pool).await.unwrap();
                let full_msg4 = msg4.clone().get_files(&pool).await.unwrap();

                assert_eq!(full_msg.message, msg);
                assert_eq!(full_msg.files.len(), 3);
                assert_eq!(full_msg.files[0], att);
                assert_eq!(full_msg.files[1], att3);
                assert_eq!(full_msg.files[2], att5);

                assert_eq!(full_msg2.message, msg2);
                assert_eq!(full_msg2.files.len(), 2);
                assert_eq!(full_msg2.files[0], att2);
                assert_eq!(full_msg2.files[1], att4);

                assert_eq!(full_msg3.message, msg3);
                assert_eq!(full_msg3.files.len(), 4);
                assert_eq!(full_msg3.files[0], att6);
                assert_eq!(full_msg3.files[1], att7);
                assert_eq!(full_msg3.files[2], att8);
                assert_eq!(full_msg3.files[3], att9);

                assert_eq!(full_msg4.message, msg4);
                assert!(full_msg4.files.is_empty());

                let msgs_t = DbFullMessage::get_for_ticket(ticket.pkey(), &pool).await.unwrap();
                let msgs_ch = DbFullMessage::get_for_chat(chat.pkey(), &pool).await.unwrap();
                let msgs_his1 = DbFullMessage::get_history(chat.pkey(), Some(2), Some(1), &pool).await.unwrap();

                assert_eq!(msgs_t, msgs_ch);

                assert_eq!(msgs_t.len(), 4);

                assert_eq!(msgs_t[0], full_msg);
                assert_eq!(msgs_t[1], full_msg2);
                assert_eq!(msgs_t[2], full_msg3);
                assert_eq!(msgs_t[3], full_msg4);

                assert_eq!(msgs_his1.len(), 1);
                assert_eq!(msgs_his1[0].message.pkey(), 3);
                assert_eq!(&msgs_his1[0].message.external_id, "PWRR-001/M-0972");
                assert_eq!(msgs_his1[0].files.len(), 4);

            Ok(())
        },
    )
    .await
}
