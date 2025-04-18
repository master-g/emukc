use chrono::{DateTime, Utc};
use sea_orm::{ActiveValue, entity::prelude::*};

use emukc_model::user::account::Account;

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "account")]
pub struct Model {
	/// Primary key, `uid`
	#[sea_orm(primary_key, auto_increment = true)]
	pub uid: i64,

	pub name: String,

	pub secret: String,

	pub create_time: DateTime<Utc>,

	pub last_login: DateTime<Utc>,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Token`
	#[sea_orm(has_many = "super::token::Entity")]
	Token,

	/// Relation to `Profile`
	#[sea_orm(has_many = "crate::entity::profile::Entity")]
	Profile,
}

impl Related<super::token::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Token.def()
	}
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Account> for ActiveModel {
	fn from(t: Account) -> Self {
		Self {
			uid: ActiveValue::NotSet,
			name: ActiveValue::Set(t.name),
			secret: ActiveValue::Set(t.secret),
			create_time: ActiveValue::Set(t.create_time),
			last_login: ActiveValue::Set(t.last_login),
		}
	}
}

impl From<Model> for Account {
	fn from(value: Model) -> Self {
		Self {
			uid: value.uid,
			name: value.name,
			secret: value.secret,
			create_time: value.create_time,
			last_login: value.last_login,
		}
	}
}
