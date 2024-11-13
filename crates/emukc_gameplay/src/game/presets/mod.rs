use async_trait::async_trait;
use deck::{
	apply_preset_deck_impl, delete_preset_deck_impl, expand_preset_deck_capacity_impl,
	find_preset_deck_impl, get_preset_decks_impl, register_preset_deck_impl,
};
use emukc_db::{
	entity::profile::{
		fleet,
		preset::{
			preset_caps, preset_deck,
			preset_slot::{self},
		},
	},
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::profile::{
	preset_deck::{PresetDeck, PresetDeckItem},
	preset_slot::PresetSlot,
};
use slot::{
	apply_preset_slot_impl, delete_preset_slot_impl, expand_preset_slot_capacity_impl,
	get_preset_slots_impl, register_preset_slot_impl, toggle_preset_slot_ex_flag_impl,
	toggle_preset_slot_locked_impl, update_preset_slot_name_impl,
};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	fleet::{get_fleets_impl, update_fleet_ships_impl},
	slot_item::get_unset_slot_items_impl,
	use_item::deduct_use_item_impl,
};

pub(crate) mod deck;
pub(crate) mod slot;

#[async_trait]
pub trait PresetOps {
	/// Get preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_preset_decks(&self, profile_id: i64) -> Result<PresetDeck, GameplayError>;

	/// Find preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn find_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<PresetDeckItem, GameplayError>;

	/// Get preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_preset_slots(&self, profile_id: i64) -> Result<PresetSlot, GameplayError>;

	/// Register preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset`: The preset deck item.
	async fn register_preset_deck(
		&self,
		profile_id: i64,
		preset: &PresetDeckItem,
	) -> Result<preset_deck::Model, GameplayError>;

	/// Register preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	/// - `ship_id`: The ship ID.
	async fn register_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
	) -> Result<preset_slot::Model, GameplayError>;

	/// Delete preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn delete_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Expand preset deck capacity
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn expand_preset_deck_capacity(&self, profile_id: i64) -> Result<(), GameplayError>;

	/// Expand preset slot capacity
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn expand_preset_slot_capacity(&self, profile_id: i64) -> Result<i64, GameplayError>;

	/// Apply preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `deck_id`: The deck ID.
	/// - `preset_no`: The preset number.
	async fn apply_preset_deck(
		&self,
		profile_id: i64,
		deck_id: i64,
		preset_no: i64,
	) -> Result<fleet::Model, GameplayError>;

	/// Apply preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	/// - `ship_id`: The ship ID.
	/// - `mode`: The mode.
	async fn apply_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
		mode: i64,
	) -> Result<i64, GameplayError>;

	/// Delete preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn delete_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Toggle preset slot ex flag
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn toggle_preset_slot_ex_flag(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Toggle preset slot locked
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	async fn toggle_preset_slot_locked(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError>;

	/// Update preset slot name
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `preset_no`: The preset number.
	/// - `name`: The new name.
	async fn update_preset_slot_name(
		&self,
		profile_id: i64,
		preset_no: i64,
		name: &str,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PresetOps for T {
	async fn get_preset_decks(&self, profile_id: i64) -> Result<PresetDeck, GameplayError> {
		let db = self.db();

		let (caps, decks) = get_preset_decks_impl(db, profile_id).await?;

		Ok(PresetDeck {
			max_num: caps.deck_limit,
			records: decks.into_iter().map(Into::into).collect(),
		})
	}

	async fn find_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<PresetDeckItem, GameplayError> {
		let db = self.db();

		let model = find_preset_deck_impl(db, profile_id, preset_no).await?;

		Ok(model.into())
	}

	async fn get_preset_slots(&self, profile_id: i64) -> Result<PresetSlot, GameplayError> {
		let db = self.db();

		let (caps, slots) = get_preset_slots_impl(db, profile_id).await?;

		Ok(PresetSlot {
			profile_id,
			max_num: caps.slot_limit,
			records: slots.into_iter().map(Into::into).collect(),
		})
	}

	async fn register_preset_deck(
		&self,
		profile_id: i64,
		preset: &PresetDeckItem,
	) -> Result<preset_deck::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = register_preset_deck_impl(&tx, profile_id, preset).await?;

		tx.commit().await?;

		Ok(m)
	}

	async fn register_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
	) -> Result<preset_slot::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = register_preset_slot_impl(&tx, profile_id, preset_no, ship_id).await?;

		tx.commit().await?;

		Ok(m)
	}

	async fn delete_preset_deck(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		delete_preset_deck_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn expand_preset_deck_capacity(&self, profile_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		expand_preset_deck_capacity_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn expand_preset_slot_capacity(&self, profile_id: i64) -> Result<i64, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let new_cap = expand_preset_slot_capacity_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(new_cap)
	}

	async fn apply_preset_deck(
		&self,
		profile_id: i64,
		deck_id: i64,
		preset_no: i64,
	) -> Result<fleet::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = apply_preset_deck_impl(&tx, profile_id, deck_id, preset_no).await?;

		tx.commit().await?;

		Ok(m)
	}

	async fn apply_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
		ship_id: i64,
		mode: i64,
	) -> Result<i64, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let bauxite =
			apply_preset_slot_impl(&tx, codex, profile_id, preset_no, ship_id, mode).await?;

		tx.commit().await?;

		Ok(bauxite)
	}

	async fn delete_preset_slot(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		delete_preset_slot_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn toggle_preset_slot_ex_flag(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		toggle_preset_slot_ex_flag_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn toggle_preset_slot_locked(
		&self,
		profile_id: i64,
		preset_no: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		toggle_preset_slot_locked_impl(&tx, profile_id, preset_no).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn update_preset_slot_name(
		&self,
		profile_id: i64,
		preset_no: i64,
		name: &str,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_preset_slot_name_impl(&tx, profile_id, preset_no, name).await?;

		tx.commit().await?;

		Ok(())
	}
}

pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let caps_am: preset_caps::ActiveModel = preset_caps::ActiveModel {
		id: ActiveValue::set(profile_id),
		deck_limit: ActiveValue::set(3),
		slot_limit: ActiveValue::set(4),
	};

	caps_am.insert(c).await?;
	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	preset_caps::Entity::delete_by_id(profile_id).exec(c).await?;
	preset_deck::Entity::delete_many()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	preset_slot::Entity::delete_many()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
