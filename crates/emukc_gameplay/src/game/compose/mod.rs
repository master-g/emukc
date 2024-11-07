//! The compose here means this trait is a composition of other gameplay logics.

use async_trait::async_trait;
use emukc_db::{
	entity::profile::ship,
	sea_orm::{entity::prelude::*, TransactionTrait},
};
use emukc_model::kc2::{KcApiChargeKind, KcApiChargeResp};
use supply::supply_fleet_impl;

use crate::{err::GameplayError, gameplay::HasContext};

pub(crate) mod marriage;
pub(crate) mod supply;

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
}
