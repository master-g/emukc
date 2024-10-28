use async_trait::async_trait;
use emukc_db::{
	entity::profile::preset::{preset_caps, preset_deck, preset_slot},
	sea_orm::{entity::prelude::*, ActiveValue, QueryOrder},
};
use emukc_model::profile::{preset_deck::PresetDeck, preset_slot::PresetSlot};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for preset related gameplay.
#[async_trait]
pub trait PresetOps {
	/// Get preset deck
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_preset_deck(&self, profile_id: i64) -> Result<PresetDeck, GameplayError>;

	/// Get preset slot
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_preset_slot(&self, profile_id: i64) -> Result<PresetSlot, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> PresetOps for T {
	async fn get_preset_deck(&self, profile_id: i64) -> Result<PresetDeck, GameplayError> {
		let db = self.db();

		let (caps, decks) = get_preset_deck_impl(db, profile_id).await?;

		Ok(PresetDeck {
			profile_id,
			max_num: caps.deck_limit,
			records: decks.into_iter().map(Into::into).collect(),
		})
	}

	async fn get_preset_slot(&self, profile_id: i64) -> Result<PresetSlot, GameplayError> {
		let db = self.db();

		let (caps, slots) = get_preset_slot_impl(db, profile_id).await?;

		Ok(PresetSlot {
			profile_id,
			max_num: caps.slot_limit,
			records: slots.into_iter().map(Into::into).collect(),
		})
	}
}

pub async fn get_preset_deck_impl<C>(
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

pub async fn get_preset_slot_impl<C>(
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
