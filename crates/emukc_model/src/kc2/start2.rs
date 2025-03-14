//! Models used in the infamous `start2` API endpoint.

use std::{collections::BTreeMap, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::Result;

/// KC2 Game Data Manifest
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiManifest {
	/// BGM data.
	pub api_mst_bgm: Vec<ApiMstBgm>,
	/// Constants used in the game.
	pub api_mst_const: ApiMstConst,
	/// What can be equipped in the extra slot.
	pub api_mst_equip_exslot: Vec<i64>,
	/// What can be equipped in the extra slot, but specific to ship.
	pub api_mst_equip_exslot_ship: BTreeMap<String, ApiMstEquipExslotShip>,
	/// What ship can equip.
	pub api_mst_equip_ship: Vec<ApiMstEquipShip>,
	/// Furniture data.
	pub api_mst_furniture: Vec<ApiMstFurniture>,
	/// Furniture graph data.
	pub api_mst_furnituregraph: Vec<ApiMstFurnituregraph>,
	/// Item shop data.
	pub api_mst_item_shop: ApiMstItemShop,
	/// Map area data.
	pub api_mst_maparea: Vec<ApiMstMaparea>,
	/// Map BGM data.
	pub api_mst_mapbgm: Vec<ApiMstMapbgm>,
	/// Map info data.
	pub api_mst_mapinfo: Vec<ApiMstMapinfo>,
	/// Mission data, for expeditions.
	pub api_mst_mission: Vec<ApiMstMission>,
	/// Pay item data.
	pub api_mst_payitem: Vec<ApiMstPayitem>,
	/// Ship data.
	pub api_mst_ship: Vec<ApiMstShip>,
	/// Ship graph data.
	pub api_mst_shipgraph: Vec<ApiMstShipgraph>,
	/// Ship upgrade data.
	pub api_mst_shipupgrade: Vec<ApiMstShipUpgrade>,
	/// Slot item data.
	pub api_mst_slotitem: Vec<ApiMstSlotitem>,
	/// Slot item equip type data.
	pub api_mst_slotitem_equiptype: Vec<ApiMstSlotitemEquiptype>,
	/// Ship type data.
	pub api_mst_stype: Vec<ApiMstStype>,
	/// Use item data.
	pub api_mst_useitem: Vec<ApiMstUseitem>,
}

/// BGM data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstBgm {
	/// BGM ID.
	pub api_id: i64,
	/// BGM name.
	pub api_name: String,
}

/// Constants used in the game.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstConst {
	/// Hard cap on the number of ships the player can have.
	pub api_boko_max_ships: ApiMstValue,
	/// ???.
	pub api_dpflag_quest: ApiMstValue,
	/// Hard cap on the number of quests the player can carry in parallel.
	pub api_parallel_quest_max: ApiMstValue,
}

/// Value type used in `ApiMstConst`.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstValue {
	/// Integer value.
	pub api_int_value: i64,
	/// String value.
	pub api_string_value: String,
}

/// What can be equipped in the extra slot.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstEquipExslotShip {
	/// Key: Ship family ID. value: 1 - can equip, None - cannot equip.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ctypes: Option<BTreeMap<String, i64>>,
	/// Level required to equip. 0 for no requirement.
	pub api_req_level: i64,
	/// Key: Ship ID. value: 1 - can equip, None - cannot equip.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ship_ids: Option<BTreeMap<String, i64>>,
	/// Key: Ship type ID. value: 1 - can equip, None - cannot equip.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_stypes: Option<BTreeMap<String, i64>>,
}

/// What ship can equip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstEquipShip {
	/// Ship ID.
	pub api_ship_id: i64,
	/// Equipment type ID.
	pub api_equip_type: Vec<i64>,
}

/// Furniture data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstFurniture {
	/// Furniture ID.
	pub api_id: i64,
	/// index in its category. starts from 0.
	pub api_no: i64,
	/// ???
	pub api_active_flag: i64,
	/// Description.
	pub api_description: String,
	/// ???
	pub api_outside_id: i64,
	/// Price.
	pub api_price: i64,
	/// Rarity.
	pub api_rarity: i64,
	/// Sale flag.
	pub api_saleflg: i64,
	/// Season. ???
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_season: Option<i64>,
	/// Furniture name.
	pub api_title: String,
	/// Type. 0: floor, 1: wall, 2: window, 3: wall-hanging, 4: ???, 5: desk
	pub api_type: i64,
	/// Version.
	pub api_version: i64,
}

