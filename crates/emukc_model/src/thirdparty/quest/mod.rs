pub mod debug;

use serde::{Deserialize, Serialize};

use crate::kc2::{
	KcApiQuestClearItemBonusType, KcApiQuestListRewardItem, KcApiQuestType, KcSortieResult,
	KcUseItemType,
};

/// Quest data converted from thirdparty source
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuest {
	/// Quest ID (API number)
	pub api_no: i64,
	/// Wiki ID
	pub wiki_id: String,
	/// Quest category
	pub category: Kc3rdQuestCategory,
	/// Quest type
	pub period: Kc3rdQuestPeriod,
	/// Quest name
	pub name: String,
	/// Quest detail
	pub detail: String,
	/// Label type
	pub label_type: i64,
	/// Quest reward fuel
	pub reward_fuel: i64,
	/// Quest reward ammo
	pub reward_ammo: i64,
	/// Quest reward steel
	pub reward_steel: i64,
	/// Quest reward bauxite
	pub reward_bauxite: i64,
	/// Quest prerequisites, Quest ID (API number)
	pub prerequisite: Vec<i64>,
	/// Quest additional rewards
	pub additional_rewards: Vec<Kc3rdQuestReward>,
	/// Quest choice rewards
	pub choice_rewards: Vec<Kc3rdQuestChoiceReward>,
	/// Quest requirements
	pub requirements: Kc3rdQuestRequirement,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kc3rdQuestCategory {
	/// 編成
	Composition = 1,

	/// 出撃
	Sortie = 2,

	/// 演習
	Excercise = 3,

	/// 遠征
	Expedition = 4,

	/// 補給/入渠
	SupplyOrDocking = 5,

	/// 工廠
	Factory = 6,

	/// 近代化改修
	Mordenization = 7,

	/// 出撃/演習
	SortieExercises = 8,

	/// 出撃3
	Sortie3 = 9,

	/// 出撃4
	Sortie4 = 10,

	/// 工廠2
	Factory2 = 11,
}

impl From<i64> for Kc3rdQuestCategory {
	fn from(value: i64) -> Self {
		match value {
			1 => Self::Composition,
			2 => Self::Sortie,
			3 => Self::Excercise,
			4 => Self::Expedition,
			5 => Self::SupplyOrDocking,
			6 => Self::Factory,
			7 => Self::Mordenization,
			8 => Self::SortieExercises,
			9 => Self::Sortie3,
			10 => Self::Sortie4,
			11 => Self::Factory2,
			_ => panic!("Invalid value for Kc3rdQuestCategory: {}", value),
		}
	}
}

/// Quest period
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Kc3rdQuestPeriod {
	#[default]
	Oneshot = 1,
	Daily = 2,
	Weekly = 3,
	Daily3rd7th0th = 4,
	Daily2nd8th = 5,
	Monthly = 6,
	Quarterly = 7,
	Annual = 8,
}

impl From<i64> for Kc3rdQuestPeriod {
	fn from(value: i64) -> Self {
		match value {
			1 => Self::Oneshot,
			2 => Self::Daily,
			3 => Self::Weekly,
			4 => Self::Daily3rd7th0th,
			5 => Self::Daily2nd8th,
			6 => Self::Monthly,
			7 => Self::Quarterly,
			8 => Self::Annual,
			_ => panic!("Invalid value for Kc3rdQuestPeriod: {}", value),
		}
	}
}

impl Kc3rdQuestPeriod {
	pub fn to_api_type(&self) -> i64 {
		match &self {
			Kc3rdQuestPeriod::Oneshot => KcApiQuestType::Oneshot as i64,
			Kc3rdQuestPeriod::Daily | Kc3rdQuestPeriod::Daily3rd7th0th => {
				KcApiQuestType::Daily as i64
			}
			Kc3rdQuestPeriod::Daily2nd8th => KcApiQuestType::Daily as i64,
			Kc3rdQuestPeriod::Weekly => KcApiQuestType::Weekly as i64,
			Kc3rdQuestPeriod::Monthly => KcApiQuestType::Monthly as i64,
			Kc3rdQuestPeriod::Quarterly | Kc3rdQuestPeriod::Annual => KcApiQuestType::Other as i64,
		}
	}
}

/// Quest reward
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestReward {
	/// API number
	pub api_id: i64,

	/// Reward category
	pub category: Kc3rdQuestRewardCategory,

	/// Reward amount
	pub amount: i64,

	/// stars for slot item
	pub stars: i64,
}

/// Quest reward category
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kc3rdQuestRewardCategory {
	Material = 1,
	Slotitem = 2,
	Ship = 3,
	Furniture = 4,
	UseItem = 5,
	FleetUnlock = 6,
	LargeShipConstructionUnlock = 7,
	FactoryImprovementUnlock = 8,
	WarResult = 9,
	ExpeditionSupplyUnlock = 10,
	AirbaseUnlock = 11,
}

/*
impl From<i64> for Kc3rdQuestRewardCategory {
	fn from(value: i64) -> Self {
		match value {
			1 => Self::Slotitem,
			2 => Self::Ship,
			3 => Self::Furniture,
			_ => panic!("Invalid value for Kc3rdQuestRewardCategory: {}", value),
		}
	}
}
*/

/// Quest choice reward
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestChoiceReward {
	pub choices: Vec<Kc3rdQuestReward>,
}

// Quest requirements
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestRequirement {
	/// All conditions must be met
	And(Vec<Kc3rdQuestCondition>),

	/// At least one condition must be met
	OneOf(Vec<Kc3rdQuestCondition>),

	/// Conditions must be met in sequence
	Sequential(Vec<Kc3rdQuestCondition>),
}

impl Default for Kc3rdQuestRequirement {
	fn default() -> Self {
		Self::And(vec![])
	}
}

/// Quest condition
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestCondition {
	Composition(Kc3rdQuestConditionComposition),
	Construct(i64),
	Excercise(Kc3rdQuestConditionExcerise),
	Expedition(Vec<Kc3rdQuestConditionExpedition>),
	ModelConversion(Kc3rdQuestConditionModelConversion),
	Modernization(Kc3rdQuestConditionModernization),
	Repair(i64),
	ResourceConsumption(Kc3rdQuestConditionMaterialConsumption),
	Resupply(i64),
	ScrapAnyEquipment(i64),
	ScrapAnyShip(i64),
	Sink(Kc3rdQuestConditionShip, i64),
	SlotItemConstruction(i64),
	SlotItemConsumption(Vec<Kc3rdQuestConditionSlotItem>),
	SlotItemImprovement(i64),
	SlotItemScrap(Vec<Kc3rdQuestConditionSlotItem>),
	Sortie(Kc3rdQuestConditionSortie),
	SortieCount(i64),
	UseItemConsumption(Vec<Kc3rdQuestConditionUseItemConsumption>),
}

/// Quest condition material consumption
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionMaterialConsumption {
	pub fuel: i64,
	pub ammo: i64,
	pub steel: i64,
	pub bauxite: i64,
}

/// Quest condition use item consumption
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionUseItemConsumption {
	/// use item id
	pub api_id: i64,

	/// amount
	pub amount: i64,
}

/// Quest condition slot item type
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestConditionSlotItemType {
	/// Exact slot item
	Equipment(i64),

	/// One of the slot items
	Equipments(Vec<i64>),

	/// Exact slot item type
	EquipType(i64),

	/// One of the slot item types
	EquipTypes(Vec<i64>),
}

/// Quest condition slot item
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionSlotItem {
	/// slot item type
	pub item_type: Kc3rdQuestConditionSlotItemType,

	/// amount
	pub amount: i64,

	/// remodel level
	pub stars: i64,

	/// some quest requires fully skilled aircraft
	pub fully_skilled: bool,
}

/// Quest condition equipment in ship's slot
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionEquipInSlot {
	/// slot item type
	pub item: Kc3rdQuestConditionSlotItem,

	/// slot position
	pub pos: i64,

	/// will the slot item keep stars after quest
	pub keep_stars: bool,
}

/// Quest condition excercise
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionExcerise {
	/// excercise times
	pub times: i64,

	/// excercise result
	pub expect_result: KcSortieResult,

	/// will the quest expire next day
	pub expire_next_day: bool,

	/// composition requirement
	#[serde(skip_serializing_if = "Option::is_none")]
	pub groups: Option<Vec<Kc3rdQuestConditionShipGroup>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestShipNavy {
	USN,
	RN,
	RNN,
	RAN,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestConditionShip {
	Any,
	Ship(i64),
	Ships(Vec<i64>),
	ShipType(i64),
	ShipTypes(Vec<i64>),
	ShipClass(i64),
	ShipClasses(Vec<i64>),
	Navy(Kc3rdQuestShipNavy),
	Navies(Vec<Kc3rdQuestShipNavy>),
	HighSpeed,
	LowSpeed,
	Aviation,
	Carrier,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestShipAmount {
	Exactly(i64),
	Range(i64, i64),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionShipGroup {
	pub ship: Kc3rdQuestConditionShip,
	pub amount: Kc3rdQuestShipAmount,
	pub lv: i64,
	pub position: i64, // 0: any, 1: first, 2: second, etc.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub white_list: Option<Vec<i64>>, // ship id list
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionModernization {
	// which ship type to modernize
	pub target_ship: Kc3rdQuestConditionShip,
	// ships used as modernization material
	pub material_ship: Kc3rdQuestConditionShip,
	// how many material ships are needed for each modernization
	pub batch_size: i64,
	// total number of modernizations needed
	pub times: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionExpedition {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub list: Option<Vec<String>>,
	pub times: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionComposition {
	/// ship group
	pub groups: Vec<Kc3rdQuestConditionShipGroup>,

	/// disallowed ship
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disallowed: Option<Vec<Kc3rdQuestConditionShip>>,

	/// fleet id
	pub fleet_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionModelConversion {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secretary: Option<Kc3rdQuestConditionShip>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub banned_secretary: Option<Kc3rdQuestConditionShip>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub slots: Option<Vec<Kc3rdQuestConditionEquipInSlot>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionMapInfo {
	pub area: i64,
	pub number: i64,
	pub phase: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestConditionSortieMap {
	One(Kc3rdQuestConditionMapInfo),
	All(Vec<Kc3rdQuestConditionMapInfo>),
	AnyOf(Vec<Kc3rdQuestConditionMapInfo>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionSortie {
	/// ship group
	#[serde(skip_serializing_if = "Option::is_none")]
	pub composition: Option<Kc3rdQuestConditionComposition>,

	/// need to defeat boss
	pub defeat_boss: bool,

	/// fleet id
	pub fleet_id: i64,

	/// sortie map specification
	#[serde(skip_serializing_if = "Option::is_none")]
	pub map: Option<Kc3rdQuestConditionSortieMap>,

	/// sortie result
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<KcSortieResult>,

	/// how many times to sortie
	pub times: i64,
}

impl Kc3rdQuestRequirement {
	pub fn lost_badges(&self) -> i64 {
		match self {
			Kc3rdQuestRequirement::And(conds) => conds
				.iter()
				.map(|c| match c {
					Kc3rdQuestCondition::UseItemConsumption(item) => {
						item.iter().fold(0, |acc, f| {
							acc + if f.api_id == KcUseItemType::Medal as i64 {
								f.amount
							} else {
								0
							}
						})
					}
					_ => 0,
				})
				.sum(),
			Kc3rdQuestRequirement::OneOf(_) | Kc3rdQuestRequirement::Sequential(_) => 0,
		}
	}
}

impl Kc3rdQuest {
	pub fn bonus_flag(&self) -> i64 {
		for r in &self.choice_rewards {
			for choice in &r.choices {
				if choice.category == Kc3rdQuestRewardCategory::Ship {
					return 2;
				}
			}
		}
		for r in &self.additional_rewards {
			if r.category == Kc3rdQuestRewardCategory::Ship {
				return 2;
			}
		}
		1
	}

	pub fn to_api_reward_selection(&self) -> Option<Vec<Vec<KcApiQuestListRewardItem>>> {
		if self.choice_rewards.is_empty() {
			return None;
		}

		let mut results: Vec<Vec<KcApiQuestListRewardItem>> = Vec::new();
		for group in &self.choice_rewards {
			let result: Vec<KcApiQuestListRewardItem> = group
				.choices
				.iter()
				.enumerate()
				.map(|(i, choice)| KcApiQuestListRewardItem {
					api_no: i as i64 + 1,
					api_kind: match choice.category {
						Kc3rdQuestRewardCategory::Material => {
							KcApiQuestClearItemBonusType::Material as i64
						}
						Kc3rdQuestRewardCategory::Slotitem => {
							KcApiQuestClearItemBonusType::SlotItem as i64
						}
						Kc3rdQuestRewardCategory::Ship => {
							KcApiQuestClearItemBonusType::ShipBonus as i64
						}
						Kc3rdQuestRewardCategory::Furniture => {
							KcApiQuestClearItemBonusType::Furniture as i64
						}
						Kc3rdQuestRewardCategory::UseItem => {
							KcApiQuestClearItemBonusType::UseItem as i64
						}
						Kc3rdQuestRewardCategory::FleetUnlock => {
							KcApiQuestClearItemBonusType::UnlockDeck as i64
						}
						Kc3rdQuestRewardCategory::LargeShipConstructionUnlock => {
							KcApiQuestClearItemBonusType::UnlockLargeBuild as i64
						}
						Kc3rdQuestRewardCategory::FactoryImprovementUnlock => {
							KcApiQuestClearItemBonusType::TuckYouTanaka as i64
						}
						Kc3rdQuestRewardCategory::WarResult => {
							KcApiQuestClearItemBonusType::WarResult as i64
						}
						Kc3rdQuestRewardCategory::ExpeditionSupplyUnlock => {
							KcApiQuestClearItemBonusType::ExtraSupply as i64
						}
						Kc3rdQuestRewardCategory::AirbaseUnlock => {
							KcApiQuestClearItemBonusType::AirUnitBase as i64
						}
					},
					api_mst_id: choice.api_id,
					api_slotitem_level: choice.stars,
					api_count: choice.amount,
				})
				.collect();

			results.push(result);
		}

		Some(results)
	}
}

pub type Kc3rdQuestMap = std::collections::BTreeMap<i64, Kc3rdQuest>;
