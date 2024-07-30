use emukc_model::start2::{ApiMstConst, ApiMstValue};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_mst_const")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,
	pub boko_max_ships_int: i64,
	pub boko_max_ships_str: String,
	pub dpflag_quest_int: i64,
	pub dpflag_quest_str: String,
	pub parallel_quest_max_int: i64,
	pub parallel_quest_max_str: String,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<ApiMstConst> for ActiveModel {
	fn from(value: ApiMstConst) -> Self {
		Self {
			id: ActiveValue::Set(0),
			boko_max_ships_int: ActiveValue::Set(value.api_boko_max_ships.api_int_value),
			boko_max_ships_str: ActiveValue::Set(value.api_boko_max_ships.api_string_value),
			dpflag_quest_int: ActiveValue::Set(value.api_dpflag_quest.api_int_value),
			dpflag_quest_str: ActiveValue::Set(value.api_dpflag_quest.api_string_value),
			parallel_quest_max_int: ActiveValue::Set(value.api_parallel_quest_max.api_int_value),
			parallel_quest_max_str: ActiveValue::Set(value.api_parallel_quest_max.api_string_value),
		}
	}
}

impl From<Model> for ApiMstConst {
	fn from(value: Model) -> Self {
		Self {
			api_boko_max_ships: ApiMstValue {
				api_int_value: value.boko_max_ships_int,
				api_string_value: value.boko_max_ships_str,
			},
			api_dpflag_quest: ApiMstValue {
				api_int_value: value.dpflag_quest_int,
				api_string_value: value.dpflag_quest_str,
			},
			api_parallel_quest_max: ApiMstValue {
				api_int_value: value.parallel_quest_max_int,
				api_string_value: value.parallel_quest_max_str,
			},
		}
	}
}
