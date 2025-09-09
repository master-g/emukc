use async_trait::async_trait;
use emukc_db::{
	entity::profile::{fleet, ship},
	sea_orm::{ActiveValue, QueryOrder, TransactionTrait, TryIntoModel, entity::prelude::*},
};
use emukc_model::profile::fleet::Fleet;

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for fleet related gameplay.
#[async_trait]
pub trait FleetOps {
	/// Unlock new deck port.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The fleet index, must be one of 2, 3, 4.
	async fn unlock_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError>;

	/// Get single deck port.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
	async fn get_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError>;

	/// Get ship from deck port.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
	async fn get_fleet_ships(
		&self,
		profile_id: i64,
		index: i64,
	) -> Result<Vec<ship::Model>, GameplayError>;

	/// Get all deck ports.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_fleets(&self, profile_id: i64) -> Result<Vec<Fleet>, GameplayError>;

	/// Change ship position in deck port.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
	/// - `ship_ids`: The ship IDs, must be 6 elements.
	async fn update_fleet_ships(
		&self,
		profile_id: i64,
		index: i64,
		ship_ids: &[i64; 6],
	) -> Result<Fleet, GameplayError>;

	/// Update deck name.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `index`: The deck index.
	/// - `name`: The new deck name.
	async fn update_deck_name(
		&self,
		profile_id: i64,
		index: i64,
		name: &str,
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> FleetOps for T {
	async fn unlock_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = unlock_fleet_impl(&tx, profile_id, index).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn get_fleet(&self, profile_id: i64, index: i64) -> Result<Fleet, GameplayError> {
		let db = self.db();
		let fleet = get_fleet_impl(db, profile_id, index).await?;

		Ok(fleet)
	}

	async fn get_fleets(&self, profile_id: i64) -> Result<Vec<Fleet>, GameplayError> {
		let db = self.db();
		let fleets = get_fleets_impl(db, profile_id).await?;

		Ok(fleets.into_iter().map(std::convert::Into::into).collect())
	}

	async fn get_fleet_ships(
		&self,
		profile_id: i64,
		index: i64,
	) -> Result<Vec<ship::Model>, GameplayError> {
		let db = self.db();

		let ships = get_fleet_ships_impl(db, profile_id, index).await?;

		Ok(ships)
	}

	async fn update_fleet_ships(
		&self,
		profile_id: i64,
		index: i64,
		ship_ids: &[i64; 6],
	) -> Result<Fleet, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;
		let m = update_fleet_ships_impl(&tx, profile_id, index, ship_ids).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn update_deck_name(
		&self,
		profile_id: i64,
		index: i64,
		name: &str,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_deck_name_impl(&tx, profile_id, index, name).await?;

		tx.commit().await?;

		Ok(())
	}
}

pub(crate) async fn find_fleet<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = fleet::Entity::find()
		.filter(fleet::Column::ProfileId.eq(profile_id))
		.filter(fleet::Column::Index.eq(index))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"fleet {index} not found for profile {profile_id}",
			))
		})?;

	Ok(fleet)
}

/// Unlock new deck port.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The fleet index, must be one of 2, 3, 4.
#[allow(unused)]
pub(crate) async fn unlock_fleet_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = Fleet::new(profile_id, index).unwrap();

	let mut am: fleet::ActiveModel = fleet.into();

	let m = am.save(c).await?;

	Ok(m.try_into_model()?)
}

/// Get single fleet.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
#[allow(unused)]
pub(crate) async fn get_fleet_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<Fleet, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = find_fleet(c, profile_id, index).await?;

	Ok(fleet.into())
}

/// Get all deck ports.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub(crate) async fn get_fleets_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<fleet::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let fleets: Vec<fleet::Model> = fleet::Entity::find()
		.filter(fleet::Column::ProfileId.eq(profile_id))
		.order_by_asc(fleet::Column::Index)
		.all(c)
		.await?;

	Ok(fleets)
}

pub(crate) async fn get_fleet_ships_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
) -> Result<Vec<ship::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = get_fleet_impl(c, profile_id, index).await?;
	let mut ships = ship::Entity::find().filter(ship::Column::Id.is_in(fleet.ships)).all(c).await?;
	ships.sort_by_key(|ship| fleet.ships.iter().position(|&id| id == ship.id).unwrap());

	Ok(ships)
}

/// Change ship position in deck port.
///
/// # Parameters
///
/// - `profile_id`: The profile ID.
/// - `index`: The fleet index, must be one of 1, 2, 3, 4.
/// - `ship_ids`: The ship IDs, must be 6 elements.
pub(crate) async fn update_fleet_ships_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
	ship_ids: &[i64; 6],
) -> Result<fleet::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = find_fleet(c, profile_id, index).await?;
	let mut am: fleet::ActiveModel = fleet.into();
	am.ship_1 = ActiveValue::Set(ship_ids[0]);
	am.ship_2 = ActiveValue::Set(ship_ids[1]);
	am.ship_3 = ActiveValue::Set(ship_ids[2]);
	am.ship_4 = ActiveValue::Set(ship_ids[3]);
	am.ship_5 = ActiveValue::Set(ship_ids[4]);
	am.ship_6 = ActiveValue::Set(ship_ids[5]);

	let m = am.update(c).await?;

	Ok(m.try_into_model()?)
}

pub(crate) async fn update_deck_name_impl<C>(
	c: &C,
	profile_id: i64,
	index: i64,
	name: &str,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let fleet = find_fleet(c, profile_id, index).await?;
	let mut am: fleet::ActiveModel = fleet.into();
	am.name = ActiveValue::Set(name.to_string());

	am.update(c).await?;

	Ok(())
}

/// Initialize deck ports for a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	unlock_fleet_impl(c, profile_id, 1).await?;

	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	fleet::Entity::delete_many().filter(fleet::Column::ProfileId.eq(profile_id)).exec(c).await?;

	Ok(())
}
