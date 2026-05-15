//! Tests for non-boss battle pending state management
//!
//! After a non-boss battle result, the sortie should continue (not finish).
//! After a boss battle result that clears the map, the sortie should end.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_context() -> crate::TestContext {
        crate::TestContext::new().await
    }

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-pending", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "pending-tester").await.unwrap();
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

    #[tokio::test]
    async fn battle_then_next_then_goback_port() {
        // Full flow: start → battle → result → next → battle → result → goback_port
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
        let current_cell = start.cell_no;
        let boss_cell = start.boss_cell_no;

        // Battle at first cell
        let _battle = context.sortie_battle(pid, 1).await.unwrap();
        let result = context.sortie_battle_result(pid).await.unwrap();

        // If first cell is boss, map clears and sortie ends
        if current_cell == boss_cell {
            assert!(result.api_first_clear > 0 || result.api_win_rank != "D");
            return;
        }

        // Non-boss: try to advance
        match context.next_sortie(pid, None).await {
            Ok(_next) => {}
            Err(_) => {
                // Dead-end cell — sortie ends after non-boss battle
                let goback = context.sortie_goback_port(pid).await;
                assert!(goback.is_err(), "sortie should already be cleaned up after dead-end");
                return;
            }
        }

        // Second cell battle
        let _battle = context.sortie_battle(pid, 1).await.unwrap();
        let _result = context.sortie_battle_result(pid).await.unwrap();

        // Goback port to clean up
        let _ = context.sortie_goback_port(pid).await;

        // Verify cleanup
        let second_goback = context.sortie_goback_port(pid).await;
        assert!(second_goback.is_err(), "no active sortie should remain after goback");
    }

    #[tokio::test]
    async fn start_sortie_twice_clears_previous_state() {
        let context = new_context().await;
        let pid = new_profile(&context).await;
        setup_fleet(&context, pid).await;

        let first = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
        assert_eq!(first.maparea_id, 1);
        assert_eq!(first.mapinfo_no, 1);

        let _ = context.sortie_goback_port(pid).await;

        let second = context.start_sortie(pid, 1, 1, 1, 1).await;
        assert!(second.is_ok(), "second start_sortie should succeed");
        let second = second.unwrap();
        assert_eq!(second.maparea_id, 1);

        let records = context.get_map_records(pid).await.unwrap();
        let rec = records.iter().find(|r| r.map_id == 11);
        if let Some(rec) = rec {
            assert!(!rec.cleared, "map should not be cleared from interrupted sortie");
        }

        let _ = context.sortie_goback_port(pid).await;
    }
}
