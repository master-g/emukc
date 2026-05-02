//! Tests for remodel HP restoration and CT repair time modifier.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        (db, codex)
    }

    async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
        let account = context.sign_up("test-remodel-hp", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "remodel-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn ship_partial_hp_fully_healed_after_remodel() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add 睦月 (mst_id=1, HP 13) → remodels to 睦月改 (mst_id=254, HP 24)
        let ship = context.add_ship(pid, 1).await.unwrap();
        assert_eq!(ship.api_nowhp, 13, "base ship should start at full HP");

        // Damage the ship: set HP to 5
        let mut damaged = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(damaged.api_maxhp, 13);
        damaged.api_nowhp = 5;
        context.update_ship(&damaged).await.unwrap();

        // Add materials for remodel (100 ammo + 100 steel)
        context
            .add_material(pid, &[(MaterialCategory::Ammo, 200), (MaterialCategory::Steel, 200)])
            .await
            .unwrap();

        // Remodel
        context.remodel(pid, ship.api_id).await.unwrap();

        // Verify HP fully restored to new max
        let remodeled = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(
            remodeled.api_nowhp, remodeled.api_maxhp,
            "ship HP should be fully restored after remodel"
        );
        assert!(
            remodeled.api_maxhp >= 24,
            "remodeled ship max HP should be >= 24 (got {})",
            remodeled.api_maxhp
        );
    }

    #[tokio::test]
    async fn ship_full_hp_fully_healed_after_remodel_with_higher_max() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add 睦月 (mst_id=1, HP 13) at full HP
        let ship = context.add_ship(pid, 1).await.unwrap();
        assert_eq!(ship.api_nowhp, 13);
        assert_eq!(ship.api_maxhp, 13);

        // Add materials for remodel
        context
            .add_material(pid, &[(MaterialCategory::Ammo, 200), (MaterialCategory::Steel, 200)])
            .await
            .unwrap();

        // Remodel
        context.remodel(pid, ship.api_id).await.unwrap();

        // Verify HP = new max HP (higher than before)
        let remodeled = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(
            remodeled.api_nowhp, remodeled.api_maxhp,
            "ship at full HP before remodel should still have nowhp == maxhp after"
        );
        assert!(
            remodeled.api_maxhp > 13,
            "remodeled max HP should be higher than original (got {})",
            remodeled.api_maxhp
        );
    }

    #[test]
    fn ct_repair_time_uses_correct_modifier() {
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();

        let ct_mst = codex.find::<ApiMstShip>(&154).unwrap(); // 香取 (CT, stype=21)
        assert_eq!(ct_mst.api_stype, 21, "香取 should be stype 21 (CT)");

        let cl_mst = codex.find::<ApiMstShip>(&21).unwrap(); // 長良 (CL, stype=3)
        assert_eq!(cl_mst.api_stype, 3, "長良 should be stype 3 (CL)");

        let lv = 50;
        let hp_lost = 10;

        let ct_cost = codex.cal_ship_docking_cost(&ct_mst, lv, hp_lost).unwrap();
        let cl_cost = codex.cal_ship_docking_cost(&cl_mst, lv, hp_lost).unwrap();

        // CT and CL share the same modifier (1.0), so repair times should be proportional
        // only to time_base (same level) and time_factor. Since both use modifier 1.0
        // and same level, the ratio depends only on the formula's level component.
        assert_eq!(
            ct_cost.duration_sec, cl_cost.duration_sec,
            "CT and CL at same level with same HP lost should have equal repair time (both mod=1.0)"
        );
    }

    #[test]
    fn ct_repair_time_distinct_from_heavier_types() {
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();

        let ct_mst = codex.find::<ApiMstShip>(&154).unwrap(); // 香取 (CT, mod=1.0)
        let ca_mst = codex.find::<ApiMstShip>(&59).unwrap(); // 古鷹 (CA, mod=1.5)

        let lv = 50;
        let hp_lost = 10;

        let ct_cost = codex.cal_ship_docking_cost(&ct_mst, lv, hp_lost).unwrap();
        let ca_cost = codex.cal_ship_docking_cost(&ca_mst, lv, hp_lost).unwrap();

        assert!(
            ct_cost.duration_sec < ca_cost.duration_sec,
            "CT (mod=1.0) repair time ({}) should be less than CA (mod=1.5) repair time ({})",
            ct_cost.duration_sec,
            ca_cost.duration_sec
        );
    }
}
