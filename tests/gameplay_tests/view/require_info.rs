//! Tests for `Ctx::require_info_view`.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::{ApiMstSlotitem, MaterialCategory};

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-require-info", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "require-info-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn create_slotitem_is_visible_in_require_info() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;

        let before = context.require_info_view(pid).await.unwrap();
        let craftable =
            context.codex.slotitem_extra_info.values().find(|item| item.craftable).unwrap().api_id;
        let costs = vec![
            (MaterialCategory::Fuel, 10),
            (MaterialCategory::Ammo, 10),
            (MaterialCategory::Steel, 10),
            (MaterialCategory::Bauxite, 10),
            (MaterialCategory::DevMat, 1),
        ];

        let (ids, _materials) = context.create_slotitem(pid, &[craftable], &costs).await.unwrap();
        let created_id = ids[0];
        assert!(created_id > 0);

        let after = context.require_info_view(pid).await.unwrap();
        assert_eq!(after.slot_items.len(), before.slot_items.len() + 1);
        assert!(after.slot_items.iter().any(|item| item.api_id == created_id));

        let type3 =
            context.codex.find::<ApiMstSlotitem>(&craftable).unwrap().api_type[2].to_string();
        let unset_key = format!("api_slottype{type3}");
        assert!(after.unset_slots.get(&unset_key).is_some_and(|items| items.contains(&created_id)));
    }
}
