//! Tests for the one-shot scenario / state builder (U3).

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-scenario", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "scenario-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn builder_produces_exact_state_without_grinding() {
        // Covers AE4: fleet at requested levels, full materials, a damaged
        // flagship — no PvP, real sortie, or repair performed.
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;

        let scenario = Scenario {
            fleet: vec![
                ShipSpec::new(951, 20).with_hp(5), // damaged flagship
                ShipSpec::new(951, 15),
            ],
            materials: vec![(MaterialCategory::Fuel, 5000), (MaterialCategory::Ammo, 5000)],
            ..Default::default()
        };
        let ids = apply_scenario(&ctx, pid, &scenario).await.unwrap();
        assert_eq!(ids.len(), 2);

        let flag = ctx.find_ship(ids[0]).await.unwrap().unwrap();
        assert_eq!(flag.api_lv, 20, "flagship level");
        assert_eq!(flag.api_nowhp, 5, "damaged flagship hp override persists");

        let second = ctx.find_ship(ids[1]).await.unwrap().unwrap();
        assert_eq!(second.api_lv, 15, "second ship level");

        let mat = ctx.get_materials(pid).await.unwrap();
        assert!(mat.fuel >= 5000, "fuel seeded: {}", mat.fuel);
        assert!(mat.ammo >= 5000, "ammo seeded: {}", mat.ammo);
    }

    #[tokio::test]
    async fn fresh_1_1_preset_can_sortie_1_1() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;
        apply_scenario(&ctx, pid, &Scenario::fresh_1_1()).await.unwrap();

        ctx.start_sortie(pid, 1, 1, 1).await.expect("fresh_1_1 fleet should sortie 1-1");
    }

    #[tokio::test]
    async fn hp_override_below_max_persists_through_find_ship() {
        // Mirrors the ammo-survival guard: an override below max survives the
        // add_ship → update_ship → find_ship round trip.
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;
        let scenario = Scenario {
            fleet: vec![ShipSpec::new(951, 1).with_hp(7)],
            ..Default::default()
        };
        let ids = apply_scenario(&ctx, pid, &scenario).await.unwrap();

        let ship = ctx.find_ship(ids[0]).await.unwrap().unwrap();
        assert!(ship.api_nowhp < ship.api_maxhp, "override must be below max");
        assert_eq!(ship.api_nowhp, 7);
    }

    #[tokio::test]
    async fn clearing_prerequisite_unlocks_dependent_for_sortie() {
        // Clearing 1-4 (map 14) cascades the unlock to 2-1 (map 21). Assert via
        // start_sortie (which gates on `unlocked`), not just get_map_infos.
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;

        // Fleet only — 2-1 is still locked on a fresh profile.
        apply_scenario(
            &ctx,
            pid,
            &Scenario {
                fleet: vec![ShipSpec::new(951, 1); 2],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(
            ctx.start_sortie(pid, 1, 2, 1).await.is_err(),
            "2-1 must be locked before clearing 1-4"
        );

        // Clear 1-4 → cascade unlock 2-1.
        apply_scenario(
            &ctx,
            pid,
            &Scenario {
                clear_maps: vec![14],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        ctx.start_sortie(pid, 1, 2, 1)
            .await
            .expect("2-1 should be sortie-able after clearing its prerequisite 1-4");
    }

    #[tokio::test]
    async fn leveled_for_mid_boss_preset_reaches_2_1_end_to_end() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;
        apply_scenario(&ctx, pid, &Scenario::leveled_for_mid_boss()).await.unwrap();

        let ships = ctx.get_ships(pid).await.unwrap();
        assert!(ships.iter().any(|s| s.api_lv == 30), "fleet leveled to 30");

        ctx.start_sortie(pid, 1, 2, 1)
            .await
            .expect("leveled_for_mid_boss should reach the 2-1 mid-boss area end-to-end");
    }
}
