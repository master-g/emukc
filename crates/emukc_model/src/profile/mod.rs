//! An `EmuKC` account can has multiple game profiles

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
/// In game user items, including `UseItem` and `PayItem`
pub mod user_item;

pub trait BuildKcApiItem<T> {
	fn build_kc_api_item(&self) -> T;
}
