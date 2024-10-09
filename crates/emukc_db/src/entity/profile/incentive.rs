//! Incentive entity
#![allow(missing_docs)]

use emukc_model::kc2::{KcApiIncentiveMode, KcApiIncentiveType};
use sea_orm::entity::prelude::*;

/// Incentive type
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum, enumn::N,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum IncentiveType {
	/// Ship
	#[sea_orm(num_value = 1)]
	Ship = 1,

	/// Slot item
	#[sea_orm(num_value = 2)]
	SlotItem = 2,

	/// Use item
	#[sea_orm(num_value = 3)]
	UseItem = 3,

	/// Resource
	#[sea_orm(num_value = 4)]
	Resource = 4,

	/// Furniture
	#[sea_orm(num_value = 5)]
	Furniture = 5,
}

#[allow(missing_docs)]
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum, enumn::N,
)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum IncentiveMode {
	#[sea_orm(num_value = 1)]
	PreRegister = 1,
	#[sea_orm(num_value = 2)]
	Reception = 2,
	#[sea_orm(num_value = 3)]
	MonthlyOrPresent = 3,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "incentive")]
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

	/// amount
	pub amount: i64,

	/// for slot item
	pub stars: Option<i64>,

	/// for slot item, aircraft level
	pub alv: Option<i64>,
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

#[cfg(test)]
mod tests {
	use crate::entity::profile::incentive::IncentiveType;

	#[test]
	fn test_enum() {
		assert_eq!(IncentiveType::Ship as i64, 1);
		assert_eq!(IncentiveType::SlotItem as i64, 2);
		assert_eq!(IncentiveType::UseItem as i64, 3);

		assert_eq!(IncentiveType::n(1).unwrap(), IncentiveType::Ship);
	}
}
