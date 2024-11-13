use serde::{Deserialize, Serialize};

/// Ship extra information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipPicturebookInfo {
	/// `api_id`, ship id
	pub api_id: i64,

	/// ship info in picture book
	pub info: String,
}

/// Ship class name information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipClassNameInfo {
	/// `api_id`, ship class id
	pub api_id: i64,

	/// ship class name
	pub name: String,
}

/// Ship extra information map
pub type Kc3rdShipPicturebookInfoMap = std::collections::BTreeMap<i64, Kc3rdShipPicturebookInfo>;

/// Ship class name information map
pub type Kc3rdShipClassNameMap = std::collections::BTreeMap<i64, Kc3rdShipClassNameInfo>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipSlotInfo {
	/// how many plane the slot can hold
	pub onslot: i64,

	/// initial equipment manifest id
	pub item_id: i64,

	/// initial equipment level
	pub stars: i64,
}

/// Requirement needed for remodeling to this ship
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipRemodelRequirement {
	/// level requirement
	pub level: i64,

	/// ammo consumption, `api_afterbull`
	pub ammo: i64,

	/// steel consumption, `api_afterfuel`
	pub steel: i64,

	/// `Blueprint` consumption
	pub blueprint: i64,

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
	pub armament: i64,

	/// `Boiler` consumption
	pub boiler: i64,

	/// `OverseasWarshipTechnology` consumption
	pub overseas: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShip {
	/// `api_id`, ship manifest id
	pub api_id: i64,

	/// kaihi, evasion
	pub kaih: [i64; 2],

	/// taisen, aws (anti-submarine warfare)
	pub tais: [i64; 2],

	/// sakuteki, los (line of sight)
	pub saku: [i64; 2],

	/// luck
	pub luck: [i64; 2],

	/// luck bonus when used as material ship in modernization
	pub luck_bonus: f64,

	/// armor bonus when used as material ship in modernization
	pub armor_bonus: i64,

	/// cnum, construction number or class number
	pub cnum: i64,

	/// is buildable
	pub buildable: bool,

	/// is buildable in LSC (Large Ship Construction)
	pub buildable_lsc: bool,

	/// `slots.len()`: how many slots the ship has
	/// `slots[n]`: how many aircraft the n-th slot can hold
	pub slots: Vec<Kc3rdShipSlotInfo>,

	/// requirement to remodel to this ship
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remodel: Option<Kc3rdShipRemodelRequirement>,

	/// remodel to previous ship, `api_id` of the previous ship
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remodel_back_to: Option<i64>,

	/// requirement to remodel back to previous ship
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remodel_back_requirement: Option<Kc3rdShipRemodelRequirement>,
}

/// Third party ship extra information map
pub type Kc3rdShipMap = std::collections::BTreeMap<i64, Kc3rdShip>;
