/// Entities for `KanColle` file cache.
pub mod cache;
/// Entities for `EmuKC` profile related stuff.
pub mod profile;
/// Entities for `EmuKC` user related stuff.
pub mod user;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DbConn) -> Result<(), sea_orm::error::DbErr> {
	// cache
	cache::bootstrap(db).await?;
	// user
	user::bootstrap(db).await?;
	// profile
	profile::bootstrap(db).await?;

	Ok(())
}
