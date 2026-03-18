//! Expedition condition models for internal use

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Expedition condition mapping table
pub type Kc3rdExpeditionConditionMap = HashMap<i64, Kc3rdExpeditionCondition>;

/// Expedition composition condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionCondition {
	/// Expedition ID
	pub api_id: i64,

	/// Expedition code (e.g., "1", "A1")
	pub code: String,

	/// Area number
	pub area: i64,

	/// Multi-language name
	pub name: Kc3rdExpeditionName,

	/// Expedition time (minutes)
	pub time_minutes: i64,

	/// Resource reward [fuel, ammo, steel, bauxite]
	pub resource_reward: [i64; 4],

	/// Item rewards
	pub item_rewards: Vec<Kc3rdExpeditionItemReward>,

	/// Admiral experience points
	pub admiral_exp: i64,

	/// Fleet experience points
	pub fleet_exp: i64,

	/// Composition requirements
	pub requirements: Kc3rdExpeditionRequirements,
}

/// Expedition item reward
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionItemReward {
	/// Item ID
	pub item_id: i64,
	/// Count
	pub count: i64,
}

/// Multi-language name
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionName {
	pub ja: String,
	pub ko: String,
	pub en: String,
	pub zh_cn: String,
	pub zh_tw: String,
}

/// Expedition composition requirements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionRequirements {
	/// Required number of ships in fleet
	pub ship_count: i64,

	/// Flagship level requirement (optional, support expeditions may not have)
	pub flagship_level: Option<i64>,

	/// Fleet total level requirement (optional)
	pub fleet_level: Option<i64>,

	/// Flagship type requirement (optional)
	pub flagship_type: Option<i64>,

	/// Composition condition (OR condition list)
	pub composition: Vec<Kc3rdCompositionAlternative>,

	/// Fleet firepower requirement (optional)
	pub total_firepower: Option<i64>,

	/// Fleet ASW requirement (optional)
	pub total_asw: Option<i64>,

	/// Fleet LOS requirement (optional)
	pub total_los: Option<i64>,

	/// Drum canister requirements (optional)
	pub drum_requirements: Option<Kc3rdDrumRequirements>,
}

/// Composition condition branch (one option in OR)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdCompositionAlternative {
	/// AND condition list (must all be satisfied)
	pub conditions: Vec<Kc3rdShipTypeRequirement>,
}

/// Ship type count requirement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipTypeRequirement {
	/// Allowed ship type ID list (OR)
	pub ship_types: Vec<i64>,
	/// Required count
	pub count: i64,
}

/// Drum canister requirements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdDrumRequirements {
	/// Number of ships carrying drum canisters
	pub ship_count: i64,
	/// Total count of drum canisters
	pub total_count: i64,
	/// Whether optional
	pub optional: bool,
}
