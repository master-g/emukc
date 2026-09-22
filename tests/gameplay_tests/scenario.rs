//! Tests for the one-shot scenario / state builder (U3).

#[cfg(test)]
mod tests {
    use emukc_internal::crypto::rng;
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

    // -----------------------------------------------------------------------
    // Equipment-bearing presets (U6 / R5)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn declared_equipment_lands_on_the_ship() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;

        // 長門 has four slots; the fifth declaration has nowhere to go.
        let scenario = Scenario {
            fleet: vec![ShipSpec::new(80, 99).with_slots([7, 7, 12, 25, 25])],
            ..Default::default()
        };
        let ids = apply_scenario(&ctx, pid, &scenario).await.unwrap();

        let ship = ctx.find_ship(ids[0]).await.unwrap().unwrap();
        assert_eq!(ship.api_slotnum, 4);
        let equipped: Vec<i64> = ship.api_slot.iter().copied().filter(|id| *id > 0).collect();
        assert_eq!(equipped.len(), 4, "the fifth declaration is ignored: {:?}", ship.api_slot);
        assert_eq!(ship.api_slot[4], -1, "an undeclared slot stays at the -1 sentinel");

        let mut mst_ids = Vec::new();
        for item_id in equipped {
            mst_ids.push(ctx.find_slot_item(item_id).await.unwrap().api_slotitem_id);
        }
        assert_eq!(mst_ids, vec![7, 7, 12, 25]);
    }

    /// `ShipSpec::asw_mod` is the one modernisation lever a preset has: it is an
    /// input to `cal_ship_status` rather than a derived stat, so unlike
    /// `api_taisen[0]` it survives the recalculation every write path ends in.
    #[tokio::test]
    async fn declared_asw_modernisation_reaches_the_recalculated_stat() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;

        let plain = Scenario {
            fleet: vec![ShipSpec::new(629, 99)],
            ..Default::default()
        };
        let modernised = Scenario {
            fleet: vec![ShipSpec::new(629, 99).with_asw_mod(9)],
            ..Default::default()
        };
        let plain_ids = apply_scenario(&ctx, pid, &plain).await.unwrap();
        let modernised_ids = apply_scenario(&ctx, pid, &modernised).await.unwrap();

        let plain_asw = ctx.find_ship(plain_ids[0]).await.unwrap().unwrap().api_taisen[0];
        let modernised_ship = ctx.find_ship(modernised_ids[0]).await.unwrap().unwrap();
        assert_eq!(modernised_ship.api_kyouka[6], 9);
        assert_eq!(
            modernised_ship.api_taisen[0],
            plain_asw + 9,
            "modernisation is an input to the recalculation, not a value it overwrites"
        );
    }

    #[tokio::test]
    async fn opening_asw_preset_crosses_the_destroyer_threshold() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;
        let ids = apply_scenario(&ctx, pid, &Scenario::opening_asw()).await.unwrap();

        // Slot 0 is the routing carrier; the destroyers behind it are the ones
        // that have to clear the requirement of 100. Level and the sonar are
        // the only two levers a preset has, and both are needed.
        let ship = ctx.find_ship(ids[1]).await.unwrap().unwrap();
        assert!(
            ship.api_taisen[0] >= 100,
            "opening ASW needs ASW >= 100, got {}",
            ship.api_taisen[0]
        );
    }

    #[tokio::test]
    async fn hp_override_survives_the_equipment_recalculation() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;

        let scenario = Scenario {
            fleet: vec![ShipSpec::new(80, 99).with_slots([7, 7]).with_hp(9)],
            ..Default::default()
        };
        let ids = apply_scenario(&ctx, pid, &scenario).await.unwrap();

        let ship = ctx.find_ship(ids[0]).await.unwrap().unwrap();
        assert_eq!(ship.api_nowhp, 9, "equipping must not restore the declared damage");
    }

    #[tokio::test]
    async fn opening_asw_preset_fires_opening_anti_submarine() {
        let ctx = crate::TestContext::new().await;
        let pid = new_profile(&ctx).await;
        apply_scenario(&ctx, pid, &Scenario::opening_asw()).await.unwrap();

        // 4-3 routes any fleet containing a 正規空母 to the one cell off its
        // start point whose every composition is submarines, so the single
        // battle the gate simulates actually reaches the phase. Driven across
        // several seeds because that routing claim is the point of the preset:
        // a seed-dependent result would mean the routing is not what the
        // comment says it is.
        let baseline = ctx.get_ships(pid).await.unwrap();
        for seed in [1_u64, 2, 3] {
            ctx.clear_sortie_state_if_any(pid).await;
            for ship in &baseline {
                ctx.update_ship(ship).await.unwrap();
            }
            rng::seed(seed);

            ctx.start_sortie(pid, 1, 4, 3).await.expect("opening_asw should sortie 4-3");
            ctx.sortie_battle(pid, 1).await.expect("battle should resolve");

            let session = ctx.sortie_store().get_pending_battle(pid).expect("pending battle");
            assert_eq!(session.packet.opening_taisen_flag, 1, "seed {seed}: opening ASW must fire");
            let taisen = session.packet.opening_taisen.as_ref().expect("opening ASW payload");
            assert!(
                taisen.api_at_type.iter().all(|t| *t == 0),
                "seed {seed}: opening ASW reports attack type 0: {:?}",
                taisen.api_at_type
            );
        }
        rng::reseed_from_entropy();
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
