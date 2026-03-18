//! KCanotify expedition data types

use serde::{Deserialize, Serialize};

/// KCanotify expedition data root structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KCanotifyExpedition {
	/// Expedition number (1-46, 100-115, etc.)
	#[serde(rename = "no")]
	pub id: String,

	/// Expedition code (e.g., "1", "A1", "B2")
	pub code: String,

	/// Area number (1-7, 99)
	pub area: i64,

	/// Multi-language name
	pub name: KCanotifyExpeditionName,

	/// Expedition time (minutes)
	pub time: i64,

	/// Resource reward [fuel, ammo, steel, bauxite]
	pub resource: [i64; 4],

	/// Item rewards [[item_id, count], ...]
	pub reward: Vec<[i64; 2]>,

	/// Experience points [admiral_exp, fleet_exp]
	pub exp: [i64; 2],

	/// Required number of ships in fleet
	#[serde(rename = "total-num")]
	pub total_num: i64,

	/// Flagship level requirement (optional, support expeditions may not have)
	#[serde(rename = "flag-lv")]
	pub flagship_level: Option<i64>,

	/// Fleet total level requirement (optional)
	#[serde(rename = "total-lv")]
	pub total_level: Option<i64>,

	/// Flagship type requirement (optional)
	#[serde(rename = "flag-cond")]
	pub flagship_type: Option<String>,

	/// Composition condition expression (optional)
	#[serde(rename = "total-cond")]
	pub total_condition: Option<String>,

	/// Fleet firepower requirement (optional)
	#[serde(rename = "total-firepower")]
	pub total_firepower: Option<i64>,

	/// Fleet firepower requirement alias (optional)
	#[serde(rename = "total-fp")]
	pub total_fp: Option<i64>,

	/// Fleet ASW requirement (optional)
	#[serde(rename = "total-asw")]
	pub total_asw: Option<i64>,

	/// Fleet LOS requirement (optional)
	#[serde(rename = "total-los")]
	pub total_los: Option<i64>,

	/// Number of ships carrying drum canisters requirement (optional)
	#[serde(rename = "drum-ship")]
	pub drum_ship: Option<i64>,

	/// Total drum canisters requirement (optional)
	#[serde(rename = "drum-num")]
	pub drum_num: Option<i64>,

	/// Drum canister count requirement (optional, used for expedition 24, etc.)
	#[serde(rename = "drum-num-optional")]
	pub drum_num_optional: Option<i64>,
}

/// Multi-language expedition name
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KCanotifyExpeditionName {
	pub jp: String,
	pub ko: String,
	pub en: String,
	pub scn: String,
	pub tcn: String,
}
