//! U5/U6: a 連合艦隊 sorties, fights the day battle and the night battle, and
//! settles — with both decks reported separately on the wire and both decks
//! written back to the database.
//!
//! The guards are in here too: a 警戒航行序列 the escort deck is too small for,
//! and an endpoint that does not serve the player's combined type. Both are
//! cheap to get wrong and silent when wrong — the client would render a packet
//! ordered for a different fleet shape.

#[cfg(test)]
mod tests {
    /// 第1艦隊 is deliberately shorter than six: the client reads 第2艦隊 from
    /// packet index 6 regardless, so a short 第1艦隊 is what exercises the gap.
    const MAIN_DECK: usize = 2;
    const ESCORT_DECK: usize = 4;
    /// A level-1 ship, so the settlement has experience to pay: an unmarried
    /// ship at level 99 earns none.
    const SHIP_MST: i64 = 1;

    fn slots(ship_ids: &[i64]) -> [i64; 6] {
        let mut slots = [-1; 6];
        for (slot, id) in slots.iter_mut().zip(ship_ids) {
            *slot = *id;
        }
        slots
    }

    /// A profile with both decks crewed and 空母機動部隊 formed.
    async fn combined_profile(context: &crate::TestContext, name: &str) -> (i64, Vec<i64>) {
        let account = context.sign_up(name, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, name).await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let pid = session.profile.id;

        context.unlock_fleet(pid, 2).await.unwrap();

        let mut main = Vec::new();
        for _ in 0..MAIN_DECK {
            main.push(context.add_ship(pid, SHIP_MST).await.unwrap().api_id);
        }
        let mut escort = Vec::new();
        for _ in 0..ESCORT_DECK {
            escort.push(context.add_ship(pid, SHIP_MST).await.unwrap().api_id);
        }
        context.update_fleet_ships(pid, 1, &slots(&main)).await.unwrap();
        context.update_fleet_ships(pid, 2, &slots(&escort)).await.unwrap();
        context.set_combined_type(pid, 1).await.unwrap();

        (pid, escort)
    }

    #[tokio::test]
    async fn combined_sortie_rejects_a_formation_the_escort_deck_is_too_small_for() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-formation").await;
        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        // 第三警戒航行序列 (13) needs five ships in fleet 2; this one has four.
        let err = context.sortie_combined_battle(pid, 13).await.unwrap_err();
        assert!(err.to_string().contains("at least 5"), "{err}");

