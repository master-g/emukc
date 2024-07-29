pub mod quest;

use emukc_crypto::SimpleHash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

use crate::UserOwnedFurniture;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiUserBasic {
	/// User ID
	pub api_member_id: i64,
	/// User nickname
	pub api_nickname: String,
	/// Hash value of the user nickname
	pub api_nickname_id: String,
	/// ???, always 1, not used in main.js
	pub api_active_flag: i64,
	/// login timestamp of this session, in milliseconds
	pub api_starttime: i64,
	/// Command HQ level
	pub api_level: i64,
	/// User HQ rank, see `UserHQRank`
	pub api_rank: i64,
	/// User experience
	pub api_experience: i64,
	/// ???, not used
	pub api_fleetname: Option<String>,
	/// User comment
	pub api_comment: String,
	/// Hash value of the user comment
	pub api_comment_id: String,
	/// Kanmusu maximum capacity
	pub api_max_chara: i64,
	/// Slotitem maximum capacity
	pub api_max_slotitem: i64,
	/// ???, Not used even in main.js
	pub api_max_kagu: i64,
	/// ???, always 0, not used in main.js
	pub api_playtime: i64,
	/// ???, always 0, not used in main.js
	pub api_tutorial: i64,
	/// Furniture setting, [0]: floor, [1]: wallpaper, [2]: window, [3]: wallhanging, [4]: shelf, [5]: desk
	pub api_furniture: Vec<i64>,
	/// Deck count
	pub api_count_deck: i64,
	/// Construction dock count
	pub api_count_kdock: i64,
	/// Repair dock count
	pub api_count_ndock: i64,
	/// Furniture coin amount, value read from the `FCoin` use item
	pub api_fcoin: i64,
	/// Sortie win count
	pub api_st_win: i64,
	/// Sortie lose count
	pub api_st_lose: i64,
	/// Mission total count
	pub api_ms_count: i64,
	/// Mission success count
	pub api_ms_success: i64,
	/// Practice win count
	pub api_pt_win: i64,
	/// Practice lose count
	pub api_pt_lose: i64,
	/// Practice challenged count
	pub api_pt_challenged: i64,
	/// Practice challenged win count
	pub api_pt_challenged_win: i64,
	/// New player flag, 0: new player, 1: old player
	pub api_firstflag: i64,
	/// Tutorial progress register
	pub api_tutorial_progress: i64,
	/// ???, always [0, 0], not used in main.js
	pub api_pvp: Vec<i64>,
	/// Number of class A medal obtained
	pub api_medals: i64,
	/// Large construction dock unlock flag, not exist in origin API, 0: locked, 1: unlocked
	pub api_large_dock: i64,
	/// Maxium parallel quest count, not exist in origin API
	pub api_max_quests: i64,
	/// Extra supply enable flag, [0] expendition, [1] battle
	pub api_extra_supply: Vec<i64>,
	/// War result points, not exist in origin API
	pub api_war_result: i64,
}

