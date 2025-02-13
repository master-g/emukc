//! Slot item entity

use emukc_model::{kc2::KcApiSlotItem, profile::slot_item::SlotItem};
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "slot_item")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Manifest ID
	pub mst_id: i64,

	/// type3 in `api_mst_slotitem`
	pub type3: i64,

	/// locked
	pub locked: bool,

	/// modify level
	pub level: i64,

	/// aircraft level
	pub aircraft_lv: i64,

	/// equip on ship instance id
	pub equip_on: i64,
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

impl From<Model> for KcApiSlotItem {
	fn from(value: Model) -> Self {
		Self {
			api_id: value.id,
			api_slotitem_id: value.mst_id,
			api_locked: value.locked as i64,
			api_level: value.level,
			api_alv: (value.aircraft_lv > 0).then_some(value.aircraft_lv),
		}
	}
}
