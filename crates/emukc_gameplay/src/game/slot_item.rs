use emukc_db::{
	entity::profile::item::slot_item,
	sea_orm::{entity::prelude::*, ActiveValue},
};

use crate::err::GameplayError;

/// Add slot item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `stars`: The stars of the item.
#[allow(unused)]
pub async fn add_slot_item_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	stars: i64,
) -> Result<slot_item::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let am = slot_item::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		mst_id: ActiveValue::Set(mst_id),
		locked: ActiveValue::Set(false),
		level: ActiveValue::Set(stars),
		aircraft_lv: ActiveValue::Set(0),
	};

	let model = am.save(c).await?;

	Ok(model)
}