/// Furniture graph data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstFurnituregraph {
	/// Furniture ID.
	pub api_id: i64,
	/// ???
	pub api_no: i64,
	/// ???
	pub api_filename: String,
	/// ???
	pub api_type: i64,
	/// ???
	pub api_version: String,
}

/// Item shop data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstItemShop {
	/// items in first cabinet, `ApiMstPayItem` ID.
	pub api_cabinet_1: Vec<i64>,
	/// items in second cabinet, empty slot is -1.
	pub api_cabinet_2: Vec<i64>,
}

/// Map area data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMaparea {
	/// Map area ID.
	pub api_id: i64,
	/// Map area name.
	pub api_name: String,
	/// Map area type. 0: normal, 1: event
	pub api_type: i64,
}

/// Map BGM data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMapbgm {
	/// Map BGM ID.
	pub api_id: i64,
	/// Map BGM name.
	pub api_no: i64,
	/// ???
	pub api_boss_bgm: Vec<i64>,
	/// ???
	pub api_map_bgm: Vec<i64>,
	/// ???
	pub api_maparea_id: i64,
	/// ???
	pub api_moving_bgm: i64,
}

/// Map info details.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMapinfo {
	/// Map ID.
	pub api_id: i64,
	/// Map area ID.
	pub api_maparea_id: i64,
	/// Number in the map area
	pub api_no: i64,
	/// info text
	pub api_infotext: String,
	/// item drop
	pub api_item: Vec<i64>,
	/// level required
	pub api_level: i64,
	/// map HP
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_max_maphp: Option<serde_json::Value>,
	/// map name
	pub api_name: String,
	/// operation text
	pub api_opetext: String,
	/// how many times needed to defeat the map
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_required_defeat_count: Option<i64>,
	/// fleet, combination require flags
	pub api_sally_flag: Vec<i64>,
}

/// Mission data, for expeditions.
#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMission {
	/// Mission ID.
	pub api_id: i64,
	/// Display number.
	pub api_disp_no: String,
	pub api_damage_type: i64,
	pub api_deck_num: i64,
	pub api_details: String,
	pub api_difficulty: i64,
	pub api_maparea_id: i64,
	pub api_name: String,
	pub api_reset_type: i64,
	pub api_return_flag: i64,
	pub api_sample_fleet: [i64; 6],
	pub api_time: i64,
	pub api_use_bull: f64,
	pub api_use_fuel: f64,
	pub api_win_item1: [i64; 2],
	pub api_win_item2: [i64; 2],
	pub api_win_mat_level: [i64; 4],
}

/// Pay item data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstPayitem {
	/// Pay item ID.
	pub api_id: i64,
	/// Description.
	pub api_description: String,
	/// items get after purchase
	/// [0]: fuel, [1]: ammo, [2]: steel, [3]: bauxite, [4]: instant construction, [5]: instant repair, [6]: development material, [7]: dock key
	pub api_item: [i64; 8],
	/// Name.
	pub api_name: String,
	/// Price.
	pub api_price: i64,
	/// ???
	pub api_shop_description: String,
	/// ???
	pub api_type: i64,
}

