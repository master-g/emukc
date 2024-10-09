//! Construction Docks

use chrono::{DateTime, Utc};
use emukc_model::profile::kdock::{ConstructionContext, ConstructionDock, ConstructionDockStatus};
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

	/// Construction completed
	#[sea_orm(string_value = "completed")]
	Completed,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "kdock")]
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

	/// is current constuction large
	pub is_large: bool,

	/// fuel consumption
	pub fuel: i64,

	/// ammo consumption
	pub ammo: i64,

	/// steel consumption
	pub steel: i64,

	/// bauxite consumption
	pub bauxite: i64,

	/// development material consumption
	pub devmat: i64,
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

impl From<ConstructionDockStatus> for Status {
	fn from(value: ConstructionDockStatus) -> Self {
		match value {
			ConstructionDockStatus::Locked => Status::Locked,
			ConstructionDockStatus::Idle => Status::Idle,
			ConstructionDockStatus::Busy => Status::Busy,
			ConstructionDockStatus::Completed => Status::Completed,
		}
	}
}

impl From<Status> for ConstructionDockStatus {
	fn from(value: Status) -> Self {
		match value {
			Status::Locked => ConstructionDockStatus::Locked,
			Status::Idle => ConstructionDockStatus::Idle,
			Status::Busy => ConstructionDockStatus::Busy,
			Status::Completed => ConstructionDockStatus::Completed,
		}
	}
}

impl From<ConstructionDock> for ActiveModel {
	fn from(value: ConstructionDock) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			index: ActiveValue::Set(value.index),
			status: ActiveValue::Set(value.status.into()),
			ship_id: ActiveValue::Set(value.context.as_ref().map(|x| x.ship_id).unwrap_or(0)),
			complete_time: ActiveValue::Set(value.context.as_ref().map(|x| x.complete_time)),
			is_large: ActiveValue::Set(value.context.as_ref().map(|x| x.is_large).unwrap_or(false)),
			fuel: ActiveValue::Set(value.context.as_ref().map(|x| x.fuel).unwrap_or(0)),
			ammo: ActiveValue::Set(value.context.as_ref().map(|x| x.ammo).unwrap_or(0)),
			steel: ActiveValue::Set(value.context.as_ref().map(|x| x.steel).unwrap_or(0)),
			bauxite: ActiveValue::Set(value.context.as_ref().map(|x| x.bauxite).unwrap_or(0)),
			devmat: ActiveValue::Set(value.context.as_ref().map(|x| x.devmat).unwrap_or(0)),
		}
	}
}

impl From<Model> for ConstructionDock {
	fn from(value: Model) -> Self {
		let context = match value.status {
			Status::Busy | Status::Completed => Some(ConstructionContext {
				ship_id: value.ship_id,
				complete_time: value.complete_time.unwrap_or_else(Utc::now),
				is_large: value.is_large,
				fuel: value.fuel,
				ammo: value.ammo,
				steel: value.steel,
				bauxite: value.bauxite,
				devmat: value.devmat,
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
