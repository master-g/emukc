//! Tests for the full sortie → battle → result chain.
//!
//! Verifies enemy ship IDs, HP tracking, and sinking protection across
//! the complete integration path: `start_sortie` → `sortie_battle` → result.

#[cfg(test)]
mod tests {

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-sortie-battle", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "sortie-battle-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn setup_fleet(context: &crate::TestContext, pid: i64) {
        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();
    }

    /// A smoke screen once laid is spent for the sortie (wikiwiki 発煙装置: 1出撃につき1回):
    /// `next` tells the client to offer it no more, and a second order lays none.
    #[tokio::test]
    async fn a_smoke_screen_is_laid_once_a_sortie() {
        use emukc_internal::gameplay::scenario::{Scenario, apply_scenario};
        use emukc_internal::prelude::SortieOrder;

        let context = new_context().await;
        let pid = new_profile(&context).await;
        apply_scenario(&context, pid, &Scenario::smoke_screen()).await.unwrap();
        let baseline = context.get_ships(pid).await.unwrap();
        let smoke = SortieOrder {
            formation_id: 1,
            smoke: true,
        };

        let mut second_battles = 0;
        for seed in 1..=20 {
            for ship in &baseline {
                context.update_ship(ship).await.unwrap();
            }
            emukc_internal::crypto::rng::seed(seed);
            let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
            assert_eq!(start.limit_state, 0);
            let first = context.sortie_battle(pid, smoke).await.unwrap();
            assert!((1..=3).contains(&first.api_smoke_type), "seed {seed}: never fails at n = 3");
            context.sortie_battle_result(pid).await.unwrap();

            let next = context.next_sortie(pid, None).await.unwrap();
            assert_eq!(next.limit_state, Some(1), "seed {seed}");
            if matches!(next.event_id, 4 | 5) {
                let second = context.sortie_battle(pid, smoke).await.unwrap();
                assert_eq!(second.api_smoke_type, 0, "seed {seed}");
                context.sortie_battle_result(pid).await.unwrap();
                second_battles += 1;
            }
            context.sortie_goback_port(pid).await.ok();
        }
        assert!(second_battles > 0, "no seed reached a second battle");

        // Not asked, none is laid, and the next step still offers it.
        for ship in &baseline {
            context.update_ship(ship).await.unwrap();
        }
        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let plain = context.sortie_battle(pid, 1).await.unwrap();
        assert_eq!(plain.api_smoke_type, 0);
        context.sortie_battle_result(pid).await.unwrap();
        assert_eq!(context.next_sortie(pid, None).await.unwrap().limit_state, Some(0));
    }

    /// 1-5's first cell fields submarines alone: 8% of the fuel and no ammunition
    /// (wikiwiki 資材「燃料・弾薬の消費」), where an ordinary battle takes a fifth of each.
    #[tokio::test]
    async fn a_submarine_cell_costs_a_little_fuel_and_no_ammunition() {
        use emukc_internal::gameplay::scenario::{Scenario, apply_scenario};

        let context = new_context().await;
        let pid = new_profile(&context).await;
        // Six ships at full supply, with 1-1 to 1-4 cleared.
        apply_scenario(&context, pid, &Scenario::opening_asw()).await.unwrap();
        let supply = |ships: Vec<emukc_internal::prelude::KcApiShip>| {
            ships.into_iter().map(|ship| (ship.api_id, (ship.api_fuel, ship.api_bull)))
        };
        let before = supply(context.get_ships(pid).await.unwrap()).collect::<Vec<_>>();

        context.start_sortie(pid, 1, 1, 5).await.unwrap();
        let battle = context.sortie_battle(pid, 1).await.unwrap();
        context.sortie_battle_result(pid).await.unwrap();

        let fought = battle.api_f_nowhps.len();
        let after = supply(context.get_ships(pid).await.unwrap())
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut charged = 0;
        for (id, (fuel, ammo)) in before {
            let (fuel_left, ammo_left) = after[&id];
            if fuel_left == fuel {
                continue;
            }
            charged += 1;
            assert_eq!(fuel - fuel_left, (fuel * 8 / 100).max(1), "ship {id}'s fuel");
            assert_eq!(ammo_left, ammo, "ship {id}'s ammunition");
        }
        assert_eq!(charged, fought, "every ship that fought paid for it");
    }

    #[tokio::test]
    async fn sortie_1_1_battle_enemy_ship_ids_are_abyssal() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        let battle = context.sortie_battle(pid, 1).await.unwrap();

        // All enemy ship IDs must be abyssal (>= 1500) not friendly kanmusu
        for (i, &ship_id) in battle.api_ship_ke.iter().enumerate() {
            assert!(
                ship_id >= 1500,
                "enemy ship [{i}] ID {ship_id} must be abyssal (>= 1500), not a friendly kanmusu"
            );
        }
    }

    #[tokio::test]
    async fn sortie_1_1_battle_friendly_nowhps_persisted_after_result() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let battle = context.sortie_battle(pid, 1).await.unwrap();
        let _result = context.sortie_battle_result(pid).await.unwrap();

        // Verify HP tracking: battle response has valid HP arrays
        assert!(!battle.api_f_nowhps.is_empty(), "battle response should have friendly HP array");
        // All friendly ships should have positive HP (sinking protection)
        for (i, &hp) in battle.api_f_nowhps.iter().enumerate() {
            assert!(hp > 0, "friendly ship [{i}] HP must be > 0 after battle, got {hp}");
        }
    }

    #[tokio::test]
    async fn sortie_1_1_no_friendly_ship_sunk_during_sortie() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        let _start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let _battle = context.sortie_battle(pid, 1).await.unwrap();
        let _result = context.sortie_battle_result(pid).await.unwrap();

        // No friendly ship should be sunk (HP > 0 for all)
        let ships = context.get_ships(pid).await.unwrap();
        for ship in &ships {
            assert!(
                ship.api_nowhp > 0,
                "friendly ship {} (ID {}) must not be sunk — HP = {}",
                ship.api_id,
                ship.api_ship_id,
                ship.api_nowhp
            );
        }
    }
}
