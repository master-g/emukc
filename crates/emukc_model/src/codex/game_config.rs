//! Game configuration.

use serde::{Deserialize, Serialize};

use crate::profile::material::MaterialConfig;

/// Picture book configuration
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PicturebookConfig {
	/// unveiled all ships in picturebook
	pub unlock_all_ships: bool,

	/// unveiled all equipments in picturebook
	pub unlock_all_slotitems: bool,
}

/// Repair config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockingConfig {
	/// Repair time factor
	pub time_factor: f64,

	/// Repair cost factor
	pub cost_factor: f64,
}

impl Default for DockingConfig {
	fn default() -> Self {
		Self {
			time_factor: 1.0,
			cost_factor: 1.0,
		}
	}
}

/// Game configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameConfig {
	/// Material configuration.
	pub material: MaterialConfig,

	/// Picture book for ships.
	pub picturebook: PicturebookConfig,

	/// Docking configuration.
	pub docking: DockingConfig,
}
