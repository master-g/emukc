use emukc_db::{
	entity::profile::furniture,
	sea_orm::{entity::prelude::*, ActiveValue},
};

use crate::err::GameplayError;

/// Add furniture to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The furniture master ID.
#[allow(unused)]
pub async fn add_furniture<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
) -> Result<furniture::record::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let record = furniture::record::Entity::find()
		.filter(furniture::record::Column::ProfileId.eq(profile_id))
		.filter(furniture::record::Column::FurnitureId.eq(mst_id))
		.one(c)
		.await?;

	if let Some(record) = record {
		return Ok(record.into());
	}

	let am = furniture::record::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		furniture_id: ActiveValue::Set(mst_id),
	};

	let model = am.save(c).await?;

	Ok(model)
}

#[cfg(test)]
mod tests {
	use emukc_db::sea_orm::TransactionTrait;

	use crate::user::{AccountOps, ProfileOps};

	#[tokio::test]
	async fn test_furniture_record() {
		let db = emukc_db::prelude::new_mem_db().await.unwrap();
		let codex = emukc_model::codex::Codex::default();

		let context = (db, codex);

		let account = context.sign_up("test", "1234567").await.unwrap();
		let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();

		let profile_id = profile.profile.id;

		let tx = context.0.begin().await.unwrap();

		super::add_furniture(&tx, profile_id, 1).await.unwrap();

		tx.commit().await.unwrap();
		println!("add furniture 1");

		let tx = context.0.begin().await.unwrap();

		let item = super::add_furniture(&tx, profile_id, 1).await.unwrap();

		tx.commit().await.unwrap();
		println!("add furniture 1 again");

		assert_eq!(item.id.unwrap(), 1);
	}
}
