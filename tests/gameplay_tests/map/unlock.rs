//! Tests for map unlock progression system

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-unlock", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "unlock-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    /// Regular-sea maps (areas 1-7) are progression-gated; event maps
    /// (areas outside 1-7, e.g. an active event area) are unlocked by default
    /// while the codex carries them. Upstream added event area 62 in 6.3.x.
    fn regular_map_ids(ids: &[i64]) -> Vec<i64> {
        ids.iter().copied().filter(|id| (1..=7).contains(&(id / 10))).collect()
    }

    #[tokio::test]
    async fn new_profile_mapinfo_only_shows_map_1_1() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let infos = context.get_map_infos(pid).await.unwrap();
        let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();

        assert_eq!(
            regular_map_ids(&map_ids),
            vec![11],
            "expected only map 1-1 among regular-sea maps for new account, got: {map_ids:?}"
        );
        // Everything beyond area 1-7 must be an event map
        for id in &map_ids {
            assert!(id / 10 <= 7 || id / 10 >= 60, "unexpected map {id}");
        }
    }

    #[tokio::test]
    async fn clearing_1_1_unlocks_1_2() {
        // Verifies the public API path: after clearing 1-1 through the
        // sortie flow, get_map_infos should include newly unlocked maps.
        // The cascade logic itself is also tested in the crate-internal
        // test clearing_map_1_1_unlocks_dependents_via_cascade.
        //
        // Retries sorties until the boss is reached and defeated. Map 1-1
        // has a dead-end non-boss cell, so routing may end the sortie before
        // reaching the boss.

        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Initially only 1-1 visible among regular-sea maps
        let infos = context.get_map_infos(pid).await.unwrap();
        let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();
        assert_eq!(
            regular_map_ids(&map_ids),
            vec![11],
            "expected only map 1-1 among regular-sea maps for new account, got: {map_ids:?}"
        );

        // Set up a leveled fleet with stocked materials. Compass routing in
        // 1-1 sends most sorties to a dead-end cell, and sortie damage plus
        // fuel/ammo consumption accumulate across retries (no auto-repair in
        // the sortie flow), so the loop below restores the fleet before every
        // attempt and uses a budget that bounds all-dead-end streaks.
        let scenario = Scenario {
            fleet: vec![ShipSpec::new(951, 30); 6],
            materials: vec![(MaterialCategory::Fuel, 10000), (MaterialCategory::Ammo, 10000)],
            ..Default::default()
        };
        let ship_ids = apply_scenario(&context, pid, &scenario).await.unwrap();

        // Retry sorties until boss is defeated and map clears.
        // 30 attempts bounds all-dead-end routing streaks.
        let mut cleared = false;
        for _attempt in 0..30 {
            // Restore HP/fuel/ammo so retries start from a healthy fleet.
            for id in &ship_ids {
                let Some(mut ship) = context.find_ship(*id).await.unwrap() else {
                    continue;
                };
                let mst = context.codex().manifest.find_ship(ship.api_ship_id);
                ship.api_nowhp = ship.api_maxhp;
                if let Some(mst) = mst {
                    ship.api_fuel = mst.api_fuel_max.unwrap_or(ship.api_fuel);
                    ship.api_bull = mst.api_bull_max.unwrap_or(ship.api_bull);
                }
                context.update_ship(&ship).await.unwrap();
            }

            let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
            let mut current_cell = start.cell_no;
            let boss_cell = start.boss_cell_no;

            loop {
                let _battle = context.sortie_battle(pid, 1).await.unwrap();
                let result = context.sortie_battle_result(pid).await.unwrap();

                if current_cell == boss_cell || result.api_first_clear > 0 {
                    cleared = result.api_first_clear > 0;
                    break;
                }

                // Advance to next cell; if sortie ended (dead-end cell), start a new sortie
                match context.next_sortie(pid, None).await {
                    Ok(next) => current_cell = next.cell_no,
                    Err(_) => break, // sortie ended — dead-end or no more cells
                }
            }

            if cleared {
                break;
            }
        }
        assert!(cleared, "boss should be defeated within 30 sorties");

        // After clearing 1-1, mapinfo should include newly unlocked maps
        let infos = context.get_map_infos(pid).await.unwrap();
        let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();
        assert!(
            map_ids.contains(&12),
            "expected map 1-2 unlocked after clearing 1-1, got: {map_ids:?}"
        );
    }

    #[tokio::test]
    async fn start_sortie_after_incomplete_previous_sortie_succeeds() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();

        let first = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        assert_eq!(first.maparea_id, 1);

        let second = context.start_sortie(pid, 1, 1, 1).await;
        assert!(second.is_ok(), "second start_sortie should succeed after incomplete first");
    }

    #[tokio::test]
    async fn start_sortie_to_locked_map_fails() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add ships to fleet 1
        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();

        // Attempt to sortie to 2-1 (locked for new account)
        let result = context.start_sortie(pid, 1, 2, 1).await;

        assert!(result.is_err(), "sortie to locked map 2-1 should fail");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("locked"), "error should mention locked, got: {msg}",);
    }
}
