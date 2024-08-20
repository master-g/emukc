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

	/// Married
	pub married: bool,

	/// current hp
	pub hp_now: i64,

	/// maximum hp
	pub hp_max: i64,

	/// speed, soku
	pub speed: i64,

	/// range, leng
	pub range: i64,

	/// slots, first
	pub slot_1: i64,

	/// slots, second
	pub slot_2: i64,

	/// slots, third
	pub slot_3: i64,

	/// slots, fourth
	pub slot_4: i64,

	/// slots, fifth
	pub slot_5: i64,

	/// extra slots
	pub slot_ex: i64,

	/// aircraft capacity left
	pub onslot_1: i64,

	/// aircraft capacity left
	pub onslot_2: i64,

	/// aircraft capacity left
	pub onslot_3: i64,

	/// aircraft capacity left
	pub onslot_4: i64,

	/// aircraft capacity left
	pub onslot_5: i64,

	/// modrenization, firepower
	pub mod_firepower: i64,

	/// modrenization, torpedo
	pub mod_torpedo: i64,

	/// modrenization, AA
	pub mod_aa: i64,

	/// modrenization, armor
	pub mod_armor: i64,

	/// modrenization, luck
	pub mod_luck: i64,

	/// modrenization, HP
	pub mod_hp: i64,

	/// modrenization, ASW
	pub mod_asw: i64,

	/// fuel left
	pub fuel: i64,

	/// ammo left
	pub ammo: i64,
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
