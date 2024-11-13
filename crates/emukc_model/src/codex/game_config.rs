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

/// Game configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameConfig {
	/// Material configuration.
	pub material: MaterialConfig,

	/// Picture book for ships.
	pub picturebook: PicturebookConfig,
}
