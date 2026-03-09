use emukc_db::{
	entity::profile::preset::{preset_caps, preset_dev_item},
	sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, entity::prelude::*},
};
use emukc_model::{kc2::KcUseItemType, profile::preset_dev_item::PresetDevItemElement};

use crate::err::GameplayError;

use super::deduct_use_item_impl;

pub(crate) async fn get_preset_dev_items_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(preset_caps::Model, Vec<preset_dev_item::Model>), GameplayError>
where
	C: ConnectionTrait,
{
	let Some(caps) = preset_caps::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::EntryNotFound(format!(
			"preset_caps for profile_id {profile_id}",
		)));
	};

	let items = preset_dev_item::Entity::find()
		.filter(preset_dev_item::Column::ProfileId.eq(profile_id))
		.order_by_asc(preset_dev_item::Column::Index)
		.all(c)
		.await?;

	Ok((caps, items))
}

pub(crate) async fn register_preset_dev_item_impl<C>(
	c: &C,
	profile_id: i64,
	preset: &PresetDevItemElement,
) -> Result<preset_dev_item::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_dev_item::Entity::find()
		.filter(preset_dev_item::Column::ProfileId.eq(profile_id))
		.filter(preset_dev_item::Column::Index.eq(preset.index))
		.one(c)
		.await?;

	let mut am = record.map(IntoActiveModel::into_active_model).unwrap_or_else(|| {
		preset_dev_item::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			index: ActiveValue::Set(preset.index),
			..Default::default()
		}
	});

	am.name = ActiveValue::Set(preset.name.clone());
	am.item1 = ActiveValue::Set(preset.item1);
	am.item2 = ActiveValue::Set(preset.item2);
	am.item3 = ActiveValue::Set(preset.item3);
	am.item4 = ActiveValue::Set(preset.item4);

	let m = match am.id {
		ActiveValue::NotSet => am.insert(c).await?,
		_ => am.update(c).await?,
	};

	Ok(m)
}

pub(crate) async fn delete_preset_dev_item_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	preset_dev_item::Entity::delete_many()
		.filter(preset_dev_item::Column::ProfileId.eq(profile_id))
		.filter(preset_dev_item::Column::Index.eq(preset_no))
		.exec(c)
		.await?;

	Ok(())
}

pub(crate) async fn update_preset_dev_item_name_impl<C>(
	c: &C,
	profile_id: i64,
	preset_no: i64,
	name: String,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let record = preset_dev_item::Entity::find()
		.filter(preset_dev_item::Column::ProfileId.eq(profile_id))
		.filter(preset_dev_item::Column::Index.eq(preset_no))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"preset_dev_item for profile_id {profile_id} and index {preset_no}",
			))
		})?;

	let mut am = record.into_active_model();
	am.name = ActiveValue::Set(name);
	am.save(c).await?;

	Ok(())
}

pub(crate) async fn expand_preset_dev_item_capacity_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<i64, GameplayError>
where
	C: ConnectionTrait,
{
	const PRESET_DEVELOP_LIMIT: i64 = 6;

	let Some(caps) = preset_caps::Entity::find_by_id(profile_id).one(c).await? else {
		return Err(GameplayError::EntryNotFound(format!(
			"preset_caps for profile_id {profile_id}",
		)));
	};

	if caps.dev_item_limit >= PRESET_DEVELOP_LIMIT {
		return Err(GameplayError::EntryNotFound(
			"preset_dev_item capacity already at maximum".to_string(),
		));
	}

	deduct_use_item_impl(c, profile_id, KcUseItemType::DockKey as i64, 1).await?;

	let mut am = caps.into_active_model();
	am.dev_item_limit = ActiveValue::Set(am.dev_item_limit.unwrap() + 1);
	let updated = am.update(c).await?;

	Ok(updated.dev_item_limit)
}
