use serde::{Deserialize, Serialize};

/// Slot item extra information map
pub type Kc3rdSlotItemMap = std::collections::BTreeMap<i64, Kc3rdSlotItem>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Kc3rdSlotItemAswDamageType {
	/// DCP
	DepthCargeProjector,

	/// DCR
	DepthChargeRack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemImproveBaseConsumption {
	/// fuel consumption
	pub fuel: i64,

	/// ammo consumption
	pub ammo: i64,

	/// steel consumption
	pub steel: i64,

	/// bauxite consumption
	pub bauxite: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemImproveItemConsumption {
	/// item consumption `mst_id`
	pub id: i64,

	/// item consumption count
	pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemImproveSecretary {
	/// secretary ship `mst_id`
	pub id: i64,

	pub monday: bool,
	pub tuesday: bool,
	pub wednesday: bool,
	pub thursday: bool,
	pub friday: bool,
	pub saturday: bool,
	pub sunday: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemImprovePerLevelConsumption {
	/// devmat min consumption
	pub dev_mat_min: i64,

	/// devmat max consumption
	pub dev_mat_max: i64,

	/// screw min consumption
	pub screw_min: i64,

	/// screw max consumption
	pub screw_max: i64,

	/// improvement slot item consumption
	#[serde(skip_serializing_if = "Option::is_none")]
	pub slot_item_consumption: Option<Vec<Kc3rdSlotItemImproveItemConsumption>>,

	/// improvement use item consumption
	#[serde(skip_serializing_if = "Option::is_none")]
	pub use_item_consumption: Option<Vec<Kc3rdSlotItemImproveItemConsumption>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemRemodelVariant {
	/// slot item id after fully improved and transform
	pub slot_item_id: i64,

	/// initial stars after fully improved and transform
	pub initial_stars: i64,

	/// improvement requirements
	pub requirements: Kc3rdSlotItemImproveRequirements,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemImproveRequirements {
	/// level 0 to 5(inclusive) consumption
	#[serde(skip_serializing_if = "Option::is_none")]
	pub first_half: Option<Kc3rdSlotItemImprovePerLevelConsumption>,

	/// level 6 to 9(inclusive) consumption
	#[serde(skip_serializing_if = "Option::is_none")]
	pub second_half: Option<Kc3rdSlotItemImprovePerLevelConsumption>,

	/// remodel consumption
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remodel: Option<Kc3rdSlotItemImprovePerLevelConsumption>,

	/// secretary ship
	pub secretary: Vec<Kc3rdSlotItemImproveSecretary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItemImprovment {
	/// base material consumption
	pub base_consumption: Kc3rdSlotItemImproveBaseConsumption,

	/// level 0 to 9(inclusive) consumption
	#[serde(skip_serializing_if = "Option::is_none")]
	pub level_consumption: Option<Kc3rdSlotItemImproveRequirements>,

	/// remodel variants
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remodel_variants: Option<Vec<Kc3rdSlotItemRemodelVariant>>,
}

/// Slot item thirdparty information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdSlotItem {
	/// `api_id`, slot item id
	pub api_id: i64,

	/// slot item name
	pub name: String,

	/// info in picture book
	pub info: String,

	/// can be crafted
	pub craftable: bool,

	/// can attack installations
	pub can_attack_installations: bool,

	/// initial level
	#[serde(skip_serializing_if = "Option::is_none")]
	pub stars: Option<i64>,

	/// flight cost
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flight_cost: Option<i64>,

	/// flight range
	#[serde(skip_serializing_if = "Option::is_none")]
	pub flight_range: Option<i64>,

	/// asw damage type
	#[serde(skip_serializing_if = "Option::is_none")]
	pub asw_damage_type: Option<Kc3rdSlotItemAswDamageType>,

	/// improvement
	#[serde(skip_serializing_if = "Option::is_none")]
	pub improvement: Option<Kc3rdSlotItemImprovment>,
}
