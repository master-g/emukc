//! Gameplay logic.

pub mod battle;

pub use compose::{ComposeOps, PowerupResp, SlotDepriveParams};
pub use expedition::{
    ExpeditionCompletion, ExpeditionItemReward, ExpeditionOps, ExpeditionStartInfo,
};
pub(crate) use init::{init_profile_game_data, wipe_profile_game_data};
pub(crate) use map::{clear_and_unlock_map_impl, unlock_map_impl};
pub use practice::PracticeOps;
pub use presets::PresetOps;
pub use quest::QuestOps;
pub use settings::SettingsOps;
pub use ship::ShipOps;
pub use sortie::{
    SortieAirSearch, SortieCellData, SortieEnemyDeckPreview, SortieHappening, SortieItemGet,
    SortieNextResponse, SortieOps, SortieStartResponse,
};
pub use sortie_store::PracticeStore;
pub use sortie_store::SortieStore;

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
    ComposeOps + ExpeditionOps + SettingsOps + PracticeOps + PresetOps + QuestOps + ShipOps + SortieOps
{
}

#[async_trait::async_trait]
impl<T: HasContext + ?Sized> GameOps for T {}

pub mod ops {
    //! The ops traits prelude.

    #[doc(hidden)]
    pub use crate::game::{
        ComposeOps, ExpeditionOps, GameOps, PracticeOps, PresetOps, QuestOps, SettingsOps, ShipOps,
        SortieOps,
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
