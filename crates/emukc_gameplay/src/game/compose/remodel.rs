use emukc_db::{
	entity::profile::{item::slot_item, ship},
	sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, entity::prelude::*},
};
use emukc_model::{
	codex::Codex,
	kc2::{KcSlotItemType3, KcUseItemType, MaterialCategory},
	prelude::ApiMstShip,
};

use crate::{
	err::GameplayError,
	game::{
		material::deduct_material_impl, slot_item::add_slot_item_impl,
		use_item::deduct_use_item_impl,
	},
};

pub(crate) async fn remodel_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	ship_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let ship_model = ship::Entity::find_by_id(ship_id)
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship for ID {}", ship_id)))?;

	let (after_mst, _, requirements) = codex.find_ship_after(ship_model.mst_id)?;

	let mst = codex.find::<ApiMstShip>(&ship_model.mst_id)?;

	debug!(
		"remodel ship({}) {}[{}] to {}[{}], requirements: {:?}",
		ship_id, mst.api_id, mst.api_name, after_mst.api_id, after_mst.api_name, requirements
	);

	// new ship and new items
	let (mut new_ship, mut new_slot_items) = codex
		.new_ship(after_mst.api_id)
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship for ID {}", after_mst.api_id)))?;

	// deduct boiler
	{
		let free_boiler =
			get_free_slot_item_by_type3_impl(c, profile_id, KcSlotItemType3::EngineBoost as i64)
				.await?;
		if free_boiler.len() < requirements.boiler as usize {
			return Err(GameplayError::Insufficient(format!(
				"boiler (required: {}, available: {})",
				requirements.boiler,
				free_boiler.len()
			)));
		}
		for boiler in free_boiler.iter().take(requirements.boiler as usize) {
			slot_item::Entity::delete_by_id(boiler.id).exec(c).await?;
		}
	}

	// undress the ship
	for slot_item_id in [
		ship_model.slot_1,
		ship_model.slot_2,
		ship_model.slot_3,
		ship_model.slot_4,
		ship_model.slot_5,
		ship_model.slot_ex,
	] {
		if slot_item_id > 0 {
			let m = slot_item::Entity::find_by_id(slot_item_id).one(c).await?.ok_or_else(|| {
				GameplayError::EntryNotFound(format!("slot item for ID {}", slot_item_id))
			})?;
			let mut am = m.into_active_model();
			am.equip_on = ActiveValue::Set(0);
			am.update(c).await?;
		}
	}

	// deduct resources
	deduct_material_impl(
		c,
		profile_id,
		&[
			(MaterialCategory::Ammo, requirements.ammo),
			(MaterialCategory::Steel, requirements.steel),
			(MaterialCategory::DevMat, requirements.devmat),
			(MaterialCategory::Torch, requirements.torch),
		],
	)
	.await?;

	// deduct use items
	if requirements.blueprint > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::Blueprint as i64,
			requirements.blueprint,
		)
		.await?;
	}
	if requirements.catapult > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::ProtoCatapult as i64,
			requirements.catapult,
		)
		.await?;
	}
	if requirements.report > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::ActionReport as i64,
			requirements.report,
		)
		.await?;
	}
	if requirements.aviation > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::NewAviationMaterial as i64,
			requirements.aviation,
		)
		.await?;
	}
	if requirements.artillery > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::NewArtilleryMaterial as i64,
			requirements.artillery,
		)
		.await?;
	}
	if requirements.armament > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::NewArmamentMaterial as i64,
			requirements.armament,
		)
		.await?;
	}
	if requirements.overseas > 0 {
		deduct_use_item_impl(
			c,
			profile_id,
			KcUseItemType::OverseasWarshipTechnology as i64,
			requirements.overseas,
		)
		.await?;
	}

	// add new slot items
	if !new_slot_items.is_empty() {
		for (i, item) in new_slot_items.iter_mut().enumerate() {
			let m = add_slot_item_impl(
				c,
				codex,
				profile_id,
				item.api_slotitem_id,
				item.api_level,
				item.api_alv.unwrap_or_default(),
			)
			.await?;

			item.api_id = m.id;
			new_ship.api_onslot[i] = m.id;
		}
	}

	new_ship.api_id = ship_id;
	new_ship.api_lv = ship_model.level;
	new_ship.api_exp[0] = ship_model.exp_now;
	new_ship.api_exp[1] = ship_model.exp_next;
	new_ship.api_exp[2] = ship_model.exp_progress;
	new_ship.api_slot_ex = if ship_model.slot_ex != 0 {
		-1
	} else {
		0
	};
	// remodel will reset [firepower, torpedo, aa, armor]
	new_ship.api_kyouka[0] = 0;
	new_ship.api_kyouka[1] = 0;
	new_ship.api_kyouka[2] = 0;
	new_ship.api_kyouka[3] = 0;
	new_ship.api_kyouka[4] = ship_model.mod_luck;
	new_ship.api_kyouka[5] = ship_model.mod_hp;
	new_ship.api_kyouka[6] = ship_model.mod_asw;

	codex.cal_ship_status(&mut new_ship, &new_slot_items)?;

	// save new ship to db

	let mut am: ship::ActiveModel = new_ship.into();
	am.id = ActiveValue::Unchanged(ship_id);
	am.profile_id = ActiveValue::Unchanged(profile_id);
	am.has_locked_euqip = ActiveValue::Set(false);

	am.update(c).await?;

	Ok(())
}

pub(crate) async fn get_free_slot_item_by_type3_impl<C>(
	c: &C,
	profile_id: i64,
	slot_type3: i64,
) -> Result<Vec<slot_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let m = slot_item::Entity::find()
		.filter(slot_item::Column::ProfileId.eq(profile_id))
		.filter(slot_item::Column::Type3.eq(slot_type3))
		.filter(slot_item::Column::EquipOn.gt(0))
		.filter(slot_item::Column::Locked.eq(false))
		.order_by_asc(slot_item::Column::Level)
		.all(c)
		.await?;

	Ok(m)
}
