use super::*;
use crate::core_schema::test_cfg::TestCfg;
use crate::error::DbError;

#[tokio::test]
async fn test_platform_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let mut platform = DbNewPlatform::new(ApiId::Vk, "Wagongram")
                .insert(&pool)
                .await
                .unwrap();
            let p_id = platform.pkey();

            let pf = DbPlatform::get_by_id(p_id, &pool).await.unwrap();
            assert_eq!(platform.id, p_id);
            assert_eq!(platform.api_id, pf.api_id);
            assert_eq!(platform.name, pf.name);
            assert_eq!(platform.altered_on, pf.altered_on);
            platform.created_on = pf.created_on; //Костыль пока что.
            assert_eq!(platform, pf);

            platform.name = "Handcartgram".to_string();
            platform.update(&pool).await.unwrap();

            let pf2 = DbPlatform::get_by_id(p_id, &pool).await.unwrap();
            assert_eq!(platform, pf2);

            platform.delete(&pool).await.unwrap();
            let err = DbPlatform::get_by_id(p_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_platform_mirror_crud() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Wagongram")
                .insert(&pool)
                .await
                .unwrap();
            let p_id = platform.pkey();

            let mut mirror = DbNewPlatformMirror::new(&platform, "wagon-gram.io", "")
                .insert(&pool)
                .await
                .unwrap();

            let m = DbPlatformMirror::get_by_id(p_id, &pool).await.unwrap();
            assert_eq!(mirror, m);

            mirror.note = "Mirror mirror on the wall".to_string();
            mirror.update(&pool).await.unwrap();

            let m2 = DbPlatformMirror::get_by_id(p_id, &pool).await.unwrap();
            assert_eq!(mirror, m2);

            m2.delete(&pool).await.unwrap();
            let err = DbPlatformMirror::get_by_id(p_id, &pool).await.unwrap_err();

            platform.delete(&pool).await.unwrap();
            let err2 = DbPlatform::get_by_id(p_id, &pool).await.unwrap_err();

            assert!(matches!(err, DbError::RawSql(sqlx::Error::RowNotFound)));
            assert!(matches!(err2, DbError::RawSql(sqlx::Error::RowNotFound)));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_full_platform() {
    crate::test_frame::run_test_postgres::<TestCfg, _, ()>(
        "../../sql/core/",
        "../../sql/core/",
        "tests/sql/postgres/drop_core",
        |pool| async move {
            let platform = DbNewPlatform::new(ApiId::Vk, "Wagongram")
                .insert(&pool)
                .await
                .unwrap();
            let p_id = platform.pkey();

            // Alphabetically last, inserted first
            let m4 = DbNewPlatformMirror::new(&platform, "wagon-gram.io", "")
                .insert(&pool)
                .await
                .unwrap();
            // Alphabetically first, inserted second
            let m1 = DbNewPlatformMirror::new(&platform, "cart-gram.io", "")
                .insert(&pool)
                .await
                .unwrap();
            //Alphabetically third, inserted third
            let m3 = DbNewPlatformMirror::new(&platform, "ranger-gram.io", "")
                .insert(&pool)
                .await
                .unwrap();
            // Alphabetically second, inserted last
            let m2 = DbNewPlatformMirror::new(&platform, "pigeon-gram.io", "")
                .insert(&pool)
                .await
                .unwrap();

            let p1 = DbPlatform::get_by_id(p_id, &pool).await.unwrap();
            let fp = DbFullPlatform::get_by_id(p_id, &pool).await.unwrap();
            let fp2 = p1.clone().get_mirrors(&pool).await.unwrap();
            assert_eq!(fp, fp2);

            assert_eq!(fp.mirrors.len(), 4);
            assert_eq!(fp.mirrors[0].url, m1.url);
            assert_eq!(fp.mirrors[1].url, m2.url);
            assert_eq!(fp.mirrors[2].url, m3.url);
            assert_eq!(fp.mirrors[3].url, m4.url);

            let platform2 = DbNewPlatform::new(ApiId::Telegram, "pidgeon")
                .insert(&pool)
                .await
                .unwrap();

            // Alphabetically last, inserted first
            DbNewPlatformMirror::new(&platform2, "wagon-pidgein.com", "")
                .insert(&pool)
                .await
                .unwrap();
            // Alphabetically first, inserted second
            DbNewPlatformMirror::new(&platform2, "coo-coo.io", "")
                .insert(&pool)
                .await
                .unwrap();

            let all = DbFullPlatform::get_all(&pool).await.unwrap();

            assert_eq!(all.len(), 2);
            assert_eq!(all[0].mirrors.len(), 4);
            assert_eq!(all[1].mirrors.len(), 2);
            assert_eq!(all[0].platform.api_id, ApiId::Vk);
            assert_eq!(&all[0].platform.name, "Wagongram");
            assert_eq!(all[1].platform.api_id, ApiId::Telegram);
            assert_eq!(&all[1].platform.name, "pidgeon");

            Ok(())
        },
    )
    .await
}
