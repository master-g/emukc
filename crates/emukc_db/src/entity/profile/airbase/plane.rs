//! Aircrafts belonging to an airbase.

use emukc_model::profile::airbase::{PlaneInfo, PlaneState};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Status {
	/// Unassigned
	#[sea_orm(num_value = 0)]
	UNASSIGNED,

	/// Assigned
	#[sea_orm(num_value = 1)]
	ASSIGNED,

	/// Reassigning
	#[sea_orm(num_value = 2)]
	REASSIGNING,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "plane_info")]
pub struct Model {
	/// Slot id, slot item instance id
	#[sea_orm(primary_key)]
	pub slot_id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Area id
	pub area_id: i64,

	/// Airbase id
	pub rid: i64,

	/// Squadron id, index
	pub squadron_id: i64,

	/// Plane status
	pub state: Status,

	/// Condition
	pub condition: i64,

	/// Plane count
	pub count: i64,

	/// Max count
	pub max_count: i64,
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

impl From<Status> for PlaneState {
	fn from(value: Status) -> Self {
		match value {
			Status::UNASSIGNED => PlaneState::UNASSIGNED,
			Status::ASSIGNED => PlaneState::ASSIGNED,
			Status::REASSIGNING => PlaneState::REASSIGNING,
		}
	}
}

impl From<PlaneState> for Status {
	fn from(value: PlaneState) -> Self {
		match value {
			PlaneState::UNASSIGNED => Status::UNASSIGNED,
			PlaneState::ASSIGNED => Status::ASSIGNED,
			PlaneState::REASSIGNING => Status::REASSIGNING,
		}
	}
}

impl From<PlaneInfo> for ActiveModel {
	fn from(value: PlaneInfo) -> Self {
		Self {
			slot_id: ActiveValue::Set(value.slot_id),
			profile_id: ActiveValue::Set(value.id),
			area_id: ActiveValue::Set(value.area_id),
			rid: ActiveValue::Set(value.rid),
			squadron_id: ActiveValue::Set(value.squadron_id),
			state: ActiveValue::Set(value.state.into()),
			condition: ActiveValue::Set(value.condition),
			count: ActiveValue::Set(value.count),
			max_count: ActiveValue::Set(value.max_count),
		}
	}
}

impl From<Model> for PlaneInfo {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			area_id: value.area_id,
			rid: value.rid,
			slot_id: value.slot_id,
			squadron_id: value.squadron_id,
			state: value.state.into(),
			condition: value.condition,
			count: value.count,
			max_count: value.max_count,
		}
	}
}
