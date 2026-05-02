//! Tests for useitem ↔ material table sync (bucket/torch/devmat/screw).

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        (db, codex)
    }

    async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
        let account = context.sign_up("test-useitem-sync", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "sync-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    fn find_useitem(items: &[KcApiUserItem], api_id: i64) -> Option<i64> {
        items.iter().find(|i| i.api_id == api_id).map(|i| i.api_count)
    }

    #[tokio::test]
    async fn useitem_returns_material_bucket_after_add() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add 5 buckets via material table (simulates expedition reward)
        context.add_material(pid, &[(MaterialCategory::Bucket, 5)]).await.unwrap();

        let items = context.get_use_items(pid).await.unwrap();
        let bucket_count = find_useitem(&items, 1).expect("bucket entry should exist");
        let mats = context.get_materials(pid).await.unwrap();

        assert_eq!(bucket_count, mats.bucket, "useitem bucket count should match material table");
        assert!(bucket_count >= 5, "bucket count should reflect the addition");
    }

    #[tokio::test]
    async fn useitem_returns_material_bucket_after_deduct() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add buckets first
        context.add_material(pid, &[(MaterialCategory::Bucket, 10)]).await.unwrap();

        // Deduct 3 (simulates instant repair / ndock highspeed)
        context.deduct_material(pid, &[(MaterialCategory::Bucket, 3)]).await.unwrap();

        let items = context.get_use_items(pid).await.unwrap();
        let bucket_count = find_useitem(&items, 1).expect("bucket entry should exist");
        let mats = context.get_materials(pid).await.unwrap();

        assert_eq!(bucket_count, mats.bucket, "useitem bucket should match after deduct");
    }

    #[tokio::test]
    async fn useitem_includes_all_four_material_resources() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add all four
        context
            .add_material(
                pid,
                &[
                    (MaterialCategory::Bucket, 7),
                    (MaterialCategory::Torch, 11),
                    (MaterialCategory::DevMat, 13),
                    (MaterialCategory::Screw, 17),
                ],
            )
            .await
            .unwrap();

        let items = context.get_use_items(pid).await.unwrap();
        let mats = context.get_materials(pid).await.unwrap();

        assert_eq!(find_useitem(&items, 1), Some(mats.bucket), "bucket (api_id=1) mismatch");
        assert_eq!(find_useitem(&items, 2), Some(mats.torch), "torch (api_id=2) mismatch");
        assert_eq!(find_useitem(&items, 3), Some(mats.devmat), "devmat (api_id=3) mismatch");
        assert_eq!(find_useitem(&items, 4), Some(mats.screw), "screw (api_id=4) mismatch");
    }

    #[tokio::test]
    async fn useitem_returns_material_values_for_new_profile() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // New profile has no use_item records for bucket/torch/devmat/screw
        let items = context.get_use_items(pid).await.unwrap();
        let mats = context.get_materials(pid).await.unwrap();

        // Even without use_item records, the response must include these 4 entries
        assert_eq!(
            find_useitem(&items, 1),
            Some(mats.bucket),
            "bucket should come from material table"
        );
        assert_eq!(
            find_useitem(&items, 2),
            Some(mats.torch),
            "torch should come from material table"
        );
        assert_eq!(
            find_useitem(&items, 3),
            Some(mats.devmat),
            "devmat should come from material table"
        );
        assert_eq!(
            find_useitem(&items, 4),
            Some(mats.screw),
            "screw should come from material table"
        );
    }

    #[tokio::test]
    async fn useitem_non_material_items_remain_from_use_item_table() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add a non-material use item (mst_id=5, not in 1-4 range)
        context.add_use_item(pid, 5, 42).await.unwrap();

        let items = context.get_use_items(pid).await.unwrap();
        assert_eq!(
            find_useitem(&items, 5),
            Some(42),
            "non-material use item should come from use_item table"
        );
    }

    #[tokio::test]
    async fn incentive_useitem_bucket_updates_material_table() {
        use emukc_internal::prelude::{KcApiIncentiveItem, KcApiIncentiveMode, KcApiIncentiveType};

        let context = new_context().await;
        let pid = new_profile(&context).await;

        let mats_before = context.get_materials(pid).await.unwrap();

        // Add incentive with UseItem type for bucket (mst_id=1)
        context
            .add_incentive(
                pid,
                &[KcApiIncentiveItem {
                    api_mode: KcApiIncentiveMode::PreRegister as i64,
                    api_type: KcApiIncentiveType::UseItem as i64,
                    api_mst_id: 1, // Bucket
                    api_getmes: None,
                    api_slotitem_level: None,
                    amount: 5,
                    alv: 0,
                }],
            )
            .await
            .unwrap();

        context.confirm_incentives(pid).await.unwrap();

        let mats_after = context.get_materials(pid).await.unwrap();
        assert_eq!(
            mats_after.bucket,
            mats_before.bucket + 5,
            "UseItem incentive for bucket should update material table"
        );

        // Verify useitem also reflects the change
        let items = context.get_use_items(pid).await.unwrap();
        assert_eq!(
            find_useitem(&items, 1),
            Some(mats_after.bucket),
            "useitem should match material table after incentive"
        );
    }
}
