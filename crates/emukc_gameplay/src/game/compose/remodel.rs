use emukc_db::{
	entity::profile::{item::slot_item, ship},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel},
};
use emukc_model::{
	codex::Codex,
	kc2::{KcSlotItemType3, KcUseItemType, MaterialCategory},
	prelude::Kc3rdShip,
};

use crate::{
	err::GameplayError,
	game::{
		compose::get_unset_slot_items_by_types_impl, material::deduct_material_impl,
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

	let (after_mst, after_extra, requirements) = codex.find_ship_after(ship_id)?;

	let after_ship = codex
		.new_ship(after_mst.api_id)
		.ok_or_else(|| GameplayError::EntryNotFound(format!("ship for ID {}", after_mst.api_id)))?;

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

	// boiler
	let free_boiler = get_unset_slot_items_by_types_impl(
		c,
		codex,
		profile_id,
		&[KcSlotItemType3::EngineBoost as i64],
	)
	.await?;

	todo!()
}

pub(crate) async fn get_free_slot_item_by_type3_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	slot_type3: i64,
) -> Result<Option<slot_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	// slot_item::Entity::find()
	// .filter(slot_item::Column::ProfileId.eq(profile_id))
	// .filter(slot_item::Column::SlotItemType3.eq(slot_type3))

	todo!()
}
