//! Gameplay logic.

pub mod battle;

pub use compose::{PowerupResp, SlotDepriveParams};
pub use expedition::{ExpeditionCompletion, ExpeditionItemReward, ExpeditionStartInfo};
pub(crate) use init::{init_profile_game_data, wipe_profile_game_data};
pub(crate) use map::{clear_and_unlock_map_impl, unlock_map_impl};
pub use sortie::{
    SortieAirSearch, SortieCellData, SortieEnemyDeckPreview, SortieHappening, SortieItemGet,
    SortieNextResponse, SortieStartResponse,
};
pub use sortie_store::PracticeStore;
pub use sortie_store::SortieStore;
pub use view::{PortView, QuestListItem, QuestListView, RequireInfoView};

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
mod view;

pub mod types {
    //! The types prelude.

    #[doc(hidden)]
    pub use crate::game::{
        ExpeditionCompletion, ExpeditionItemReward, ExpeditionStartInfo, PortView, PowerupResp,
        QuestListItem, QuestListView, RequireInfoView, SlotDepriveParams, SortieAirSearch,
        SortieCellData, SortieEnemyDeckPreview, SortieHappening, SortieItemGet, SortieNextResponse,
        SortieStartResponse,
    };
}
