//! Deck preset entity

use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "preset_caps")]
pub struct Model {
	/// Instance ID, use `profile_id` as primary key
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,

	/// preset deck max limit
	pub deck_limit: i64,

	/// preset slot max limit
	pub slot_limit: i64,
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
