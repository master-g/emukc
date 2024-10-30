#[doc(hidden)]
pub mod quest;

#[doc(inline)]
#[allow(unused_imports)]
pub use quest::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
	pub api_furniture: [i64; 6],
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
	pub api_pvp: [i64; 2],
	/// Number of class A medal obtained
	pub api_medals: i64,
	/// Large construction dock unlock flag, not exist in origin API, 0: locked, 1: unlocked
	pub api_large_dock: i64,
	/// Maxium parallel quest count, not exist in origin API
	pub api_max_quests: i64,
	/// Extra supply enable flag, [0] expedition, [1] battle
	pub api_extra_supply: [i64; 2],
	/// War result points, not exist in origin API
	pub api_war_result: i64,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, PartialOrd, Debug, enumn::N)]
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
	/// Ship experience
	/// [0]: current
	/// [1]: needed for next level
	/// [2]: progress percentage, 0-100
	pub api_exp: [i64; 3],
	pub api_nowhp: i64,
	pub api_maxhp: i64,
	/// Speed
	/// 0: base, 5: slow, 10: fast, 15: fast+, 20: fastest
	pub api_soku: i64,
	/// Range, 0: none, 1: short, 2: medium, 3: long, 4: very long, 5: very long+
	pub api_leng: i64,
	/// Slots, length 5, -1 for empty slot
	pub api_slot: [i64; 5],
	/// Aircraft capacity
	pub api_onslot: [i64; 5],
	/// Extra slot, 0 for locked, -1 for empty
	pub api_slot_ex: i64,
	/// Modernization, [0]: firepower, [1]: torpedo, [2]: AA, [3]: armor, [4]: luck, [5]: HP, [6]: ASW
	pub api_kyouka: [i64; 7],
	/// Rarity
	pub api_backs: i64,
	pub api_fuel: i64,
	pub api_bull: i64,
	/// Number of slots
	pub api_slotnum: i64,
	/// Repair time, in milliseconds
	pub api_ndock_time: i64,
	/// Material consumption for repair, [0]: fuel, [1]: steel
	pub api_ndock_item: [i64; 2],
	/// Modernization level, 0 for not modernized
	pub api_srate: i64,
	/// Morale
	pub api_cond: i64,
	/// fire power, [0]: including equipment, [1]: max
	pub api_karyoku: [i64; 2],
	/// torpedo, [0]: current, [1]: max
	pub api_raisou: [i64; 2],
	/// AA, [0]: current, [1]: max
	pub api_taiku: [i64; 2],
	/// armor, [0]: current, [1]: max
	pub api_soukou: [i64; 2],
	/// evasion, [0]: current, [1]: max
	pub api_kaihi: [i64; 2],
	/// anti-submarine, [0]: current, [1]: max
	pub api_taisen: [i64; 2],
	/// line of sight, [0]: current, [1]: max
	pub api_sakuteki: [i64; 2],
	/// luck, [0]: current, [1]: max
	pub api_lucky: [i64; 2],
	/// Is locked, 0: no, 1: yes
	pub api_locked: i64,
	/// Equip any locked equipment, 0: no, 1: yes
	pub api_locked_equip: i64,
	/// Sally area, used when there is event
	pub api_sally_area: i64,
	/// Special effect item attachment info
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_sp_effect_items: Option<Vec<KcApiSpEffectOnShip>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiSpEffectOnShip {
	/// 1: azure ribbon (useitem 98), 2: white sash (useitem 99)
	pub api_kind: i64,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_houg: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_raig: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_kaih: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_souk: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiPresetDeckElement {
	pub api_preset_no: i64,
	pub api_name: String,
	pub api_name_id: String,
	pub api_ship: [i64; 7],
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
	/// always "" for now
	pub api_name_id: String,
	/// expedition
	///
	/// 0: status, 0: idle, 1: in mission, 2: returning, 3: force returning
	/// 1: mission id
	/// 2: return time, in milliseconds
	/// 3: always 0
	pub api_mission: [i64; 4],
	/// always "0" for now
	pub api_flagship: String,
	/// ship id, -1 for empty slot
	pub api_ship: [i64; 6],
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
	pub api_oss_items: [i64; 8],
	/// Secretary ship position, 0: right, 1: center
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
			api_oss_items: [1; 8],
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct KcApiIncentive {
	pub api_count: i64,
	pub api_item: Option<Vec<KcApiIncentiveItem>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, enumn::N)]
pub enum KcApiIncentiveMode {
	PreRegister = 1,
	Reception = 2,
	MonthlyOrPresent = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, enumn::N)]
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

	/// when type is 3(UseItem)
	/// 903: 装備運用枠+3
	/// 904: 装備運用枠+4
	/// 905: 装備運用枠+5
	/// 906: 装備運用枠+2
	/// 907: 装備運用枠+?
	/// 908: 装備運用枠+10
	pub api_mst_id: i64,
	pub api_getmes: Option<String>,
	pub api_slotitem_level: Option<i64>,

	/// hidden field for internal use
	#[serde(skip_serializing)]
	pub amount: i64,
	#[serde(skip_serializing)]
	pub alv: i64,
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
	pub api_m10: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_sally_flag: Option<[i64; 3]>,
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
	pub api_experience: [i64; 2], // [0]: current value, [1]: always 0
	pub api_friend: i64,          // 0: default
	pub api_ship: [i64; 2],       // [0]: current, [1]: max
	pub api_slotitem: [i64; 2],   // [0]: current, [1]: max
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiPracticeResp {
	pub api_create_kind: i64,
	pub api_selected_kind: i64,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_entry_limit: Option<i64>,

	pub api_list: Vec<KcApiPracticeRival>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiShipQVoiceInfo {
	pub api_no: i64,
	pub api_voice_id: i64,
	pub api_icon_id: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiPictureBookShip {
	pub api_index_no: i64,
	/// [][0]: default
	/// [][1]: medium damage
	/// [][2]: married
	/// [][3]: ?
	/// [][4]: ?
	pub api_state: Vec<[i64; 5]>,
	pub api_q_voice_info: Vec<KcApiShipQVoiceInfo>,
	pub api_table_id: Vec<i64>,
	pub api_name: String,
	pub api_yomi: String,
	pub api_stype: i64,
	pub api_cnum: i64,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_taik: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_souk: Option<i64>,

	pub api_kaih: i64,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_houg: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_raig: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_tyku: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_tais: Option<i64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_leng: Option<i64>,

	pub api_sinfo: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiPictureBookSlotItem {
	pub api_index_no: i64,
	pub api_state: Vec<i64>,
	pub api_table_id: Vec<i64>,
	pub api_name: String,
	pub api_type: Vec<i64>,
	pub api_souk: i64,
	pub api_houg: i64,
	pub api_raig: i64,
	pub api_soku: i64,
	pub api_baku: i64,
	pub api_tyku: i64,
	pub api_tais: i64,
	pub api_houm: i64,
	pub api_houk: i64,
	pub api_saku: i64,
	pub api_leng: i64,
	pub api_flag: Vec<i64>,
	pub api_info: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiLogElement {
	pub api_no: i64,
	pub api_type: String,
	pub api_state: String,
	pub api_message: String,
}
