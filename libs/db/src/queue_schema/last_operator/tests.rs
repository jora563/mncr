use super::*;
use crate::queue_schema::test_cfg::TestCfg;
use crate::queue_schema::{DbLastOperator, DbNewLastOperator, DbNewQueuedTicket};

#[tokio::test]
async fn test_queue_db_1() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/queue/",
        "../../sql/queue/",
        "tests/sql/postgres/drop_queue",
        |pool| async move {
            DbNewQueuedTicket::new(1, "Good Project", 1)
                .insert(&pool)
                .await
                .unwrap();
            DbNewQueuedTicket::new(2, "Good Project", 2)
                .insert(&pool)
                .await
                .unwrap();
            DbNewQueuedTicket::new(3, "Good Project", 2)
                .insert(&pool)
                .await
                .unwrap();

            DbNewLastOperator::new("bob-XGR000", 1)
                .insert(&pool)
                .await
                .unwrap();
            DbNewLastOperator::new("bob-XGR000", 2)
                .insert(&pool)
                .await
                .unwrap();
            let last_time = std::time::Instant::now();

            let lop_err = DbNewLastOperator::new("bob-XGR000", 1).insert(&pool).await;
            let lop_err2 = DbNewLastOperator::new("bob-XGR001", 1).insert(&pool).await;
            let mut lop3 = DbNewLastOperator::new("bob-XGR001", 3)
                .insert(&pool)
                .await
                .unwrap();

            assert!(lop_err.is_err());
            assert!(lop_err2.is_err());
            assert_eq!(
                &lop_err.unwrap_err().to_string(),
                "Cannot validate Last Operator: Ticket with \"id\" 1 is already assigned."
            );
            assert_eq!(
                &lop_err2.unwrap_err().to_string(),
                "Cannot validate Last Operator: Ticket with \"id\" 1 is already assigned."
            );
            assert!(lop3.in_work);
            assert_eq!(lop3.work_started, lop3.last_check_in);

            let lci3 = lop3.last_check_in;
            lop3.update_check_in(&pool).await.unwrap();
            assert!(lop3.last_check_in > lci3);

            let lci3 = lop3.last_check_in;
            lop3.end_work(&pool).await.unwrap();
            assert!(lop3.last_check_in > lci3);
            assert!(!lop3.in_work);

            let lci3 = lop3.last_check_in;
            lop3.start_work(&pool).await.unwrap();
            assert!(lop3.last_check_in > lci3);
            assert!(lop3.in_work);

            // NB: This test is going to be flaky.
            DbLastOperator::delete_older(last_time.elapsed().as_millis() as u32, &pool)
                .await
                .unwrap();

            let ops = sqlx::query_as::<_, DbLastOperator>("SELECT * FROM last_operator")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert_eq!(ops.len(), 1);
            assert_eq!(ops[0].pkey(), lop3.pkey());
            assert_eq!(ops[0].last_ticket_id, lop3.last_ticket_id);
            assert_eq!(ops[0].work_started, lop3.work_started);
            assert_eq!(ops[0].in_work, lop3.in_work);
            // Times can differ a little bit since postgres truncates.
            assert_eq!(
                ops[0].last_check_in.truncate_to_microsecond(),
                lop3.last_check_in.truncate_to_microsecond()
            );

            Ok(())
        },
    )
    .await
}
