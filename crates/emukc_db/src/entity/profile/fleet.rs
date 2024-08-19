//! Fleet entity

use chrono::{DateTime, Utc};
use emukc_model::profile::fleet::{Fleet, FleetMissionContext, FleetMissionStatus};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum MissionStatus {
	/// Idle
	#[sea_orm(num_value = 0)]
	Idle,

	/// In mission
	#[sea_orm(num_value = 1)]
	InMission,

	/// Returning
	#[sea_orm(num_value = 2)]
	Returning,

	/// Force returning
	#[sea_orm(num_value = 3)]
	ForceReturning,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "fleet")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Fleet ID, 1-4
	pub index: i64,

	/// Fleet name
	pub name: String,

	/// Mission status
	pub mission_status: MissionStatus,

	/// Mission id
	pub mission_id: i64,

	/// Mission return time
	pub return_time: Option<DateTime<Utc>>,

	/// Ship 1
	pub ship_1: i64,

	/// Ship 2
	pub ship_2: i64,

	/// Ship 3
	pub ship_3: i64,

	/// Ship 4
	pub ship_4: i64,

	/// Ship 5
	pub ship_5: i64,

	/// Ship 6
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
	#[allow(unused_variables)]
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<FleetMissionStatus> for MissionStatus {
	fn from(value: FleetMissionStatus) -> Self {
		match value {
			FleetMissionStatus::Idle => MissionStatus::Idle,
			FleetMissionStatus::InMission => MissionStatus::InMission,
			FleetMissionStatus::Returning => MissionStatus::Returning,
			FleetMissionStatus::ForceReturning => MissionStatus::ForceReturning,
		}
	}
}

impl From<MissionStatus> for FleetMissionStatus {
	fn from(value: MissionStatus) -> Self {
		match value {
			MissionStatus::Idle => FleetMissionStatus::Idle,
			MissionStatus::InMission => FleetMissionStatus::InMission,
			MissionStatus::Returning => FleetMissionStatus::Returning,
			MissionStatus::ForceReturning => FleetMissionStatus::ForceReturning,
		}
	}
}

impl From<Fleet> for ActiveModel {
	fn from(value: Fleet) -> Self {
		let mission_status =
			value.mission.as_ref().map(|x| x.status.into()).unwrap_or(MissionStatus::Idle);
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			index: ActiveValue::Set(value.index),
			name: ActiveValue::Set(value.name),
			mission_status: ActiveValue::Set(mission_status),
			mission_id: ActiveValue::Set(value.mission.as_ref().map(|x| x.id).unwrap_or(0)),
			return_time: ActiveValue::Set(value.mission.as_ref().and_then(|x| x.return_time)),
			ship_1: ActiveValue::Set(value.ships[0]),
			ship_2: ActiveValue::Set(value.ships[1]),
			ship_3: ActiveValue::Set(value.ships[2]),
			ship_4: ActiveValue::Set(value.ships[3]),
			ship_5: ActiveValue::Set(value.ships[4]),
			ship_6: ActiveValue::Set(value.ships[5]),
		}
	}
}

impl From<Model> for Fleet {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			index: value.index,
			name: value.name,
			mission: if value.mission_status == MissionStatus::Idle {
				None
			} else {
				Some(FleetMissionContext {
					id: value.mission_id,
					status: value.mission_status.into(),
					return_time: value.return_time,
				})
			},
			ships: [
				value.ship_1,
				value.ship_2,
				value.ship_3,
				value.ship_4,
				value.ship_5,
				value.ship_6,
			],
		}
	}
}
