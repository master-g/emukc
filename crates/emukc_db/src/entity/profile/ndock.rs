//! Construction Docks

use chrono::{DateTime, Utc};
use emukc_model::profile::ndock::{RepairContext, RepairDock, RepairDockStatus};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Status {
	/// Locked
	#[sea_orm(string_value = "locked")]
	Locked,

	/// Idle
	#[sea_orm(string_value = "idle")]
	Idle,

	/// In construction
	#[sea_orm(string_value = "busy")]
	Busy,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "ndock")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Index
	pub index: i64,

	/// Status
	pub status: Status,

	/// ship ID
	pub ship_id: i64,

	/// complete time
	pub complete_time: Option<DateTime<Utc>>,

	/// last update time
	pub last_update: Option<DateTime<Utc>>,

	/// fuel consumption
	pub fuel: i64,

	/// steel consumption
	pub steel: i64,
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

impl From<RepairDockStatus> for Status {
	fn from(value: RepairDockStatus) -> Self {
		match value {
			RepairDockStatus::Locked => Status::Locked,
			RepairDockStatus::Idle => Status::Idle,
			RepairDockStatus::Busy => Status::Busy,
		}
	}
}

impl From<Status> for RepairDockStatus {
	fn from(value: Status) -> Self {
		match value {
			Status::Locked => RepairDockStatus::Locked,
			Status::Idle => RepairDockStatus::Idle,
			Status::Busy => RepairDockStatus::Busy,
		}
	}
}

impl From<RepairDock> for ActiveModel {
	fn from(value: RepairDock) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			index: ActiveValue::Set(value.index),
			status: ActiveValue::Set(value.status.into()),
			ship_id: ActiveValue::Set(value.context.as_ref().map(|x| x.ship_id).unwrap_or(0)),
			complete_time: ActiveValue::Set(value.context.as_ref().map(|x| x.complete_time)),
			last_update: ActiveValue::Set(value.context.as_ref().map(|x| x.last_update)),
			fuel: ActiveValue::Set(value.context.as_ref().map(|x| x.fuel).unwrap_or(0)),
			steel: ActiveValue::Set(value.context.as_ref().map(|x| x.steel).unwrap_or(0)),
		}
	}
}

impl From<Model> for RepairDock {
	fn from(value: Model) -> Self {
		let context = match value.status {
			Status::Busy => Some(RepairContext {
				ship_id: value.ship_id,
				complete_time: value.complete_time.unwrap_or_else(Utc::now),
				last_update: value.last_update.unwrap_or_else(Utc::now),
				fuel: value.fuel,
				steel: value.steel,
			}),
			_ => None,
		};

		Self {
			id: value.profile_id,
			index: value.index,
			status: value.status.into(),
			context,
		}
	}
}
