//! The single quest observation point.
//!
//! Domain modules never talk to quest progress directly. They describe what
//! they just wrote as [`GameplayOutcome`] values and hand them to [`observe`],
//! which is the only place that turns gameplay facts into [`QuestActionEvent`]s
//! and feeds them to [`update_quest_progress_for_action`]. So "what advances a
//! quest" is answerable from this one file.
//!
//! Adding a `GameplayOutcome` variant without mapping it here does not compile:
//! the `match` below is exhaustive.
//!
//! [`GameplayOutcome::SlotItemImproved`] has no producer yet. Four quests
//! (618, 619, 1166, 1167) require slot item improvement and the matcher already
//! handles them, but `api_req_kousyou/remodel_slot*` is unimplemented (P1 in
//! `docs/api_coverage.md`). The variant stays so that the day that handler
//! lands, the mapping is already here.

use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::{
    codex::Codex,
    kc2::KcSortieResultRank,
    thirdparty::{ExpeditionResult, FleetShipSnapshot, QuestActionEvent},
};

use crate::err::GameplayError;

use super::update::update_quest_progress_for_action;

/// Something a domain module just persisted, described in gameplay terms.
///
/// One variant per [`QuestActionEvent`], same field names and types, minus the
/// `api_` vocabulary: domain modules build these, [`observe`] translates them.
#[derive(Debug, Clone)]
pub(crate) enum GameplayOutcome {
    /// A ship rolled off the construction dock.
    ShipConstructed {
        /// The constructed ship's manifest ID.
        ship_mst_id: i64,
        /// Whether it was a large ship construction.
        large: bool,
    },
    /// A slot item was built in the factory.
    SlotItemConstructed {
        /// The built item's manifest ID.
        item_mst_id: i64,
    },
    /// A ship was scrapped.
    ShipScrapped {
        /// The scrapped ship's manifest ID.
        ship_mst_id: i64,
    },
    /// A slot item was scrapped.
    SlotItemScrapped {
        /// The scrapped item's manifest ID.
        item_mst_id: i64,
        /// The item's improvement stars at the time it was scrapped.
        stars: i64,
    },
    /// A ship finished repairing.
    ShipRepaired {
        /// The repaired ship's instance ID.
        ship_id: i64,
    },
    /// A ship was resupplied.
    ShipResupplied {
        /// The resupplied ship's instance ID.
        ship_id: i64,
    },
    /// An expedition returned.
    ExpeditionCompleted {
        /// The expedition's mission ID.
        mission_id: i64,
        /// How the expedition ended.
        result: ExpeditionResult,
        /// The fleet that ran it.
        fleet_id: i64,
    },
    /// A practice battle was settled.
    ExerciseBattleCompleted {
        /// The fleet that fought.
        fleet_id: i64,
        /// The battle result rank.
        win_rank: KcSortieResultRank,
        /// The fleet composition at the time of the battle.
        fleet_ships: Vec<FleetShipSnapshot>,
    },
    /// A sortie battle was settled.
    SortieBattleCompleted {
        /// The map area ID.
        maparea_id: i64,
        /// The map number within the area.
        mapinfo_no: i64,
        /// Whether the battle happened on a boss cell.
        boss_cell: bool,
        /// The battle result rank.
        win_rank: KcSortieResultRank,
        /// The fleet that fought.
        fleet_id: i64,
    },
    /// A modernization was applied.
    ModernizationCompleted {
        /// The modernized ship's manifest ID.
        target_ship_mst_id: i64,
        /// The consumed ships' manifest IDs.
        material_ship_mst_ids: Vec<i64>,
    },
    /// An enemy ship was sunk.
    EnemyShipSunk {
        /// The sunk enemy's ship type.
        ship_stype: i64,
    },
    /// A slot item was improved. See the module docs: no producer yet.
    #[allow(dead_code, reason = "no producer until remodel_slot is implemented")]
    SlotItemImproved {
        /// The improved item's manifest ID.
        item_mst_id: i64,
        /// The item's improvement stars after the upgrade.
        stars: i64,
    },
}

impl GameplayOutcome {
    /// The quest event this outcome stands for.
    fn as_quest_event(&self) -> QuestActionEvent {
        match self {
            Self::ShipConstructed {
                ship_mst_id,
                large,
            } => QuestActionEvent::ShipConstructed {
                ship_mst_id: *ship_mst_id,
                large: *large,
            },
            Self::SlotItemConstructed {
                item_mst_id,
            } => QuestActionEvent::SlotItemConstructed {
                item_mst_id: *item_mst_id,
            },
            Self::ShipScrapped {
                ship_mst_id,
            } => QuestActionEvent::ShipScrapped {
                ship_mst_id: *ship_mst_id,
            },
            Self::SlotItemScrapped {
                item_mst_id,
                stars,
            } => QuestActionEvent::SlotItemScrapped {
                item_mst_id: *item_mst_id,
                stars: *stars,
            },
            Self::ShipRepaired {
                ship_id,
            } => QuestActionEvent::ShipRepaired {
                ship_id: *ship_id,
            },
            Self::ShipResupplied {
                ship_id,
            } => QuestActionEvent::ShipResupplied {
                ship_id: *ship_id,
            },
            Self::ExpeditionCompleted {
                mission_id,
                result,
                fleet_id,
            } => QuestActionEvent::ExpeditionCompleted {
                mission_id: *mission_id,
                result: *result,
                fleet_id: *fleet_id,
            },
            Self::ExerciseBattleCompleted {
                fleet_id,
                win_rank,
                fleet_ships,
            } => QuestActionEvent::ExerciseBattleCompleted {
                fleet_id: *fleet_id,
                win_rank: *win_rank,
                fleet_ships: fleet_ships.clone(),
            },
            Self::SortieBattleCompleted {
                maparea_id,
                mapinfo_no,
                boss_cell,
                win_rank,
                fleet_id,
            } => QuestActionEvent::SortieBattleCompleted {
                maparea_id: *maparea_id,
                mapinfo_no: *mapinfo_no,
                boss_cell: *boss_cell,
                win_rank: *win_rank,
                fleet_id: *fleet_id,
            },
            Self::ModernizationCompleted {
                target_ship_mst_id,
                material_ship_mst_ids,
            } => QuestActionEvent::ModernizationCompleted {
                target_ship_mst_id: *target_ship_mst_id,
                material_ship_mst_ids: material_ship_mst_ids.clone(),
            },
            Self::EnemyShipSunk {
                ship_stype,
            } => QuestActionEvent::EnemyShipSunk {
                ship_stype: *ship_stype,
            },
            Self::SlotItemImproved {
                item_mst_id,
                stars,
            } => QuestActionEvent::SlotItemImproved {
                item_mst_id: *item_mst_id,
                stars: *stars,
            },
        }
    }
}

/// Advance quest progress for everything a domain method just wrote.
///
/// Call this once per `Ctx` method, after the domain writes are done and
/// before the transaction commits, passing `c` as that same transaction.
/// Outcomes are observed in slice order.
///
/// # Parameters
///
/// - `c`: The database connection, normally the caller's open transaction.
/// - `codex`: The game manifest snapshot.
/// - `profile_id`: The profile ID.
/// - `outcomes`: What just happened, in the order it happened.
pub(crate) async fn observe<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    outcomes: &[GameplayOutcome],
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    for outcome in outcomes {
        update_quest_progress_for_action(c, codex, profile_id, &outcome.as_quest_event()).await?;
    }

    Ok(())
}
