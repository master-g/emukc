//! Tests that remodel preserves sally_area and ex-slot equipment.

#[cfg(test)]
mod tests {
	use emukc_internal::prelude::*;

	async fn new_context() -> (emukc_internal::db::sea_orm::DbConn, Codex) {
		let db = new_mem_db().await.unwrap();
		let codex = Codex::load_without_cache_source(".data/codex").unwrap();
		(db, codex)
	}

	async fn new_profile(context: &(emukc_internal::db::sea_orm::DbConn, Codex)) -> i64 {
		let account = context.sign_up("test-remodel-fields", "1234567").await.unwrap();
		let profile =
			context.new_profile(&account.access_token.token, "remodel-tester").await.unwrap();
		let session =
			context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
		session.profile.id
	}

	#[tokio::test]
	async fn remodel_preserves_sally_area_and_ex_slot() {
		let context = new_context().await;
		let pid = new_profile(&context).await;

		// 睦月 (mst_id=1) → 睦月改 (mst_id=254)
		let ship = context.add_ship(pid, 1).await.unwrap();

		// Set sally_area > 0
		let mut modified = context.find_ship(ship.api_id).await.unwrap().unwrap();
		let target_sally_area = 7;
		modified.api_sally_area = target_sally_area;
		context.update_ship(&modified).await.unwrap();

		// Add materials + reinforce expansion for ex-slot
		context
			.add_material(pid, &[(MaterialCategory::Ammo, 200), (MaterialCategory::Steel, 200)])
			.await
			.unwrap();

		// Open ex-slot (requires ReinforceExpansion use item — grant it first)
		context
			.add_use_item(pid, KcUseItemType::ReinforceExpansion as i64, 1)
			.await
			.unwrap();
		context.open_ship_exslot(pid, ship.api_id).await.unwrap();

		// Add an item and equip it to ex-slot
		let item = context.add_slot_item(pid, 25, 0, 0).await.unwrap(); // 25mm単装機銃
		context.set_exslot_item(ship.api_id, item.api_id).await.unwrap();

		// Verify preconditions
		let before = context.find_ship(ship.api_id).await.unwrap().unwrap();
		assert_eq!(before.api_sally_area, target_sally_area);
		assert_eq!(before.api_slot_ex, item.api_id);

		// Remodel
		context.remodel(pid, ship.api_id).await.unwrap();

		// Verify sally_area preserved
		let after = context.find_ship(ship.api_id).await.unwrap().unwrap();
		assert_eq!(
			after.api_sally_area, target_sally_area,
			"sally_area should be preserved across remodel"
		);

		// Verify ex-slot item still equipped
		assert_eq!(
			after.api_slot_ex, item.api_id,
			"ex-slot item ID should be preserved across remodel"
		);
	}
}
