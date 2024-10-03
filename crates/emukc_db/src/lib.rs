//! Emukc Database Library

/// Entry point for the `emukc_db` crate
pub mod entity;

#[doc(hidden)]
mod mem;

/// re-export `sea_orm`
pub use sea_orm;

pub mod prelude {
	//! The `emukc_db` crate prelude.
	#[doc(hidden)]
	pub use crate::entity::{bootstrap, bootstrap_cache};

	#[doc(hidden)]
	pub use crate::mem::new_mem_db;
}
