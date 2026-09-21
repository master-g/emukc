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
}
