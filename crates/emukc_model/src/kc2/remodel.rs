use serde::{Deserialize, Serialize};

/// A ship remodel requirement.
/// Extracted from the `main.js`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcShipRemodelRequirement {
	/// ship id before remodel
	pub id_from: i64,

	/// ship id after remodel
	pub id_to: i64,

	/// ammo consumption, `api_afterbull`
	pub ammo: i64,

	/// steel consumption, `api_afterfuel`
	pub steel: i64,

	/// `Blueprint` consumption
	pub drawing: i64,

	/// `ProtoCatapult` consumption
	pub catapult: i64,

	/// `ActionReport` consumption
	pub report: i64,

	/// `DevMaterial` consumption
	pub devmat: i64,

	/// `Torch` comsumption
	pub torch: i64,

	/// `NewAviationMaterial` consumption
	pub aviation: i64,

	/// `NewArtilleryMaterial` consumption
	pub artillery: i64,

	/// `NewArmamentMaterial` consumption
	pub arms: i64,

	/// `Boiler` consumption
	pub boiler: i64,
}

/// A map of ship remodel requirements.
///
/// The key is a tuple of `(id_from, id_to)`.
pub type KcShipRemodelRequirementMap =
	std::collections::BTreeMap<(i64, i64), KcShipRemodelRequirement>;
