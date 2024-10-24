use async_trait::async_trait;
use emukc_db::{
	entity::profile::kdock,
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::{kc2::MaterialCategory, prelude::ApiMstShip, profile::material::Material};
use emukc_time::chrono;

use crate::{
	err::GameplayError,
	game::{kdock::find_kdock_impl, material::deduct_material_impl, slot_item::add_slot_item_impl},
	gameplay::HasContext,
};

/// A trait for factory related gameplay.
#[async_trait]
pub trait FactoryOps {
	/// Create slot items.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The slot item manifest ID.
	/// - `consumption`: The materials consumption.
	async fn create_slotitem(
		&self,
		profile_id: i64,
		mst_id: &[i64],
		consumption: &[(MaterialCategory, i64)],
	) -> Result<(Vec<i64>, Material), GameplayError>;

	/// Create a ship.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `kdock_id`: The construction dock ID.
	/// - `mst_id`: The ship manifest ID.
	/// - `large`: Whether it is a large ship construction.
	/// - `fast`: Whether it is a high-speed construction.
	/// - `consumption`: The materials consumption.
	async fn create_ship(
		&self,
		profile_id: i64,
		kdock_id: i64,
		mst_id: i64,
		large: bool,
		fast: bool,
		consumption: &[(MaterialCategory, i64)],
	) -> Result<(), GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> FactoryOps for T {
	async fn create_slotitem(
		&self,
		profile_id: i64,
		mst_id: &[i64],
		consumption: &[(MaterialCategory, i64)],
	) -> Result<(Vec<i64>, Material), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let mut slot_ids: Vec<i64> = Vec::new();

		// deduct material consumption
		let m = deduct_material_impl(&tx, profile_id, consumption).await?;
		let m: Material = m.into();

		// add items
		for id in mst_id.iter() {
			if *id > 0 {
				let m = add_slot_item_impl(&tx, codex, profile_id, *id, 0, 0).await?;
				slot_ids.push(m.id);
			} else {
				slot_ids.push(*id);
			}
		}

		tx.commit().await?;

		Ok((slot_ids, m))
	}

	async fn create_ship(
		&self,
		profile_id: i64,
		kdock_id: i64,
		mst_id: i64,
		large: bool,
		fast: bool,
		consumption: &[(MaterialCategory, i64)],
	) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		// deduct material consumption
		deduct_material_impl(&tx, profile_id, consumption).await?;

		// create ship
		let ship_mst = codex.find::<ApiMstShip>(&mst_id)?;

		// update kdock
		let kdock = find_kdock_impl(&tx, profile_id, kdock_id).await?;
		let mut kdock_am: kdock::ActiveModel = kdock.into();
		kdock_am.ship_id = ActiveValue::Set(mst_id);

		for (category, amount) in consumption {
			match category {
				MaterialCategory::Fuel => kdock_am.fuel = ActiveValue::Set(*amount),
				MaterialCategory::Ammo => kdock_am.ammo = ActiveValue::Set(*amount),
				MaterialCategory::Steel => kdock_am.steel = ActiveValue::Set(*amount),
				MaterialCategory::Bauxite => kdock_am.bauxite = ActiveValue::Set(*amount),
				MaterialCategory::DevMat => kdock_am.devmat = ActiveValue::Set(*amount),
				_ => {}
			}
		}

		kdock_am.is_large = ActiveValue::Set(large);

		if fast {
			kdock_am.complete_time = ActiveValue::Set(None);
			kdock_am.status = ActiveValue::Set(kdock::Status::Completed);
		} else {
			let build_time_in_ms = ship_mst.api_buildtime.unwrap_or(1) * 60 * 1000;
			let complete_time =
				chrono::Utc::now() + chrono::Duration::milliseconds(build_time_in_ms);
			kdock_am.status = ActiveValue::Set(kdock::Status::Busy);
			kdock_am.complete_time = ActiveValue::Set(Some(complete_time));
		}

		kdock_am.update(&tx).await?;

		tx.commit().await?;

		Ok(())
	}
}
