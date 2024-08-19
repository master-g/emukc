//! Practice rival entity

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
	#[sea_orm(has_many = "super::ship::Entity")]
	Ship,
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
