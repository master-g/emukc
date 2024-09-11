use crate::entity::bootstrap;

/// Create a new in-memory database
pub async fn new_mem_db() -> Result<sea_orm::DbConn, sea_orm::DbErr> {
	let db = sea_orm::Database::connect("sqlite::memory:").await?;
	bootstrap(&db).await?;

	Ok(db)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_new_mem_db() {
		let db = new_mem_db().await.unwrap();
		assert!(db.ping().await.is_ok());
	}
}
