//! Furniture inventory entity

use sea_orm::entity::prelude::*;

#[expect(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "furniture_record")]
pub struct Model {
    /// Instance ID
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    /// Profile ID
    pub profile_id: i64,

    /// Furniture ID
    pub furniture_id: i64,
}

crate::entity::profile_relation!("Column::ProfileId");
