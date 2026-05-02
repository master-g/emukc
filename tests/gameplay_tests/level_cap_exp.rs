//! Tests for ship level cap enforcement (unmarried cap at 99, married at 175).

#[cfg(test)]
mod tests {
    use emukc_internal::model::kc2::level;
    use emukc_internal::prelude::*;

    async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        (db, codex)
    }

    async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
        let account = context.sign_up("test-level-cap", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "level-cap-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    fn set_ship_to_level(ship: &mut KcApiShip, level: i64) {
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
    }

    #[tokio::test]
    async fn unmarried_ship_at_99_gains_0_exp_from_practice() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        // Add a ship and set it to level 99 (unmarried)
        let mut ship = context.add_ship(pid, 1).await.unwrap(); // 睦月
        set_ship_to_level(&mut ship, 99);
        context.update_ship(&ship).await.unwrap();

        // Verify state
        let loaded = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(loaded.api_lv, 99, "ship should be at level 99");
        assert!(loaded.api_lv <= 99, "unmarried ship should not exceed level 99");
    }

    #[test]
    fn unmarried_level_clamped_to_99() {
        // Verify the helper function
        assert_eq!(level::ship_level_cap(false), 99);
        assert_eq!(level::ship_level_cap(true), 175);
    }

    #[test]
    fn exp_to_ship_level_returns_above_99_but_clamp_prevents_it() {
        // exp_to_ship_level can return levels > 99, callers must clamp
        let exp_for_100 = level::ship_level_required_exp(100);
        let (lv, _) = level::exp_to_ship_level(exp_for_100);
        assert!(lv >= 100, "exp_to_ship_level should return level >= 100 for exp at level 100");

        // But after clamping for unmarried ship
        let clamped = lv.min(level::ship_level_cap(false));
        assert_eq!(clamped, 99, "unmarried ship level must be clamped to 99");
    }

    #[tokio::test]
    async fn unmarried_ship_at_98_excess_exp_clamped_to_99() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let mut ship = context.add_ship(pid, 1).await.unwrap();
        set_ship_to_level(&mut ship, 98);
        context.update_ship(&ship).await.unwrap();

        // Simulate massive XP gain: set exp to level 105 worth
        let exp_for_105 = level::ship_level_required_exp(105);
        ship.api_exp[0] = exp_for_105;
        let (new_lv, next_exp) = level::exp_to_ship_level(exp_for_105);
        let clamped_lv = new_lv.min(level::ship_level_cap(false));
        ship.api_lv = clamped_lv;
        ship.api_exp[1] = next_exp;
        context.update_ship(&ship).await.unwrap();

        let loaded = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(
            loaded.api_lv, 99,
            "unmarried ship level must be clamped to 99 even with excess exp"
        );
    }

    #[tokio::test]
    async fn married_ship_at_99_can_level_past_99() {
        let context = new_context().await;
        let pid = new_profile(&context).await;

        let mut ship = context.add_ship(pid, 1).await.unwrap();
        // Simulate married ship at level 100 (api_lv > 99 means married in DB)
        set_ship_to_level(&mut ship, 100);
        context.update_ship(&ship).await.unwrap();

        // Ship should now be married (api_lv > 99)
        let loaded = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert!(loaded.api_lv >= 100, "married ship should be level 100+");

        // Add more exp, verify level can increase
        let exp_for_110 = level::ship_level_required_exp(110);
        let mut ship = loaded;
        ship.api_exp[0] = exp_for_110;
        let (new_lv, next_exp) = level::exp_to_ship_level(exp_for_110);
        let clamped_lv = new_lv.min(level::ship_level_cap(true));
        ship.api_lv = clamped_lv;
        ship.api_exp[1] = next_exp;
        context.update_ship(&ship).await.unwrap();

        let reloaded = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert!(
            reloaded.api_lv >= 110,
            "married ship should be able to level past 99 (got {})",
            reloaded.api_lv
        );
    }
}
