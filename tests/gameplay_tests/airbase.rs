//! Land-base squadrons and the equipment they hold.
//!
//! A piece of equipment is in use when a ship carries it or a squadron flies
//! it. Equipping, scrapping and the unequipped list all ask the same question,
//! and a squadron that has finished relocating no longer holds its plane.

#[cfg(test)]
mod tests {
    use emukc_internal::model::profile::airbase::PlaneState;

    /// 零式艦戦21型: a carrier fighter a land base can also fly.
    const FIGHTER: i64 = 20;
    const AREA: i64 = 6;

    async fn profile(context: &crate::TestContext, name: &str) -> i64 {
        let account = context.sign_up(name, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, name).await.unwrap();
        let pid =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        context.unlock_airbase(pid.profile.id, AREA, 1).await.unwrap();
        pid.profile.id
    }

    /// A fighter flying in squadron 1 of airbase 6/1.
    async fn deployed_fighter(context: &crate::TestContext, pid: i64) -> i64 {
        let fighter = context.add_slot_item(pid, FIGHTER, 0, 0).await.unwrap().api_id;
        context.set_airbase_plane(pid, AREA, 1, 1, fighter).await.unwrap();
        fighter
    }

    async fn unset_ids(context: &crate::TestContext, pid: i64) -> Vec<i64> {
        context.get_unset_slot_items(pid).await.unwrap().iter().map(|i| i.api_id).collect()
    }

    #[tokio::test]
    async fn assigning_relocating_and_settling_a_squadron() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-cycle").await;
        let fighter = deployed_fighter(&context, pid).await;

        let base = context.get_airbases(pid).await.unwrap().remove(0);
        assert_eq!(base.planes[0].slot_id, fighter);
        assert!(matches!(base.planes[0].state, PlaneState::Assigned));

        let cleared = context.set_airbase_plane(pid, AREA, 1, 1, -1).await.unwrap();
        assert!(matches!(cleared.updated[0].state, PlaneState::Reassigning));

        let base = context.get_airbases(pid).await.unwrap().remove(0);
        assert_eq!(base.planes[0].slot_id, 0, "the next airbase read settles the relocation");
    }

    #[tokio::test]
    async fn a_deployed_fighter_cannot_board_a_ship() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-equip").await;
        let fighter = deployed_fighter(&context, pid).await;
        let ship = context.add_ship(pid, 951).await.unwrap();

        let err = context.set_slot_item(ship.api_id, 0, fighter).await.unwrap_err();
        assert!(err.to_string().contains("airbase"), "{err}");
    }

    #[tokio::test]
    async fn a_deployed_fighter_is_scrapped_only_once_its_relocation_settles() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-scrap").await;
        let fighter = deployed_fighter(&context, pid).await;

        let err = context.destroy_items(pid, &[fighter]).await.unwrap_err();
        assert!(err.to_string().contains("airbase"), "{err}");

        // Released but still relocating: nothing reads the airbases in between,
        // yet the scrap settles the relocation itself and goes through.
        context.set_airbase_plane(pid, AREA, 1, 1, -1).await.unwrap();
        context.destroy_items(pid, &[fighter]).await.unwrap();
        let ids: Vec<i64> =
            context.get_slot_items(pid).await.unwrap().iter().map(|i| i.api_id).collect();
        assert!(!ids.contains(&fighter));
    }

    #[tokio::test]
    async fn the_unequipped_list_hides_a_deployed_fighter_until_it_is_released() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-unset").await;
        let fighter = deployed_fighter(&context, pid).await;

        assert!(!unset_ids(&context, pid).await.contains(&fighter));

        context.set_airbase_plane(pid, AREA, 1, 1, -1).await.unwrap();
        assert!(unset_ids(&context, pid).await.contains(&fighter));
    }

    #[tokio::test]
    async fn equipment_on_a_ship_cannot_be_scrapped_but_goes_with_the_ship() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-ship-scrap").await;
        let gun = context.add_slot_item(pid, 2, 0, 0).await.unwrap().api_id;
        let ship = context.add_ship(pid, 951).await.unwrap();
        context.set_slot_item(ship.api_id, 0, gun).await.unwrap();

        let err = context.destroy_items(pid, &[gun]).await.unwrap_err();
        assert!(err.to_string().contains("ship"), "{err}");

        context.destroy_ship(pid, ship.api_id, false).await.unwrap();
        let ids: Vec<i64> =
            context.get_slot_items(pid).await.unwrap().iter().map(|i| i.api_id).collect();
        assert!(!ids.contains(&gun), "scrapping the ship scraps what it carried");
    }
}
