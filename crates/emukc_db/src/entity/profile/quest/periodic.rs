//! Periodic quest record

use chrono::{DateTime, Utc};
use emukc_model::profile::quest::QuestPeriodicRecord;
use sea_orm::{ActiveValue, entity::prelude::*};

use super::{HasTimestampAndPeriod, Period};

#[expect(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "quest_record_periodic")]
pub struct Model {
    /// Instance ID
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    /// Profile ID
    pub profile_id: i64,

    /// Quest ID
    pub quest_id: i64,

    /// Complete time
    pub complete_time: DateTime<Utc>,

    /// Period
    pub period: Period,
}

crate::entity::profile_relation!("Column::ProfileId");

impl HasTimestampAndPeriod for Model {
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.complete_time
    }

    fn period(&self) -> Period {
        self.period
    }
}

impl From<QuestPeriodicRecord> for ActiveModel {
    fn from(record: QuestPeriodicRecord) -> Self {
        Self {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(record.id),
            quest_id: ActiveValue::Set(record.quest_id),
            complete_time: ActiveValue::Set(record.complete_time),
            period: ActiveValue::Set(
                record.period.try_into().expect("unknown period filtered upstream"),
            ),
        }
    }
}

impl From<Model> for QuestPeriodicRecord {
    fn from(value: Model) -> Self {
        Self {
            id: value.profile_id,
            quest_id: value.quest_id,
            complete_time: value.complete_time,
            period: value.period.into(),
        }
    }
}
