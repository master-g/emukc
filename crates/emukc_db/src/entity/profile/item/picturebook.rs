//! Slot item picture entity

use emukc_model::profile::picture_book::PictureBookSlotItem;
use sea_orm::{ActiveValue, entity::prelude::*};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "slotitem_record")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// slotitem sort number
	pub sort_num: i64,
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

impl From<PictureBookSlotItem> for ActiveModel {
	fn from(value: PictureBookSlotItem) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			sort_num: ActiveValue::Set(value.sort_num),
		}
	}
}

impl From<Model> for PictureBookSlotItem {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			sort_num: value.sort_num,
		}
	}
}
