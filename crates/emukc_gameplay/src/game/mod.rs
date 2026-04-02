//! Gameplay logic.

pub mod battle;

pub use airbase::AirbaseOps;
pub use basic::BasicOps;
pub use compose::{ComposeOps, PowerupResp, SlotDepriveParams};
pub use expedition::{
	ExpeditionCompletion, ExpeditionItemReward, ExpeditionOps, ExpeditionStartInfo,
};
pub use factory::FactoryOps;
pub use fleet::FleetOps;
pub use furniture::FurnitureOps;
pub use incentive::IncentiveOps;
pub(crate) use init::{init_profile_game_data, wipe_profile_game_data};
pub use kdock::KDockOps;
pub use map::MapOps;
pub use material::MaterialOps;
pub use ndock::NDockOps;
pub use pay_item::PayItemOps;
pub use picturebook::PictureBookOps;
pub use practice::PracticeOps;
pub use presets::PresetOps;
pub use quest::QuestOps;
pub use settings::SettingsOps;
pub use ship::ShipOps;
pub use slot_item::SlotItemOps;
pub use sortie::{
	SortieAirSearch, SortieCellData, SortieEnemyDeckPreview, SortieNextResponse, SortieOps,
	SortieStartResponse,
};
pub use use_item::UseItemOps;

use crate::gameplay::HasContext;

// modules

mod airbase;
mod basic;
mod compose;
mod expedition;
mod factory;
mod fleet;
mod furniture;
mod incentive;
mod init;
mod kdock;
mod map;
mod map_progress;
mod map_route;
mod material;
mod ndock;
mod pay_item;
mod picturebook;
mod practice;
mod presets;
mod quest;
mod settings;
mod ship;
mod slot_item;
mod sortie;
mod sortie_result;
mod use_item;

/// A trait for gameplay logic.
#[async_trait::async_trait]
pub trait GameOps:
	BasicOps
	+ AirbaseOps
	+ ComposeOps
	+ ExpeditionOps
	+ FactoryOps
	+ FleetOps
	+ FurnitureOps
	+ SettingsOps
	+ IncentiveOps
	+ KDockOps
	+ MapOps
	+ MaterialOps
	+ NDockOps
	+ PayItemOps
	+ PictureBookOps
	+ PracticeOps
	+ PresetOps
	+ QuestOps
	+ ShipOps
	+ SlotItemOps
	+ SortieOps
	+ UseItemOps
{
}

#[async_trait::async_trait]
impl<T: HasContext + ?Sized> GameOps for T {}

pub mod ops {
	//! The ops traits prelude.

	#[doc(hidden)]
	pub use crate::game::{
		AirbaseOps, BasicOps, ComposeOps, ExpeditionOps, FactoryOps, FleetOps, FurnitureOps,
		GameOps, IncentiveOps, KDockOps, MapOps, MaterialOps, NDockOps, PayItemOps, PictureBookOps,
		PracticeOps, PresetOps, QuestOps, SettingsOps, ShipOps, SlotItemOps, SortieOps, UseItemOps,
	};
}

pub mod types {
	//! The types prelude.

	#[doc(hidden)]
	pub use crate::game::{
		ExpeditionCompletion, ExpeditionItemReward, ExpeditionStartInfo, PowerupResp,
		SlotDepriveParams, SortieAirSearch, SortieCellData, SortieEnemyDeckPreview,
		SortieNextResponse, SortieStartResponse,
	};
}
