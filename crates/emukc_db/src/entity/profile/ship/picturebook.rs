//! Ship record entity

use emukc_model::profile::picture_book::PictureBookShip;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "ship_record")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// ship sort number
	pub sort_num: i64,

	/// Has damaged record
	pub damaged: bool,

	/// Has married record
	pub married: bool,
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

impl From<PictureBookShip> for ActiveModel {
	fn from(value: PictureBookShip) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			sort_num: ActiveValue::Set(value.sort_num),
			damaged: ActiveValue::Set(value.damaged),
			married: ActiveValue::Set(value.married),
		}
	}
}

impl From<Model> for PictureBookShip {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			sort_num: value.sort_num,
			damaged: value.damaged,
			married: value.married,
		}
	}
}
