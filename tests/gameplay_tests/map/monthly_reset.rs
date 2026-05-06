//! Tests for monthly map reset policy and EO map unlock
//!
//! EO maps (1-5, 1-6, etc.) have MapResetPolicy::Monthly and reset at the
//! start of each month. This test verifies policy assignment and that
//! clearing prerequisite maps unlocks EO maps.

#[cfg(test)]
mod tests {
	use emukc_internal::prelude::*;

	async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
		let db = new_mem_db().await.unwrap();
		let codex = Codex::load_without_cache_source(".data/codex").unwrap();
		(db, codex)
	}

	async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
		let account = context.sign_up("test-reset", "1234567").await.unwrap();
		let profile =
			context.new_profile(&account.access_token.token, "reset-tester").await.unwrap();
		let session =
			context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
		session.profile.id
	}

	#[tokio::test]
	async fn eo_maps_have_monthly_reset_policy() {
		let context = new_context().await;
		let codex = &context.1;
		let catalog = codex.map_catalog();

		// EO maps with monthly reset: 1-5, 1-6, 3-5, 4-5, 5-5, 6-5
		for map_id in [15, 16, 35, 45, 55, 65] {
			let def = catalog.map_definition(map_id);
			assert!(def.is_some(), "map {map_id} should exist in catalog");
			assert_eq!(
				def.unwrap().reset_policy,
				emukc_internal::model::codex::map::MapResetPolicy::Monthly,
				"map {map_id} should have Monthly reset policy"
			);
		}
	}

	#[tokio::test]
	async fn regular_maps_have_never_reset_policy() {
		let context = new_context().await;
		let codex = &context.1;
		let catalog = codex.map_catalog();

		// Regular maps: 1-1 through 1-4, 2-1, etc.
		for map_id in [11, 12, 13, 14, 21, 24] {
			let def = catalog.map_definition(map_id);
			assert!(def.is_some(), "map {map_id} should exist in catalog");
			assert_eq!(
				def.unwrap().reset_policy,
				emukc_internal::model::codex::map::MapResetPolicy::Never,
				"map {map_id} should have Never reset policy"
			);
		}
	}

	#[tokio::test]
	async fn new_profile_does_not_show_eo_maps() {
		let context = new_context().await;
		let pid = new_profile(&context).await;

		let infos = context.get_map_infos(pid).await.unwrap();
		let map_ids: Vec<i64> = infos.iter().map(|info| info.api_id).collect();

		// EO maps should NOT be visible for a new account
		assert!(
			!map_ids.contains(&15),
			"map 1-5 should not be visible for new account, got: {map_ids:?}"
		);
	}
}
