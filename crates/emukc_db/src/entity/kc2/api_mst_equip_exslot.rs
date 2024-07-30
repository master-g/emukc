use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_mst_equip_exslot")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,
	pub slotitem_type: String,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<&[i64]> for ActiveModel {
	fn from(value: &[i64]) -> Self {
		Self {
			id: ActiveValue::Set(0),
			slotitem_type: ActiveValue::Set(serde_json::to_string(value).unwrap()),
		}
	}
}

impl From<Model> for Vec<i64> {
	fn from(value: Model) -> Self {
		let value: Vec<i64> = serde_json::from_str(&value.slotitem_type).unwrap();
		value
	}
}
