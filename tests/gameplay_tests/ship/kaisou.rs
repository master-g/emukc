//! Tests for the kaisou (equipment) operations driven by client form fields.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext, username: &str) -> i64 {
        let account = context.sign_up(username, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, username).await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn slot_exchange_rejects_an_index_outside_the_five_slots() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "kaisou-range").await;
        let ship = context.add_ship(pid, 1).await.unwrap();

        for (src, dst) in [(0, 5), (5, 0), (-1, 0), (0, i64::MAX)] {
            let result = context.slot_exchange(pid, ship.api_id, src, dst).await;
            assert!(
                matches!(result, Err(GameplayError::WrongType(_))),
                "({src}, {dst}) gave {result:?}"
            );
        }
        assert!(context.slot_exchange(pid, ship.api_id, 0, 4).await.is_ok());
    }

    #[tokio::test]
    async fn another_profiles_ship_and_equipment_read_as_missing() {
        let context = crate::TestContext::new().await;
        let owner = new_profile(&context, "kaisou-owner").await;
        let other = new_profile(&context, "kaisou-other").await;

        let ship = context.add_ship(owner, 1).await.unwrap();
        // 12.7cm連装砲
        let gun = context.add_slot_item(owner, 2, 0, 0).await.unwrap();
        context.set_slot_item(owner, ship.api_id, 0, gun.api_id).await.unwrap();
        let before = context.find_ship(owner, ship.api_id).await.unwrap().unwrap();
        assert_eq!(before.api_slot[0], gun.api_id);

        let own_ship = context.add_ship(other, 1).await.unwrap();
        let attempts = [
            context.set_slot_item(other, ship.api_id, 1, -1).await,
            context.set_slot_item(other, own_ship.api_id, 0, gun.api_id).await,
            context.set_exslot_item(other, own_ship.api_id, gun.api_id).await,
            context.unset_all_slots(other, ship.api_id).await,
            context.slot_exchange(other, ship.api_id, 0, 1).await.map(|_| ()),
        ];
        for (i, result) in attempts.into_iter().enumerate() {
            assert!(
                matches!(result, Err(GameplayError::EntryNotFound(_))),
                "attempt {i} gave {result:?}"
            );
        }

        assert!(context.find_ship(other, ship.api_id).await.unwrap().is_none());
        let after = context.find_ship(owner, ship.api_id).await.unwrap().unwrap();
        assert_eq!(after.api_slot, before.api_slot);
        let own_after = context.find_ship(other, own_ship.api_id).await.unwrap().unwrap();
        assert_eq!(own_after.api_slot, own_ship.api_slot);
    }
}
