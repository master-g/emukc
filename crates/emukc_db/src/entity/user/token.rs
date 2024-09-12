use chrono::{DateTime, Utc};
use emukc_model::user::token::{Token, TokenType};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "token")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	pub uid: i64,

	pub profile_id: i64,

	pub typ: TokenTypeDef,

	pub token: String,

	pub expire: DateTime<Utc>,
}

/// Token type definition
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(1))")]
pub enum TokenTypeDef {
	/// Access token
	#[sea_orm(string_value = "A")]
	Access,

	/// Refresh token
	#[sea_orm(string_value = "R")]
	Refresh,

	/// Game Session
	#[sea_orm(string_value = "S")]
	Session,
}

impl From<TokenType> for TokenTypeDef {
	fn from(t: TokenType) -> Self {
		match t {
			TokenType::Access => TokenTypeDef::Access,
			TokenType::Refresh => TokenTypeDef::Refresh,
			TokenType::Session => TokenTypeDef::Session,
		}
	}
}

impl From<TokenTypeDef> for TokenType {
	fn from(t: TokenTypeDef) -> Self {
		match t {
			TokenTypeDef::Access => TokenType::Access,
			TokenTypeDef::Refresh => TokenType::Refresh,
			TokenTypeDef::Session => TokenType::Session,
		}
	}
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Account`
	#[sea_orm(
		belongs_to = "super::account::Entity",
		from = "Column::Uid",
		to = "super::account::Column::Uid"
	)]
	Account,
}

impl Related<super::account::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Account.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Token> for ActiveModel {
	fn from(t: Token) -> Self {
		Self {
			id: ActiveValue::NotSet,
			uid: ActiveValue::Set(t.uid),
			profile_id: ActiveValue::Set(t.profile_id),
			typ: ActiveValue::Set(TokenTypeDef::from(t.typ)),
			token: ActiveValue::Set(t.token),
			expire: ActiveValue::Set(t.expire),
		}
	}
}

impl From<Model> for Token {
	fn from(value: Model) -> Self {
		Self {
			uid: value.uid,
			profile_id: value.profile_id,
			typ: value.typ.into(),
			token: value.token,
			expire: value.expire,
		}
	}
}
