//! Airbase Entity

use emukc_model::profile::airbase::{Airbase, AirbaseAction};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Action {
	#[sea_orm(num_value = 0)]
	Idle,
	#[sea_orm(num_value = 1)]
	Attack,
	#[sea_orm(num_value = 2)]
	Defense,
	#[sea_orm(num_value = 3)]
	Evasion,
	#[sea_orm(num_value = 4)]
	Resort,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "airbase")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Area ID
	pub area_id: i64,

	/// Airbase ID
	pub rid: i64,

	/// action
	pub action: Action,

	/// base range
	pub base_range: i64,

	/// bonus range
	pub bonus_range: i64,

	/// name
	pub name: String,
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

impl From<AirbaseAction> for Action {
	fn from(value: AirbaseAction) -> Self {
		match value {
			AirbaseAction::Idle => Action::Idle,
			AirbaseAction::Attack => Action::Attack,
			AirbaseAction::Defense => Action::Defense,
			AirbaseAction::Evasion => Action::Evasion,
			AirbaseAction::Resort => Action::Resort,
		}
	}
}

impl From<Action> for AirbaseAction {
	fn from(value: Action) -> Self {
		match value {
			Action::Idle => AirbaseAction::Idle,
			Action::Attack => AirbaseAction::Attack,
			Action::Defense => AirbaseAction::Defense,
			Action::Evasion => AirbaseAction::Evasion,
			Action::Resort => AirbaseAction::Resort,
		}
	}
}

impl From<Airbase> for ActiveModel {
	fn from(t: Airbase) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(t.id),
			area_id: ActiveValue::Set(t.area_id),
			rid: ActiveValue::Set(t.rid),
			action: ActiveValue::Set(t.action.into()),
			base_range: ActiveValue::Set(t.base_range),
			bonus_range: ActiveValue::Set(t.bonus_range),
			name: ActiveValue::Set(t.name),
		}
	}
}

impl From<Model> for Airbase {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			area_id: value.area_id,
			rid: value.rid,
			action: value.action.into(),
			base_range: value.base_range,
			bonus_range: value.bonus_range,
			name: value.name,
		}
	}
}
