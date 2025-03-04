use emukc_model::cache::KcFileEntry;
use sea_orm::{ActiveValue, entity::prelude::*};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "cache")]
pub struct Model {
	/// Primary key
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	pub path: String,

	pub md5: String,

	pub version: Option<String>,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
	async fn before_save<C>(self, _db: &C, _insert: bool) -> Result<Self, DbErr>
	where
		C: ConnectionTrait,
	{
		let original = self.path.as_ref();
		let sanitized = original.replace('\\', "/");
		let sanitized = sanitized.trim_start_matches('/');
		if sanitized != original {
			Err(DbErr::Custom(format!(
				"[before_save] Path contains invalid characters: {}",
				original
			)))
		} else {
			Ok(self)
		}
	}
}

impl From<KcFileEntry> for ActiveModel {
	fn from(entry: KcFileEntry) -> Self {
		Self {
			id: ActiveValue::NotSet,
			path: ActiveValue::Set(entry.path),
			md5: ActiveValue::Set(entry.md5),
			version: ActiveValue::Set(entry.version),
		}
	}
}

impl From<Model> for KcFileEntry {
	fn from(model: Model) -> Self {
		Self::from_model(model.path, model.md5, model.version)
	}
}

/// Boostrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());

	{
		let stmt = schema.create_table_from_entity(Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
