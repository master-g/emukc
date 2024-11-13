use emukc_db::{
	entity::profile::{item::slot_item, ship},
	sea_orm::{entity::prelude::*, ActiveValue},
};
use emukc_model::codex::Codex;

use crate::{
	err::GameplayError,
	game::{
		compose::get_unset_slot_items_by_types_impl, ship::recalculate_ship_status_with_model,
		slot_item::find_slot_item_impl,
	},
};

use super::{SlotDepriveParams, SlotDepriveResp};

fn get_slot_mut(ship: &mut ship::Model, is_ex_slot: bool, slot_idx: Option<i64>) -> &mut i64 {
	if is_ex_slot {
		&mut ship.slot_ex
	} else {
		match slot_idx {
			Some(0) => &mut ship.slot_1,
			Some(1) => &mut ship.slot_2,
			Some(2) => &mut ship.slot_3,
			Some(3) => &mut ship.slot_4,
			Some(4) => &mut ship.slot_5,
			_ => unreachable!(),
		}
	}
}

pub(crate) async fn slot_deprive_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	params: &SlotDepriveParams,
) -> Result<SlotDepriveResp, GameplayError>
where
	C: ConnectionTrait,
{
	// from ship
	let mut from_ship_model =
		ship::Entity::find_by_id(params.from_ship_id).one(c).await?.ok_or_else(|| {
			GameplayError::EntryNotFound(format!("ship for ID {}", params.from_ship_id))
		})?;

	let from_slot_idx = if params.from_ex_slot {
		None
	} else {
		Some(params.from_slot_idx)
	};
	let from_slot = get_slot_mut(&mut from_ship_model, params.from_ex_slot, from_slot_idx);
	let slot_item_id = *from_slot;
	*from_slot = -1;

	// to ship
	let mut to_ship_model =
		ship::Entity::find_by_id(params.to_ship_id).one(c).await?.ok_or_else(|| {
			GameplayError::EntryNotFound(format!("ship for ID {}", params.to_ship_id))
		})?;

	let to_slot_idx = if params.to_ex_slot {
		None
	} else {
		Some(params.to_slot_idx)
	};
	let to_slot = get_slot_mut(&mut to_ship_model, params.to_ex_slot, to_slot_idx);
	let unset_slot_item_id = *to_slot;
	*to_slot = slot_item_id;

	// update slot item
	{
		let set_slot_item_model = find_slot_item_impl(c, slot_item_id).await?;
		let mut am: slot_item::ActiveModel = set_slot_item_model.into();
		am.equip_on = ActiveValue::Set(params.to_ship_id);
		let m = am.update(c).await?;

		debug!(
			"deprive slot item from ship {} on slot {}, is_ex {}, to ship {} on slot {}, is_ex {}, slot item id {} is now on ship {}",
			params.from_ship_id,
			params.from_slot_idx,
			params.from_ex_slot,
			params.to_ship_id,
			params.to_slot_idx,
			params.to_ex_slot,
			m.id,
			m.equip_on,
		);
	}
	let (unset_type3, unset_id_list) = if unset_slot_item_id > 0 {
		let unset_slot_item_model = find_slot_item_impl(c, unset_slot_item_id).await?;
		let type3 = unset_slot_item_model.type3;
		let mut am: slot_item::ActiveModel = unset_slot_item_model.into();
		am.equip_on = ActiveValue::Set(0);
		am.update(c).await?;

		let unset_map = get_unset_slot_items_by_types_impl(c, profile_id, &[type3]).await?;
		let unset_list = if let Some(unset_list) = unset_map.get(&type3) {
			unset_list.to_owned()
		} else {
			vec![]
		};

		(Some(type3), Some(unset_list))
	} else {
		(None, None)
	};

	// recalculating stats
	let from_ship = {
		let am = recalculate_ship_status_with_model(c, codex, &from_ship_model).await?;
		am.update(c).await?
	};
	let to_ship = {
		let am = recalculate_ship_status_with_model(c, codex, &to_ship_model).await?;
		am.update(c).await?
	};

	let resp = SlotDepriveResp {
		from_ship,
		to_ship,
		unset_type3,
		unset_id_list,
		bauxite: 0,
	};

	Ok(resp)
}
