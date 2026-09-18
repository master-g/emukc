//! Deck preset entity

use sea_orm::entity::prelude::*;

#[expect(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "preset_caps")]
pub struct Model {
    /// Instance ID, use `profile_id` as primary key
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,

    /// preset deck max limit
    pub deck_limit: i64,

    /// preset slot max limit
    pub slot_limit: i64,

    /// preset dev item max limit
    pub dev_item_limit: i64,
}

crate::entity::profile_relation!("Column::Id");
