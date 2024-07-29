use emukc_model::start2::ApiMstBgm;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_mst_bgm")]
pub struct Model {
	/// Primary key, `api_id`
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,

	pub name: String,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<ApiMstBgm> for ActiveModel {
	fn from(t: ApiMstBgm) -> Self {
		Self {
			id: ActiveValue::Set(t.api_id),
			name: ActiveValue::Set(t.api_name),
		}
	}
}

impl From<Model> for ApiMstBgm {
	fn from(t: Model) -> Self {
		Self {
			api_id: t.id,
			api_name: t.name,
		}
	}
}
