//! The compose here means this trait is a composition of other gameplay logics.

use async_trait::async_trait;
use remodel::remodel_impl;
use slot_deprive::slot_deprive_impl;
use slot_exchange::slot_exchange_impl;
use std::collections::BTreeMap;

use emukc_db::{
	entity::profile::ship,
	sea_orm::{TransactionTrait, entity::prelude::*},
};
use emukc_model::{
	kc2::{KcApiChargeKind, KcApiChargeResp},
	profile::fleet::Fleet,
};
use supply::supply_fleet_impl;

use crate::{
	err::GameplayError, game::slot_item::get_unset_slot_items_by_types_impl, gameplay::HasContext,
};

use super::fleet::get_fleets_impl;

pub(crate) mod marriage;
pub(crate) mod powerup;
pub(crate) mod remodel;
pub(crate) mod slot_deprive;
pub(crate) mod slot_exchange;
pub(crate) mod supply;

#[derive(Debug, Clone)]
pub struct PowerupResp {
	pub success: bool,
	pub ship: ship::Model,
	pub fleets: Vec<Fleet>,
	pub unset_slot_items: Option<BTreeMap<i64, Vec<i64>>>,
}

#[derive(Debug, Clone, Copy)]
pub struct SlotDepriveParams {
	pub from_ship_id: i64,
	pub to_ship_id: i64,
	pub from_ex_slot: bool,
	pub to_ex_slot: bool,
	pub from_slot_idx: i64,
	pub to_slot_idx: i64,
}

#[derive(Debug, Clone)]
pub struct SlotDepriveResp {
	pub from_ship: ship::Model,
	pub to_ship: ship::Model,
	pub unset_type3: Option<i64>,
	pub unset_id_list: Option<Vec<i64>>,
	pub bauxite: i64,
}

/// A trait for gameplay logic that composed by one or more other trait implements.
#[async_trait]
pub trait ComposeOps {
	/// Execute a resupply operation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ship_ids`: The ship IDs.
	/// - `mode`: The resupply mode.
	/// - `supply_aircrafts`: Whether to resupply aircrafts.
	async fn charge_supply(
		&self,
		profile_id: i64,
		ship_ids: &[i64],
		mode: KcApiChargeKind,
		supply_aircrafts: bool,
	) -> Result<KcApiChargeResp, GameplayError>;

	/// Execute a marriage operation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ship_id`: The ship ID.
	async fn marriage(&self, profile_id: i64, ship_id: i64) -> Result<ship::Model, GameplayError>;

	/// Execute a powerup operation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ship_id`: The ship ID.
	/// - `material_ships`: The material ship IDs.
	/// - `keep_slot_items`: Whether to keep slot items.
	async fn powerup(
		&self,
		profile_id: i64,
		ship_id: i64,
		material_ships: &[i64],
		keep_slot_items: bool,
	) -> Result<PowerupResp, GameplayError>;

	/// Execute a remodel operation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `ship_id`: The ship ID.
	async fn remodel(&self, profile_id: i64, ship_id: i64) -> Result<(), GameplayError>;

	/// Execute a slot deprive operation.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `params`: The slot deprive parameters.
	async fn slot_deprive(
		&self,
		profile_id: i64,
		params: &SlotDepriveParams,
	) -> Result<SlotDepriveResp, GameplayError>;

	/// Execute a slot exchange operation.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship ID.
	/// - `src_idx`: The source slot index.
	/// - `dst_idx`: The target slot index.
	async fn slot_exchange(
		&self,
		ship_id: i64,
		src_idx: i64,
		dst_idx: i64,
	) -> Result<ship::Model, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> ComposeOps for T {
	async fn charge_supply(
		&self,
		profile_id: i64,
		ship_ids: &[i64],
		mode: KcApiChargeKind,
		supply_aircrafts: bool,
	) -> Result<KcApiChargeResp, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let resp =
			supply_fleet_impl(&tx, codex, profile_id, ship_ids, mode, supply_aircrafts).await?;
		tx.commit().await?;

		Ok(resp)
	}

	async fn marriage(&self, profile_id: i64, ship_id: i64) -> Result<ship::Model, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let ship = marriage::marriage_impl(&tx, codex, profile_id, ship_id).await?;
		tx.commit().await?;

		Ok(ship)
	}

	async fn powerup(
		&self,
		profile_id: i64,
		ship_id: i64,
		material_ships: &[i64],
		keep_slot_items: bool,
	) -> Result<PowerupResp, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let result =
			powerup::powerup_impl(&tx, codex, profile_id, ship_id, material_ships, keep_slot_items)
				.await?;
		tx.commit().await?;

		let fleets = get_fleets_impl(db, profile_id).await?;

		let unset_slot_items = if let Some(item_types) = result.unset_slot_item_types {
			let types: Vec<i64> = item_types.iter().copied().collect();
			let unset_slot_items =
				get_unset_slot_items_by_types_impl(db, profile_id, &types).await?;
			Some(unset_slot_items)
		} else {
			None
		};

		Ok(PowerupResp {
			success: result.success,
			ship: result.ship.unwrap(),
			fleets: fleets.into_iter().map(Into::into).collect(),
			unset_slot_items,
		})
	}

	async fn remodel(&self, profile_id: i64, ship_id: i64) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		remodel_impl(&tx, codex, profile_id, ship_id).await?;
		tx.commit().await?;

		Ok(())
	}

	async fn slot_deprive(
		&self,
		profile_id: i64,
		params: &SlotDepriveParams,
	) -> Result<SlotDepriveResp, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let resp = slot_deprive_impl(&tx, codex, profile_id, params).await?;
		tx.commit().await?;

		Ok(resp)
	}

	async fn slot_exchange(
		&self,
		ship_id: i64,
		src_idx: i64,
		dst_idx: i64,
	) -> Result<ship::Model, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let ship = slot_exchange_impl(&tx, ship_id, src_idx, dst_idx).await?;

		tx.commit().await?;

		Ok(ship)
	}
}
