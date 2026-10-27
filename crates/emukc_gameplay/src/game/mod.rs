//! Gameplay logic.

pub mod battle;

pub use compose::{PowerupResp, SlotDepriveParams};
pub use expedition::{ExpeditionCompletion, ExpeditionItemReward, ExpeditionStartInfo};
pub use import::{
    FieldMismatch, ImportReport, MissingMasters, ShipMismatch, import_account_impl, missing_masters,
};
pub(crate) use init::{init_profile_game_data, wipe_profile_game_data};
pub(crate) use map::{clear_and_unlock_map_impl, unlock_map_impl};
pub use ranking::{RankingEntry, RankingPage};
pub use remodel_slot::{RemodelSlotDetail, RemodelSlotListEntry, RemodelSlotResult};
pub use route_probe::{RouteProbe, RouteProbeFleet, RouteProbeShip, probe_route};
pub use sortie::{
    SortieAirSearch, SortieCellData, SortieEnemyDeckPreview, SortieEscortSuccess, SortieHappening,
    SortieItemGet, SortieNextResponse, SortieOrder, SortieStartResponse,
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
mod import;
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
mod proficiency;
mod quest;
mod ranking;
mod remodel_slot;
mod route_probe;
mod settings;
mod ship;
mod slot_item;
mod sortie;
mod sortie_result;
pub(crate) mod sortie_store;
mod transport;
mod use_item;
mod view;

pub mod types {
    //! The types prelude.

    #[doc(hidden)]
    pub use crate::game::{
        ExpeditionCompletion, ExpeditionItemReward, ExpeditionStartInfo, PortView, PowerupResp,
        QuestListItem, QuestListView, RankingEntry, RankingPage, RemodelSlotDetail,
        RemodelSlotListEntry, RemodelSlotResult, RequireInfoView, RouteProbe, RouteProbeFleet,
        RouteProbeShip, SlotDepriveParams, SortieAirSearch, SortieCellData, SortieEnemyDeckPreview,
        SortieEscortSuccess, SortieHappening, SortieItemGet, SortieNextResponse,
        SortieStartResponse,
    };
}
