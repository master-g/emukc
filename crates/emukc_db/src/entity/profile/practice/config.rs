//! Practice config entities

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use emukc_model::profile::practice::RivalType as RivalTypeModel;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum RivalType {
	/// First group
	#[sea_orm(num_value = 0)]
	FirstGroup,

	/// Second group
	#[sea_orm(num_value = 1)]
	SecondGroup,

	/// All
	#[sea_orm(num_value = 2)]
	All,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "practice_config")]
pub struct Model {
	/// Profile ID
	#[sea_orm(primary_key)]
	pub id: i64,

	/// Selected rival type
	pub selected_type: RivalType,

	/// Generated rival type
	pub generated_type: RivalType,

	/// Last generated time
	pub last_generated: DateTime<Utc>,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::Id",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<RivalTypeModel> for RivalType {
	fn from(value: RivalTypeModel) -> Self {
		match value {
			RivalTypeModel::FirstGroup => RivalType::FirstGroup,
			RivalTypeModel::SecondaryGroup => RivalType::SecondGroup,
			RivalTypeModel::All => RivalType::All,
		}
	}
}

impl From<RivalType> for RivalTypeModel {
	fn from(value: RivalType) -> Self {
		match value {
			RivalType::FirstGroup => RivalTypeModel::FirstGroup,
			RivalType::SecondGroup => RivalTypeModel::SecondaryGroup,
			RivalType::All => RivalTypeModel::All,
		}
	}
}
