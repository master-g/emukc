use async_trait::async_trait;
use emukc_db::{
	entity::profile::{kdock, ship},
	sea_orm::{ActiveValue, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
	kc2::{KcApiShip, KcApiSlotItem, MaterialCategory},
	prelude::ApiMstShip,
	profile::{material::Material, slot_item::SlotItem},
};
use emukc_time::chrono;

use crate::{
	err::GameplayError,
	game::{
		kdock::find_kdock_impl,
		material::{add_material_impl, deduct_material_impl},
		slot_item::{add_slot_item_impl, destroy_items_impl},
	},
	gameplay::HasContext,
};

use super::{ship::add_ship_impl, slot_item::find_slot_item_impl};

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

	/// High-speed construction.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `kdock_id`: The construction dock ID.
	async fn speed_up_ship_construction(
		&self,
		profile_id: i64,
		kdock_id: i64,
	) -> Result<(), GameplayError>;

	/// Destroy a ship.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ship_id`: The ship ID.
	/// - `keep_equipment`: Whether to keep the equipment.
	async fn destroy_ship(
		&self,
		profile_id: i64,
		ship_id: i64,
		keep_equipment: bool,
	) -> Result<(), GameplayError>;

	/// Complete ship construction.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `kdock_id`: The construction dock ID.
	async fn complete_ship_construction(
		&self,
		profile_id: i64,
		kdock_id: i64,
	) -> Result<(KcApiShip, Vec<KcApiSlotItem>), GameplayError>;
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

	async fn speed_up_ship_construction(
		&self,
		profile_id: i64,
		kdock_id: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let kdock = find_kdock_impl(&tx, profile_id, kdock_id).await?;

		let torch_cost = if kdock.is_large {
			10
		} else {
			1
		};

		// deduct material first
		deduct_material_impl(&tx, profile_id, [(MaterialCategory::Torch, torch_cost)].as_slice())
			.await?;

		// change kdock status

		let mut kdock_am: kdock::ActiveModel = kdock.into();
		kdock_am.complete_time = ActiveValue::Set(None);
		kdock_am.status = ActiveValue::Set(kdock::Status::Completed);

		kdock_am.update(&tx).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn destroy_ship(
		&self,
		profile_id: i64,
		ship_id: i64,
		keep_equipment: bool,
	) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let ship_model = ship::Entity::find_by_id(ship_id)
			.one(&tx)
			.await?
			.ok_or_else(|| GameplayError::EntryNotFound(format!("ship {ship_id} not found")))?;

		let ship_mst = codex.find::<ApiMstShip>(&ship_model.mst_id)?;

		if !keep_equipment {
			let slot_ids: Vec<i64> = [
				ship_model.slot_1,
				ship_model.slot_2,
				ship_model.slot_3,
				ship_model.slot_4,
				ship_model.slot_5,
				ship_model.slot_ex,
			]
			.iter()
			.filter_map(|&id| {
				if id > 0 {
					Some(id)
				} else {
					None
				}
			})
			.collect();

			destroy_items_impl(&tx, codex, profile_id, &slot_ids).await?;
		}

		let mut scrap_materials = [
			(MaterialCategory::Fuel, 0),
			(MaterialCategory::Ammo, 0),
			(MaterialCategory::Steel, 0),
			(MaterialCategory::Bauxite, 0),
		];

		if let Some(broken) = ship_mst.api_broken {
			for (i, amount) in broken.iter().enumerate() {
				scrap_materials[i].1 += amount;
			}
		}

		add_material_impl(&tx, codex, profile_id, scrap_materials.as_slice()).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn complete_ship_construction(
		&self,
		profile_id: i64,
		kdock_id: i64,
	) -> Result<(KcApiShip, Vec<KcApiSlotItem>), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		// find the construction dock
		let kdock = find_kdock_impl(&tx, profile_id, kdock_id).await?;
		let ship_id = kdock.ship_id;

		// add ship to profile
		let (_, ship) = add_ship_impl(&tx, codex, profile_id, ship_id).await?;
		let slot_item_ids: Vec<i64> = ship
			.api_slot
			.iter()
			.filter_map(|id| {
				if *id > 0 {
					Some(*id)
				} else {
					None
				}
			})
			.collect();

		// find slot items
		let mut slot_items: Vec<KcApiSlotItem> = Vec::new();
		for id in slot_item_ids.iter() {
			let slot_item = find_slot_item_impl(&tx, *id).await?;
			let slot_item: SlotItem = slot_item.into();
			let slot_item: KcApiSlotItem = slot_item.into();
			slot_items.push(slot_item);
		}

		// reset construction dock
		let mut kdock_am: kdock::ActiveModel = kdock.into();
		kdock_am.status = ActiveValue::Set(kdock::Status::Idle);
		kdock_am.ship_id = ActiveValue::Set(0);
		kdock_am.complete_time = ActiveValue::Set(None);
		kdock_am.is_large = ActiveValue::Set(false);
		kdock_am.fuel = ActiveValue::Set(0);
		kdock_am.ammo = ActiveValue::Set(0);
		kdock_am.steel = ActiveValue::Set(0);
		kdock_am.bauxite = ActiveValue::Set(0);
		kdock_am.devmat = ActiveValue::Set(0);

		kdock_am.update(&tx).await?;

		// commit transaction
		tx.commit().await?;

		Ok((ship, slot_items))
	}
}
