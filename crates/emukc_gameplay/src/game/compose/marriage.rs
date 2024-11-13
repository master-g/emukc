use emukc_db::{
	entity::profile::ship,
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel},
};
use emukc_model::{codex::Codex, kc2::KcUseItemType, prelude::ApiMstShip};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use crate::{
	err::GameplayError,
	game::{
		picturebook::add_ship_to_picturebook_impl, ship::recalculate_ship_status_with_model,
		use_item::deduct_use_item_impl,
	},
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
	let mut ship = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!(
			"No ship found for profile ID {} and ship ID {}",
			profile_id, ship_id
		))
	})?;

	if ship.married {
		return Ok(ship);
	}

	let ship_mst_id = ship.mst_id;
	let mst = codex.find::<ApiMstShip>(&ship_mst_id)?;

	// Deduct ring first
	deduct_use_item_impl(c, profile_id, KcUseItemType::Ring as i64, 1).await?;

	// mark in picture book
	add_ship_to_picturebook_impl(c, profile_id, ship.sort_num, None, Some(true)).await?;

	// update ship status
	ship.level = 100;
	let mut rng = SmallRng::from_entropy();
	ship.mod_luck = rng.gen_range(3..=6);

	ship.fuel = mst.api_fuel_max.unwrap_or(0);
	ship.ammo = mst.api_bull_max.unwrap_or(0);

	recalculate_ship_status_with_model(c, codex, &mut ship).await?;

	ship.hp_now = ship.hp_max;
	let max_eq = mst.api_maxeq.unwrap_or([0; 5]);
	ship.onslot_1 = max_eq[0];
	ship.onslot_2 = max_eq[1];
	ship.onslot_3 = max_eq[2];
	ship.onslot_4 = max_eq[3];
	ship.onslot_5 = max_eq[4];

	// save to db
	let mut am: ship::ActiveModel = ship.into_active_model();
	am.id = ActiveValue::Unchanged(ship.id);
	am.profile_id = ActiveValue::Unchanged(ship.profile_id);

	let new_ship = am.update(c).await?;

	Ok(new_ship)
}
