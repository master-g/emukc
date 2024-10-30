//! Map record entity

use chrono::{DateTime, Utc};
use emukc_model::profile::map_record::MapSelectRank;
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum SelectedRank {
	/// Not set
	#[sea_orm(num_value = 0)]
	NotSet = 0,

	/// 丁
	#[sea_orm(num_value = 1)]
	Casual = 1,

	/// 丙
	#[sea_orm(num_value = 2)]
	Easy = 2,

	/// 乙
	#[sea_orm(num_value = 3)]
	Normal = 3,

	/// 甲
	#[sea_orm(num_value = 4)]
	Hard = 4,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "map_record")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Map ID
	pub map_id: i64,

	/// Has cleared
	pub cleared: bool,

	/// Last cleared time
	pub last_cleared_at: Option<DateTime<Utc>>,

	/// Defeat count
	pub defeat_count: Option<i64>,

	/// Current map HP
	pub current_hp: Option<i64>,

	/// Event selected rank
	pub selected_rank: SelectedRank,
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
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<SelectedRank> for MapSelectRank {
	fn from(value: SelectedRank) -> Self {
		match value {
			SelectedRank::NotSet => MapSelectRank::NotSet,
			SelectedRank::Casual => MapSelectRank::Casual,
			SelectedRank::Easy => MapSelectRank::Easy,
			SelectedRank::Normal => MapSelectRank::Normal,
			SelectedRank::Hard => MapSelectRank::Hard,
		}
	}
}

impl From<MapSelectRank> for SelectedRank {
	fn from(value: MapSelectRank) -> Self {
		match value {
			MapSelectRank::NotSet => SelectedRank::NotSet,
			MapSelectRank::Casual => SelectedRank::Casual,
			MapSelectRank::Easy => SelectedRank::Easy,
			MapSelectRank::Normal => SelectedRank::Normal,
			MapSelectRank::Hard => SelectedRank::Hard,
		}
	}
}
