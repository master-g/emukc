use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcApiQuestType {
	Daily = 1,
	Weekly = 2,
	Monthly = 3,
	Oneshot = 4,
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
	EventAreaUnlock = 99,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct KcApiQuestClearItemGetBonusItem {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_ship_id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_slotitem_id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_getmes: Option<String>,
	/// only use for `AirUnitBase` bonus
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_message_a: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_message: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_slotitem_level: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_id_from: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub api_id_to: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
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
	pub api_material: [i64; 4],
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
