use emukc_model::kc2::{KcApiIncentiveItem, KcApiIncentiveMode, KcApiIncentiveType};
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum IncentiveType {
	#[sea_orm(num_value = 1)]
	Ship,
	#[sea_orm(num_value = 2)]
	SlotItem,
	#[sea_orm(num_value = 3)]
	UseItem,
	#[sea_orm(num_value = 4)]
	Resource,
	#[sea_orm(num_value = 5)]
	Furniture,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum IncentiveMode {
	#[sea_orm(num_value = 1)]
	PreRegister,
	#[sea_orm(num_value = 2)]
	Reception,
	#[sea_orm(num_value = 3)]
	MonthlyOrPresent,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "profile")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Incentive mode
	pub mode: IncentiveMode,

	/// Incentive type
	pub typ: IncentiveType,

	/// manifest ID
	pub mst_id: i64,

	/// for ship
	pub get_me: Option<String>,

	/// for slot item
	pub stars: Option<i64>,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<crate::entity::profile::Entity> for Entity {
	#[allow(unused_variables)]
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<KcApiIncentiveType> for IncentiveType {
	fn from(value: KcApiIncentiveType) -> Self {
		match value {
			KcApiIncentiveType::Ship => IncentiveType::Ship,
			KcApiIncentiveType::SlotItem => IncentiveType::SlotItem,
			KcApiIncentiveType::UseItem => IncentiveType::UseItem,
			KcApiIncentiveType::Resource => IncentiveType::Resource,
			KcApiIncentiveType::Furniture => IncentiveType::Furniture,
		}
	}
}

impl From<IncentiveType> for KcApiIncentiveType {
	fn from(value: IncentiveType) -> Self {
		match value {
			IncentiveType::Ship => KcApiIncentiveType::Ship,
			IncentiveType::SlotItem => KcApiIncentiveType::SlotItem,
			IncentiveType::UseItem => KcApiIncentiveType::UseItem,
			IncentiveType::Resource => KcApiIncentiveType::Resource,
			IncentiveType::Furniture => KcApiIncentiveType::Furniture,
		}
	}
}

impl From<KcApiIncentiveMode> for IncentiveMode {
	fn from(value: KcApiIncentiveMode) -> Self {
		match value {
			KcApiIncentiveMode::PreRegister => IncentiveMode::PreRegister,
			KcApiIncentiveMode::Reception => IncentiveMode::Reception,
			KcApiIncentiveMode::MonthlyOrPresent => IncentiveMode::MonthlyOrPresent,
		}
	}
}

impl From<IncentiveMode> for KcApiIncentiveMode {
	fn from(value: IncentiveMode) -> Self {
		match value {
			IncentiveMode::PreRegister => KcApiIncentiveMode::PreRegister,
			IncentiveMode::Reception => KcApiIncentiveMode::Reception,
			IncentiveMode::MonthlyOrPresent => KcApiIncentiveMode::MonthlyOrPresent,
		}
	}
}

impl From<Model> for KcApiIncentiveItem {
	fn from(value: Model) -> Self {
		Self {
			api_mode: value.mode as i64,
			api_type: value.typ as i64,
			api_mst_id: value.mst_id,
			api_getmes: value.get_me.to_owned(),
			api_slotitem_level: value.stars,
		}
	}
}
