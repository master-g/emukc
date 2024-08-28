//! Parsers for various data sources.

pub mod error;
pub mod kaisou;
pub mod kccp;
pub mod kcwikizh_kcdata;
pub mod kcwikizh_ships;
pub mod tsunkit_quest;

pub use kaisou::parse as parse_kaisou;
pub use kccp::quest::parse as parse_kccp_quests;
pub use kcwikizh_kcdata::parse as parse_kcdata;
pub use kcwikizh_ships::parse as parse_ships_nedb;
pub use tsunkit_quest::parse as parse_tsunkit_quests;
