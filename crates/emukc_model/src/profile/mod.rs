//! An `EmuKC` account can has multiple game profiles

use serde::{Deserialize, Serialize};

/// User airbases
pub mod airbase;
/// User expeditions
pub mod expedition;
/// In game deck ports
pub mod fleet;
/// In game furnitures
pub mod furniture;
/// In game construction dock
pub mod kdock;
/// User map progress record
pub mod map_record;
/// In game materials
pub mod material;
/// In game repair dock
pub mod ndock;
/// Picture book
pub mod picture_book;
/// Practice
pub mod practice;
/// Quest progress
pub mod quest;
/// In game slot items
pub mod slot_item;
/// In game user items, including `UseItem` and `PayItem`
pub mod user_item;

/// User profile
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Profile {
	/// Account id
	pub account_id: i64,

	/// Profile id
	pub id: i64,

	/// World id
	pub world_id: i64,

	/// Profile name
	pub name: String,
}
