//! Deck preset entity

use emukc_model::profile::preset_deck::PresetDeckItem;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "preset_deck")]
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

	/// ship 1
	pub ship_1: i64,

	/// ship 2
	pub ship_2: i64,

	/// ship 3
	pub ship_3: i64,

	/// ship 4
	pub ship_4: i64,

	/// ship 5
	pub ship_5: i64,

	/// ship 6
	pub ship_6: i64,
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

impl From<PresetDeckItem> for ActiveModel {
	fn from(value: PresetDeckItem) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.profile_id),
			index: ActiveValue::Set(value.index),
			name: ActiveValue::Set(value.name),
			ship_1: ActiveValue::Set(value.ships[0]),
			ship_2: ActiveValue::Set(value.ships[1]),
			ship_3: ActiveValue::Set(value.ships[2]),
			ship_4: ActiveValue::Set(value.ships[3]),
			ship_5: ActiveValue::Set(value.ships[4]),
			ship_6: ActiveValue::Set(value.ships[5]),
		}
	}
}

impl From<Model> for PresetDeckItem {
	fn from(value: Model) -> Self {
		Self {
			profile_id: value.profile_id,
			index: value.index,
			name: value.name,
			ships: [
				value.ship_1,
				value.ship_2,
				value.ship_3,
				value.ship_4,
				value.ship_5,
				value.ship_6,
				-1,
			],
		}
	}
}
