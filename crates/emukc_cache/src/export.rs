use emukc_model::cache::KcFileEntry;

use crate::{Kache, error::Error};
use emukc_db::{entity::cache, sea_orm::*};

const CHUNK_SIZE: usize = 256;

impl Kache {
	/// Export all records from the database.
	///
	/// Return a list of `KcFileEntry`.
	pub async fn export(&self) -> Result<Vec<KcFileEntry>, Error> {
		let db = &*self.db;
		let models = cache::Entity::find().all(db).await?;
		Ok(models.into_iter().map(Into::into).collect())
	}

	/// Import records to the database.
	///
	/// # Arguments
	///
	/// * `entries` - The list of `KcFileEntry`.
	///
	/// Return a tuple of `(new_entries, updated_entries)`.
	#[instrument(skip_all)]
	pub async fn import(&self, entries: &[KcFileEntry]) -> Result<(usize, usize), Error> {
		let db = &*self.db;

		trace!("importing {} entries", entries.len());
		let mut not_exists = vec![];
		let mut updates = vec![];

		let mut now = std::time::SystemTime::now();

		for entry in entries {
			let model =
				cache::Entity::find().filter(cache::Column::Path.eq(&entry.path)).one(db).await?;

			let Some(model) = model else {
				not_exists.push(entry);
				continue;
			};

			let old_entry: KcFileEntry = model.clone().into();
			if entry.version_cmp(&old_entry) == std::cmp::Ordering::Greater {
				let mut am: cache::ActiveModel = model.into();
				am.version = ActiveValue::Set(entry.version.clone());
				updates.push(am);
			}
		}

		trace!("filtering done, elapsed: {:?}", now.elapsed().unwrap());

		let updated = updates.len();
		debug!("{} entries need to be updated", updated);
		now = std::time::SystemTime::now();
		{
			let chunks = updates.chunks(CHUNK_SIZE);
			let mut processed = 0;
			for chunk in chunks {
				let txn = db.begin().await?;
				for am in chunk.iter().cloned() {
					am.update(&txn).await?;
				}
				txn.commit().await?;
				processed += chunk.len();
				debug!("processed {}/{}", processed, updated);
			}
		}
		trace!("updating done, elapsed: {:?}", now.elapsed().unwrap());

		let new_entries = not_exists.len();
		debug!("{} new entries", new_entries);
		now = std::time::SystemTime::now();
		{
			let chunks = not_exists.chunks(CHUNK_SIZE);
			let mut processed = 0;
			for chunk in chunks {
				let models = chunk
					.iter()
					.map(|entry| cache::ActiveModel {
						id: ActiveValue::NotSet,
						path: ActiveValue::Set(entry.path.clone()),
						md5: ActiveValue::Set(entry.md5.clone()),
						version: ActiveValue::Set(entry.version.clone()),
					})
					.collect::<Vec<_>>();
				cache::Entity::insert_many(models).exec(db).await?;

				processed += chunk.len();
				debug!("processed {}/{}", processed, new_entries);
			}
		}
		trace!("inserting done, elapsed: {:?}", now.elapsed().unwrap());

		Ok((new_entries, updated))
	}
}
