//! Ship special effect equipment info entity

use emukc_model::kc2::KcApiSpEffectOnShip;
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "ship_sp_effect_item")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// profile ID
	pub profile_id: i64,

	/// ship instance ID
	pub ship_id: i64,

	/// index in `api_sp_effect_items`
	pub index: i64,

	/// kind
	pub kind: i64,

	/// firepower boost
	pub houg: i64,

	/// torpedo boost
	pub raig: i64,

	/// evasion boost
	pub kaih: i64,

	/// armor boost
	pub souk: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Ship`
	#[sea_orm(
		belongs_to = "crate::entity::profile::ship::Entity",
		from = "Column::ShipId",
		to = "crate::entity::profile::ship::Column::Id"
	)]
	Ship,

	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<crate::entity::profile::ship::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Ship.def()
	}
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for KcApiSpEffectOnShip {
	fn from(t: Model) -> Self {
		Self {
			api_kind: t.kind,
			api_houg: (t.houg > 0).then_some(t.houg),
			api_raig: (t.raig > 0).then_some(t.raig),
			api_kaih: (t.kaih > 0).then_some(t.kaih),
			api_souk: (t.souk > 0).then_some(t.souk),
		}
	}
}
