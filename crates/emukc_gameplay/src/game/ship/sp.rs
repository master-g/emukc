use emukc_db::{
	entity::profile::ship::sp_effect_item,
	sea_orm::{ConnectionTrait, QueryOrder, entity::prelude::*},
};

use crate::err::GameplayError;

pub(super) async fn find_ship_sp_effect_items_impl<C>(
	c: &C,
	ship_id: i64,
) -> Result<Vec<sp_effect_item::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let items = sp_effect_item::Entity::find()
		.filter(sp_effect_item::Column::ShipId.eq(ship_id))
		.order_by_asc(sp_effect_item::Column::Index)
		.all(c)
		.await?;

	Ok(items)
}
