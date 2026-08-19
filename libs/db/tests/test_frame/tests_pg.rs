use super::PostgresTestConfig as Ptc;
use db::test_frame::run_test_postgres;

/// Do not write tests like this! This is just to demonstrate that the library
/// allows to run 4 tests in parallel.
#[tokio::test]
async fn test_1_test_4_dbs() {
    let f1 = run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            let res = sqlx::query("SELECT name FROM people WHERE age > 30")
                .fetch_all(&pool)
                .await?;
            assert_eq!(
                res.len(),
                2,
                "failed `SELECT name FROM people WHERE age > 30`"
            );
            Ok(())
        },
    );
    let f2 = run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            let res = sqlx::query("SELECT name FROM people WHERE age > 60")
                .fetch_all(&pool)
                .await?;
            assert_eq!(
                res.len(),
                1,
                "failed `SELECT name FROM people WHERE age > 60`"
            );
            Ok(())
        },
    );
    let f3 = run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            let res = sqlx::query("SELECT name FROM people WHERE age < 30")
                .fetch_all(&pool)
                .await?;
            assert_eq!(
                res.len(),
                4,
                "failed `SELECT name FROM people WHERE age < 30`"
            );
            Ok(())
        },
    );
    let f4 = run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            let res = sqlx::query("SELECT name FROM people WHERE age < 20")
                .fetch_all(&pool)
                .await?;
            assert_eq!(
                res.len(),
                2,
                "failed `SELECT name FROM people WHERE age > 20`"
            );
            Ok(())
        },
    );
    let (_, _, _, _) = tokio::join!(f1, f2, f3, f4);
}

#[tokio::test]
async fn test_2_select() {
    run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            let res = sqlx::query("SELECT name FROM people")
                .fetch_all(&pool)
                .await?;
            assert_eq!(res.len(), 6);
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_3_select() {
    run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            println!("Inside test_3");
            let res = sqlx::query("SELECT * FROM addresses WHERE region LIKE 'S'")
                .fetch_all(&pool)
                .await?;
            println!("res len:{}", res.len());
            assert_eq!(res.len(), 2);
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_4() {
    run_test_postgres::<Ptc, _, ()>(
        "tests/sql/postgres/",
        "tests/sql/postgres/extra_fixtures",
        "tests/sql/postgres/",
        |pool| async move {
            // Initially relations are empty
            let r = sqlx::query("SELECT * FROM  people_addresses_rel")
                .fetch_all(&pool)
                .await?;
            assert!(r.is_empty());

            // Thereafter the relations are inserted
            sqlx::query(
                "
            INSERT INTO people_addresses_rel(people_id, addresses_id)
            VALUES(1,1),(4,3)",
            )
            .execute(&pool)
            .await?;
            // And now the relations should not be empty.
            let r = sqlx::query("SELECT * FROM  people_addresses_rel")
                .fetch_all(&pool)
                .await?;
            assert_eq!(r.len(), 2);
            Ok(())
        },
    )
    .await
}
