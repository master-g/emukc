//! Ship morale regeneration timer

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[expect(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "ship_morale_timer")]
pub struct Model {
    /// Instance ID, use profile ID
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,

    pub last_time_regen: Option<DateTime<Utc>>,
}

crate::entity::profile_relation!("Column::Id");
