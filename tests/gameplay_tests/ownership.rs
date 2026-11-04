//! Operations that take a ship or dock id from the client act only on the
//! caller's own rows.

#[cfg(test)]
mod tests {
    use emukc_internal::db::{
        entity::profile::ndock,
        sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter},
    };
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext, username: &str) -> i64 {
        let account = context.sign_up(username, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, username).await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    fn assert_not_found<T: std::fmt::Debug>(name: &str, result: Result<T, GameplayError>) {
        assert!(
            matches!(result, Err(GameplayError::EntryNotFound(_))),
            "{name} on another profile's ship gave {result:?}"
        );
    }

    #[tokio::test]
    async fn another_profiles_ship_cannot_be_changed() {
        let context = crate::TestContext::new().await;
        let owner = new_profile(&context, "owner").await;
        let other = new_profile(&context, "other").await;

        // 睦月, which remodels and takes powerup
        let theirs = context.add_ship(owner, 1).await.unwrap();
        let gun = context.add_slot_item(owner, 2, 0, 0).await.unwrap();
        context.set_slot_item(owner, theirs.api_id, 0, gun.api_id).await.unwrap();
        let before = context.find_ship(owner, theirs.api_id).await.unwrap().unwrap();

        let mine = context.add_ship(other, 1).await.unwrap();
        context.add_use_item(other, KcUseItemType::ReinforceExpansion as i64, 1).await.unwrap();
        context.register_preset_slot(other, 1, mine.api_id).await.unwrap();

        let id = theirs.api_id;
        assert_not_found("destroy_ship", context.destroy_ship(other, id, true).await);
        assert_not_found("remodel", context.remodel(other, id).await);
        assert_not_found("marriage", context.marriage(other, id).await);
        assert_not_found("powerup target", context.powerup(other, id, &[mine.api_id], true).await);
        assert_not_found(
            "powerup material",
            context.powerup(other, mine.api_id, &[id], true).await,
        );
        assert_not_found("open_ship_exslot", context.open_ship_exslot(other, id).await);
        assert_not_found("register_preset_slot", context.register_preset_slot(other, 2, id).await);
        assert_not_found("apply_preset_slot", context.apply_preset_slot(other, 1, id, 1).await);
        for (from, to) in [(id, mine.api_id), (mine.api_id, id)] {
            let params = SlotDepriveParams {
                from_ship_id: from,
                to_ship_id: to,
                from_ex_slot: false,
                to_ex_slot: false,
                from_slot_idx: 0,
                to_slot_idx: 0,
            };
            assert_not_found("slot_deprive", context.slot_deprive(other, &params).await);
        }
        assert_not_found(
            "update_fleet_ships",
            context.update_fleet_ships(other, 1, &[mine.api_id, id, -1, -1, -1, -1]).await,
        );

        let after = context.find_ship(owner, id).await.unwrap().unwrap();
        assert_eq!(
            (after.api_ship_id, after.api_slot, after.api_kyouka, after.api_slot_ex),
            (before.api_ship_id, before.api_slot, before.api_kyouka, before.api_slot_ex)
        );
        let fleet = context.get_fleet(other, 1).await.unwrap();
        assert!(!fleet.ships.contains(&id), "fleet {:?}", fleet.ships);
    }

    async fn repairing(context: &crate::TestContext, pid: i64) -> Option<i64> {
        context.get_ndock(pid, 1).await.unwrap().context.map(|repair| repair.ship_id)
    }

    #[tokio::test]
    async fn a_repair_dock_number_names_the_callers_own_dock() {
        let context = crate::TestContext::new().await;
        let owner = new_profile(&context, "dock-owner").await;
        let other = new_profile(&context, "dock-other").await;

        let ship = context.add_ship(other, 1).await.unwrap();
        let mut damaged = context.find_ship(other, ship.api_id).await.unwrap().unwrap();
        damaged.api_nowhp = 1;
        context.update_ship(&damaged).await.unwrap();

        context.ndock_start_repair(other, 1, ship.api_id, false).await.unwrap();

        assert_eq!(repairing(&context, other).await, Some(ship.api_id));
        assert_eq!(repairing(&context, owner).await, None);
    }

    #[tokio::test]
    async fn slot_deprive_rejects_a_slot_index_outside_the_five_slots() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "deprive-range").await;
        let from = context.add_ship(pid, 1).await.unwrap();
        let to = context.add_ship(pid, 1).await.unwrap();

        for (from_slot_idx, to_slot_idx) in [(5, 0), (0, 5), (-1, 0)] {
            let params = SlotDepriveParams {
                from_ship_id: from.api_id,
                to_ship_id: to.api_id,
                from_ex_slot: false,
                to_ex_slot: false,
                from_slot_idx,
                to_slot_idx,
            };
            let result = context.slot_deprive(pid, &params).await;
            assert!(
                matches!(result, Err(GameplayError::WrongType(_))),
                "({from_slot_idx}, {to_slot_idx}) gave {result:?}"
            );
        }
    }

    async fn damaged_ship(context: &crate::TestContext, pid: i64) -> i64 {
        let ship = context.add_ship(pid, 1).await.unwrap();
        let mut damaged = context.find_ship(pid, ship.api_id).await.unwrap().unwrap();
        damaged.api_nowhp = 1;
        context.update_ship(&damaged).await.unwrap();
        ship.api_id
    }

    async fn finish_dock_now(context: &crate::TestContext, pid: i64, ship_id: i64) {
        let dock = ndock::Entity::find()
            .filter(ndock::Column::ProfileId.eq(pid))
            .filter(ndock::Column::Index.eq(1))
            .one(context.db())
            .await
            .unwrap()
            .unwrap();
        let mut am: ndock::ActiveModel = dock.into();
        am.status = ActiveValue::Set(ndock::Status::Busy);
        am.ship_id = ActiveValue::Set(ship_id);
        am.complete_time = ActiveValue::Set(Some(
            emukc_internal::time::chrono::Utc::now()
                - emukc_internal::time::chrono::Duration::seconds(1),
        ));
        am.update(context.db()).await.unwrap();
    }

    #[tokio::test]
    async fn a_finished_repair_frees_the_dock_when_its_ship_is_gone() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context, "dock-scrap").await;
        let ship_id = damaged_ship(&context, pid).await;

        context.ndock_start_repair(pid, 1, ship_id, false).await.unwrap();
        context.destroy_ship(pid, ship_id, true).await.unwrap();
        finish_dock_now(&context, pid, ship_id).await;

        let docks = context.get_ndocks(pid).await.unwrap();
        assert!(docks[0].context.is_none(), "dock 1 {:?}", docks[0]);
    }

    #[tokio::test]
    async fn a_dock_left_holding_another_profiles_ship_clears_and_heals_it() {
        let context = crate::TestContext::new().await;
        let owner = new_profile(&context, "dock-stale-a").await;
        let other = new_profile(&context, "dock-stale-b").await;
        let ship_id = damaged_ship(&context, other).await;

        finish_dock_now(&context, owner, ship_id).await;

        let docks = context.get_ndocks(owner).await.unwrap();
        assert!(docks[0].context.is_none(), "dock 1 {:?}", docks[0]);
        let ship = context.find_ship(other, ship_id).await.unwrap().unwrap();
        assert_eq!(ship.api_nowhp, ship.api_maxhp);
    }
}
