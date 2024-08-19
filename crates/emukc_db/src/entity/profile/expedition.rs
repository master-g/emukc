//! User expedition record

use emukc_model::profile::expedition::{Expedition, ExpeditionState};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Status {
	/// Never started
	#[sea_orm(num_value = 0)]
	NeverStarted,

	/// Unfinished
	#[sea_orm(num_value = 1)]
	Unfinished,

	/// Completed
	#[sea_orm(num_value = 2)]
	Completed,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "expedition")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Expedition ID
	pub mission_id: i64,

	/// Expedition state
	pub state: Status,
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

impl From<Status> for ExpeditionState {
	fn from(value: Status) -> Self {
		match value {
			Status::NeverStarted => ExpeditionState::NeverStarted,
			Status::Unfinished => ExpeditionState::Unfinished,
			Status::Completed => ExpeditionState::Completed,
		}
	}
}

impl From<ExpeditionState> for Status {
	fn from(value: ExpeditionState) -> Self {
		match value {
			ExpeditionState::NeverStarted => Status::NeverStarted,
			ExpeditionState::Unfinished => Status::Unfinished,
			ExpeditionState::Completed => Status::Completed,
		}
	}
}

impl From<Expedition> for ActiveModel {
	fn from(t: Expedition) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(t.id),
			mission_id: ActiveValue::Set(t.mission_id),
			state: ActiveValue::Set(t.state.into()),
		}
	}
}
