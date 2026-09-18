//! Gameplay logic.

pub mod battle;

pub use basic::BasicOps;
pub use compose::{ComposeOps, PowerupResp, SlotDepriveParams};
pub use expedition::{
    ExpeditionCompletion, ExpeditionItemReward, ExpeditionOps, ExpeditionStartInfo,
};
pub use factory::FactoryOps;
pub(crate) use init::{init_profile_game_data, wipe_profile_game_data};
pub use map::MapOps;
pub(crate) use map::{clear_and_unlock_map_impl, unlock_map_impl};
pub use ndock::NDockOps;
pub use practice::PracticeOps;
pub use presets::PresetOps;
pub use quest::QuestOps;
pub use settings::SettingsOps;
pub use ship::ShipOps;
pub use slot_item::SlotItemOps;
pub use sortie::{
    SortieAirSearch, SortieCellData, SortieEnemyDeckPreview, SortieHappening, SortieItemGet,
    SortieNextResponse, SortieOps, SortieStartResponse,
};
pub use sortie_store::PracticeStore;
pub use sortie_store::SortieStore;
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
pub(crate) mod sortie_store;
mod use_item;

/// A trait for gameplay logic.
#[async_trait::async_trait]
pub trait GameOps:
    BasicOps
    + ComposeOps
    + ExpeditionOps
    + FactoryOps
    + SettingsOps
    + MapOps
    + NDockOps
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
        BasicOps, ComposeOps, ExpeditionOps, FactoryOps, GameOps, MapOps, NDockOps, PracticeOps,
        PresetOps, QuestOps, SettingsOps, ShipOps, SlotItemOps, SortieOps, UseItemOps,
    };
}

pub mod types {
    //! The types prelude.

    #[doc(hidden)]
    pub use crate::game::{
        ExpeditionCompletion, ExpeditionItemReward, ExpeditionStartInfo, PowerupResp,
        SlotDepriveParams, SortieAirSearch, SortieCellData, SortieEnemyDeckPreview,
        SortieHappening, SortieItemGet, SortieNextResponse, SortieStartResponse,
    };
}
