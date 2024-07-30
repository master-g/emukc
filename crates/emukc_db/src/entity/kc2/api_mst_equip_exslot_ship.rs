use emukc_model::start2::ApiMstEquipExslotShip;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_mst_equip_exslot_ship")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,
	pub config: String,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<(i64, ApiMstEquipExslotShip)> for ActiveModel {
	fn from(value: (i64, ApiMstEquipExslotShip)) -> Self {
		Self {
			id: ActiveValue::Set(value.0),
			config: ActiveValue::Set(serde_json::to_string(&value.1).unwrap()),
		}
	}
}

impl From<Model> for (i64, ApiMstEquipExslotShip) {
	fn from(value: Model) -> Self {
		(value.id, serde_json::from_str(&value.config).unwrap())
	}
}
