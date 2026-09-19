//! `Ctx::destroy_items` must commit: the scrapped items disappear and the refund lands.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    #[tokio::test]
    async fn destroy_items_persists_removal_and_refund() {
        let context = crate::TestContext::new().await;
        let account = context.sign_up("test-destroy-items", "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "scrapper").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let pid = session.profile.id;

        let item = context.add_slot_item(pid, 25, 0, 0).await.unwrap(); // 25mm単装機銃
        let before = context.get_materials(pid).await.unwrap();

        let refund = context.destroy_items(pid, &[item.api_id]).await.unwrap();

        let ids: Vec<i64> =
            context.get_slot_items(pid).await.unwrap().iter().map(|i| i.api_id).collect();
        assert!(!ids.contains(&item.api_id), "scrapped item must be gone after destroy_items");

        let after = context.get_materials(pid).await.unwrap();
        for (cat, amount) in refund {
            let (b, a) = match cat {
                MaterialCategory::Fuel => (before.fuel, after.fuel),
                MaterialCategory::Ammo => (before.ammo, after.ammo),
                MaterialCategory::Steel => (before.steel, after.steel),
                MaterialCategory::Bauxite => (before.bauxite, after.bauxite),
                other => panic!("unexpected scrap refund category {other:?}"),
            };
            assert_eq!(a, b + amount, "{cat:?} refund must be persisted");
        }
    }
}
