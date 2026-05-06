//! Tests for sortie retreat (goback_port) behavior

#[cfg(test)]
mod tests {
	use emukc_internal::prelude::*;

	async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
		let db = new_mem_db().await.unwrap();
		let codex = Codex::load_without_cache_source(".data/codex").unwrap();
		(db, codex)
	}

	async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
		let account = context.sign_up("test-retreat", "1234567").await.unwrap();
		let profile =
			context.new_profile(&account.access_token.token, "retreat-tester").await.unwrap();
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
	async fn goback_port_after_starting_sortie_clears_state() {
		let context = new_context().await;
		let pid = new_profile(&context).await;
		setup_fleet(&context, pid).await;

		let _start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

		let result = context.sortie_goback_port(pid).await;
		assert!(result.is_ok(), "goback_port should succeed after starting sortie");

		// Second goback should fail — no active sortie
		let second = context.sortie_goback_port(pid).await;
		assert!(second.is_err(), "goback_port without active sortie should error");
	}

	#[tokio::test]
	async fn goback_port_after_battle_clears_state() {
		let context = new_context().await;
		let pid = new_profile(&context).await;
		setup_fleet(&context, pid).await;

		let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
		let mut current_cell = start.cell_no;
		let boss_cell = start.boss_cell_no;

		loop {
			let _battle = context.sortie_battle(pid, 1).await.unwrap();
			let _result = context.sortie_battle_result(pid).await.unwrap();

			if current_cell == boss_cell {
				break;
			}

			match context.next_sortie(pid, None).await {
				Ok(next) => current_cell = next.cell_no,
				Err(_) => break,
			}
		}

		// Goback should succeed (sortie may or may not still be active depending on boss)
		let _ = context.sortie_goback_port(pid).await;

		// After goback, no active sortie remains
		let second = context.sortie_goback_port(pid).await;
		assert!(second.is_err(), "goback_port without active sortie should error");
	}

	#[tokio::test]
	async fn goback_port_without_active_sortie_errors() {
		let context = new_context().await;
		let pid = new_profile(&context).await;

		let result = context.sortie_goback_port(pid).await;
		assert!(result.is_err(), "goback_port without active sortie should error");
	}
}
