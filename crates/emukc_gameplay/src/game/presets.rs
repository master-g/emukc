use async_trait::async_trait;
use emukc_db::{
	entity::profile::preset::{preset_caps, preset_deck, preset_slot},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait},
};
use emukc_model::{
	kc2::KcUseItemType,
	profile::{
		preset_deck::{PresetDeck, PresetDeckItem},
		preset_slot::PresetSlot,
	},
};

use crate::{err::GameplayError, gameplay::HasContext};

use super::use_item::deduct_use_item_impl;

/// A trait for preset related gameplay.
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

		let am = register_preset_deck_impl(&tx, profile_id, preset).await?;

		tx.commit().await?;

		Ok(am)
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
}

pub(crate) async fn get_preset_decks_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(preset_caps::Model, Vec<preset_deck::Model>), GameplayError>
where
	C: ConnectionTrait,
{
	let Some(caps) = preset_caps::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::EntryNotFound(format!(
			"preset_caps for profile_id {}",
			profile_id
		)));
	};

	let decks = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.order_by_asc(preset_deck::Column::Index)
		.all(c)
		.await?;

	Ok((caps, decks))
}

pub(crate) async fn find_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<preset_deck::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_deck for profile_id {} and index {}",
				profile_id, preset_no
			))
		})?;

	Ok(record)
}

pub(crate) async fn get_preset_slots_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(preset_caps::Model, Vec<preset_slot::Model>), GameplayError>
where
	C: ConnectionTrait,
{
	let Some(caps) = preset_caps::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::EntryNotFound(format!(
			"preset_caps for profile_id {}",
			profile_id
		)));
	};

	let slots = preset_slot::Entity::find()
		.filter(preset_slot::Column::ProfileId.eq(profile_id))
		.order_by_asc(preset_slot::Column::Index)
		.all(c)
		.await?;

	Ok((caps, slots))
}

pub(crate) async fn register_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	preset: &PresetDeckItem,
) -> Result<preset_deck::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_deck::Entity::find()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset.index))
		.one(c)
		.await?;

	let mut am =
		record.map(emukc_db::sea_orm::IntoActiveModel::into_active_model).unwrap_or_else(|| {
			preset_deck::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(preset.index),
				..Default::default()
			}
		});

	am.name = ActiveValue::Set(preset.name.clone());
	am.ship_1 = ActiveValue::Set(preset.ships[0]);
	am.ship_2 = ActiveValue::Set(preset.ships[1]);
	am.ship_3 = ActiveValue::Set(preset.ships[2]);
	am.ship_4 = ActiveValue::Set(preset.ships[3]);
	am.ship_5 = ActiveValue::Set(preset.ships[4]);
	am.ship_6 = ActiveValue::Set(preset.ships[5]);

	let m = match am.id {
		ActiveValue::NotSet => am.insert(c).await?,
		_ => am.update(c).await?,
	};

	Ok(m)
}

pub(crate) async fn delete_preset_deck_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	preset_deck::Entity::delete_many()
		.filter(preset_deck::Column::ProfileId.eq(profile_id))
		.filter(preset_deck::Column::Index.eq(preset_no))
		.exec(c)
		.await?;

	Ok(())
}

pub(crate) async fn expand_preset_deck_capacity_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// deduct dock key
	deduct_use_item_impl(c, profile_id, KcUseItemType::DockKey as i64, 1).await?;

	// change deck limit
	let caps = preset_caps::Entity::find_by_id(profile_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!("preset_caps for profile_id {}", profile_id))
	})?;

	let new_cap = caps.deck_limit + 1;
	let mut am = caps.into_active_model();
	am.deck_limit = ActiveValue::Set(new_cap);

	am.update(c).await?;

	Ok(())
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

	Ok(())
}
