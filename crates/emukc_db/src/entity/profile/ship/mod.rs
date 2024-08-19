//! Ship related entities
use sea_orm::entity::prelude::*;

pub mod picturebook;

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "ship")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// sort number
	pub sort_num: i64,

	/// Manifest ID
	pub mst_id: i64,

	/// ship level
	pub level: i64,

	/// experience
	pub exp: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
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

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// picturebook
	{
		let stmt = schema.create_table_from_entity(picturebook::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
