use emukc_model::profile::Profile;
use sea_orm::{entity::prelude::*, ActiveValue};

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
	/// Relation to `Token`
	#[sea_orm(has_one = "material::Entity")]
	Material,
}

impl Related<material::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Material.def()
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
