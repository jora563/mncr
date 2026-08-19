use super::*;
use crate::queue_schema::test_cfg::TestCfg;
use crate::queue_schema::{DbNewLastOperator, DbNewQueuedTicket, DbQueuedTicket};

#[tokio::test]
async fn test_queue_db_1() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/queue/",
        "../../sql/queue/",
        "tests/sql/postgres/drop_queue",
        |pool| async move {
            let operator_ext = "bob-XGR000";
            let operator_ext2 = "bob-XGR001";

            let pp = vec!["Good Project".to_owned(), "Bad Project".to_owned()];

            let t1 = DbNewQueuedTicket::new(1, "Good Project", 1)
                .insert(&pool)
                .await
                .unwrap();
            let t2 = DbNewQueuedTicket::new(2, "Good Project", 2)
                .insert(&pool)
                .await
                .unwrap();
            let t3 = DbNewQueuedTicket::new(3, "Good Project", 1)
                .insert(&pool)
                .await
                .unwrap();

            // The VIP gets picket first.
            let got = DbQueuedTicket::get_next(operator_ext, &pp, &pool)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(got.pkey(), t2.pkey());
            // Simuate adding an operator.
            DbNewLastOperator::new(operator_ext, t2.pkey())
                .insert(&pool)
                .await
                .unwrap();

            // The old non-vip gets picked next.
            let got = DbQueuedTicket::get_next(operator_ext, &pp, &pool)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(got.pkey(), t1.pkey());
            // Simuate adding an operator.
            DbNewLastOperator::new(operator_ext, t1.pkey())
                .insert(&pool)
                .await
                .unwrap();

            // The new non-vip gets picked next.
            let got3 = DbQueuedTicket::get_next(operator_ext, &pp, &pool)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(got3.pkey(), t3.pkey());
            // Simuate adding an operator.
            DbNewLastOperator::new(operator_ext, t3.pkey())
                .insert(&pool)
                .await
                .unwrap();

            // This operator already holds the ticket.
            let got = DbQueuedTicket::get_next(operator_ext, &pp, &pool)
                .await
                .unwrap();
            assert!(got.is_none(), "{got:#?}");

            // There is no valid ticket left in the queue to pick.
            let got = DbQueuedTicket::get_next(operator_ext2, &pp, &pool)
                .await
                .unwrap();
            assert!(got.is_none(), "{got:#?}");

            // The last valid ticket for the operator is going to exit.
            let got_again = DbQueuedTicket::try_get_last_for_operator(operator_ext, &pool)
                .await
                .unwrap();
            let got_again = got_again.unwrap();

            assert_eq!(got_again, got3);

            Ok(())
        },
    )
    .await
}