impl Default for KcApiUserBasic {
	fn default() -> Self {
		Self {
			api_member_id: Default::default(),
			api_nickname: Default::default(),
			api_nickname_id: Default::default(),
			api_active_flag: 1,
			api_starttime: 0,
			api_level: 1,
			api_rank: 10,
			api_experience: 0,
			api_fleetname: None,
			api_comment: Default::default(),
			api_comment_id: Default::default(),
			api_max_chara: 100,
			api_max_slotitem: 497,
			api_max_kagu: 0,
			api_playtime: 0,
			api_tutorial: 0,
			api_furniture: UserOwnedFurniture::default().records,
			api_count_deck: 1,
			api_count_kdock: 2,
			api_count_ndock: 2,
			api_fcoin: 0,
			api_st_win: 0,
			api_st_lose: 0,
			api_ms_count: 0,
			api_ms_success: 0,
			api_pt_win: 0,
			api_pt_lose: 0,
			api_pt_challenged: 0,
			api_pt_challenged_win: 0,
			api_firstflag: 0,
			api_tutorial_progress: 0,
			api_pvp: vec![0, 0],
			api_medals: 0,
			api_large_dock: 0,
			api_max_quests: 3,
			api_extra_supply: vec![0, 0],
			api_war_result: 0,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum MaterialCategory {
	/// 燃料
	Fuel = 1,
	/// 弾薬
	Ammo = 2,
	/// 鋼材
	Steel = 3,
	/// ボーキサイト
	Bauxite = 4,
	/// 高速建造材
	Torch = 5,
	/// 高速修復材
	Bucket = 6,
	/// 開発資材
	DevMat = 7,
	/// 改修資材
	Screw = 8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiMaterialElement {
	/// User ID
	pub api_member_id: i64,
	/// Material ID, see `MaterialCategory`
	pub api_id: i64,
	/// Amount
	pub api_value: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiSlotItem {
	pub api_id: i64,
	pub api_slotitem_id: i64,
	pub api_locked: i64,
	pub api_level: i64,
	/// Airplane lv, exists only if greater than 0
	pub api_alv: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiShip {
	pub api_id: i64,
	pub api_sortno: i64,
	pub api_ship_id: i64,
	pub api_lv: i64,
	pub api_exp: Vec<i64>,
	pub api_nowhp: i64,
	pub api_maxhp: i64,
	pub api_soku: i64,
	pub api_leng: i64,
	pub api_slot: Vec<i64>,
	pub api_onslot: Vec<i64>,
	pub api_slot_ex: i64,
	pub api_kyouka: Vec<i64>,
	pub api_backs: i64,
	pub api_fuel: i64,
	pub api_bull: i64,
	pub api_slotnum: i64,
	pub api_ndock_time: i64,
	pub api_ndock_item: Vec<i64>,
	pub api_srate: i64,
	pub api_cond: i64,
	pub api_karyoku: Vec<i64>,
	pub api_raisou: Vec<i64>,
	pub api_taiku: Vec<i64>,
	pub api_soukou: Vec<i64>,
	pub api_kaihi: Vec<i64>,
	pub api_taisen: Vec<i64>,
	pub api_sakuteki: Vec<i64>,
	pub api_lucky: Vec<i64>,
	pub api_locked: i64,
	pub api_locked_equip: i64,
	pub api_sally_area: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiPresetDeckElement {
	pub api_preset_no: i64,
	pub api_name: String,
	pub api_name_id: String,
	pub api_ship: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiPresetDeck {
	pub api_max_num: i64,
	pub api_deck: BTreeMap<String, KcApiPresetDeckElement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiPresetSlotItemElement {
	pub api_id: i64,
	pub api_level: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiPresetSlotElement {
	pub api_preset_no: i64,
	pub api_name: String,
	pub api_selected_mode: i64,
	pub api_lock_flag: i64,
	pub api_slot_ex_flag: i64,
	pub api_slot_item: Vec<KcApiPresetSlotItemElement>,
	pub api_slot_item_ex: Option<KcApiPresetSlotItemElement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiPresetSlot {
	pub api_max_num: i64,
	pub api_preset_items: Vec<KcApiPresetSlotElement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiDeckPort {
	pub api_member_id: i64,
	pub api_id: i64,
	pub api_name: String,
	pub api_name_id: String,
	pub api_mission: Vec<i64>,
	pub api_flagship: String,
	pub api_ship: Vec<i64>,
}

#[derive(Error, Debug)]
pub enum KcApiDeckPortError {
	/// Invalid deck port ID
	#[error("invalid deck port id: {0}")]
	InvalidDeckPortId(i64),
}

impl KcApiDeckPort {
	pub fn new(uid: i64, id: i64) -> Result<Self, KcApiDeckPortError> {
		if !(1..=4).contains(&id) {
			return Err(KcApiDeckPortError::InvalidDeckPortId(id));
		}

		let api_ship = if id == 1 {
			vec![1, -1, -1, -1, -1, -1]
		} else {
			vec![-1; 6]
		};

		let name = format!("\u{7b2c} {} \u{8266}\u{968a}", id);
		let name_id = name.simple_hash();

		Ok(Self {
			api_member_id: uid,
			api_id: id,
			api_name: name,
			api_name_id: name_id,
			api_mission: vec![0; 4],
			api_flagship: "0".to_string(),
			api_ship,
		})
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiKDock {
	/// Construction Dock ID
	pub api_id: i64,
	/// Dock state, -1: locked, 0: empty, 2: building, 3: complete
	pub api_state: i64,
	/// Ship Manifest ID
	pub api_created_ship_id: i64,
	/// Complete time, in milliseconds
	pub api_complete_time: i64,
	/// Complete time, in readable string
	pub api_complete_time_str: String,
	/// Fuel consumed
	pub api_item1: i64,
	/// Ammo consumed
	pub api_item2: i64,
	/// Steel consumed
	pub api_item3: i64,
	/// Bauxite consumed
	pub api_item4: i64,
	/// Development material consumed
	pub api_item5: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiNDock {
	pub api_member_id: i64,
	/// Repair Dock ID
	pub api_id: i64,
	/// Dock state, -1: locked, 0: empty, 1: repairing
	pub api_state: i64,
	/// Ship Instance ID
	pub api_ship_id: i64,
	/// Complete time, in milliseconds
	pub api_complete_time: i64,
	/// Complete time, in readable string
	pub api_complete_time_str: String,
	/// Fuel consumed
	pub api_item1: i64,
	/// Ammo consumed
	pub api_item2: i64,
	/// Steel consumed
	pub api_item3: i64,
	/// Bauxite consumed
	pub api_item4: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiGameSetting {
	/// Language type, 0: Japanese, 1: English
	pub api_language_type: i64,
	/// Ship sorting filters, 0: BB, 1: CV, 2: CA, 3: CL, 4: DD, 5: DE, 6: SS, 7: other
	pub api_oss_items: Vec<i64>,
	/// Secretary ship position
	pub api_position_id: i64,
	/// UI skin ID
	pub api_skin_id: i64,
	/// Port music ID
	pub api_p_bgm_id: i64,
	/// Call for reinforcement flag, 0: off, 1: on
	pub api_friend_fleet_request_flag: i64,
	/// Type of reinforcement called
	pub api_friend_fleet_request_type: i64,
}

impl Default for KcApiGameSetting {
	fn default() -> Self {
		Self {
			api_language_type: 0,
			api_oss_items: vec![],
			api_position_id: 0,
			api_skin_id: 101,
			api_p_bgm_id: 101,
			api_friend_fleet_request_flag: 0,
			api_friend_fleet_request_type: 0,
		}
	}
}

/// User item, include use item and pay item
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiUserItem {
	pub api_id: i64,
	pub api_count: i64,
}

/// User map progress
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiMapRecord {
	pub api_id: i64,
	pub api_cleared: i64,
	pub api_defeat_count: Option<i64>,
	pub api_now_maphp: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KcApiIncentive {
	pub api_count: i64,
	pub api_item: Option<Vec<KcApiIncentiveItem>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum KcApiIncentiveMode {
	PreRegister = 1,
	Reception = 2,
	MonthlyOrPresent = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum KcApiIncentiveType {
	Ship = 1,
	SlotItem = 2,
	UseItem = 3,
	Resource = 4,
	Furniture = 5,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KcApiIncentiveItem {
	pub api_mode: i64,
	pub api_type: i64,
	pub api_mst_id: i64,
	pub api_getmes: Option<String>,
	pub api_slotitem_level: Option<i64>,
}

pub type KcApiUnsetSlot = BTreeMap<String, Vec<i64>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct KcApiFurniture {
	pub api_id: i64,
	pub api_furniture_type: i64,
	pub api_furniture_no: i64,
	pub api_furniture_id: i64,
}

// mapinfo

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiAirBase {
	pub api_action_kind: i64,
	pub api_area_id: i64,
	pub api_distance: KcApiDistance,
	pub api_name: String,
	pub api_plane_info: Vec<KcApiPlaneInfo>,
	pub api_rid: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiDistance {
	pub api_base: i64,
	pub api_bonus: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiPlaneInfo {
	pub api_cond: Option<i64>,
	pub api_count: Option<i64>,
	pub api_max_count: Option<i64>,
	pub api_slotid: i64,
	pub api_squadron_id: i64,
	pub api_state: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiAirBaseExpandedInfo {
	pub api_area_id: i64,
	pub api_maintenance_level: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapInfo {
	pub api_cleared: i64,
	pub api_id: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_defeat_count: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_gauge_num: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_gauge_type: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_required_defeat_count: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_air_base_decks: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_eventmap: Option<KcApiEventmap>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_s_no: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_sally_flag: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiEventmap {
	pub api_max_maphp: i64,
	pub api_now_maphp: i64,
	pub api_selected_rank: i64,
	pub api_state: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMission {
	pub api_mission_id: i64,
	pub api_state: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiPracticeRival {
	pub api_enemy_id: i64,
	pub api_enemy_name: String,
	pub api_enemy_name_id: String,
	pub api_enemy_level: i64,
	pub api_enemy_rank: String,
	pub api_enemy_flag: i64, // 旗フラグ? 1=銅, 2=銀, 3=金
	pub api_enemy_flag_ship: i64,
	pub api_enemy_comment: String,
	pub api_enemy_comment_id: String,
	pub api_state: i64, // 0=未挑戦, 1=E敗北?, 2=D敗北?, 3=C敗北, 4=B勝利, 5=A勝利, 6=S勝利
	pub api_medals: i64,
}

// practice enemy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiPracticeEnemyInfo {
	pub api_member_id: i64,
	pub api_nickname: String,
	pub api_nickname_id: String,
	pub api_cmt: String,
	pub api_cmt_id: String,
	pub api_level: i64,
	pub api_rank: i64,
	pub api_experience: Vec<i64>, // [0]: current value, [1]: always 0
	pub api_friend: i64,          // 0: default
	pub api_ship: Vec<i64>,       // [0]: current, [1]: max
	pub api_slotitem: Vec<i64>,   // [0]: current, [1]: max
	pub api_furniture: i64,
	pub api_deckname: String,
	pub api_deckname_id: String,
	pub api_deck: KcApiPracticeEnemyDeck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiPracticeEnemyDeck {
	pub api_ships: Vec<KcApiPracticeEnemyShip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiPracticeEnemyShip {
	pub api_id: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ship_id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_level: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_star: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KcApiPracticeResp {
	pub api_create_kind: i64,
	pub api_selected_kind: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_entry_limit: Option<i64>,
	pub api_list: Vec<KcApiPracticeRival>,
}
