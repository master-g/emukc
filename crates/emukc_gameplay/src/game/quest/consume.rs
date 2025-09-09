use emukc_db::{
	entity::profile::fleet,
	sea_orm::{ConnectionTrait, EntityTrait},
};
use emukc_model::thirdparty::{
	Kc3rdQuestConditionModelConversion, Kc3rdQuestConditionSlotItemType,
};

use crate::{
	err::GameplayError,
	game::{fleet::find_fleet, ship::find_ship_impl},
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
				Kc3rdQuestConditionSlotItemType::Equipment(id) => {
					handle_slotitem_consumption(c, profile_id, info, &[*id]).await?;
				}
				Kc3rdQuestConditionSlotItemType::Equipments(ids) => {
					handle_slotitem_consumption(c, profile_id, info, ids).await?;
				}
				Kc3rdQuestConditionSlotItemType::EquipType(_) => todo!(),
				Kc3rdQuestConditionSlotItemType::EquipTypes(items) => {
					todo!()
				}
			}
		}
	}

	Ok(())
}

async fn handle_slotitem_consumption<C>(
	c: &C,
	profile_id: i64,
	info: &Kc3rdQuestConditionModelConversion,
	id: &[i64],
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// TODO: find slotitem candidates on secretary first
	if info.secretary.is_some() {
		let first_fleet = find_fleet(c, profile_id, 1).await?;
		let secretary = first_fleet.ship_1;
		let secretary_ship = find_ship_impl(c, secretary).await?;
	};

	Ok(())
}
