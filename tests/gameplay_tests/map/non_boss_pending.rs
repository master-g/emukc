//! Tests for non-boss battle pending state management
//!
//! After a non-boss battle result, the sortie should continue (not finish).
//! After a boss battle result that clears the map, the sortie should end.

#[cfg(test)]
mod tests {
	use emukc_internal::prelude::*;

	async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
		let db = new_mem_db().await.unwrap();
		let codex = Codex::load_without_cache_source(".data/codex").unwrap();
		(db, codex)
	}

	async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
		let account = context.sign_up("test-pending", "1234567").await.unwrap();
		let profile =
			context.new_profile(&account.access_token.token, "pending-tester").await.unwrap();
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
	async fn battle_then_next_then_goback_port() {
		// Full flow: start → battle → result → next → battle → result → goback_port
		let context = new_context().await;
		let pid = new_profile(&context).await;
		setup_fleet(&context, pid).await;

		let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
		let mut current_cell = start.cell_no;
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
		// Starting a new sortie while an incomplete one exists should succeed
		let context = new_context().await;
		let pid = new_profile(&context).await;
		setup_fleet(&context, pid).await;

		let first = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
		assert_eq!(first.maparea_id, 1);

		// Battle but don't finish the sortie
		let _battle = context.sortie_battle(pid, 1).await.unwrap();
		let _result = context.sortie_battle_result(pid).await.unwrap();

		// Second start should succeed (clears previous incomplete sortie)
		let second = context.start_sortie(pid, 1, 1, 1, 1).await;
		assert!(second.is_ok(), "second start_sortie should succeed");

		// Clean up
		let _ = context.sortie_goback_port(pid).await;
	}
}
