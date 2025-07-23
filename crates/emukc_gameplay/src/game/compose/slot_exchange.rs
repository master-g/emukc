use emukc_db::{
	entity::profile::ship,
	sea_orm::{ActiveValue, IntoActiveModel, entity::prelude::*},
};
use emukc_model::fields::MoveValueToEnd;

use crate::err::GameplayError;

pub(crate) async fn slot_exchange_impl<C>(
	c: &C,
	ship_id: i64,
	src_slot_idx: i64,
	dst_slot_idx: i64,
) -> Result<ship::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let ship = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship for ID {ship_id}")))?;

	let mut slots = [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5];

	// swap slots
	slots.swap(src_slot_idx as usize, dst_slot_idx as usize);
	slots.move_value_to_end(-1);

	let mut am = ship.into_active_model();
	am.slot_1 = ActiveValue::Set(slots[0]);
	am.slot_2 = ActiveValue::Set(slots[1]);
	am.slot_3 = ActiveValue::Set(slots[2]);
	am.slot_4 = ActiveValue::Set(slots[3]);
	am.slot_5 = ActiveValue::Set(slots[4]);

	let m = am.update(c).await?;

	Ok(m)
}
