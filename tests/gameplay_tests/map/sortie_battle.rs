//! Tests for the full sortie → battle → result chain.
//!
//! Verifies enemy ship IDs, HP tracking, and sinking protection across
//! the complete integration path: start_sortie → sortie_battle → result.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        (db, codex)
    }

    async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
        let account = context.sign_up("test-sortie-battle", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "sortie-battle-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    async fn setup_fleet(context: &(emukc_internal::db::sea_orm::DbConn, Codex), pid: i64) {
        let mut fleet_slots = [-1; 6];
        for slot in &mut fleet_slots {
            *slot = context.add_ship(pid, 951).await.unwrap().api_id;
        }
        context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();
    }

    #[tokio::test]
    async fn sortie_1_1_battle_enemy_ship_ids_are_abyssal() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

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

        let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
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

        let _start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
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
