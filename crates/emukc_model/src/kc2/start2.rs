use std::{collections::BTreeMap, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiManifest {
	pub api_mst_bgm: Vec<ApiMstBgm>,
	pub api_mst_const: ApiMstConst,
	pub api_mst_equip_exslot: Vec<i64>,
	pub api_mst_equip_exslot_ship: BTreeMap<String, ApiMstEquipExslotShip>,
	pub api_mst_equip_ship: Vec<ApiMstEquipShip>,
	pub api_mst_furniture: Vec<ApiMstFurniture>,
	pub api_mst_furnituregraph: Vec<ApiMstFurnituregraph>,
	pub api_mst_item_shop: ApiMstItemShop,
	pub api_mst_maparea: Vec<ApiMstMaparea>,
	pub api_mst_mapbgm: Vec<ApiMstMapbgm>,
	pub api_mst_mapinfo: Vec<ApiMstMapinfo>,
	pub api_mst_mission: Vec<ApiMstMission>,
	pub api_mst_payitem: Vec<ApiMstPayitem>,
	pub api_mst_ship: Vec<ApiMstShip>,
	pub api_mst_shipgraph: Vec<ApiMstShipgraph>,
	pub api_mst_shipupgrade: Vec<ApiMstShipUpgrade>,
	pub api_mst_slotitem: Vec<ApiMstSlotitem>,
	pub api_mst_slotitem_equiptype: Vec<ApiMstSlotitemEquiptype>,
	pub api_mst_stype: Vec<ApiMstStype>,
	pub api_mst_useitem: Vec<ApiMstUseitem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstBgm {
	pub api_id: i64,
	pub api_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstConst {
	/// Hard cap on the number of ships the player can have.
	pub api_boko_max_ships: ApiMstValue,
	pub api_dpflag_quest: ApiMstValue,
	/// Hard cap on the number of quests the player can carry in parallel.
	pub api_parallel_quest_max: ApiMstValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstValue {
	pub api_int_value: i64,
	pub api_string_value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstEquipExslotShip {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ctypes: Option<BTreeMap<String, i64>>,
	pub api_req_level: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ship_ids: Option<BTreeMap<String, i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_stypes: Option<BTreeMap<String, i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstEquipShip {
	pub api_ship_id: i64,
	pub api_equip_type: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstFurniture {
	pub api_id: i64,
	pub api_no: i64,
	pub api_active_flag: i64,
	pub api_description: String,
	pub api_outside_id: i64,
	pub api_price: i64,
	pub api_rarity: i64,
	pub api_saleflg: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_season: Option<i64>,
	pub api_title: String,
	pub api_type: i64,
	pub api_version: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstFurnituregraph {
	pub api_id: i64,
	pub api_no: i64,
	pub api_filename: String,
	pub api_type: i64,
	pub api_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstItemShop {
	pub api_cabinet_1: Vec<i64>,
	pub api_cabinet_2: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMaparea {
	pub api_id: i64,
	pub api_name: String,
	pub api_type: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMapbgm {
	pub api_id: i64,
	pub api_no: i64,
	pub api_boss_bgm: Vec<i64>,
	pub api_map_bgm: Vec<i64>,
	pub api_maparea_id: i64,
	pub api_moving_bgm: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMapinfo {
	pub api_id: i64,
	pub api_no: i64,
	pub api_infotext: String,
	pub api_item: Vec<i64>,
	pub api_level: i64,
	pub api_maparea_id: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_max_maphp: Option<serde_json::Value>,
	pub api_name: String,
	pub api_opetext: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_required_defeat_count: Option<i64>,
	pub api_sally_flag: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstMission {
	pub api_id: i64,
	pub api_disp_no: String,
	pub api_damage_type: i64,
	pub api_deck_num: i64,
	pub api_details: String,
	pub api_difficulty: i64,
	pub api_maparea_id: i64,
	pub api_name: String,
	pub api_reset_type: i64,
	pub api_return_flag: i64,
	pub api_sample_fleet: Vec<i64>,
	pub api_time: i64,
	pub api_use_bull: f64,
	pub api_use_fuel: f64,
	pub api_win_item1: Vec<i64>,
	pub api_win_item2: Vec<i64>,
	pub api_win_mat_level: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstPayitem {
	pub api_id: i64,
	pub api_description: String,
	pub api_item: Vec<i64>,
	pub api_name: String,
	pub api_price: i64,
	pub api_shop_description: String,
	pub api_type: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
	pub api_broken: Option<Vec<i64>>,

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
	pub api_houg: Option<Vec<i64>>,

	/// Range, 0 = none, 1 = short, 2 = medium, 3 = long, 4 = very long.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_leng: Option<i64>,

	/// Luck. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_luck: Option<Vec<i64>>,

	/// Aircraft capacity for each slot.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_maxeq: Option<Vec<i64>>,

	/// The ship's name.
	pub api_name: String,

	/// Power up points provided as material for the modernization of other ships.
	/// [0]: firepower, [1]: torpedo, [2]: Anti-air, [3]: armor
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_powup: Option<Vec<i64>>,

	/// Torpedo. [0] = initial, [1] = max.
	/// `raig` is the Japanese word for `raigeki` which means lightning strike.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_raig: Option<Vec<i64>>,

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
	pub api_souk: Option<Vec<i64>>,

	/// The ship's type id, see `emukc_model::KcShipType`.
	pub api_stype: i64,

	/// Hitpoints. [0] = initial, [1] = max.
	/// `taik` is the Japanese word for `taikyuu` which means endurance.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_taik: Option<Vec<i64>>,

	/// Anti-air. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_tyku: Option<Vec<i64>>,

	/// Voice setting flag
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_voicef: Option<i64>,

	/// yomi name
	pub api_yomi: String,

	/// Anti-submarine warfare. [0] = initial, [1] = max.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_tais: Option<Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstShipgraph {
	pub api_id: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_battle_d: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_battle_n: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_boko_d: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_boko_n: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ensyue_n: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ensyuf_d: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ensyuf_n: Option<Vec<i64>>,
	pub api_filename: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaisyu_d: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaisyu_n: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaizo_d: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaizo_n: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_map_d: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_map_n: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_pa: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_pab: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_sortno: Option<i64>,
	pub api_version: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_weda: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_wedb: Option<Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstSlotitem {
	pub api_id: i64,
	pub api_atap: i64,
	pub api_bakk: i64,
	pub api_baku: i64,
	pub api_broken: Vec<i64>,
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
	pub api_type: Vec<i64>,
	pub api_usebull: String,
	pub api_version: Option<i64>,
	pub api_cost: Option<i64>,
	pub api_distance: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstSlotitemEquiptype {
	pub api_id: i64,
	pub api_name: String,
	pub api_show_flg: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMstStype {
	pub api_id: i64,
	pub api_equip_type: BTreeMap<String, i64>,
	pub api_kcnt: i64,
	pub api_name: String,
	pub api_scnt: i64,
	pub api_sortno: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

		let data: ApiManifest = if let Ok(obj) = serde_json::from_str::<serde_json::Value>(cleaned)
		{
			if let Some(api_data) = obj.get("api_data") {
				ApiManifest::deserialize(api_data)?
			} else {
				ApiManifest::deserialize(&obj)?
			}
		} else {
			serde_json::from_str(raw)?
		};

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
