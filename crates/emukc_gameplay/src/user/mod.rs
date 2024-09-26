//! User related modules

#[doc(hidden)]
pub mod account;

#[doc(hidden)]
pub mod auth;

#[doc(hidden)]
pub mod err;

#[doc(hidden)]
pub mod profile;

pub use account::{AccountInfo, AccountOps, AuthInfo};
pub use err::UserError;
pub use profile::{ProfileOps, StartGameInfo};
