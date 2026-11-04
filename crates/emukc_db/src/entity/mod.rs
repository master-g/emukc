/// Entities for `EmuKC` profile related stuff.
pub mod profile;
/// Entities for `EmuKC` user related stuff.
pub mod user;

/// Declare the single `belongs_to` relation a per-profile entity has.
///
/// `find_related` is never called, but `Schema::create_table_from_entity` reads
/// `Relation` to emit the `FOREIGN KEY` clause, so the declaration has to stay.
/// `$from` names the local column holding the profile id.
///
/// `rustfmt::skip`: rustfmt re-indents the `sea_orm` attribute inside a macro body
/// on every run, so `cargo fmt --all --check` never converges without it.
#[rustfmt::skip]
macro_rules! profile_relation {
    ($from:literal) => {
        /// Relation
        #[derive(Copy, Clone, Debug, sea_orm::EnumIter, sea_orm::DeriveRelation)]
        pub enum Relation {
            /// Relation to `Profile`
            #[sea_orm(
                belongs_to = "crate::entity::profile::Entity",
                from = $from,
                to = "crate::entity::profile::Column::Id"
            )]
            Profile,
        }

        impl sea_orm::Related<crate::entity::profile::Entity> for Entity {
            fn to() -> sea_orm::RelationDef {
                sea_orm::RelationTrait::def(&Relation::Profile)
            }
        }

        impl sea_orm::ActiveModelBehavior for ActiveModel {}
    };
}

pub(crate) use profile_relation;

/// Create the table backing `e` if it does not exist yet.
pub(crate) async fn create_table<E: sea_orm::EntityTrait>(
    db: &sea_orm::DbConn,
    e: E,
) -> Result<(), sea_orm::error::DbErr> {
    use sea_orm::ConnectionTrait;

    let schema = sea_orm::Schema::new(db.get_database_backend());
    let stmt = schema.create_table_from_entity(e).if_not_exists().to_owned();
    db.execute(&stmt).await?;

    Ok(())
}

/// Run an `ALTER TABLE ... ADD COLUMN` statement. A column that already exists
/// is not an error; SQLite reports it only through the message.
pub(crate) async fn add_column(db: &sea_orm::DbConn, sql: &str) -> Result<(), sea_orm::DbErr> {
    use sea_orm::{ConnectionTrait, DbErr, RuntimeErr};

    match db.execute_unprepared(sql).await {
        Ok(_) => Ok(()),
        Err(DbErr::Exec(RuntimeErr::SqlxError(e)))
            if e.as_database_error()
                .is_some_and(|d| d.message().starts_with("duplicate column name")) =>
        {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DbConn) -> Result<(), sea_orm::error::DbErr> {
    // user
    user::bootstrap(db).await?;
    // profile
    profile::bootstrap(db).await?;

    Ok(())
}
