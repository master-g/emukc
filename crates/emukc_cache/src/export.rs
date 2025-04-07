use redb::ReadableTable;

use crate::{Kache, error::Error, kache::KACHE_TABLE};

impl Kache {
	/// Export all records from the database.
	///
	/// Return a list of `KcFileEntry`.
	pub async fn export(&self) -> Result<Vec<(String, Option<String>)>, Error> {
		let read_txn = self.db.begin_read()?;
		let table = read_txn.open_table(KACHE_TABLE)?;

		let records = table
			.iter()?
			.filter_map(|kv| {
				let (k, v) = kv.ok()?;
				let k = k.value();
				let v = v.value().map(std::string::ToString::to_string);
				Some((k.to_owned(), v))
			})
			.collect();

		Ok(records)
	}
}
