//! User expedition record

use chrono::{DateTime, Utc};
use emukc_model::profile::expedition::ExpeditionState;
use sea_orm::entity::prelude::*;

#[expect(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Status {
    /// Not started
    #[sea_orm(num_value = 0)]
    NotStarted,

    /// Unfinished
    #[sea_orm(num_value = 1)]
    Unfinished,

    /// Completed
    #[sea_orm(num_value = 2)]
    Completed,
}

#[expect(missing_docs)]
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

    /// Last completed time
    pub last_completed_at: Option<DateTime<Utc>>,
}

crate::entity::profile_relation!("Column::ProfileId");

impl From<Status> for ExpeditionState {
    fn from(value: Status) -> Self {
        match value {
            Status::NotStarted => ExpeditionState::NotStarted,
            Status::Unfinished => ExpeditionState::Unfinished,
            Status::Completed => ExpeditionState::Completed,
        }
    }
}

impl From<ExpeditionState> for Status {
    fn from(value: ExpeditionState) -> Self {
        match value {
            ExpeditionState::NotStarted => Status::NotStarted,
            ExpeditionState::Unfinished => Status::Unfinished,
            ExpeditionState::Completed => Status::Completed,
        }
    }
}
