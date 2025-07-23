use async_trait::async_trait;
use emukc_db::{
	entity::profile::{ndock, ship},
	sea_orm::{
		ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, TryIntoModel,
		entity::prelude::*,
	},
};
use emukc_model::{
	codex::{Codex, repair::RepairCost},
	kc2::{KcUseItemType, MaterialCategory},
	prelude::ApiMstShip,
	profile::ndock::RepairDock,
};
use emukc_time::chrono;

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
	material::deduct_material_impl, ship::recalculate_ship_status_with_model,
	use_item::deduct_use_item_impl,
};

/// A trait for repair dock related gameplay.
#[async_trait]
pub trait NDockOps {
	/// Unlock new repair dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The repair dock index, must be one of 2, 3, 4.
	async fn unlock_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError>;

	/// Get single repair dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The repair dock index, must be one of 1, 2, 3, 4.
	async fn get_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError>;

	/// Get all repair docks.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_ndocks(&self, profile_id: i64) -> Result<Vec<RepairDock>, GameplayError>;

	/// Expand repair dock.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn expand_repair_dock(&self, profile_id: i64) -> Result<(), GameplayError>;

	/// Start ship repairation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ndock_id`: The repair dock ID.
	/// - `ship_id`: The ship ID.
	/// - `highspeed`: Whether to use high-speed repair.
	async fn ndock_start_repair(
		&self,
		profile_id: i64,
		ndock_id: i64,
		ship_id: i64,
		highspeed: bool,
	) -> Result<(), GameplayError>;

	/// Speed up ship repairation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ndock_id`: The repair dock ID.
	async fn speed_up_ship_repairation(
		&self,
		profile_id: i64,
		ndock_id: i64,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> NDockOps for T {
	async fn unlock_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = unlock_ndock_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn get_ndock(&self, profile_id: i64, index: i64) -> Result<RepairDock, GameplayError> {
		let db = self.db();
		let dock = get_ndock_impl(db, profile_id, index).await?;

		Ok(dock)
	}

	async fn get_ndocks(&self, profile_id: i64) -> Result<Vec<RepairDock>, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let docks = get_ndocks_impl(&tx, codex, profile_id).await?;

		tx.commit().await?;

		Ok(docks.into_iter().map(std::convert::Into::into).collect())
	}

	async fn expand_repair_dock(&self, profile_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		expand_repair_dock_impl(&tx, profile_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn ndock_start_repair(
		&self,
		profile_id: i64,
		ndock_id: i64,
		ship_id: i64,
		highspeed: bool,
	) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		ndock_start_repair_impl(&tx, codex, profile_id, ndock_id, ship_id, highspeed).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn speed_up_ship_repairation(
		&self,
		profile_id: i64,
		ndock_id: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		speed_up_ship_repairation_impl(&tx, profile_id, ndock_id).await?;

		tx.commit().await?;

		Ok(())
	}
}

async fn find_dock<C>(c: &C, profile_id: i64, index: i64) -> Result<ndock::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = ndock::Entity::find()
		.filter(ndock::Column::ProfileId.eq(profile_id))
		.filter(ndock::Column::Index.eq(index))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"Repair dock {index} not found for profile {profile_id}",
			))
		})?;

	Ok(dock)
}

/// Unlock new repair dock.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The construction dock index, must be one of 2, 3, 4.
#[allow(unused)]
pub(crate) async fn unlock_ndock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<ndock::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = find_dock(c, profile_id, index).await?;

	let mut am: ndock::ActiveModel = dock.into();
	am.status = ActiveValue::Set(ndock::Status::Idle);

	let m = am.save(c).await?;

	Ok(m.try_into_model()?)
}

/// Get single repair dock.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The construction dock index, must be one of 1, 2, 3, 4.
#[allow(unused)]
pub(crate) async fn get_ndock_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<RepairDock, GameplayError>
where
	C: ConnectionTrait,
{
	let dock = find_dock(c, profile_id, index).await?;

	Ok(dock.into())
}

/// Get all repair docks.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub(crate) async fn get_ndocks_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<Vec<ndock::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let models: Vec<ndock::Model> = ndock::Entity::find()
		.filter(ndock::Column::ProfileId.eq(profile_id))
		.order_by_asc(ndock::Column::Index)
		.all(c)
		.await?;

	let mut docks = vec![];

	for model in models {
		if model.ship_id > 0 {
			let ship_id = model.ship_id;
			if let Some(complete_time) = model.complete_time {
				if complete_time <= chrono::Utc::now() {
					let dock_id = model.id;

					let mut am: ndock::ActiveModel = model.into();
					am.status = ActiveValue::Set(ndock::Status::Idle);
					am.ship_id = ActiveValue::Set(0);
					am.fuel = ActiveValue::Set(0);
					am.steel = ActiveValue::Set(0);
					am.complete_time = ActiveValue::Set(None);

					let m = am.update(c).await?;
					docks.push(m);

					// update ship
					let mut ship =
						ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
							GameplayError::EntryNotFound(format!(
								"Ship {ship_id} not found for repair dock {dock_id}",
							))
						})?;

					ship.hp_now = ship.hp_max;
					if ship.condition < 40 {
						ship.condition = 40;
					}
					let ship_am = recalculate_ship_status_with_model(c, codex, &ship).await?;
					ship_am.update(c).await?;

					continue;
				}
			}
		}

		docks.push(model);
	}

	Ok(docks)
}