        // 第四警戒航行序列 (14) needs four, which this fleet has.
        context.sortie_combined_battle(pid, 14).await.unwrap();
    }

    #[tokio::test]
    async fn combined_sortie_rejects_an_endpoint_that_does_not_serve_the_fleet_type() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-endpoint").await;
        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        // The fleet is 空母機動部隊, so 水上打撃's endpoint must turn it away —
        // its shelling order is the mirror image.
        let wrong = context.sortie_combined_battle_water(pid, 11).await.unwrap_err();
        assert!(wrong.to_string().contains("CombinedWater"), "{wrong}");

        // So must the single-fleet endpoint.
        let single = context.sortie_battle(pid, 11).await.unwrap_err();
        assert!(single.to_string().contains("Single"), "{single}");
    }

    /// A night-start cell is the same split as a night battle after a day one:
    /// 第2艦隊 fights alone and lands in the `_combined` arrays, 第1艦隊 is
    /// reported in the plain ones at the HP it entered with.
    #[tokio::test]
    async fn combined_sp_midnight_is_fought_by_the_escort_deck_alone() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-sp-midnight").await;
        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        let night = context.sortie_combined_sp_midnight_battle(pid, 1).await.unwrap();

        assert_eq!(night.api_f_nowhps.len(), MAIN_DECK, "api_f_nowhps is 第1艦隊 alone");
        assert_eq!(night.api_fParam.len(), MAIN_DECK);
        assert_eq!(night.api_f_nowhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert_eq!(night.api_f_maxhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert_eq!(night.api_fParam_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));

        if let Some(hougeki) = night.api_hougeki.as_ref() {
            for (eflag, attacker) in hougeki.api_at_eflag.iter().zip(hougeki.api_at_list.iter()) {
                if *eflag == 0 {
                    assert!(
                        *attacker >= 6,
                        "only 第2艦隊 fights, so every friendly attacker sits at packet \
                         index 6 or above: {attacker}"
                    );
                }
            }
        }

        // The node still settles over both decks.
        let result = context.sortie_battle_result(pid).await.unwrap();
        assert_eq!(result.api_get_ship_exp.len(), MAIN_DECK + 1);
        assert_eq!(result.api_get_ship_exp_combined.as_ref().map(Vec::len), Some(ESCORT_DECK + 1));
    }

    /// The single-fleet night-start endpoint must turn a combined fleet away:
    /// it would drop 第2艦隊 — the only deck that fights at night.
    #[tokio::test]
    async fn single_fleet_sp_midnight_rejects_a_combined_fleet() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-sp-guard").await;
        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        let err = context.sortie_sp_midnight_battle(pid, 1).await.unwrap_err();
        assert!(err.to_string().contains("Single"), "{err}");
    }

    #[tokio::test]
    async fn combined_day_battle_reports_each_deck_in_its_own_arrays() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-day").await;
        context.start_sortie(pid, 1, 1, 1).await.unwrap();

        let day = context.sortie_combined_battle(pid, 11).await.unwrap();

        assert_eq!(day.api_f_nowhps.len(), MAIN_DECK, "api_f_nowhps is 第1艦隊 alone");
        assert_eq!(day.api_f_maxhps.len(), MAIN_DECK);
        assert_eq!(day.api_fParam.len(), MAIN_DECK);
        assert_eq!(day.api_f_nowhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert_eq!(day.api_f_maxhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert_eq!(day.api_fParam_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
    }

    /// The aerial and long-distance cells report both decks the same way the
    /// shelling one does, and each runs only the phases its battle type has.
    #[tokio::test]
    async fn combined_air_and_long_distance_cells_report_both_decks() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-air").await;

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let air = context.sortie_combined_airbattle(pid, 11).await.unwrap();
        assert_eq!(air.api_f_nowhps.len(), MAIN_DECK);
        assert_eq!(air.api_f_nowhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert_eq!(air.api_hourai_flag[0], 0, "航空戦 runs no shelling round");
        assert_eq!(air.api_hourai_flag[3], 0, "航空戦 runs no torpedo phase");
        context.sortie_goback_port(pid).await.unwrap();

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let ld_air = context.sortie_combined_ld_airbattle(pid, 11).await.unwrap();
        assert_eq!(ld_air.api_f_nowhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert_eq!(ld_air.api_midnight_flag, 0, "長距離空襲戦 never goes to night battle");
        assert!(ld_air.api_opening_taisen.is_none(), "長距離空襲戦 runs no opening ASW");
        context.sortie_goback_port(pid).await.unwrap();

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        // 第四警戒航行序列 (14) is what the client sends on a レーダー射撃マス.
        let ld_shooting = context.sortie_combined_ld_shooting(pid, 14).await.unwrap();
        assert_eq!(ld_shooting.api_f_nowhps_combined.as_ref().map(Vec::len), Some(ESCORT_DECK));
        assert!(ld_shooting.api_kouku.is_none(), "レーダー射撃 runs no aerial phase");
        assert!(ld_shooting.api_raigeki.is_none(), "レーダー射撃 runs no torpedo phase");
        assert_eq!(ld_shooting.api_hourai_flag[3], 0);
    }

    /// Both decks reach the settlement: 第2艦隊 spends fuel and earns experience
    /// from its own `api_get_ship_exp_combined`, not 第1艦隊's array.
    #[tokio::test]
    async fn combined_battle_result_settles_both_decks() {
        let context = crate::TestContext::new().await;
        let (pid, escort) = combined_profile(&context, "combined-result").await;

        let before = context.find_ship(escort[0]).await.unwrap().unwrap();

        context.start_sortie(pid, 1, 1, 1).await.unwrap();
        context.sortie_combined_battle(pid, 11).await.unwrap();
        let result = context.sortie_battle_result(pid).await.unwrap();

        assert!(result.api_mvp >= 1, "第1艦隊 must name an MVP");
        let mvp_combined = result.api_mvp_combined.expect("第2艦隊 must name its own MVP");
        assert!(
            (1..=ESCORT_DECK as i64).contains(&mvp_combined),
            "第2艦隊's MVP is numbered within its own deck: {mvp_combined}"
        );
        let exp_combined =
            result.api_get_ship_exp_combined.as_ref().expect("第2艦隊 must have its own exp array");
        assert_eq!(
            exp_combined.len(),
            ESCORT_DECK + 1,
            "the array is a -1 dummy plus one entry per 第2艦隊 ship"
        );
        assert_eq!(result.api_get_ship_exp.len(), MAIN_DECK + 1);

        let after = context.find_ship(escort[0]).await.unwrap().unwrap();
        assert!(after.api_exp[0] > before.api_exp[0], "第2艦隊 must be paid experience");
        assert!(after.api_fuel < before.api_fuel, "第2艦隊 must spend fuel on the node");
    }

    /// AE5: the escort deck reaches the router as its own facts, and no rule
    /// reads them, so a combined fleet routes exactly as 第1艦隊 would alone.
    /// 1-1's first branch is a fleet-size-weighted roll; replaying each seed for
    /// both shapes makes any difference in the facts show up as a different cell.
    #[tokio::test]
    async fn a_combined_fleet_routes_as_its_main_deck_alone() {
        let context = crate::TestContext::new().await;
        let (pid, _) = combined_profile(&context, "combined-route").await;
        let route = async |combined_type: i64, seed: u64| {
            context.set_combined_type(pid, combined_type).await.unwrap();
            context.start_sortie(pid, 1, 1, 1).await.unwrap();
            emukc_internal::crypto::rng::seed(seed);
            let next = context.next_sortie(pid, None).await.unwrap().cell_no;
            emukc_internal::crypto::rng::reseed_from_entropy();
            context.sortie_goback_port(pid).await.unwrap();
            next
        };

        for seed in 0..16 {
            assert_eq!(route(1, seed).await, route(0, seed).await, "seed {seed}");
        }
    }
}
