//! Emukc Database Library

/// Entry point for the `emukc_db` crate
pub mod entity;

#[doc(hidden)]
mod mem;

/// re-export `sea_orm`
pub use sea_orm;

/// prelude module for `emukc_db`
pub mod prelude {
	#[doc(hidden)]
	pub use crate::mem::new_mem_db;
}
