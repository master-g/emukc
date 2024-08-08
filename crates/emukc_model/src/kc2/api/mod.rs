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
	/// always "" for now
	pub api_name_id: String,
	/// expedition
	///
	/// 0: status, 0: idle, 1: in mission, 2: returning, 3: force returning
	/// 1: mission id
	/// 2: return time, in milliseconds
	/// 3: always 0
	pub api_mission: Vec<i64>,
	/// always "0" for now
	pub api_flagship: String,
	/// ship id, -1 for empty slot
	pub api_ship: Vec<i64>,
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KcApiShipQVoiceInfo {
	pub api_no: i64,
	pub api_voice_id: i64,
	pub api_icon_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KcApiPictureBookShip {
	pub api_index_no: i64,
	pub api_state: Vec<Vec<i64>>,
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcApiQuestType {
	Daily = 1,
	Weekly = 2,
	Monthly = 3,
	Onetime = 4,
	Other = 5,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcApiQuestClearItemMaterialType {
	Fuel = 1,
	Ammo = 2,
	Steel = 3,
	Bauxite = 4,
	Torch = 5,
	Bucket = 6,
	DevMaterial = 7,
	Screw = 8,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcApiQuestClearItemBonusType {
	TuckYouTanaka = 0,
	Material = 1,
	UnlockDeck = 2,
	FurnitureCoinBox = 3,
	UnlockLargeBuild = 4,
	AirUnitBase = 5,
	ExtraSupply = 6,
	ShipBonus = 11,
	SlotItem = 12,
	UseItem = 13,
	Furniture = 14,
	ModelChange = 15,
	ModelChange2 = 16,
	WarResult = 18,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiQuestClearItemGetBonusItem {
	pub api_id: Option<i64>,
	pub api_ship_id: Option<i64>,
	pub api_slotitem_id: Option<i64>,
	pub api_name: Option<String>,
	/// only use for `AirUnitBase` bonus
	pub api_message_a: Option<String>,
	pub api_message: Option<String>,
	pub api_slotitem_level: Option<i64>,
	pub api_id_from: Option<i64>,
	pub api_id_to: Option<i64>,
	pub api_c_flag: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiQuestClearItemGetBonus {
	pub api_type: i64,
	pub api_count: i64,
	pub api_item: Option<KcApiQuestClearItemGetBonusItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiQuestClearItemGet {
	pub api_material: Vec<i64>,
	pub api_bounus_count: i64,
	pub api_bounus: Vec<KcApiQuestClearItemGetBonus>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiQuestListRewardItem {
	pub api_no: i64,
	pub api_kind: i64, // 11: ship, 12: slotitem, 13: useitem, 14: furniture
	pub api_mst_id: i64,
	pub api_slotitem_level: i64,
	pub api_count: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KcApiQuestItem {
	pub api_no: i64,
	pub api_category: i64, // 1=編成, 2=出撃, 3=演習, 4=遠征, 5=補給/入渠, 6=工廠, 7=改装, 8=出撃/演習, 9=出撃(3), 10=出撃, 11=工廠
	pub api_type: i64,     // 1=Daily, 2=Weekly, 3=Monthly, 4=Oneshot, 5=Other
	pub api_label_type: i64, // 1=Oneshot, 2=Daily, 3=Weekly, 6=Monthly, 7=他(輸送5と空母3,クォータリー), 101=Yearly(Jan), 102=Yearly(Feb), 103=Yearly(Mar), 104=Yearly(Apr), 105=Yearly(May), 106=Yearly(Jun), 107=Yearly(Jul), 108=Yearly(Aug), 109=Yearly(Sep), 110=Yearly(Oct), 111=Yearly(Nov), 112=Yearly(Dec)
	pub api_state: i64,      // 1=未受領, 2=遂行中, 3=達成
	pub api_title: String,
	pub api_detail: String,
	pub api_lost_badges: i64,
	pub api_voice_id: i64,
	pub api_get_material: Vec<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_select_rewards: Option<Vec<Vec<KcApiQuestListRewardItem>>>,
	pub api_bonus_flag: i64,    // 1=通常, 2=艦娘
	pub api_progress_flag: i64, // 0=空白(達成含む), 1=50%以上達成, 2=80%以上達成
	pub api_invalid_flag: i64,  // 機種転換不能フラグ 0=可能, 1=不可能(装備がロックされている)
}
