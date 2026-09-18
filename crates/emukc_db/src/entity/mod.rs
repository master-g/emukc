/// Entities for `EmuKC` profile related stuff.
pub mod profile;
/// Entities for `EmuKC` user related stuff.
pub mod user;

/// Create the table backing `e` if it does not exist yet.
pub(crate) async fn create_table<E: sea_orm::EntityTrait>(
    db: &sea_orm::DbConn,
    e: E,
) -> Result<(), sea_orm::error::DbErr> {
    use sea_orm::ConnectionTrait;

    let schema = sea_orm::Schema::new(db.get_database_backend());
    let stmt = schema.create_table_from_entity(e).if_not_exists().to_owned();
    db.execute(db.get_database_backend().build(&stmt)).await?;

    Ok(())
}

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DbConn) -> Result<(), sea_orm::error::DbErr> {
    // user
    user::bootstrap(db).await?;
    // profile
    profile::bootstrap(db).await?;

    Ok(())
}