pub(crate) async fn expand_repair_dock_impl<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let dock = ndock::Entity::find()
		.filter(ndock::Column::ProfileId.eq(profile_id))
		.filter(ndock::Column::Status.eq(ndock::Status::Locked))
		.order_by_asc(ndock::Column::Index)
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!("Repair dock not found for profile {profile_id}",))
		})?;

	deduct_use_item_impl(c, profile_id, KcUseItemType::DockKey as i64, 1).await?;

	let mut am: ndock::ActiveModel = dock.into();
	am.status = ActiveValue::Set(ndock::Status::Idle);

	am.save(c).await?;

	Ok(())
}

pub(crate) async fn ndock_start_repair_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	ndock_id: i64,
	ship_id: i64,
	highspeed: bool,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let dock = ndock::Entity::find_by_id(ndock_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!(
			"Repair dock {ndock_id} not found for profile {profile_id}",
		))
	})?;

	let ship = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!("Ship {ship_id} not found for profile {profile_id}",))
	})?;

	let ship_mst = codex.find::<ApiMstShip>(&ship.mst_id)?;

	// check if there are repair info in ship model already
	let docking_cost = if ship.ndock_time <= 0 {
		// no repair info, calculate repair time
		warn!("No repair info in ship model, calculate repair time");

		codex.cal_ship_docking_cost(ship_mst, ship.level, ship.hp_max - ship.hp_now)?
	} else {
		RepairCost {
			duration_sec: ship.ndock_time / 1000,
			fuel_cost: ship.ndock_fuel,
			steel_cost: ship.ndock_steel,
		}
	};

	// update ship model
	{
		let mut am = ship.into_active_model();

		if highspeed {
			am.ndock_time = ActiveValue::Set(0);
			am.ndock_fuel = ActiveValue::Set(0);
			am.ndock_steel = ActiveValue::Set(0);
			am.hp_now = ActiveValue::Set(ship.hp_max);
			am.condition = if ship.condition < 40 {
				ActiveValue::Set(40)
			} else {
				ActiveValue::Unchanged(ship.condition)
			};
		} else {
			am.ndock_time = ActiveValue::Set(docking_cost.duration_sec * 1000);
			am.ndock_fuel = ActiveValue::Set(docking_cost.fuel_cost);
			am.ndock_steel = ActiveValue::Set(docking_cost.steel_cost);
		}

		am.update(c).await?;
	}

	// deduct material
	{
		let mut material_cost = vec![(MaterialCategory::Fuel, docking_cost.fuel_cost)];
		if highspeed {
			material_cost.push((MaterialCategory::Bucket, 1));
		}
		deduct_material_impl(c, profile_id, &material_cost).await?;
	}

	// update ndock model
	{
		let mut am = dock.into_active_model();
		if highspeed {
			am.status = ActiveValue::Set(ndock::Status::Idle);
			am.fuel = ActiveValue::Set(0);
			am.steel = ActiveValue::Set(0);
			am.ship_id = ActiveValue::Set(0);
			am.complete_time = ActiveValue::Set(None);
		} else {
			am.status = ActiveValue::Set(ndock::Status::Busy);
			am.fuel = ActiveValue::Set(docking_cost.fuel_cost);
			am.steel = ActiveValue::Set(docking_cost.steel_cost);
			am.ship_id = ActiveValue::Set(ship_id);
			am.complete_time = ActiveValue::Set(Some(
				chrono::Utc::now() + chrono::Duration::seconds(docking_cost.duration_sec),
			));
		}

		am.update(c).await?;
	}

	Ok(())
}

pub(crate) async fn speed_up_ship_repairation_impl<C>(
	c: &C,
	profile_id: i64,
	ndock_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let dock = ndock::Entity::find_by_id(ndock_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!(
			"Repair dock {ndock_id} not found for profile {profile_id}",
		))
	})?;

	// deduct material
	deduct_material_impl(c, profile_id, &[(MaterialCategory::Bucket, 1)]).await?;

	let ship = ship::Entity::find_by_id(dock.ship_id).one(c).await?.ok_or_else(|| {
		GameplayError::EntryNotFound(format!(
			"Ship {} not found for repair dock {ndock_id}",
			dock.ship_id,
		))
	})?;

	// update ndock model
	{
		let mut am = dock.into_active_model();
		am.complete_time = ActiveValue::Set(None);
		am.status = ActiveValue::Set(ndock::Status::Idle);
		am.ship_id = ActiveValue::Set(0);
		am.fuel = ActiveValue::Set(0);
		am.steel = ActiveValue::Set(0);

		am.update(c).await?;
	}

	// update ship
	{
		let mut am = ship.into_active_model();
		am.hp_now = ActiveValue::Set(ship.hp_max);
		if ship.condition < 40 {
			am.condition = ActiveValue::Set(40);
		}

		am.update(c).await?;
	}

	Ok(())
}

/// Initialize repair docks for a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let models: Vec<ndock::ActiveModel> = [1, 2, 3, 4]
		.iter()
		.map(|i| {
			let dock = RepairDock::new(profile_id, *i).unwrap();
			ndock::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				index: ActiveValue::Set(*i),
				status: ActiveValue::Set(dock.status.into()),
				ship_id: ActiveValue::Set(0),
				complete_time: ActiveValue::Set(None),
				fuel: ActiveValue::Set(0),
				steel: ActiveValue::Set(0),
			}
		})
		.collect();

	ndock::Entity::insert_many(models).exec(c).await?;

	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	ndock::Entity::delete_many().filter(ndock::Column::ProfileId.eq(profile_id)).exec(c).await?;

	Ok(())
}
