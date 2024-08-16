use emukc_model::profile::Profile;
use sea_orm::{entity::prelude::*, ActiveValue};

pub mod airbase;
pub mod expedition;
pub mod fleet;
pub mod furniture;
pub mod item;
pub mod material;

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "profile")]
pub struct Model {
	/// Profile ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// account id
	pub account_id: i64,

	/// name
	pub name: String,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Account`
	#[sea_orm(
		belongs_to = "crate::entity::user::account::Entity",
		from = "Column::AccountId",
		to = "crate::entity::user::account::Column::Uid"
	)]
	Account,

	/// Relation to `Airbase`
	#[sea_orm(has_many = "airbase::base::Entity")]
	Airbase,

	/// Relation to `AirbaseExtend`
	#[sea_orm(has_many = "airbase::extend::Entity")]
	AirbaseExtend,

	/// Relation to `Expedition`
	#[sea_orm(has_many = "expedition::Entity")]
	Expedition,

	/// Relation to `Fleet`
	#[sea_orm(has_many = "fleet::Entity")]
	Fleet,

	/// Relation to `Furniture`
	#[sea_orm(has_many = "furniture::Entity")]
	Furniture,

	/// Relation to `Material`
	#[sea_orm(has_one = "material::Entity")]
	Material,

	/// Relation to `PayItem`
	#[sea_orm(has_many = "item::pay_item::Entity")]
	PayItem,

	/// Relation to `PlaneInfo`
	#[sea_orm(has_many = "airbase::plane::Entity")]
	PlaneInfo,

	/// Relation to `SlotItem`
	#[sea_orm(has_many = "item::slot_item::Entity")]
	SlotItem,

	/// Relation to `UseItem`
	#[sea_orm(has_many = "item::use_item::Entity")]
	UseItem,
}

impl Related<material::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Material.def()
	}
}

impl Related<crate::entity::user::account::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Account.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Profile> for ActiveModel {
	fn from(t: Profile) -> Self {
		Self {
			id: ActiveValue::Set(t.id),
			account_id: ActiveValue::Set(t.account_id),
			name: ActiveValue::Set(t.name),
		}
	}
}

impl From<Model> for Profile {
	fn from(value: Model) -> Self {
		Self {
			id: value.id,
			account_id: value.account_id,
			name: value.name,
		}
	}
}

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());

	// profile
	{
		let stmt = schema.create_table_from_entity(Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// airbase
	{
		airbase::bootstrap(db).await?;
	}
	// expedition
	{
		let stmt = schema.create_table_from_entity(expedition::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// fleet
	{
		let stmt = schema.create_table_from_entity(fleet::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// furniture
	{
		let stmt = schema.create_table_from_entity(furniture::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// material
	{
		let stmt = schema.create_table_from_entity(material::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// items
	{
		item::bootstrap(db).await?;
	}

	Ok(())
}
