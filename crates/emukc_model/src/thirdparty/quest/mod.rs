pub mod composition;
pub mod debug;
pub mod extra;
pub mod matcher;
pub mod progress;
pub mod reward;

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
	/// Conversion mode
	pub conversion_mode: Kc3rdQuestConversionMode,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kc3rdQuestCategory {
	/// 編成
	Composition = 1,

	/// 出撃
	Sortie = 2,

	/// 演習
	Exercise = 3,

	/// 遠征
	Expedition = 4,

	/// 補給/入渠
	SupplyOrDocking = 5,

	/// 工廠
	Factory = 6,

	/// 近代化改修
	Modernization = 7,

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
			3 => Self::Exercise,
			4 => Self::Expedition,
			5 => Self::SupplyOrDocking,
			6 => Self::Factory,
			7 => Self::Modernization,
			8 => Self::SortieExercises,
			9 => Self::Sortie3,
			10 => Self::Sortie4,
			11 => Self::Factory2,
			_ => panic!("Invalid value for Kc3rdQuestCategory: {value}"),
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
			_ => panic!("Invalid value for Kc3rdQuestPeriod: {value}"),
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

/// Quet exchange or conversion type
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kc3rdQuestConversionMode {
	/// Model conversion
	Conversion,

	/// Slot item exchange
	Exchange,

	/// No conversion or exchange
	None,
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
	// Fleet
	Composition(Kc3rdQuestConditionComposition),

	// Combat
	Exercise(Kc3rdQuestConditionExercise),
	Sortie(Kc3rdQuestConditionSortie),
	Sink(Kc3rdQuestConditionShip, i64),

	// Expedition
	Expedition(Vec<Kc3rdQuestConditionExpedition>),

	// Factory
	Factory(Kc3rdQuestConditionFactory),

	// Scrap
	Scrap(Kc3rdQuestConditionScrap),

	// Consumption
	Consumption(Kc3rdQuestConditionConsumption),

	// Other
	Modernization(Kc3rdQuestConditionModernization),
	ModelConversion(Kc3rdQuestConditionModelConversion),
	Repair(i64),
	Resupply(i64),
}

/// Factory related condtions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestConditionFactory {
	/// Ship construction
	ShipConstruction(i64),

	/// Slot item construction
	SlotItemConstruction(i64),

	/// Slot item improvement
	SlotItemImprovement(i64),
}

/// Scrap related conditions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestConditionScrap {
	/// Any slot item
	AnyEquipment(i64),

	/// Any ship
	AnyShip(i64),

	/// Specific slot item
	SpecificItems(Vec<Kc3rdQuestConditionSlotItem>),
}

/// Consumption related conditions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kc3rdQuestConditionConsumption {
	/// Material consumption
	Resources(Kc3rdQuestConditionMaterialConsumption),

	/// Slot item consumption
	SlotItemConsumption(Vec<Kc3rdQuestConditionSlotItem>),

	/// Use item consumption
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
	/// List of `api_mst_slotitem.api_id`
	Equipment(Vec<i64>),

	/// Slot item type
	EquipType(Vec<i64>),
}

impl Kc3rdQuestConditionSlotItemType {
	pub fn single_equipment(id: i64) -> Self {
		Self::Equipment(vec![id])
	}

	pub fn single_type(id: i64) -> Self {
		Self::EquipType(vec![id])
	}
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

/// Quest condition exercise
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestConditionExercise {
	/// exercise times
	pub times: i64,

	/// exercise result
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
	Ship(Vec<i64>),
	ShipType(Vec<i64>),
	ShipClass(Vec<i64>),
	Navy(Vec<Kc3rdQuestShipNavy>),
	HighSpeed,
	LowSpeed,
	Aviation,
	Carrier,
}

impl Kc3rdQuestConditionShip {
	pub fn single_ship(id: i64) -> Self {
		Self::Ship(vec![id])
	}

	pub fn single_type(id: i64) -> Self {
		Self::ShipType(vec![id])
	}

	pub fn single_class(id: i64) -> Self {
		Self::ShipClass(vec![id])
	}

	pub fn single_navy(navy: Kc3rdQuestShipNavy) -> Self {
		Self::Navy(vec![navy])
	}
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Kc3rdQuestShipAmount {
	pub min: i64,
	pub max: i64,
}

impl Kc3rdQuestShipAmount {
	pub fn exact(amount: i64) -> Self {
		Self {
			min: amount,
			max: amount,
		}
	}

	pub fn range(min: i64, max: i64) -> Self {
		Self {
			min,
			max,
		}
	}
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
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
	/// Calculate lost badges from quest conditions
	pub fn lost_badges(&self) -> i64 {
		match self {
			Kc3rdQuestRequirement::And(conds) => conds
				.iter()
				.map(|c| match c {
					Kc3rdQuestCondition::Consumption(
						Kc3rdQuestConditionConsumption::UseItemConsumption(items),
					) => items.iter().fold(0, |acc, f| {
						acc + if f.api_id == KcUseItemType::Medal as i64 {
							f.amount
						} else {
							0
						}
					}),
					_ => 0,
				})
				.sum(),
			Kc3rdQuestRequirement::OneOf(_) | Kc3rdQuestRequirement::Sequential(_) => 0,
		}
	}
}

// magic number for quests that are not model conversion / exchange quest
// but these quests have misleading fields in original source data
static SKIP_CONVERSION_QUESTS: &[i64] =
	&[657, 661, 662, 663, 664, 665, 667, 675, 676, 677, 1138, 1139, 1140, 1148];

impl Kc3rdQuest {
	/// Get bonus flag for quest clear
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

	/// Convert choice rewards to API format
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

	/// Check if the quest is a conversion/exchange quest
	pub fn is_conversion_quest(&self) -> bool {
		!matches!(self.conversion_mode, Kc3rdQuestConversionMode::None)
			&& !self.additional_rewards.is_empty()
			&& !SKIP_CONVERSION_QUESTS.contains(&self.api_no)
	}

	/// Extract model conversion information from the quest.
	pub fn extract_model_conversion_info(&self) -> Option<(i64, i64)> {
		let conditions: &Vec<Kc3rdQuestCondition> = match &self.requirements {
			Kc3rdQuestRequirement::And(conds)
			| Kc3rdQuestRequirement::OneOf(conds)
			| Kc3rdQuestRequirement::Sequential(conds) => conds,
		};

		let mut from_id = 0;
		for cond in conditions {
			match cond {
				Kc3rdQuestCondition::ModelConversion(cond) => {
					if let Some(cond) = &cond.slots
						&& let Some(first) = cond.first()
						&& let Kc3rdQuestConditionSlotItemType::Equipment(ids) =
							&first.item.item_type
					{
						from_id = ids.first().copied().unwrap_or(0);
						break;
					}
				}
				Kc3rdQuestCondition::Consumption(
					Kc3rdQuestConditionConsumption::SlotItemConsumption(items),
				) => {
					if let Some(first) = items.first()
						&& let Kc3rdQuestConditionSlotItemType::Equipment(ids) = &first.item_type
					{
						from_id = ids.first().copied().unwrap_or(0);
						break;
					}
				}
				_ => {}
			}
		}

		for reward in self.additional_rewards.iter() {
			if reward.category == Kc3rdQuestRewardCategory::Slotitem {
				return Some((from_id, reward.api_id));
			}
		}

		None
	}
}

pub type Kc3rdQuestMap = std::collections::BTreeMap<i64, Kc3rdQuest>;
pub use composition::{ShipInstance, validate_composition};
pub use matcher::QuestActionEvent;
