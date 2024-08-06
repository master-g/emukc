use sea_orm::entity::prelude::*;

/// Id type enum
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum IdType {
	/// Account ID
	#[sea_orm(string_value = "account")]
	Account,

	/// Profile ID
	#[sea_orm(string_value = "profile")]
	Profile,

	/// Ship ID
	#[sea_orm(string_value = "ship")]
	Ship,

	/// Slot item ID
	#[sea_orm(string_value = "slot_item")]
	SlotItem,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "id_generator")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: IdType,

	pub current: i64,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
