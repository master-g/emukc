//! Player furniture configuration entity.

use emukc_model::profile::furniture::FurnitureConfig;
use sea_orm::entity::prelude::*;

#[expect(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "furniture_config")]
pub struct Model {
    /// Profile ID
    #[sea_orm(primary_key)]
    pub id: i64,

    /// Floor
    pub floor: i64,

    /// Wallpaper
    pub wallpaper: i64,

    /// Window
    pub window: i64,

    /// Wall hanging
    pub wall_hanging: i64,

    /// Shelf
    pub shelf: i64,

    /// Desk
    pub desk: i64,

    /// season, ???
    pub season: i64,
}

crate::entity::profile_relation!("Column::Id");

impl From<Model> for FurnitureConfig {
    fn from(model: Model) -> Self {
        Self {
            floor: model.floor,
            wallpaper: model.wallpaper,
            window: model.window,
            wall_hanging: model.wall_hanging,
            shelf: model.shelf,
            desk: model.desk,
            season: model.season,
        }
    }
}
