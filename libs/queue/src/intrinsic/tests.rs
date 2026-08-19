use super::*;

use db::core_schema::moma::MoMa;
use db::core_schema::*;
use db::test_frame::ConfigDriver;

struct QCfg;
struct TestCfg;

impl ConfigDriver for QCfg {
    fn initialise() -> Self {
        Self
    }
    fn db_name_root(&self) -> Box<str> {
        "intrinsic_queue_test_db".into()
    }
    fn db_host(&self) -> Box<str> {
        "postgresql://aio_core:password@127.0.0.1:5432".into()
    }
}

impl ConfigDriver for TestCfg {
    fn initialise() -> Self {
        Self
    }
    fn db_name_root(&self) -> Box<str> {
        "ai_omni_db".into()
    }
    fn db_host(&self) -> Box<str> {
        "postgresql://aio_core:password@127.0.0.1:5432".into()
    }
}

async fn create_ticket() -> Vec<DbTicket> {
    db::test_frame::run_test_postgres::<TestCfg, _, _>(
        "../../sql/core/",
        "../../sql/core/",
        "../../libs/db/tests/sql/postgres/drop_core",
        |pool| async move {
            let user = DbNewUser::new("+79451234567", "The Red Ranger");
            let project_group = DbNewProjectGroup::new("Telecorp")
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
            let new_ticket = DbNewTicket::new(&user, &project, topic, started);

            assert_eq!(
                new_ticket.insert(&pool).await.unwrap_err().to_string(),
                "Cannot validate Ticket: User The Red Ranger not part of project The Big Spam."
            );

            let new_ticket1 = DbNewTicket::new(&user, &project, topic, started);
            let new_ticket2 = DbNewTicket::new(&user, &project, topic, started);
            let new_ticket3 = DbNewTicket::new(&user, &project, topic, started);
            let new_ticket4 = DbNewTicket::new(&user, &project, topic, started);

            moma::DbProjectUser::link(&project, &user, &pool)
                .await
                .unwrap();
            let tickets = vec![
                new_ticket1.insert(&pool).await.unwrap(),
                new_ticket2.insert(&pool).await.unwrap(),
                new_ticket3.insert(&pool).await.unwrap(),
                new_ticket4.insert(&pool).await.unwrap(),
            ];
            Ok(tickets)
        },
    )
    .await
}

#[tokio::test]
async fn test_queue_complex_tests() {
    let tickets = create_ticket().await;

    db::test_frame::run_test_postgres::<QCfg, _, ()>(
        "../../sql/queue/",
        "../../sql/queue/",
        "../../libs/db/tests/sql/postgres/drop_queue",
        |_| async move {
            let pp = vec!["Good Project".to_owned(), "Bad Project".to_owned()];
            let queue = Queue::from_file("../../libs/queue/tests/test_queue_cfg.toml")
                .await
                .unwrap();

            let mut queued_tickets = vec![];
            for (t, vip) in tickets.iter().zip(vec![10, 10, 20, 10]) {
                let t = queue.insert_ticket(t, "Good Project", vip).await.unwrap();
                queued_tickets.push(t);
            }

            let operator = "OP-TC-0001";
            let (next_ticket1, op1) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let (next_ticket2, op2) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let (next_ticket3, op3) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let (next_ticket4, op4) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let x = queue.get_next_for_operator(operator, &pp).await.unwrap();
            let y = queue.get_next_for_operator(operator, &pp).await.unwrap();

            assert!(x.is_none());
            assert!(y.is_none());
            // Confirm that the order has changed.
            assert_eq!(next_ticket1.pkey(), tickets[2].pkey());
            assert_eq!(next_ticket2.pkey(), tickets[0].pkey());
            assert_eq!(next_ticket3.pkey(), tickets[1].pkey());
            assert_eq!(next_ticket4.pkey(), tickets[3].pkey());
            // Confirm correctness of operator/order
            assert_eq!(op1.ext_id, operator);
            assert_eq!(op2.ext_id, operator);
            assert_eq!(op3.ext_id, operator);
            assert_eq!(op4.ext_id, operator);
            assert_eq!(op1.last_ticket_id, tickets[2].pkey());
            assert_eq!(op2.last_ticket_id, tickets[0].pkey());
            assert_eq!(op3.last_ticket_id, tickets[1].pkey());
            assert_eq!(op4.last_ticket_id, tickets[3].pkey());
            assert!(op1.in_work);
            assert!(op2.in_work);
            assert!(op3.in_work);
            assert!(op4.in_work);

            // We end work with two tickets to the queue. The low priority `next_ticket4` and the
            // high priority `next_ticket1`. In this case, we should retrieve the high priority ticket
            // first.
            queue.end_work_with_ticket(op1).await.unwrap();
            queue.end_work_with_ticket(op4).await.unwrap();

            let (next2_ticket1, op1) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let (next2_ticket4, op4) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(next2_ticket1.pkey(), tickets[2].pkey());
            assert_eq!(next2_ticket4.pkey(), tickets[3].pkey());
            // Confirm correctness of operator/order
            assert_eq!(op1.last_ticket_id, tickets[2].pkey());
            assert_eq!(op4.last_ticket_id, tickets[3].pkey());

            // Now we return the high priority ticket to the queue and end work with the low priority
            // ticket. In this case, the low priority ticket should be picket first - since we
            // prioritize the ticket which is already at work.
            queue.end_work_return_to_queue(op1).await.unwrap();
            queue.end_work_with_ticket(op4).await.unwrap();

            let (next3_ticket1, op1) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let (next3_ticket4, op4) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(next3_ticket1.pkey(), tickets[3].pkey());
            assert_eq!(next3_ticket4.pkey(), tickets[2].pkey());
            // Confirm correctness of operator/order
            assert_eq!(op1.last_ticket_id, tickets[3].pkey());
            assert_eq!(op4.last_ticket_id, tickets[2].pkey());

            // If both tickets are returned to queue, the conventional order is maintained.
            queue.end_work_return_to_queue(op1).await.unwrap();
            queue.end_work_return_to_queue(op4).await.unwrap();

            let (next2_ticket1, op1) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();
            let (next2_ticket4, op4) = queue
                .get_next_for_operator(operator, &pp)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(next2_ticket1.pkey(), tickets[2].pkey());
            assert_eq!(next2_ticket4.pkey(), tickets[3].pkey());
            // Confirm correctness of operator/order
            assert_eq!(op1.last_ticket_id, tickets[2].pkey());
            assert_eq!(op4.last_ticket_id, tickets[3].pkey());

            // See if restoring the last ticket gets the right ticket
            // (should be next2_ticket4 for op1).
            let (next2_ticket4_r, op4_r) =
                queue.restore_for_operator(operator).await.unwrap().unwrap();

            assert_eq!(op4.last_ticket_id, op4_r.last_ticket_id);
            assert_eq!(next2_ticket4_r.pkey(), next2_ticket4.pkey());

            Ok(())
        },
    )
    .await
}
