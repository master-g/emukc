use emukc_db::{
	entity::profile::item::use_item::{self, ActiveModel},
	sea_orm::{entity::prelude::*, ActiveValue},
};

use crate::err::GameplayError;

/// Add use item to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The item master ID.
/// - `count`: The count of the item.
#[allow(unused)]
pub async fn add_use_item<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	count: i64,
) -> Result<use_item::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let record = use_item::Entity::find()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.filter(use_item::Column::MstId.eq(mst_id))
		.one(c)
		.await?;

	let am = match record {
		Some(rec) => ActiveModel {
			id: ActiveValue::Unchanged(rec.id),
			count: ActiveValue::Set(rec.count + count),
			..rec.into()
		},
		None => use_item::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			mst_id: ActiveValue::Set(mst_id),
			count: ActiveValue::Set(count),
		},
	};

	let model = am.save(c).await?;

	Ok(model)
}

#[cfg(test)]
mod tests {
	use emukc_db::sea_orm::TransactionTrait;

	use crate::user::{AccountOps, ProfileOps};

	#[tokio::test]
	async fn test_use_item() {
		let db = emukc_db::prelude::new_mem_db().await.unwrap();
		let codex = emukc_model::codex::Codex::default();

		let context = (db, codex);

		let account = context.sign_up("test", "1234567").await.unwrap();
		let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();

		let profile_id = profile.profile.id;

		let tx = context.0.begin().await.unwrap();

		super::add_use_item(&tx, profile_id, 1, 1).await.unwrap();

		tx.commit().await.unwrap();
		println!("add use item 1");

		let tx = context.0.begin().await.unwrap();

		let item = super::add_use_item(&tx, profile_id, 1, 2).await.unwrap();

		tx.commit().await.unwrap();
		println!("add use item 2");

		assert_eq!(item.count.unwrap(), 3);
	}
}
