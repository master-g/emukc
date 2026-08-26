//! Tests for hangar expansion gameplay (plan U3): useitem-105 consumption in
//! one transaction, ownership/range/balance validation, and the KTD8 capacity
//! read points (supply cap, marriage reset) using the synthesized value.

#[cfg(test)]
mod tests {
    use emukc_internal::db::sea_orm::{
        ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel,
    };
    use emukc_internal::prelude::*;

    /// 赤城, api_maxeq = [18, 18, 27, 10, 0]
    const AKAGI_MST_ID: i64 = 83;
    /// 睦月 → 睦月改 (254)
    const MUTSUKI_MST_ID: i64 = 1;
    const MUTSUKI_KAI_MST_ID: i64 = 254;
    /// 格納庫増設
    const HANGAR_EXPAND_ITEM: i64 = 105;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext, username: &str) -> i64 {
        let account = context.sign_up(username, "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "hangar-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn plus_of(context: &crate::TestContext, ship_id: i64) -> [Option<i64>; 5] {
        use emukc_internal::db::entity::profile::ship;

        let m = ship::Entity::find_by_id(ship_id).one(context.db()).await.unwrap().unwrap();
        [m.onslot_plus_1, m.onslot_plus_2, m.onslot_plus_3, m.onslot_plus_4, m.onslot_plus_5]
    }

    /// Write a slot's current load straight to the DB (fact-source setup for
    /// capacity tests, same pattern as the U2 `set_plus` helper).
    async fn set_onslot_1(context: &crate::TestContext, ship_id: i64, v: i64) {
        use emukc_internal::db::entity::profile::ship;

        let m = ship::Entity::find_by_id(ship_id).one(context.db()).await.unwrap().unwrap();
        let mut am = m.into_active_model();
        am.onslot_1 = ActiveValue::Set(v);
        am.update(context.db()).await.unwrap();
    }

    async fn item_count(context: &crate::TestContext, pid: i64) -> i64 {
        context.find_use_item(pid, HANGAR_EXPAND_ITEM).await.unwrap().api_count
    }

    #[tokio::test]
    async fn normal_expansion_consumes_item_and_returns_full_array() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-normal").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();

        let onslot_max = context.expand_hangar_slot(pid, ship.api_id, 2).await.unwrap();

        // full-slot array: unexpanded slots carry the base maxeq value
        assert_eq!(onslot_max, [18, 18, 27 + 1, 10, 0]);
        assert_eq!(item_count(&context, pid).await, 0);
        assert_eq!(plus_of(&context, ship.api_id).await, [None, None, Some(1), None, None]);
    }

    #[tokio::test]
    async fn foreign_ship_rejected() {
        let context = new_context().await;
        let pid_a = new_profile(&context, "hangar-owner").await;
        let pid_b = new_profile(&context, "hangar-foreign").await;

        let ship = context.add_ship(pid_a, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid_a, HANGAR_EXPAND_ITEM, 1).await.unwrap();
        context.add_use_item(pid_b, HANGAR_EXPAND_ITEM, 1).await.unwrap();

        let err = context.expand_hangar_slot(pid_b, ship.api_id, 0).await.unwrap_err();
        assert!(matches!(err, GameplayError::EntryNotFound(_)), "got: {err:?}");

        // both sides untouched
        assert_eq!(item_count(&context, pid_a).await, 1);
        assert_eq!(item_count(&context, pid_b).await, 1);
        assert_eq!(plus_of(&context, ship.api_id).await, [None; 5]);
    }

    #[tokio::test]
    async fn slot_pos_out_of_range_rejected() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-range").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();

        for bad_pos in [5, -1, 100] {
            let err = context.expand_hangar_slot(pid, ship.api_id, bad_pos).await.unwrap_err();
            assert!(matches!(err, GameplayError::WrongType(_)), "pos {bad_pos}: {err:?}");
        }

        assert_eq!(item_count(&context, pid).await, 1);
        assert_eq!(plus_of(&context, ship.api_id).await, [None; 5]);
    }

    #[tokio::test]
    async fn insufficient_item_rejected() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-poor").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();

        // no record at all -> EntryNotFound (deduct_use_item_impl)
        let err = context.expand_hangar_slot(pid, ship.api_id, 0).await.unwrap_err();
        assert!(matches!(err, GameplayError::EntryNotFound(_)), "got: {err:?}");
        assert_eq!(plus_of(&context, ship.api_id).await, [None; 5]);

        // record exists but balance is 0 -> Insufficient
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();
        context.expand_hangar_slot(pid, ship.api_id, 0).await.unwrap();
        assert_eq!(item_count(&context, pid).await, 0);

        let err = context.expand_hangar_slot(pid, ship.api_id, 1).await.unwrap_err();
        assert!(matches!(err, GameplayError::Insufficient(_)), "got: {err:?}");

        assert_eq!(item_count(&context, pid).await, 0);
        assert_eq!(plus_of(&context, ship.api_id).await, [Some(1), None, None, None, None]);
    }

    #[tokio::test]
    async fn expansion_visible_in_get_ships() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-port").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();
        context.expand_hangar_slot(pid, ship.api_id, 0).await.unwrap();

        let ships = context.get_ships(pid).await.unwrap();
        let found = ships.iter().find(|s| s.api_id == ship.api_id).unwrap();
        assert_eq!(found.api_onslot_max, Some([18 + 1, 18, 27, 10, 0]));
    }

    #[tokio::test]
    async fn supply_fills_to_synthesized_capacity() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-supply").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();
        context.expand_hangar_slot(pid, ship.api_id, 0).await.unwrap();
        set_onslot_1(&context, ship.api_id, 5).await;
        context
            .add_material(pid, &[(MaterialCategory::Bauxite, 1000), (MaterialCategory::Fuel, 1000)])
            .await
            .unwrap();

        let resp = context
            .charge_supply(pid, &[ship.api_id], KcApiChargeKind::Plane, false)
            .await
            .unwrap();

        // cap = maxeq[0] + 1 = 19, not the raw maxeq 18
        assert_eq!(resp.api_ship[0].api_onslot[0], 19);
        let found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_onslot[0], 19);
    }

    #[tokio::test]
    async fn remodel_preserves_expansion() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-remodel").await;

        let ship = context.add_ship(pid, MUTSUKI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();
        context.expand_hangar_slot(pid, ship.api_id, 1).await.unwrap();
        assert_eq!(plus_of(&context, ship.api_id).await, [None, Some(1), None, None, None]);

        context
            .add_material(pid, &[(MaterialCategory::Ammo, 200), (MaterialCategory::Steel, 200)])
            .await
            .unwrap();
        context.remodel(pid, ship.api_id).await.unwrap();

        // increments survive the codex.new_ship rebuild (plus columns NotSet)
        assert_eq!(plus_of(&context, ship.api_id).await, [None, Some(1), None, None, None]);

        // port synthesized value = new-mst maxeq + old increment
        let maxeq = crate::CODEX.manifest.find_ship(MUTSUKI_KAI_MST_ID).unwrap().api_maxeq.unwrap();
        let expected = std::array::from_fn::<i64, 5, _>(|i| {
            maxeq[i]
                + if i == 1 {
                    1
                } else {
                    0
                }
        });
        let found = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(found.api_ship_id, MUTSUKI_KAI_MST_ID);
        assert_eq!(found.api_onslot_max, Some(expected));
    }

    #[tokio::test]
    async fn marriage_resets_onslot_to_synthesized_capacity() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-marriage").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 1).await.unwrap();
        context.expand_hangar_slot(pid, ship.api_id, 0).await.unwrap();
        set_onslot_1(&context, ship.api_id, 5).await;

        context.add_use_item(pid, KcUseItemType::Ring as i64, 1).await.unwrap();
        let married = context.marriage(pid, ship.api_id).await.unwrap();

        // reset target = maxeq[0] + 1 = 19, not the raw maxeq 18
        assert_eq!(married.onslot_1, 19);
    }

    #[tokio::test]
    async fn double_expansion_stacks() {
        let context = new_context().await;
        let pid = new_profile(&context, "hangar-double").await;

        let ship = context.add_ship(pid, AKAGI_MST_ID).await.unwrap();
        context.add_use_item(pid, HANGAR_EXPAND_ITEM, 2).await.unwrap();

        let first = context.expand_hangar_slot(pid, ship.api_id, 2).await.unwrap();
        let second = context.expand_hangar_slot(pid, ship.api_id, 2).await.unwrap();

        assert_eq!(first, [18, 18, 27 + 1, 10, 0]);
        assert_eq!(second, [18, 18, 27 + 2, 10, 0]);
        assert_eq!(item_count(&context, pid).await, 0);
        assert_eq!(plus_of(&context, ship.api_id).await, [None, None, Some(2), None, None]);
    }
}
