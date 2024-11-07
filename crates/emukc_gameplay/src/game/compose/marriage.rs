use emukc_db::{
	entity::profile::{item::slot_item, ship},
	sea_orm::{entity::prelude::*, ActiveValue},
};
use emukc_model::{
	codex::Codex,
	kc2::{KcApiShip, KcApiSlotItem, KcUseItemType},
};

use crate::{
	err::GameplayError,
	game::{picturebook::add_ship_to_picturebook_impl, use_item::deduct_use_item_impl},
};

pub(crate) async fn marriage_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	ship_id: i64,
) -> Result<ship::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!(
			"No ship found for profile ID {} and ship ID {}",
			profile_id, ship_id
		))
	})?;

	if ship.married {
		return Ok(ship);
	}

	// Deduct ring first
	deduct_use_item_impl(c, profile_id, KcUseItemType::Ring as i64, 1).await?;

	// find ship slot items
	let slot_items =
		slot_item::Entity::find().filter(slot_item::Column::EquipOn.eq(ship_id)).all(c).await?;
	let slot_items: Vec<KcApiSlotItem> =
		slot_items.into_iter().map(std::convert::Into::into).collect();

	// mark in picture book
	add_ship_to_picturebook_impl(c, profile_id, ship.sort_num, None, Some(true)).await?;

	// update ship status
	let mut api_ship: KcApiShip = ship.into();
	api_ship.api_lv = 100;

	// FIXME: https://en.kancollewiki.net/Marriage
	codex.cal_ship_status(&mut api_ship, &slot_items)?;

	let mut am: ship::ActiveModel = ship.into();
	am.id = ActiveValue::Unchanged(ship.id);
	am.profile_id = ActiveValue::Unchanged(ship.profile_id);

	let new_ship = am.update(c).await?;

	Ok(new_ship)
}
