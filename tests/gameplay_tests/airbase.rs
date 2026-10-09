//! Land-base squadrons and the equipment they hold.
//!
//! A piece of equipment is in use when a ship carries it or a squadron flies
//! it. Equipping, scrapping and the unequipped list all ask the same question,
//! and a squadron holds its plane until its relocation has run its time and
//! the port has settled it.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        db::{
            entity::profile::airbase::plane,
            sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel},
        },
        model::profile::airbase::{AirbaseAction, PlaneState},
        time::chrono::{Duration, Utc},
    };

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

    /// Pretend the squadron flying `slot_id` last recovered, or began to
    /// relocate, `minutes` ago, at `condition` if one is given.
    async fn age(context: &crate::TestContext, slot_id: i64, minutes: i64, condition: Option<i64>) {
        let row = plane::Entity::find_by_id(slot_id).one(&*context.db).await.unwrap().unwrap();
        let mut am = row.into_active_model();
        am.since = ActiveValue::Set(Some(Utc::now() - Duration::minutes(minutes)));
        if let Some(condition) = condition {
            am.condition = ActiveValue::Set(condition);
        }
        am.update(&*context.db).await.unwrap();
    }

    async fn condition(context: &crate::TestContext, slot_id: i64) -> i64 {
        plane::Entity::find_by_id(slot_id).one(&*context.db).await.unwrap().unwrap().condition
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

        // Twelve minutes, and then a visit to the port.
        age(&context, fighter, 11, None).await;
        assert_eq!(context.port_view(pid).await.unwrap().relocating_slots, vec![fighter]);
        age(&context, fighter, 12, None).await;
        let base = context.get_airbases(pid).await.unwrap().remove(0);
        assert_eq!(base.planes[0].slot_id, fighter, "reading the airbases settles nothing");
        let port = context.port_view(pid).await.unwrap();
        assert!(port.relocating_slots.is_empty());
        // The port that releases it hands back the list it rejoins, once.
        assert!(port.returned_unset_slots.values().any(|ids| ids.contains(&fighter)));
        assert!(context.port_view(pid).await.unwrap().returned_unset_slots.is_empty());

        let base = context.get_airbases(pid).await.unwrap().remove(0);
        assert_eq!(base.planes[0].slot_id, 0);
    }

    #[tokio::test]
    async fn a_higher_maintenance_level_shortens_the_relocation() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-level").await;
        let fighter = deployed_fighter(&context, pid).await;
        context.add_use_item(pid, 73, 3).await.unwrap();
        for _ in 0..3 {
            context.expand_airbase_maintenance(pid, AREA).await.unwrap();
        }

        context.set_airbase_plane(pid, AREA, 1, 1, -1).await.unwrap();
        age(&context, fighter, 6, None).await;
        assert!(context.port_view(pid).await.unwrap().relocating_slots.is_empty());
    }

    #[tokio::test]
    async fn a_squadron_put_over_another_sends_it_into_relocation() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-overwrite").await;
        let old = deployed_fighter(&context, pid).await;
        let new = context.add_slot_item(pid, FIGHTER, 0, 0).await.unwrap().api_id;

        let set = context.set_airbase_plane(pid, AREA, 1, 1, new).await.unwrap();
        assert_eq!(set.updated[0].slot_id, new, "the slot shows the squadron that flies");
        assert!(matches!(set.updated[0].state, PlaneState::Assigned));

        assert_eq!(context.port_view(pid).await.unwrap().relocating_slots, vec![old]);
        assert!(!unset_ids(&context, pid).await.contains(&old));
    }

    #[tokio::test]
    async fn a_relocating_squadron_cannot_change_airbase() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-relocating-move").await;
        context.unlock_airbase(pid, AREA, 2).await.unwrap();
        let old = deployed_fighter(&context, pid).await;
        let new = context.add_slot_item(pid, FIGHTER, 0, 0).await.unwrap().api_id;
        context.set_airbase_plane(pid, AREA, 1, 1, new).await.unwrap();

        assert!(context.change_deployment_base(pid, AREA, 2, 1, 1, old).await.is_err());
        let moved = context.change_deployment_base(pid, AREA, 2, 1, 1, new).await.unwrap();
        assert_eq!(moved[1].planes[0].slot_id, new);
    }

    #[tokio::test]
    async fn two_squadrons_of_an_airbase_trade_slots_as_they_are() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-trade").await;
        let first = deployed_fighter(&context, pid).await;
        let second = context.add_slot_item(pid, FIGHTER, 0, 0).await.unwrap().api_id;
        context.set_airbase_plane(pid, AREA, 1, 2, second).await.unwrap();
        age(&context, first, 0, Some(12)).await;

        let bauxite = context.get_materials(pid).await.unwrap().bauxite;
        let moved = context.set_airbase_plane(pid, AREA, 1, 2, first).await.unwrap();
        assert_eq!(
            moved.updated.iter().map(|p| p.slot_id).collect::<Vec<_>>(),
            vec![second, first]
        );
        assert_eq!(moved.after_bauxite, None);
        assert_eq!(context.get_materials(pid).await.unwrap().bauxite, bauxite);
        assert_eq!(condition(&context, first).await, 12, "a move rests nobody");
        assert!(context.port_view(pid).await.unwrap().relocating_slots.is_empty());
    }

    #[tokio::test]
    async fn a_squadron_recovers_with_time_at_the_rate_of_its_orders() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-recover").await;
        let fighter = deployed_fighter(&context, pid).await;

        // 待機 gives 4 a tick: red after two ticks is 8 + 8 = 16, still red.
        age(&context, fighter, 6, Some(8)).await;
        let base = context.get_airbases(pid).await.unwrap().remove(0);
        assert_eq!(condition(&context, fighter).await, 16);
        let wire: emukc_internal::model::kc2::KcApiPlaneInfo = base.planes[0].clone().into();
        assert_eq!(wire.api_cond, Some(3));

        // The ticks so far were earned under the old orders: three at 4, and
        // only what comes after the change at 出撃's 1.
        age(&context, fighter, 9, Some(16)).await;
        context.set_airbase_actions(pid, AREA, &[(1, AirbaseAction::Attack as i64)]).await.unwrap();
        assert_eq!(condition(&context, fighter).await, 28);
        age(&context, fighter, 3, None).await;
        let recovered =
            context.recover_airbase_condition_with_time(pid, AREA, 1).await.unwrap().updated;
        assert_eq!(condition(&context, fighter).await, 29);
        let wire: emukc_internal::model::kc2::KcApiPlaneInfo = recovered[0].clone().into();
        assert_eq!(wire.api_cond, Some(2));

        // Past 40 every tick is one, whatever the orders, and 46 is the top.
        age(&context, fighter, 60 * 24, Some(39)).await;
        context.get_airbases(pid).await.unwrap();
        assert_eq!(condition(&context, fighter).await, 46);
    }

    #[tokio::test]
    async fn a_sortie_tires_the_air_corps_it_sends() {
        use emukc_internal::gameplay::scenario::{Scenario, apply_scenario};

        let context = crate::TestContext::new().await;
        let account = context.sign_up("airbase-sortie", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "airbase-sortie").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        apply_scenario(&context, pid, &Scenario::air_corps_6_4()).await.unwrap();
        context.get_airbases(pid).await.unwrap();

        // 一式陸攻, which the scenario leaves spare.
        let attacker = context
            .get_unset_slot_items(pid)
            .await
            .unwrap()
            .iter()
            .find(|item| item.api_slotitem_id == 169)
            .unwrap()
            .api_id;
        context.set_airbase_plane(pid, AREA, 1, 1, attacker).await.unwrap();
        context.set_airbase_actions(pid, AREA, &[(1, AirbaseAction::Attack as i64)]).await.unwrap();
        assert_eq!(condition(&context, attacker).await, 40);

        context.start_sortie(pid, 1, AREA, 4).await.unwrap();
        // Any cell the air corps reaches will do; both strikes go to it.
        let mut sent = false;
        for cell in 1..40 {
            if context.start_air_base(pid, &[vec![cell, cell]]).await.is_ok() {
                sent = true;
                break;
            }
        }
        assert!(sent, "no cell of 6-4 is within reach");
        assert_eq!(condition(&context, attacker).await, 34, "集中 costs 6");
    }

    #[tokio::test]
    async fn a_ration_rests_a_tired_squadron() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-ration").await;
        let fighter = deployed_fighter(&context, pid).await;
        context.add_use_item(pid, 102, 1).await.unwrap();
        age(&context, fighter, 0, Some(5)).await;

        let rested = context.recover_airbase_condition(pid, AREA, 1).await.unwrap().updated;
        assert_eq!(condition(&context, fighter).await, 40);
        let wire: emukc_internal::model::kc2::KcApiPlaneInfo = rested[0].clone().into();
        assert_eq!(wire.api_cond, Some(1));
    }

    #[tokio::test]
    async fn a_deployed_fighter_cannot_board_a_ship() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-equip").await;
        let fighter = deployed_fighter(&context, pid).await;
        let ship = context.add_ship(pid, 951).await.unwrap();

        let err = context.set_slot_item(ship.api_id, 0, fighter).await.unwrap_err();
        assert!(err.to_string().contains("airbase"), "{err}");
        let err = context.set_exslot_item(ship.api_id, fighter).await.unwrap_err();
        assert!(err.to_string().contains("airbase"), "the ex-slot is no way round: {err}");
    }

    #[tokio::test]
    async fn a_deployed_fighter_is_scrapped_only_once_its_relocation_settles() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "airbase-scrap").await;
        let fighter = deployed_fighter(&context, pid).await;

        let err = context.destroy_items(pid, &[fighter]).await.unwrap_err();
        assert!(err.to_string().contains("airbase"), "{err}");

        // Released but still relocating: the squadron holds it until the port
        // has seen the relocation through.
        context.set_airbase_plane(pid, AREA, 1, 1, -1).await.unwrap();
        let err = context.destroy_items(pid, &[fighter]).await.unwrap_err();
        assert!(err.to_string().contains("airbase"), "{err}");

        age(&context, fighter, 12, None).await;
        context.port_view(pid).await.unwrap();
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
        assert!(!unset_ids(&context, pid).await.contains(&fighter), "relocating still holds it");

        age(&context, fighter, 12, None).await;
        context.port_view(pid).await.unwrap();
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
