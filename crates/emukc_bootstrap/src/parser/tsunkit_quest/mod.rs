use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, HashMap},
	fs,
};

use emukc_model::prelude::*;
use label_type::extract_label_type;

use super::{error::ParseError, kccp::quest::KccpQuestInfo};

mod label_type;
mod requirement;
mod reward;
mod types;

pub type TsunkitQuest = HashMap<String, TsunkitQuestValue>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsunkitQuestValue {
	pub(super) game_id: i64,
	pub(super) wiki_id: String,
	pub(super) category: TsunkitQuestCategory,
	pub(super) frequency: Frequency,
	pub(super) release_date: String,
	pub(super) updated: String,
	pub(super) prereqs: Vec<i64>,
	pub(super) requirements: Requirements,
	pub(super) rewards: Rewards,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) edited: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) unavailable: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) unverified: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) memo: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(super) detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TsunkitQuestCategory {
	Composition,
	Exercise,
	Expedition,
	Factory,
	Modernization,
	Sortie,
	Supply,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
	Annual,
	Daily,
	Event,
	Monthly,
	Onetime,
	Quarterly,
	Seasonal,
	Special1,
	Special2,
	Weekly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Requirements {
	pub category: RequirementsCategory,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comp: Option<Vec<RequirementsComp>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fleet_id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disallowed: Option<Disallowed>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comp_banned: Option<Vec<CompBanned>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sortie: Option<Vec<Sortie>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subcategory: Option<RequirementsSubCategory>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub times: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub group_id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub list: Option<Vec<List>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<CombatResult>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub daily: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expeds: Option<Vec<Exped>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resources: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secretary: Option<Secretary>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub slots: Option<Vec<RequirementsSlot>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub scrap: Option<Vec<Scrap>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub consume: Option<Vec<Consume>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub batch: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secretary_banned: Option<SecretaryBanned>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub family_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementsCategory {
	And,
	Conversion,
	Equipexchange,
	Exercise,
	Expedition,
	Fleet,
	Modernization,
	Or,
	Scrapequipment,
	Simple,
	Sink,
	Sortie,
	Then,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementsSubCategory {
	Battle,
	Equipment,
	Improvement,
	Modernization,
	Repair,
	Resupply,
	Scrapequipment,
	Scrapship,
	Ship,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClassId {
	Integer(i64),
	IntegerArray(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequirementsComp {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub otherships: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub position: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ship_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub group_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lv: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub family_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub criteria: Option<Criteria>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ship: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ship_en: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class_en: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Criteria {
	pub group_id: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompBanned {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ship_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class_id: Option<ClassId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Consume {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<ConsumeCategory>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<i64>,
	pub amount: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub conversion: Option<Conversion>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stars: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsumeCategory {
	Equipgroup,
	Equipment,
	Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Conversion {
	Starskept,
	Starslost,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disallowed {
	Aviation,
	Carriers,
	Otherships,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exped {
	pub times: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
	String(String),
	StringArray(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
	pub category: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sortie: Option<Vec<Sortie>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comp: Option<Vec<RequirementsComp>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disallowed: Option<Disallowed>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<CombatResult>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub daily: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub times: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub slots: Option<Vec<RequirementsSlot>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub scrap: Option<Vec<Scrap>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resources: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub consume: Option<Vec<Consume>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scrap {
	pub category: ConsumeCategory,
	pub id: i64,
	pub amount: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CombatResult {
	A,
	B,
	C,
	#[serde(rename = "clear")]
	Clear,
	S,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sortie {
	pub map: Option<Id>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub boss: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<CombatResult>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub times: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub node: Option<Node>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub any: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Node {
	N,
	O,
	P,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Secretary {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub class_id: Option<Vec<i64>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ship_id: Option<ClassId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub family_id: Option<ClassId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretaryBanned {
	pub ship_id: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequirementsSlot {
	pub id: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub conversion: Option<Conversion>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub slot: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fullyskilled: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stars: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rewards {
	pub resources: Vec<i64>,
	pub other: Vec<Other>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Other {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<OtherCategory>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stars: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub choices: Option<Vec<Choice>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OtherCategory {
	Equipment,
	Feature,
	Furniture,
	Inventory,
	Senka,
	Ship,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
	pub category: OtherCategory,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stars: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub amount: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name_en: Option<String>,
}

impl From<TsunkitQuestCategory> for Kc3rdQuestCategory {
	fn from(value: TsunkitQuestCategory) -> Self {
		match value {
			TsunkitQuestCategory::Composition => Kc3rdQuestCategory::Composition,
			TsunkitQuestCategory::Exercise => Kc3rdQuestCategory::Excercise,
			TsunkitQuestCategory::Expedition => Kc3rdQuestCategory::Expedition,
			TsunkitQuestCategory::Factory => Kc3rdQuestCategory::Factory,
			TsunkitQuestCategory::Modernization => Kc3rdQuestCategory::Mordenization,
			TsunkitQuestCategory::Sortie => Kc3rdQuestCategory::Sortie,
			TsunkitQuestCategory::Supply => Kc3rdQuestCategory::SupplyOrDocking,
		}
	}
}

impl From<Frequency> for Kc3rdQuestPeriod {
	fn from(value: Frequency) -> Self {
		match value {
			Frequency::Annual => Kc3rdQuestPeriod::Annual,
			Frequency::Daily => Kc3rdQuestPeriod::Daily,
			Frequency::Event | Frequency::Onetime | Frequency::Seasonal => {
				Kc3rdQuestPeriod::Oneshot
			}
			Frequency::Monthly => Kc3rdQuestPeriod::Monthly,
			Frequency::Quarterly => Kc3rdQuestPeriod::Quarterly,
			Frequency::Special1 => Kc3rdQuestPeriod::Daily3rd7th0th,
			Frequency::Special2 => Kc3rdQuestPeriod::Daily2nd8th,
			Frequency::Weekly => Kc3rdQuestPeriod::Weekly,
		}
	}
}

impl TsunkitQuestValue {
	pub(super) fn to_kc3rd_quest(
		&self,
		mst: &ApiManifest,
		quest_info: &BTreeMap<i64, KccpQuestInfo>,
	) -> Kc3rdQuest {
		let game_id = if self.game_id > 240000 {
			self.game_id - 240000
		} else {
			self.game_id
		};

		let default_quest_info = KccpQuestInfo::default();
		let info = quest_info.get(&game_id).unwrap_or_else(|| {
			error!("quest info not found: {}", self.game_id);
			&default_quest_info
		});

		let period: Kc3rdQuestPeriod = self.frequency.into();

		Kc3rdQuest {
			api_no: game_id,
			wiki_id: self.wiki_id.clone(),
			category: self.category.into(),
			period,
			name: info.name.to_string(),
			detail: info.desc.to_string(),
			label_type: extract_label_type(&self.wiki_id),
			reward_fuel: self.rewards.resources[0],
			reward_ammo: self.rewards.resources[1],
			reward_steel: self.rewards.resources[2],
			reward_bauxite: self.rewards.resources[3],
			prerequisite: self.prereqs.clone(),
			additional_rewards: self
				.rewards
				.to_additional_reward(mst, &self.wiki_id)
				.unwrap_or_default(),
			choice_rewards: self.rewards.to_choice_rewards(mst, &self.wiki_id).unwrap_or_default(),
			requirements: self.requirements.to_requirements(mst),
		}
	}
}

/// Parse the tsunkit quests from the given path.
///
/// # Arguments
///
/// * `src` - The path to the tsunkit quests.
/// * `manifest` - The api manifest.
/// * `info` - The quest info.
///
/// # Returns
///
/// A map of quest api no to quest.
pub fn parse(
	src: impl AsRef<std::path::Path>,
	manifest: &ApiManifest,
	info: &BTreeMap<i64, KccpQuestInfo>,
) -> Result<Kc3rdQuestMap, ParseError> {
	let raw = fs::read_to_string(src)?;
	trace!("parsing tsunkit quests");
	let raw: TsunkitQuest = serde_json::from_str(&raw)?;

	let quests: Vec<Kc3rdQuest> = raw
		.values()
		.map(|r| {
			let span =
				span!(tracing::Level::ERROR, "quest", wiki_id = r.wiki_id, api_id = r.game_id);
			let _enter = span.enter();
			r.to_kc3rd_quest(manifest, info)
		})
		.collect();

	debug!("{} quests parsed", quests.len());

	Ok(quests.iter().map(|quest| (quest.api_no, quest.clone())).collect())
}
