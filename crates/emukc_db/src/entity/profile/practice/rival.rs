//! Practice rival entity

use emukc_model::profile::practice::{RivalFlag, RivalStatus};
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Flag {
	/// Bronze
	#[sea_orm(num_value = 1)]
	Bronze,

	/// Silver
	#[sea_orm(num_value = 2)]
	Silver,

	/// Gold
	#[sea_orm(num_value = 3)]
	Gold,
}

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Status {
	/// Untouched
	#[sea_orm(num_value = 0)]
	Untouched,

	/// Lost rank E
	#[sea_orm(num_value = 1)]
	LostRankE,

	/// Lost rank D
	#[sea_orm(num_value = 2)]
	LostRankD,

	/// Lost rank C
	#[sea_orm(num_value = 3)]
	LostRankC,

	/// Victory rank B
	#[sea_orm(num_value = 4)]
	VictoryRankB,

	/// Victory rank A
	#[sea_orm(num_value = 5)]
	VictoryRankA,

	/// Victory rank S
	#[sea_orm(num_value = 6)]
	VictoryRankS,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "rival")]
pub struct Model {
	/// rival profile ID
	#[sea_orm(primary_key)]
	pub id: i64,

	/// who this rival belongs to
	pub profile_id: i64,

	/// rival index
	pub index: i64,

	/// rival name
	pub name: String,

	/// rival comment
	pub comment: String,

	/// rival level
	pub level: i64,

	/// rival rank
	pub rank: i64,

	/// rival flag
	pub flag: Flag,

	/// rival status
	pub status: Status,

	/// rival medals
	pub medals: i64,
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

	/// Relation to `Detail`
	#[sea_orm(has_one = "super::detail::Entity")]
	Detail,

	/// Relation to `Ship`
	#[sea_orm(has_many = "super::rival_ship::Entity")]
	Ship,
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<RivalFlag> for Flag {
	fn from(value: RivalFlag) -> Self {
		match value {
			RivalFlag::Bronze => Self::Bronze,
			RivalFlag::Silver => Self::Silver,
			RivalFlag::Gold => Self::Gold,
		}
	}
}

impl From<Flag> for RivalFlag {
	fn from(value: Flag) -> Self {
		match value {
			Flag::Bronze => Self::Bronze,
			Flag::Silver => Self::Silver,
			Flag::Gold => Self::Gold,
		}
	}
}

impl From<RivalStatus> for Status {
	fn from(value: RivalStatus) -> Self {
		match value {
			RivalStatus::Untouched => Self::Untouched,
			RivalStatus::LostRankE => Self::LostRankE,
			RivalStatus::LostRankD => Self::LostRankD,
			RivalStatus::LostRankC => Self::LostRankC,
			RivalStatus::VictoryRankB => Self::VictoryRankB,
			RivalStatus::VictoryRankA => Self::VictoryRankA,
			RivalStatus::VictoryRankS => Self::VictoryRankS,
		}
	}
}

impl From<Status> for RivalStatus {
	fn from(value: Status) -> Self {
		match value {
			Status::Untouched => Self::Untouched,
			Status::LostRankE => Self::LostRankE,
			Status::LostRankD => Self::LostRankD,
			Status::LostRankC => Self::LostRankC,
			Status::VictoryRankB => Self::VictoryRankB,
			Status::VictoryRankA => Self::VictoryRankA,
			Status::VictoryRankS => Self::VictoryRankS,
		}
	}
}
