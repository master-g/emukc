//! Practice rival details entities

use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "rival_detail")]
pub struct Model {
	/// Profile ID
	#[sea_orm(primary_key)]
	pub profile_id: i64,

	/// Experience now
	pub exp_now: i64,

	/// Experience next
	pub exp_next: i64,

	/// Friend
	pub friend: i64,

	/// Current ship count
	pub current_ship_count: i64,

	/// Ship capacity
	pub ship_capacity: i64,

	/// Current slot item count
	pub current_slot_item_count: i64,

	/// Slot item capacity
	pub slot_item_capacity: i64,

	/// Furniture
	pub furniture: i64,

	/// Deck name
	pub deck_name: String,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Rival`
	#[sea_orm(
		belongs_to = "super::rival::Entity",
		from = "Column::ProfileId",
		to = "super::rival::Column::Id"
	)]
	Rival,
}

impl Related<super::rival::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Rival.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}