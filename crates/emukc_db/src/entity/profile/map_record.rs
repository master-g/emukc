//! Map record entity

use chrono::{DateTime, Utc};
use emukc_model::profile::map_record::MapSelectRank;
use sea_orm::{ConnectionTrait, Statement, entity::prelude::*};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum SelectedRank {
    /// Not set
    #[sea_orm(num_value = 0)]
    NotSet = 0,

    /// 丁
    #[sea_orm(num_value = 1)]
    Casual = 1,

    /// 丙
    #[sea_orm(num_value = 2)]
    Easy = 2,

    /// 乙
    #[sea_orm(num_value = 3)]
    Normal = 3,

    /// 甲
    #[sea_orm(num_value = 4)]
    Hard = 4,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "map_record")]
pub struct Model {
    /// Instance ID
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    /// Profile ID
    pub profile_id: i64,

    /// Map ID
    pub map_id: i64,

    /// Has cleared
    pub cleared: bool,

    /// Last cleared time
    pub last_cleared_at: Option<DateTime<Utc>>,

    /// Last reset time
    pub last_reset_at: Option<DateTime<Utc>>,

    /// Defeat count
    pub defeat_count: Option<i64>,

    /// Current map HP
    pub current_hp: Option<i64>,

    /// Current gauge index
    pub gauge_index: i64,

    /// Active map stage ID
    pub stage_id: Option<String>,

    /// Event selected rank
    pub selected_rank: SelectedRank,

    /// Event state
    pub event_state: Option<i64>,

    /// Whether this map is unlocked for the player
    pub unlocked: bool,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Relation to `Profile`
    #[sea_orm(
        belongs_to = "crate::entity::profile::Entity",
        from = "Column::ProfileId",
        to = "crate::entity::profile::Column::Id"
    )]
    Profile,
}

impl Related<crate::entity::profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Profile.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// Bootstrap the map record table.
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    let schema = sea_orm::Schema::new(db.get_database_backend());
    let stmt = schema.create_table_from_entity(Entity).if_not_exists().to_owned();
    db.execute(db.get_database_backend().build(&stmt)).await?;
    migrate_legacy_stage_id_schema(db).await?;
    migrate_unlocked_column(db).await?;
    Ok(())
}

async fn migrate_legacy_stage_id_schema<C>(c: &C) -> Result<(), sea_orm::error::DbErr>
where
    C: ConnectionTrait,
{
    let backend = c.get_database_backend();
    let columns = c
        .query_all(Statement::from_string(
            backend,
            r#"PRAGMA table_info("map_record")"#.to_string(),
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "name"))
        .collect::<Result<Vec<String>, _>>()?;

    let has_stage_id = columns.iter().any(|column| column == "stage_id");
    let has_variant_key = columns.iter().any(|column| column == "variant_key");

    if !has_stage_id {
        c.execute(Statement::from_string(
            backend,
            r#"ALTER TABLE "map_record" ADD COLUMN "stage_id" TEXT"#.to_string(),
        ))
        .await?;
    }

    if has_variant_key {
        c.execute(Statement::from_string(
			backend,
			"UPDATE \"map_record\"\nSET \"stage_id\" = COALESCE(\"stage_id\", \"variant_key\")\nWHERE \"variant_key\" IS NOT NULL"
				.to_string(),
		))
		.await?;
    }

    Ok(())
}

/// Add `unlocked` column to existing `map_record` tables.
/// Defaults to `true` for migration safety (existing accounts keep access).
async fn migrate_unlocked_column(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), sea_orm::error::DbErr> {
    let backend = db.get_database_backend();
    let columns = db
        .query_all(Statement::from_string(
            backend,
            r#"PRAGMA table_info("map_record")"#.to_string(),
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "name"))
        .collect::<Result<Vec<String>, _>>()?;

    if !columns.iter().any(|col| col == "unlocked") {
        db.execute(Statement::from_string(
            backend,
            r#"ALTER TABLE "map_record" ADD COLUMN "unlocked" INTEGER NOT NULL DEFAULT 1"#
                .to_string(),
        ))
        .await?;
    }

    Ok(())
}

impl From<SelectedRank> for MapSelectRank {
    fn from(value: SelectedRank) -> Self {
        match value {
            SelectedRank::NotSet => MapSelectRank::NotSet,
            SelectedRank::Casual => MapSelectRank::Casual,
            SelectedRank::Easy => MapSelectRank::Easy,
            SelectedRank::Normal => MapSelectRank::Normal,
            SelectedRank::Hard => MapSelectRank::Hard,
        }
    }
}

impl From<MapSelectRank> for SelectedRank {
    fn from(value: MapSelectRank) -> Self {
        match value {
            MapSelectRank::NotSet => SelectedRank::NotSet,
            MapSelectRank::Casual => SelectedRank::Casual,
            MapSelectRank::Easy => SelectedRank::Easy,
            MapSelectRank::Normal => SelectedRank::Normal,
            MapSelectRank::Hard => SelectedRank::Hard,
        }
    }
}
