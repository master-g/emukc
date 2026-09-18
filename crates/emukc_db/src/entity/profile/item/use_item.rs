//! Use Item Entity

use emukc_model::profile::user_item::UserItem;
use sea_orm::{ActiveValue, entity::prelude::*};

#[expect(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "use_item")]
pub struct Model {
    /// instance ID
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    /// Profile ID
    pub profile_id: i64,

    /// Manifest ID
    pub mst_id: i64,

    /// Item count
    pub count: i64,
}

crate::entity::profile_relation!("Column::ProfileId");

impl From<UserItem> for ActiveModel {
    fn from(t: UserItem) -> Self {
        Self {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(t.id),
            mst_id: ActiveValue::Set(t.mst_id),
            count: ActiveValue::Set(t.count),
        }
    }
}

impl From<Model> for UserItem {
    fn from(value: Model) -> Self {
        Self {
            id: value.profile_id,
            mst_id: value.mst_id,
            count: value.count,
        }
    }
}
