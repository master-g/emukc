//! Map record entity

use emukc_model::profile::map_record::MapRecord;
use sea_orm::{entity::prelude::*, ActiveValue};

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

	/// Defeat count
	pub defeat_count: Option<i64>,

	/// Current map HP
	pub current_hp: Option<i64>,
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

impl From<MapRecord> for ActiveModel {
	fn from(value: MapRecord) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			map_id: ActiveValue::Set(value.map_id),
			cleared: ActiveValue::Set(value.cleared),
			defeat_count: ActiveValue::Set(value.defeat_count),
			current_hp: ActiveValue::Set(value.current_hp),
		}
	}
}

impl From<Model> for MapRecord {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			map_id: value.map_id,
			cleared: value.cleared,
			defeat_count: value.defeat_count,
			current_hp: value.current_hp,
		}
	}
}
