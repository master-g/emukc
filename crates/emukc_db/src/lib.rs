//! Emukc Database Library

/// Entry point for the `emukc_db` crate
pub mod entity;

#[doc(hidden)]
mod mem;

/// re-export `sea_orm`
pub use sea_orm;

use sea_orm::{
    DatabaseConnection, DatabaseTransaction, DbErr, SqliteTransactionMode, TransactionOptions,
    TransactionTrait,
};

/// Start transactions that take SQLite's write lock up front.
pub trait BeginImmediate {
    /// `BEGIN IMMEDIATE`. While another process (a CLI writing the same file)
    /// holds the write lock, a deferred transaction that has already read fails
    /// at once with `database is locked`; SQLite runs the busy handler only for
    /// a transaction that holds no lock yet, so this one waits instead.
    fn begin_immediate(&self) -> impl Future<Output = Result<DatabaseTransaction, DbErr>> + Send;
}

impl BeginImmediate for DatabaseConnection {
    fn begin_immediate(&self) -> impl Future<Output = Result<DatabaseTransaction, DbErr>> + Send {
        self.begin_with_options(TransactionOptions {
            sqlite_transaction_mode: Some(SqliteTransactionMode::Immediate),
            ..Default::default()
        })
    }
}

/// prelude module for `emukc_db`
pub mod prelude {
    pub use crate::BeginImmediate;
    #[doc(hidden)]
    pub use crate::mem::new_mem_db;
}
