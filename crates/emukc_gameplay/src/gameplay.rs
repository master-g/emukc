//! A wrapper around the game's data and logic.

use std::sync::Arc;

use emukc_db::{prelude::*, sea_orm::DbConn};
use emukc_model::codex::Codex;

/// The wrapper around the game's data and logic.
#[derive(Debug, Clone)]
pub struct Gameplay {
	/// The game's codex.
	pub(crate) codex: Codex,

	/// Database connection.
	pub(crate) db: Arc<DbConn>,
}

impl Gameplay {
	/// Create a new `Gameplay` instance.
	///
	/// # Arguments
	///
	/// * `codex` - The game's codex.
	/// * `db` - Database connection.
	pub fn new(codex: Codex, db: DbConn) -> Self {
		Self {
			codex,
			db: Arc::new(db),
		}
	}

	/// Create a new mock `Gameplay` instance.
	#[allow(dead_code)]
	pub(crate) async fn new_mock() -> Self {
		let db = Arc::new(new_mem_db().await.unwrap());
		Self {
			codex: Codex::default(),
			db,
		}
	}

	/// Get the game's codex.
	pub fn codex(&self) -> &Codex {
		&self.codex
	}

	/// Get the database connection.
	pub fn db(&self) -> &DbConn {
		&self.db
	}
}
