//! Remodels charge what the client shows: start2 plus the client's own tables (#180).

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    /// 新型高温高圧缶
    const BOILER: i64 = 87;

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-remodel-reqs", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "remodel-tester").await.unwrap();
        context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id
    }

    async fn stock_up(context: &crate::TestContext, pid: i64) {
        context
            .add_material(
                pid,
                &[
                    (MaterialCategory::Ammo, 20000),
                    (MaterialCategory::Steel, 20000),
                    (MaterialCategory::DevMat, 100),
                    (MaterialCategory::Torch, 100),
                ],
            )
            .await
            .unwrap();
    }

    async fn use_item(context: &crate::TestContext, pid: i64, item: KcUseItemType) -> i64 {
        context.find_use_item(pid, item as i64).await.map_or(0, |item| item.api_count)
    }

    async fn mst_of(context: &crate::TestContext, ship_id: i64) -> i64 {
        context.find_ship(ship_id).await.unwrap().unwrap().api_ship_id
    }

    /// kcwiki writes "needs one" as `true`, which used to become 0.
    #[tokio::test]
    async fn bismarck_zwei_needs_a_blueprint() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        stock_up(&context, pid).await;
        let ship = context.add_ship(pid, 172).await.unwrap(); // Bismarck改

        // No blueprint at all: the deduction finds no row to take it from.
        assert!(context.remodel(pid, ship.api_id).await.is_err());
        assert_eq!(mst_of(&context, ship.api_id).await, 172);

        context.add_use_item(pid, KcUseItemType::Blueprint as i64, 1).await.unwrap();
        context.remodel(pid, ship.api_id).await.unwrap();
        assert_eq!(mst_of(&context, ship.api_id).await, 173); // Bismarck zwei
        assert_eq!(use_item(&context, pid, KcUseItemType::Blueprint).await, 0);
    }

    /// The client counts 新型高温高圧缶 only; a turbine of the same equipment type stays.
    #[tokio::test]
    async fn yamato_kai_ni_takes_boilers_but_not_turbines() {
        const TURBINE: i64 = 33;
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        stock_up(&context, pid).await;
        context.add_use_item(pid, KcUseItemType::Blueprint as i64, 3).await.unwrap();
        context.add_use_item(pid, KcUseItemType::ActionReport as i64, 1).await.unwrap();
        context.add_use_item(pid, KcUseItemType::NewArtilleryMaterial as i64, 3).await.unwrap();
        for item in [TURBINE, TURBINE, BOILER, BOILER] {
            context.add_slot_item(pid, item, 0, 0).await.unwrap();
        }
        let ship = context.add_ship(pid, 136).await.unwrap(); // 大和改

        context.remodel(pid, ship.api_id).await.unwrap();
        assert_eq!(mst_of(&context, ship.api_id).await, 911); // 大和改二
        let items = context.get_slot_items(pid).await.unwrap();
        let count = |id: i64| items.iter().filter(|item| item.api_slotitem_id == id).count();
        assert_eq!((count(BOILER), count(TURBINE)), (0, 2));
        // 911 needs 新型砲熕兵装資材 only when it comes from 136.
        assert_eq!(use_item(&context, pid, KcUseItemType::NewArtilleryMaterial).await, 0);
    }

    /// A reverse conversion used to charge the target's own forward remodel.
    #[tokio::test]
    async fn yamato_back_to_kai_ni_takes_no_blueprint_or_boiler() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;
        stock_up(&context, pid).await;
        context.add_use_item(pid, KcUseItemType::Blueprint as i64, 3).await.unwrap();
        for _ in 0..2 {
            context.add_slot_item(pid, BOILER, 0, 0).await.unwrap();
        }
        let ship = context.add_ship(pid, 916).await.unwrap(); // 大和改二重

        context.remodel(pid, ship.api_id).await.unwrap();
        assert_eq!(mst_of(&context, ship.api_id).await, 911); // 大和改二
        assert_eq!(use_item(&context, pid, KcUseItemType::Blueprint).await, 3);
        let boilers = context
            .get_slot_items(pid)
            .await
            .unwrap()
            .iter()
            .filter(|item| item.api_slotitem_id == BOILER)
            .count();
        assert_eq!(boilers, 2);
    }
}
