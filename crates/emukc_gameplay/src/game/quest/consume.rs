use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::{
	kc2::MaterialCategory,
	thirdparty::{
		Kc3rdQuestConditionConsumption, Kc3rdQuestConditionModelConversion,
		Kc3rdQuestConditionSlotItemType,
	},
};

use crate::{
	err::GameplayError,
	game::{material::deduct_material_impl, use_item::deduct_use_item_impl},
};

pub(super) async fn handle_module_conversion<C>(
	c: &C,
	profile_id: i64,
	info: &Kc3rdQuestConditionModelConversion,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	if let Some(slots) = info.slots.as_ref() {
		for slot in slots {
			match &slot.item.item_type {
				Kc3rdQuestConditionSlotItemType::Equipment(ids) => {
					handle_slotitem_consumption(c, profile_id, info, ids).await?;
				}
				Kc3rdQuestConditionSlotItemType::EquipType(_items) => {
					todo!()
				}
			}
		}
	}

	Ok(())
}

async fn handle_slotitem_consumption<C>(
	_c: &C,
	_profile_id: i64,
	_info: &Kc3rdQuestConditionModelConversion,
	_id: &[i64],
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// TODO: implement slotitem consumption
	Ok(())
}

pub(super) async fn handle_consumption<C>(
	c: &C,
	profile_id: i64,
	consumption: &Kc3rdQuestConditionConsumption,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	match consumption {
		Kc3rdQuestConditionConsumption::Resources(res) => {
			let mats = vec![
				(MaterialCategory::Fuel, res.fuel),
				(MaterialCategory::Ammo, res.ammo),
				(MaterialCategory::Steel, res.steel),
				(MaterialCategory::Bauxite, res.bauxite),
			];
			deduct_material_impl(c, profile_id, &mats).await?;
		}
		Kc3rdQuestConditionConsumption::SlotItemConsumption(_) => {}
		Kc3rdQuestConditionConsumption::UseItemConsumption(items) => {
			for item in items {
				deduct_use_item_impl(c, profile_id, item.api_id, item.amount).await?;
			}
		}
	}
	Ok(())
}
