//! Preset development item.
use sea_orm::entity::prelude::*;

/// Preset development item.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "preset_dev_item")]
pub struct Model {
    /// primary key
    #[sea_orm(primary_key)]
    pub id: i64,

    /// foreign key to `profile` table
    pub profile_id: i64,

    /// index of the item in the preset, starting from 0
    pub index: i64,

    /// name of the item
    pub name: String,

    /// fuel
    pub item1: i64,

    /// ammo
    pub item2: i64,

    /// steel
    pub item3: i64,

    /// bauxite
    pub item4: i64,
}

crate::entity::profile_relation!("Column::ProfileId");
