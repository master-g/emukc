use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "preset_dev_item")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i64,
	pub profile_id: i64,
	pub index: i64,
	pub name: String,
	pub item1: i64,
	pub item2: i64,
	pub item3: i64,
	pub item4: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::super::Entity",
		from = "Column::ProfileId",
		to = "super::super::Column::Id"
	)]
	Profile,
}

impl Related<super::super::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
