//! Slot item entity

use emukc_model::profile::slot_item::SlotItem;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "slot_item")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Manifest ID
	pub mst_id: i64,

	/// locked
	pub locked: bool,

	/// modify level
	pub level: i64,

	/// aircraft level
	pub aircraft_lv: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(belongs_to = "super::Entity", from = "Column::ProfileId", to = "super::Column::Id")]
	Profile,
}

impl Related<super::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<SlotItem> for ActiveModel {
	fn from(t: SlotItem) -> Self {
		Self {
			id: ActiveValue::Set(t.instance_id),
			profile_id: ActiveValue::Set(t.id),
			mst_id: ActiveValue::Set(t.mst_id),
			locked: ActiveValue::Set(t.locked),
			level: ActiveValue::Set(t.level),
			aircraft_lv: ActiveValue::Set(t.aircraft_lv),
		}
	}
}

impl From<Model> for SlotItem {
	fn from(value: Model) -> Self {
		Self {
			instance_id: value.id,
			id: value.profile_id,
			mst_id: value.mst_id,
			locked: value.locked,
			level: value.level,
			aircraft_lv: value.aircraft_lv,
		}
	}
}
