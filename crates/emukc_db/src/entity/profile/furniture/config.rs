//! Player furniture configuration entity.

use emukc_model::profile::furniture::FurnitureConfig;
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
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

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::Id",
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
