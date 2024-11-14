//! Calculate ship repairation info.

use crate::{kc2::KcShipType, prelude::ApiMstShip};

use super::{Codex, CodexError};

/// Ship repairation cost.
#[derive(Debug, Clone, Copy)]
pub struct RepairCost {
	/// Duration in seconds.
	pub duration_sec: i64,

	/// Fuel cost.
	pub fuel_cost: i64,

	/// Steel cost.
	pub steel_cost: i64,
}

impl Codex {
	/// Calculate ship repairation info.
	///
	/// # Arguments
	///
	/// * `mst_id` - Ship master ID.
	/// * `lv` - Ship level.
	/// * `hp_lost` - HP lost.
	pub fn cal_ship_docking_cost(
		&self,
		ship_mst: &ApiMstShip,
		lv: i64,
		hp_lost: i64,
	) -> Result<RepairCost, CodexError> {
		if hp_lost <= 0 {
			return Ok(RepairCost {
				duration_sec: 0,
				fuel_cost: 0,
				steel_cost: 0,
			});
		}

		let time_base = if lv < 12 {
			(lv as f64) * 10.0
		} else {
			(lv as f64) * 5.0 + (lv as f64 - 11.0).sqrt().floor() * 10.0 + 50.0
		};

		let ship_type = KcShipType::n(ship_mst.api_stype).ok_or_else(|| {
			CodexError::NotFound(format!("Invalid ship type: {}", ship_mst.api_stype))
		})?;
		let ship_type_mod = match ship_type {
			KcShipType::BB
			| KcShipType::BBV
			| KcShipType::CV
			| KcShipType::CVB
			| KcShipType::AR => 2.0,
			KcShipType::CA
			| KcShipType::CAV
			| KcShipType::FBB
			| KcShipType::CVL
			| KcShipType::AS => 1.5,
			KcShipType::CL
			| KcShipType::CLT
			| KcShipType::CT
			| KcShipType::DD
			| KcShipType::SSV
			| KcShipType::AV
			| KcShipType::LHA => 1.0,
			KcShipType::SS | KcShipType::DE => 0.5,
			_ => 0.1,
		};

		let duration_sec =
			(hp_lost as f64) * time_base * ship_type_mod * self.game_cfg.docking.time_factor + 30.0;

		let fuel_max = ship_mst.api_fuel_max.unwrap_or(0) as f64;
		let fuel_cost = (hp_lost as f64) * fuel_max * 0.032;
		let steel_cost = (hp_lost as f64) * 0.06;

		let cost = RepairCost {
			duration_sec: duration_sec.floor() as i64,
			fuel_cost: fuel_cost.floor() as i64,
			steel_cost: steel_cost.floor() as i64,
		};
		debug!("Repair cost: {:?}", cost);

		Ok(cost)
	}
}