/// Ship data.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstShip {
	/// The ship's ID.
	pub api_id: i64,

	/// Ammo consumption for remodeling.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_afterbull: Option<i64>,

	/// Steel consumption for remodeling.
	/// the dev of the game has a typo here
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_afterfuel: Option<i64>,

	/// Level required for remodeling.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_afterlv: Option<i64>,

	/// The ship's ID after remodeling.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_aftershipid: Option<String>,

	/// The ship's background image
	/// 1 = blue, 2 = green, 3 = azure, 4 = silver, 5 = gold, 6 = rainbow etc.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_backs: Option<i64>,

	/// The materials reclaimed when the ship is scrapped.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_broken: Option<[i64; 4]>,

	/// The ship's build time in minutes.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_buildtime: Option<i64>,

	/// Maxium ammo consumption.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_bull_max: Option<i64>,

	/// The ship's class type.
	pub api_ctype: i64,

	/// Maxium fuel consumption.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_fuel_max: Option<i64>,

	/// Lines spoken when player gets the ship.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_getmes: Option<String>,

	/// Firepower. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_houg: Option<[i64; 2]>,

	/// Range, 0 = none, 1 = short, 2 = medium, 3 = long, 4 = very long.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_leng: Option<i64>,

	/// Luck. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_luck: Option<[i64; 2]>,

	/// Aircraft capacity for each slot.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_maxeq: Option<[i64; 5]>,

	/// The ship's name.
	pub api_name: String,

	/// Power up points provided as material for the modernization of other ships.
	/// [0]: firepower, [1]: torpedo, [2]: Anti-air, [3]: armor
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_powup: Option<[i64; 4]>,

	/// Torpedo. [0] = initial, [1] = max.
	/// `raig` is the Japanese word for `raigeki` which means lightning strike.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_raig: Option<[i64; 2]>,

	/// The number of slots for equipment.
	pub api_slot_num: i64,

	/// Speed. 0 = base, 5 = slow, 10 = fast, 15 = (fast+), 20 = (max).
	pub api_soku: i64,

	/// Sort number for port
	pub api_sort_id: i64,

	/// Sort number in picture book.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_sortno: Option<i64>,

	/// Armor. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_souk: Option<[i64; 2]>,

	/// The ship's type id, see `emukc_model::KcShipType`.
	pub api_stype: i64,

	/// Hitpoints. [0] = initial, [1] = max.
	/// `taik` is the Japanese word for `taikyuu` which means endurance.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_taik: Option<[i64; 2]>,

	/// Anti-air. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_tyku: Option<[i64; 2]>,

	/// Voice setting flag
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_voicef: Option<i64>,

	/// yomi name
	pub api_yomi: String,

	/// Anti-submarine warfare. [0] = initial.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_tais: Option<[i64; 1]>,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstShipgraph {
	pub api_id: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_battle_d: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_battle_n: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_boko_d: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_boko_n: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ensyue_n: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ensyuf_d: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ensyuf_n: Option<[i64; 2]>,
	pub api_filename: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaisyu_d: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaisyu_n: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaizo_d: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaizo_n: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_map_d: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_map_n: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_pa: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_pab: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_sortno: Option<i64>,
	pub api_version: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_weda: Option<[i64; 2]>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_wedb: Option<[i64; 2]>,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstShipUpgrade {
	pub api_id: i64,               // read as `mst_id_after` in main.js
	pub api_current_ship_id: i64,  // read as `mst_id_before` in main.js
	pub api_original_ship_id: i64, // not used in main.js
	pub api_upgrade_type: i64,
	pub api_upgrade_level: i64,
	pub api_drawing_count: i64,
	pub api_catapult_count: i64,
	pub api_report_count: i64,
	pub api_aviation_mat_count: i64,
	pub api_arms_mat_count: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_boiler_count: Option<i64>,
	pub api_sortno: i64,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstSlotitem {
	pub api_id: i64,
	pub api_atap: i64,
	pub api_bakk: i64,
	pub api_baku: i64,
	pub api_broken: [i64; 4],
	pub api_houg: i64,
	pub api_houk: i64,
	pub api_houm: i64,
	pub api_leng: i64,
	pub api_luck: i64,
	pub api_name: String,
	pub api_raig: i64,
	pub api_raik: i64,
	pub api_raim: i64,
	pub api_rare: i64,
	pub api_sakb: i64,
	pub api_saku: i64,
	pub api_soku: i64,
	pub api_sortno: i64,
	pub api_souk: i64,
	pub api_taik: i64,
	pub api_tais: i64,
	pub api_tyku: i64,
	/// [0]: category, [1]: picturebook, [2]: equiptype, [3]: category, [4]: icon
	pub api_type: [i64; 5],
	pub api_usebull: String,
	pub api_version: Option<i64>,
	pub api_cost: Option<i64>,
	pub api_distance: Option<i64>,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstSlotitemEquiptype {
	pub api_id: i64,
	pub api_name: String,
	pub api_show_flg: i64,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstStype {
	pub api_id: i64,
	pub api_equip_type: BTreeMap<String, i64>,
	pub api_kcnt: i64,
	pub api_name: String,
	pub api_scnt: i64,
	pub api_sortno: i64,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstUseitem {
	pub api_id: i64,
	pub api_category: i64,
	pub api_description: Vec<String>,
	pub api_name: String,
	pub api_price: i64,
	pub api_usetype: i64,
}

impl FromStr for ApiManifest {
	type Err = serde_json::Error;

	fn from_str(raw: &str) -> Result<Self> {
		const PREFIX: &str = "svdata=";
		let cleaned = if raw.starts_with(PREFIX) {
			raw.strip_prefix(PREFIX).unwrap_or("error while stripping svdata prefix")
		} else {
			raw
		};

		let data: ApiManifest = serde_json::from_str::<serde_json::Value>(cleaned)
			.and_then(|obj| {
				let api_data = obj.get("api_data").unwrap_or(&obj);
				ApiManifest::deserialize(api_data)
			})
			.or_else(|_| serde_json::from_str(raw))?;

		Ok(data)
	}
}

impl ApiManifest {
	/// Find a slot item by its name.
	///
	/// # Arguments
	///
	/// * `name` - The exact name of the slot item.
	///
	/// # Returns
	///
	/// A reference to `ApiMstSlotitem` if found, otherwise `None`.
	pub fn find_slotitem_by_name(&self, name: &str) -> Option<&ApiMstSlotitem> {
		self.api_mst_slotitem.iter().find(|m| m.api_name == name)
	}

	/// Find a use item by its name.
	///
	/// # Arguments
	///
	/// * `name` - The exact name of the use item.
	///
	/// # Returns
	///
	/// A reference to `ApiMstUseitem` if found, otherwise `None`.
	pub fn find_useitem_by_name(&self, name: &str) -> Option<&ApiMstUseitem> {
		self.api_mst_useitem.iter().find(|m| m.api_name == name)
	}

	/// Find a ship by its name.
	///
	/// # Arguments
	///
	/// * `name` - The exact name of the ship.
	///
	/// # Returns
	///
	/// A reference to `ApiMstShip` if found, otherwise `None`.
	pub fn find_ship_by_name(&self, name: &str) -> Option<&ApiMstShip> {
		self.api_mst_ship.iter().find(|m| m.api_name == name)
	}

	/// Find a ship by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the slot item.
	///
	/// # Returns
	///
	/// A reference to `ApiMstSlotitem` if found, otherwise `None`.
	pub fn find_slotitem(&self, id: i64) -> Option<&ApiMstSlotitem> {
		let value = self.api_mst_slotitem.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("slot item {} not found", id);
		}
		value
	}

	/// Find a slot item type by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the slot item type.
	///
	/// # Returns
	///
	/// A reference to `ApiMstSlotitemEquiptype` if found, otherwise `None`.
	pub fn find_slotitem_type(&self, id: i64) -> Option<&ApiMstSlotitemEquiptype> {
		let value = self.api_mst_slotitem_equiptype.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("slot item type {} not found", id);
		}
		value
	}

	/// Find a ship by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the ship.
	///
	/// # Returns
	///
	/// A reference to `ApiMstShip` if found, otherwise `None`.
	pub fn find_ship(&self, id: i64) -> Option<&ApiMstShip> {
		let value = self.api_mst_ship.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("ship {} not found", id);
		}
		value
	}

	/// Find a shipgraph by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the shipgraph.
	///
	/// # Returns
	///
	/// A reference to `ApiMstShipgraph` if found, otherwise `None`.
	pub fn find_shipgraph(&self, id: i64) -> Option<&ApiMstShipgraph> {
		let value = self.api_mst_shipgraph.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("shipgraph {} not found", id);
		}
		value
	}

	/// Find a furniture by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the furniture.
	///
	/// # Returns
	///
	/// A reference to `ApiMstFurniture` if found, otherwise `None`.
	pub fn find_furniture(&self, id: i64) -> Option<&ApiMstFurniture> {
		let value = self.api_mst_furniture.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("furniture {} not found", id);
		}
		value
	}

	/// Find a use item by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the use item.
	///
	/// # Returns
	///
	/// A reference to `ApiMstUseitem` if found, otherwise `None`.
	pub fn find_useitem(&self, id: i64) -> Option<&ApiMstUseitem> {
		let value = self.api_mst_useitem.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("use item {} not found", id);
		}
		value
	}

	/// Find a map area by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the map area.
	pub fn find_payitem(&self, id: i64) -> Option<&ApiMstPayitem> {
		let value = self.api_mst_payitem.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("pay item {} not found", id);
		}
		value
	}

	/// Find a ship type by its `api_id`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_id` of the ship type.
	///
	/// # Returns
	///
	/// A reference to `ApiMstStype` if found, otherwise `None`.
	pub fn find_ship_type(&self, id: i64) -> Option<&ApiMstStype> {
		let value = self.api_mst_stype.iter().find(|m| m.api_id == id);
		if value.is_none() {
			error!("ship type {} not found", id);
		}
		value
	}

	/// Find a ship class by its `api_ctype`.
	///
	/// # Arguments
	///
	/// * `id` - The `api_ctype` of the ship class.
	///
	/// # Returns
	///
	/// A reference to `ApiMstShip` if found, otherwise `None`.
	pub fn find_ship_class(&self, id: i64) -> Option<&ApiMstShip> {
		let value = self.api_mst_ship.iter().find(|s| s.api_ctype == id);
		if value.is_none() {
			error!("ship class {} not found", id);
		}
		value
	}
}
