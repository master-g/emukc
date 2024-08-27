//! This module is separated into its own crate to enable simple dynamic linking for `EmuKC`, and should not be used directly.

/// `use emukc::prelude::*;` to import commonly used items.
pub mod prelude;

pub use emukc_app as app;
pub use emukc_bootstrap as bootstrap;
pub use emukc_crypto as crypto;
pub use emukc_db as db;
pub use emukc_log as log;
pub use emukc_macros as macros;
pub use emukc_model as model;
pub use emukc_time as time;
