//! Gameplay logic.

pub use basic::BasicOps;
pub use fleet::FleetOps;
pub use furniture::FurnitureOps;
pub use incentive::IncentiveOps;
pub(crate) use init::{init_profile_game_data, wipe_profile_game_data};
pub use kdock::KDockOps;
pub use material::MaterialOps;
pub use ndock::NDockOps;
pub use picturebook::PictureBookOps;
pub use settings::GameSettingsOps;
pub use ship::ShipOps;
pub use slot_item::SlotItemOps;
pub use use_item::UseItemOps;

use crate::gameplay::HasContext;

// modules

mod basic;
mod fleet;
mod furniture;
mod incentive;
mod init;
mod kdock;
mod material;
mod ndock;
mod picturebook;
mod settings;
mod ship;
mod slot_item;
mod use_item;

/// A trait for gameplay logic.
#[async_trait::async_trait]
pub trait GameOps:
	BasicOps
	+ FleetOps
	+ FurnitureOps
	+ GameSettingsOps
	+ IncentiveOps
	+ KDockOps
	+ MaterialOps
	+ NDockOps
	+ PictureBookOps
	+ ShipOps
	+ SlotItemOps
	+ UseItemOps
{
}

#[async_trait::async_trait]
impl<T: HasContext + ?Sized> GameOps for T {}

pub mod ops {
	//! The ops traits prelude.

	#[doc(hidden)]
	pub use crate::game::{
		BasicOps, FleetOps, FurnitureOps, GameOps, GameSettingsOps, IncentiveOps, KDockOps,
		MaterialOps, NDockOps, PictureBookOps, ShipOps, SlotItemOps, UseItemOps,
	};
}
