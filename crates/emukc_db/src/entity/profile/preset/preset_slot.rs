//! Slot preset entity

use emukc_model::profile::preset_slot::{
	PresetSlotItemElement, PresetSlotItemSelectMode, PresetSlotItemSlot,
};
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum SelectMode {
	/// A
	#[sea_orm(num_value = 1)]
	A,

	/// B
	#[sea_orm(num_value = 2)]
	B,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "preset_slot")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// index
	pub index: i64,

	/// preset name
	pub name: String,

	/// select mode
	pub mode: SelectMode,

	/// locked
	pub locked: bool,

	/// ex slot flag
	pub ex_flag: bool,

	/// slot 1 `mst_id`
	pub mst_id_1: i64,

	/// slot 1 stars
	pub stars_1: i64,

	/// slot 2 `mst_id`
	pub mst_id_2: i64,

	/// slot 2 stars
	pub stars_2: i64,

	/// slot 3 `mst_id`
	pub mst_id_3: i64,

	/// slot 3 stars
	pub stars_3: i64,

	/// slot 4 `mst_id`
	pub mst_id_4: i64,

	/// slot 4 stars
	pub stars_4: i64,

	/// slot 5 `mst_id`
	pub mst_id_5: i64,

	/// slot 5 stars
	pub stars_5: i64,

	/// slot ex `mst_id`
	pub mst_id_ex: i64,

	/// slot ex stars
	pub stars_ex: i64,
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

impl From<SelectMode> for PresetSlotItemSelectMode {
	fn from(value: SelectMode) -> Self {
		match value {
			SelectMode::A => Self::A,
			SelectMode::B => Self::B,
		}
	}
}

impl From<Model> for PresetSlotItemElement {
	fn from(value: Model) -> Self {
		let slots: Vec<PresetSlotItemSlot> = [
			(value.mst_id_1, value.stars_1),
			(value.mst_id_2, value.stars_2),
			(value.mst_id_3, value.stars_3),
			(value.mst_id_4, value.stars_4),
			(value.mst_id_5, value.stars_5),
		]
		.iter()
		.filter_map(|(mst_id, stars)| {
			if *mst_id != 0 {
				Some(PresetSlotItemSlot {
					mst_id: *mst_id,
					stars: *stars,
				})
			} else {
				None
			}
		})
		.collect();

		let ex = if value.mst_id_ex != 0 {
			Some(PresetSlotItemSlot {
				mst_id: value.mst_id_ex,
				stars: value.stars_ex,
			})
		} else {
			None
		};

		Self {
			profile_id: value.profile_id,
			index: value.index,
			name: value.name,
			select_mode: value.mode.into(),
			locked: value.locked,
			ex_flag: value.ex_flag,
			slots,
			ex,
		}
	}
}
