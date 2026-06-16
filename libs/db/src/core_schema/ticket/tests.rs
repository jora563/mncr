use super::*;
use crate::core_schema::moma::{self, MoMa};
use crate::core_schema::test_cfg::TestCfg;
use crate::core_schema::*;
use crate::error::DbError;

#[tokio::test]
async fn test_validate_ticket() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let user = DbNewUser::new("+79451234567", "The Red Ranger");
            let project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
                .insert(&pool)
                .await
                .unwrap();
            let user = user.insert(&pool).await.unwrap();

            let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();

            let started = time::macros::datetime!(2024-01-01 00:02);
            let topic = "Сломалась сенокосилка и унитаз";
            let new_ticket = DbNewTicket::new(99_900_999, &user, &project, topic, started);

            assert_eq!(
                new_ticket.insert(&pool).await.unwrap_err().to_string(),
                "Cannot validate Ticket: User The Red Ranger not part of project The Big Spam."
            );

            let new_ticket = DbNewTicket::new(99_900_999, &user, &project, topic, started);
            moma::DbProjectUser::link(&project, &user, &pool)
                .await
                .unwrap();
            new_ticket.insert(&pool).await.unwrap();
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_ticket_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let user = DbNewUser::new("+79451234567", "The Red Ranger");
            let project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
                .insert(&pool)
                .await
                .unwrap();
            let user = user.insert(&pool).await.unwrap();

            let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();

            let started = time::macros::datetime!(2024-01-01 00:02);
            let topic = "Сломалась сенокосилка и унитаз";
            let new_ticket = DbNewTicket::new(99_900_999, &user, &project, topic, started);

            moma::DbProjectUser::link(&project, &user, &pool)
                .await
                .unwrap();
            let mut ticket = new_ticket.insert(&pool).await.unwrap();
            let t_id = ticket.pkey();

            let tck = DbTicket::get_by_id(t_id, &pool).await.unwrap();

            assert_eq!(tck.id, ticket.id);
            assert_eq!(tck.user_id, ticket.user_id);
            assert_eq!(tck.project_id, ticket.project_id);
            assert_eq!(tck.close_status, ticket.close_status);
            assert_eq!(tck.topic, ticket.topic);
            assert_eq!(tck.latest_post_on, ticket.latest_post_on);
            assert_eq!(tck.closed_on, ticket.closed_on);
            assert_eq!(tck.started_on, ticket.started_on);

            ticket.latest_post_on = Some(time::macros::datetime!(2024-01-01 00:03));
            ticket.update(&pool).await.unwrap();

            let tck2 = DbTicket::get_by_id(t_id, &pool).await.unwrap();
            assert_eq!(ticket, tck2);

            ticket.delete(&pool).await.unwrap();
            let err = DbTicket::get_by_id(t_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_full_ticket() {
    crate::test_frame::run_test_postgres::<TestCfg, _>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Rangergram")
                .insert(&pool)
                .await
                .unwrap();
            let user = DbNewUser::new("+79451234567", "The Red Ranger");
            let mut project_group = DbNewProjectGroup::new("LQIWEUDBQLWDBQW", "Telecorp")
                .insert(&pool)
                .await
                .unwrap();
            project_group.insert(&pool).await.unwrap();
            let user = user.insert(&pool).await.unwrap();

            let user_account = DbNewUserAccount::new(&user, &platform, "PWRR-001", "Red");
            let bot_account = DbNewBotAccount::new(&platform, "RB-890123", b"password".to_vec());

            let user_account = user_account.insert(&pool).await.unwrap();
            let bot_account = bot_account.insert(&pool).await.unwrap();

            let project = DbNewProject::new(&project_group, "AKUWDHWA-8691", "The Big Spam")
                .insert(&pool)
                .await
                .unwrap();

            let started = time::macros::datetime!(2024-01-01 00:02);
            let topic = "Сломалась сенокосилка и унитаз";
            let new_ticket = DbNewTicket::new(99_900_999, &user, &project, topic, started);

            moma::DbProjectUser::link(&project, &user, &pool)
                .await
                .unwrap();
            moma::DbUserAccountProject::link(&user_account, &project, &pool)
                .await
                .unwrap();
            moma::DbBotAccountProject::link(&bot_account, &project, &pool)
                .await
                .unwrap();
            let ticket = new_ticket.insert(&pool).await.unwrap();

            let started = time::macros::datetime!(2024-01-01 00:02);
            let chat_id = "XYZ-1000";
            let chat = DbNewChat::new(
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
            moma::DbTicketChat::link(&ticket, &chat, &pool)
                .await
                .unwrap();

            let content = "Help me, help me!";
            let msg1 = DbNewMessage::new_user(
                &user_account,
                1,
                "PWRR-001/M-0970",
                &chat,
                &ticket,
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
                &chat,
                &ticket,
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
                "PWRR-001/M-0972",
                &chat,
                &ticket,
                content,
            )
            .unwrap()
            .insert(&pool)
            .await
            .unwrap();

            let att3 = DbNewAttachment::new(
                &msg3,
                1,
                "PWRR-001/F-0973",
                "http://rangergram.com/files/f-0970",
                9979,
            )
            .insert(&pool)
            .await
            .unwrap();

            let att4 = DbNewAttachment::new(
                &msg3,
                1,
                "PWRR-001/F-0977",
                "http://rangergram.com/files/f-0970",
                9979,
            )
            .insert(&pool)
            .await
            .unwrap();

            let att5 = DbNewAttachment::new(
                &msg3,
                1,
                "PWRR-001/F-0981",
                "http://rangergram.com/files/f-0970",
                9979,
            )
            .insert(&pool)
            .await
            .unwrap();

            let ft = DbFullTicket::get_by_id(ticket.id, &pool).await.unwrap();

            assert_eq!(ft.ticket.id, ticket.id);
            assert_eq!(ft.ticket.user_id, ticket.user_id);
            assert_eq!(ft.ticket.project_id, ticket.project_id);
            assert_eq!(ft.ticket.close_status, ticket.close_status);
            assert_eq!(ft.ticket.topic, ticket.topic);
            assert_eq!(ft.ticket.latest_post_on, ticket.latest_post_on);
            assert_eq!(ft.ticket.closed_on, ticket.closed_on);
            assert_eq!(ft.ticket.started_on, ticket.started_on);

            assert_eq!(ft.messages.len(), 3);

            assert_eq!(ft.messages[0].message.content, msg1.content);
            assert_eq!(ft.messages[0].message.pkey(), msg1.pkey());
            assert_eq!(ft.messages[1].message.content, msg2.content);
            assert_eq!(ft.messages[1].message.pkey(), msg2.pkey());
            assert_eq!(ft.messages[2].message.content, msg3.content);
            assert_eq!(ft.messages[2].message.pkey(), msg3.pkey());

            assert_eq!(ft.messages[0].files.len(), 1);
            assert_eq!(ft.messages[0].files[0].external_id, att1.external_id);
            assert_eq!(ft.messages[0].files[0].pkey(), att1.pkey());

            assert_eq!(ft.messages[1].files.len(), 1);
            assert_eq!(ft.messages[1].files[0].external_id, att2.external_id);
            assert_eq!(ft.messages[1].files[0].pkey(), att2.pkey());

            assert_eq!(ft.messages[2].files.len(), 3);
            assert_eq!(ft.messages[2].files[0].external_id, att3.external_id);
            assert_eq!(ft.messages[2].files[0].pkey(), att3.pkey());
            assert_eq!(ft.messages[2].files[1].external_id, att4.external_id);
            assert_eq!(ft.messages[2].files[1].pkey(), att4.pkey());
            assert_eq!(ft.messages[2].files[2].external_id, att5.external_id);
            assert_eq!(ft.messages[2].files[2].pkey(), att5.pkey());

            Ok(())
        },
    )
    .await
}
