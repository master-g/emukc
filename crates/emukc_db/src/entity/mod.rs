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
