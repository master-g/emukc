use emukc_db::{
	entity::user::{account, token},
	sea_orm::{entity::*, query::*},
};
use emukc_model::user::token::{Token, TokenType};

use super::UserError;

pub(super) async fn verify_access_token<C>(c: &C, token: &str) -> Result<account::Model, UserError>
where
	C: ConnectionTrait,
{
	let record = token::Entity::find()
		.filter(token::Column::Token.eq(token))
		.filter(token::Column::Typ.eq(token::TokenTypeDef::Access))
		.one(c)
		.await?
		.ok_or_else(|| UserError::TokenInvalid)?;

	let account = account::Entity::find()
		.filter(account::Column::Uid.eq(record.uid))
		.one(c)
		.await?
		.ok_or(UserError::UserNotFound)?;

	Ok(account)
}

pub(super) async fn issue_token<C>(
	c: &C,
	uid: i64,
	profile_id: i64,
	typ: TokenType,
) -> Result<Token, UserError>
where
	C: ConnectionTrait,
{
	let token = match typ {
		TokenType::Access => Token::issue_access(uid),
		TokenType::Refresh => Token::issue_refresh(uid),
		TokenType::Session => Token::issue_session(uid, profile_id),
	};

	let db_token_type = token::TokenTypeDef::from(typ);

	// find and delete the old token if it exists
	if let Some(record) = token::Entity::find()
		.filter(token::Column::Uid.eq(uid))
		.filter(token::Column::ProfileId.eq(profile_id))
		.filter(token::Column::Typ.eq(db_token_type.clone()))
		.one(c)
		.await?
	{
		record.delete(c).await?;
	}

	// insert the new token
	token::ActiveModel {
		id: ActiveValue::NotSet,
		uid: ActiveValue::Set(uid),
		profile_id: ActiveValue::Set(profile_id),
		typ: ActiveValue::Set(db_token_type),
		token: ActiveValue::Set(token.token.clone()),
		expire: ActiveValue::Set(token.expire),
	}
	.save(c)
	.await?;

	Ok(token)
}
