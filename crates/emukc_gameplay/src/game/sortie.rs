use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use emukc_crypto::rng;
use emukc_db::entity::profile::{item::slot_item, ship};
use emukc_db::sea_orm::{TransactionTrait, entity::prelude::*};
#[cfg(test)]
use emukc_model::codex::map::{RouteOperator, RoutePredicate, RouteRule, SpeedClass};
use emukc_model::{
    codex::{
        Codex,
        map::{
            EnemyComposition, EnemyFleetDefinition, MapCellDefinition, MapDefinition,
            MapStageDefinition, MapVariantDefinition,
        },
    },
    kc2::{KcApiShip, KcApiSlotItem, MaterialCategory, UserHQRank, level, start2::ApiMstShip},
    thirdparty::QuestActionEvent,
};
#[cfg(test)]
use emukc_time::chrono::Utc;
use serde::Serialize;

use crate::{err::GameplayError, gameplay::HasContext};

use super::sortie_store::SortieStore;

#[cfg(test)]
use super::map_progress::assign_stage_id;
#[cfg(test)]
use super::map_route::{route_predicate_matches, select_route_target_for_roll};
#[cfg(test)]
use super::sortie_result::eligible_sortie_ship_drops;
use super::{
    basic::find_profile,
    battle::{
        core::{
            BattleContext, BattleMode, BattleNightHougeki, BattlePacket, BattleShipInput,
            BattleType, EngagementType,
        },
        practice::PracticeBattleResponse,
        sortie::{
            SortieBattleInput, pending_sortie_battle, simulate_and_store_sortie_day_battle,
            simulate_and_store_sortie_night_battle, simulate_and_store_sortie_sp_midnight_battle,
            take_sortie_day_battle_result,
        },
    },
    fleet::get_fleet_ships_impl,
    map::{
        active_map_catalog, check_and_unlock_dependencies_impl, ensure_map_records_impl,
        find_map_definition, find_map_record_impl, refresh_all_map_records_impl,
    },
    map_progress::resolve_record_stage_id,
    map_route::{FleetRouteContext, FleetRouteShipEntry, evaluate_route_destination},
    material::{add_material_impl, deduct_material_impl, get_mat_impl},
    quest::update::update_quest_progress_for_action,
    slot_item::find_slot_items_by_id_impl,
    sortie_result::{
        SortieBattleResultSnapshot, apply_sortie_map_result, build_sortie_quest_event,
        calculate_battle_admiral_exp, calculate_sortie_base_exp, calculate_sortie_ship_exp,
        try_grant_sortie_ship_drop, update_sortie_result_stats,
    },
};

pub use super::sortie_result::{SortieBattleResultEnemyInfo, SortieBattleResultResponse};
const DRUM_CANISTER_MST_ID: i64 = 75;

pub type SortieBattleResponse = PracticeBattleResponse;

#[derive(Debug, Clone)]
pub(super) struct ActiveSortieState {
    pub(super) deck_id: i64,
    pub(super) map_id: i64,
    pub(super) map_name: String,
    pub(super) map_level: i64,
    pub(super) stage_id: String,
    pub(super) current_cell_id: i64,
    pub(super) boss_cell_id: i64,
    pub(super) pending_battle_cell_id: Option<i64>,
    pub(super) visited_cell_ids: BTreeSet<i64>,
    pub(super) locked_enemy_composition: Option<EnemyComposition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieCellData {
    pub master_cell_id: i64,
    pub cell_no: i64,
    pub color_no: i64,
    pub passed: bool,
    pub distance: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieAirSearch {
    pub plane_type: i64,
    pub result: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieEnemyDeckPreview {
    pub kind: i64,
    pub ship_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieStartResponse {
    pub cell_data: Vec<SortieCellData>,
    pub rashin_flg: bool,
    pub rashin_id: i64,
    pub maparea_id: i64,
    pub mapinfo_no: i64,
    pub cell_no: i64,
    pub color_no: i64,
    pub event_id: i64,
    pub event_kind: i64,
    pub has_next: bool,
    pub boss_cell_no: i64,
    pub bosscomp: bool,
    pub from_cell_no: i64,
    pub limit_state: i64,
    pub airsearch: Option<SortieAirSearch>,
    pub enemy_deck_preview: Option<Vec<SortieEnemyDeckPreview>>,
}

/// Resource acquisition at a non-battle node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieItemGet {
    /// Resource type: 1=fuel, 2=ammo, 3=steel, 4=bauxite
    pub resource_type: i64,
    pub amount: i64,
}

/// Maelstrom (渦潮) resource loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieHappening {
    /// Resource type: 1=fuel, 2=ammo
    pub resource_type: i64,
    pub amount: i64,
    pub radar_reduced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieNextResponse {
    pub rashin_flg: bool,
    pub rashin_id: i64,
    pub maparea_id: i64,
    pub mapinfo_no: i64,
    pub cell_no: i64,
    pub color_no: i64,
    pub event_id: i64,
    pub event_kind: i64,
    pub has_next: bool,
    pub boss_cell_no: i64,
    pub bosscomp: bool,
    pub from_cell_no: i64,
    pub comment_kind: Option<i64>,
    pub production_kind: Option<i64>,
    pub airsearch: Option<SortieAirSearch>,
    pub enemy_deck_preview: Option<Vec<SortieEnemyDeckPreview>>,
    pub limit_state: Option<i64>,
    pub itemget: Option<Vec<SortieItemGet>>,
    pub happening: Option<SortieHappening>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize)]
pub struct SortieNightBattleResponse {
    pub api_deck_id: i64,
    pub api_formation: [i64; 3],
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_fParam: Vec<[i64; 4]>,
    pub api_ship_ke: Vec<i64>,
    pub api_ship_lv: Vec<i64>,
    pub api_e_nowhps: Vec<i64>,
    pub api_e_maxhps: Vec<i64>,
    pub api_eSlot: Vec<[i64; 5]>,
    pub api_eParam: Vec<[i64; 4]>,
    pub api_smoke_type: i64,
    pub api_balloon_cell: i64,
    pub api_atoll_cell: i64,
    pub api_touch_plane: [i64; 2],
    pub api_flare_pos: [i64; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki: Option<BattleNightHougeki>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SortieGobackPortResponse {}

#[async_trait]
pub trait SortieOps {
    async fn start_sortie(
        &self,
        profile_id: i64,
        deck_id: i64,
        maparea_id: i64,
        mapinfo_no: i64,
        formation_id: i64,
    ) -> Result<SortieStartResponse, GameplayError>;

    async fn next_sortie(
        &self,
        profile_id: i64,
        selected_cell_id: Option<i64>,
    ) -> Result<SortieNextResponse, GameplayError>;

    async fn sortie_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError>;

    async fn sortie_airbattle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError>;

    async fn sortie_ld_airbattle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError>;

    async fn sortie_ld_shooting(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError>;

    async fn sortie_battle_result(
        &self,
        profile_id: i64,
    ) -> Result<SortieBattleResultResponse, GameplayError>;

    async fn sortie_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<SortieNightBattleResponse, GameplayError>;

    async fn sortie_sp_midnight_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieNightBattleResponse, GameplayError>;

    async fn sortie_goback_port(
        &self,
        profile_id: i64,
    ) -> Result<SortieGobackPortResponse, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> SortieOps for T {
    async fn start_sortie(
        &self,
        profile_id: i64,
        deck_id: i64,
        maparea_id: i64,
        mapinfo_no: i64,
        formation_id: i64,
    ) -> Result<SortieStartResponse, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let tx = db.begin().await?;

        find_profile(&tx, profile_id).await?;
        let fleet_ships = get_fleet_ships_impl(&tx, profile_id, deck_id).await?;
        if fleet_ships.is_empty() {
            return Err(GameplayError::WrongType(format!(
                "fleet {deck_id} has no ships for sortie",
            )));
        }

        ensure_map_records_impl(&tx, codex, profile_id).await?;
        refresh_all_map_records_impl(&tx, codex, profile_id).await?;
        let definition = find_map_definition(codex, maparea_id, mapinfo_no)?;
        let record = find_map_record_impl(&tx, profile_id, definition.map_id).await?;
        if !record.unlocked {
            return Err(GameplayError::Locked(format!(
                "map {}-{} is locked",
                maparea_id, mapinfo_no,
            )));
        }
        let stage_id = resolve_record_stage_id(&definition, &record).unwrap_or_default();
        let stage = definition.stage(&stage_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "stage `{stage_id}` not found for map {}",
                definition.map_id,
            ))
        })?;
        let cell_0 = stage.cell(0).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("cell_0 not found for map {}", definition.map_id,))
        })?;
        let mut route_context = build_fleet_route_context(&tx, codex, &fleet_ships).await?;
        route_context.visited_cell_ids.insert(0);
        let first_cell = evaluate_route_destination(cell_0, stage, &route_context, None)?;
        let current_cell = stage
            .cell(first_cell)
            .ok_or_else(|| GameplayError::EntryNotFound(format!("cell {first_cell} not found")))?;
        let locked_enemy_composition =
            select_locked_enemy_composition(definition.map_id, stage, current_cell.cell_no);

        let active = ActiveSortieState {
            deck_id,
            map_id: definition.map_id,
            map_name: definition.name.clone(),
            map_level: definition.level,
            stage_id,
            current_cell_id: first_cell,
            boss_cell_id: stage.boss_cell_no,
            pending_battle_cell_id: None,
            visited_cell_ids: BTreeSet::from([first_cell]),
            locked_enemy_composition: locked_enemy_composition.clone(),
        };
        self.sortie_store().insert_active_sortie(profile_id, active);
        tx.commit().await?;

        let _ = formation_id;
        Ok(SortieStartResponse {
            cell_data: build_sortie_cell_data(definition.map_id, stage),
            rashin_flg: cell_0.next_cells.len() > 1,
            rashin_id: if cell_0.next_cells.len() > 1 {
                1
            } else {
                0
            },
            maparea_id,
            mapinfo_no,
            cell_no: current_cell.cell_no,
            color_no: current_cell.color_no,
            event_id: current_cell.event_id,
            event_kind: current_cell.event_kind,
            has_next: !current_cell.next_cells.is_empty(),
            boss_cell_no: stage.boss_cell_no,
            bosscomp: sortie_bosscomp(stage),
            from_cell_no: 0,
            limit_state: 0,
            airsearch: Some(default_sortie_airsearch()),
            enemy_deck_preview: locked_enemy_composition
                .as_ref()
                .map(build_enemy_deck_preview)
                .filter(|preview| !preview.is_empty()),
        })
    }

    async fn next_sortie(
        &self,
        profile_id: i64,
        selected_cell_id: Option<i64>,
    ) -> Result<SortieNextResponse, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let store = self.sortie_store();
        let active = store.get_active_sortie(profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "active sortie not found for profile {profile_id}",
            ))
        })?;
        if active.pending_battle_cell_id.is_some() {
            return Err(GameplayError::WrongType(
                "cannot advance sortie while a battle result is pending".to_string(),
            ));
        }

        let catalog = active_map_catalog(codex);
        let definition = catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
        })?;
        let stage = definition.stage(&active.stage_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "stage `{}` not found for map {}",
                active.stage_id, active.map_id,
            ))
        })?;
        let current = stage.cell(active.current_cell_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "cell {} not found in map {}",
                active.current_cell_id, active.map_id,
            ))
        })?;
        if current.next_cells.is_empty() {
            return Err(GameplayError::WrongType(format!(
                "cell {} has no next route",
                current.cell_no,
            )));
        }

        let tx = db.begin().await?;
        let fleet_ships = get_fleet_ships_impl(&tx, profile_id, active.deck_id).await?;
        let mut route_context = build_fleet_route_context(&tx, codex, &fleet_ships).await?;
        tx.commit().await?;
        route_context.visited_cell_ids = active.visited_cell_ids.clone();

        let next_cell_id =
            evaluate_route_destination(current, stage, &route_context, selected_cell_id)?;
        let next = stage.cell(next_cell_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("cell {next_cell_id} not found"))
        })?;
        let locked_enemy_composition =
            select_locked_enemy_composition(active.map_id, stage, next.cell_no);

        store.modify_active_sortie(profile_id, |state| {
            state.current_cell_id = next_cell_id;
            state.visited_cell_ids.insert(next_cell_id);
            state.locked_enemy_composition = locked_enemy_composition.clone();
        });

        // Resolve non-battle node effects (resource gain / maelstrom loss).
        let tx = db.begin().await?;
        let (itemget, happening) =
            resolve_non_battle_node_effect(&tx, codex, profile_id, next, &fleet_ships).await?;
        tx.commit().await?;

        let (maparea_id, mapinfo_no) = split_map_id(active.map_id);
        Ok(SortieNextResponse {
            rashin_flg: current.next_cells.len() > 1,
            rashin_id: if current.next_cells.len() > 1 {
                1
            } else {
                0
            },
            maparea_id,
            mapinfo_no,
            cell_no: next.cell_no,
            color_no: next.color_no,
            event_id: next.event_id,
            event_kind: next.event_kind,
            has_next: !next.next_cells.is_empty(),
            boss_cell_no: stage.boss_cell_no,
            bosscomp: sortie_bosscomp(stage),
            from_cell_no: current.cell_no,
            comment_kind: Some(0),
            production_kind: Some(0),
            airsearch: Some(default_sortie_airsearch()),
            enemy_deck_preview: locked_enemy_composition
                .as_ref()
                .map(build_enemy_deck_preview)
                .filter(|preview| !preview.is_empty()),
            limit_state: Some(0),
            itemget,
            happening,
        })
    }

    async fn sortie_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store(),
            self.codex(),
            self.db(),
            profile_id,
            formation_id,
            BattleType::Normal,
        )
        .await
    }

    async fn sortie_airbattle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store(),
            self.codex(),
            self.db(),
            profile_id,
            formation_id,
            BattleType::AirBattle,
        )
        .await
    }

    async fn sortie_ld_airbattle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store(),
            self.codex(),
            self.db(),
            profile_id,
            formation_id,
            BattleType::LdAirBattle,
        )
        .await
    }

    async fn sortie_ld_shooting(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store(),
            self.codex(),
            self.db(),
            profile_id,
            formation_id,
            BattleType::LdShooting,
        )
        .await
    }

    async fn sortie_battle_result(
        &self,
        profile_id: i64,
    ) -> Result<SortieBattleResultResponse, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let store = self.sortie_store();
        let tx = db.begin().await?;

        let snapshot = store.take_pending_result(profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle result not found for profile {profile_id}",
            ))
        })?;
        let session = take_sortie_day_battle_result(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        let active = store.get_active_sortie(profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "active sortie not found for profile {profile_id}",
            ))
        })?;
        let pending_cell_id = active.pending_battle_cell_id.ok_or_else(|| {
            GameplayError::WrongType("no pending sortie battle to resolve".to_string())
        })?;

        let catalog = active_map_catalog(codex);
        let definition = catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
        })?;
        let stage = definition.stage(&active.stage_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "stage `{}` not found for map {}",
                active.stage_id, active.map_id,
            ))
        })?;
        let current_cell = stage.cell(pending_cell_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("cell {pending_cell_id} not found"))
        })?;

        let snapshot = update_sortie_result_stats(&tx, codex, profile_id, snapshot).await?;
        let first_clear = apply_sortie_map_result(
            &tx,
            profile_id,
            definition,
            stage,
            current_cell.cell_no == active.boss_cell_id,
            &snapshot,
        )
        .await?;
        let ship_drop = try_grant_sortie_ship_drop(
            &tx,
            codex,
            profile_id,
            stage,
            pending_cell_id,
            &snapshot.win_rank,
        )
        .await?;
        let quest_event = build_sortie_quest_event(definition, &active, &snapshot)?;
        update_quest_progress_for_action(&tx, codex, profile_id, &quest_event).await?;

        // Fire EnemyShipSunk events for each sunk enemy ship
        for (i, &hp) in snapshot.enemy_nowhps.iter().enumerate() {
            if hp <= 0 {
                if let Some(&stype) = snapshot.enemy_ship_types.get(i) {
                    let sink_event = QuestActionEvent::EnemyShipSunk {
                        ship_stype: stype,
                    };
                    update_quest_progress_for_action(&tx, codex, profile_id, &sink_event).await?;
                }
            }
        }

        let next_map_ids = if first_clear > 0 {
            let unlocked =
                check_and_unlock_dependencies_impl(&tx, codex, profile_id, definition.map_id)
                    .await?;
            if unlocked.is_empty() {
                None
            } else {
                Some(unlocked)
            }
        } else {
            None
        };

        tx.commit().await?;

        // Update in-memory store after commit so a failed transaction cannot
        // leave the store in a state inconsistent with the database.
        // On crash after commit: store is stale but DB is correct, and the
        // store is rebuilt from DB state on restart.
        let should_finish_sortie =
            current_cell.cell_no == active.boss_cell_id || current_cell.next_cells.is_empty();
        if should_finish_sortie {
            store.remove_active_sortie(profile_id);
        } else {
            store.modify_active_sortie(profile_id, |state| {
                state.pending_battle_cell_id = None;
            });
        }

        Ok(SortieBattleResultResponse {
            api_ship_id: snapshot.enemy_ship_ids,
            api_win_rank: snapshot.win_rank,
            api_get_exp: snapshot.get_exp,
            api_mvp: snapshot.mvp,
            api_member_lv: snapshot.member_lv,
            api_member_exp: snapshot.member_exp,
            api_get_base_exp: snapshot.get_base_exp,
            api_get_ship_exp: snapshot.get_ship_exp,
            api_get_exp_lvup: snapshot.get_exp_lvup,
            api_dests: session.packet.enemy_nowhps.iter().filter(|hp| **hp <= 0).count() as i64,
            api_destsf: i64::from(session.packet.enemy_nowhps.first().copied().unwrap_or(1) <= 0),
            api_quest_name: snapshot.quest_name,
            api_quest_level: snapshot.quest_level,
            api_enemy_info: SortieBattleResultEnemyInfo {
                api_level: snapshot.enemy_level,
                api_rank: snapshot.enemy_rank,
                api_deck_name: snapshot.enemy_deck_name,
            },
            api_first_clear: first_clear,
            api_get_flag: [0, i64::from(ship_drop.is_some()), 0],
            api_get_ship: ship_drop,
            api_next_map_ids: next_map_ids,
        })
    }

    async fn sortie_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<SortieNightBattleResponse, GameplayError> {
        let codex = self.codex();
        let store = self.sortie_store();
        let pending = pending_sortie_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        if !pending.outcome.can_midnight {
            return Err(GameplayError::WrongType(
                "night battle is not available for this sortie battle".to_string(),
            ));
        }

        let night = simulate_and_store_sortie_night_battle(
            store,
            codex,
            profile_id,
            pending.packet.formation[0],
            pending.packet.formation[1],
            EngagementType::from_api_id(pending.packet.formation[2])
                .unwrap_or(EngagementType::SameCourse),
        )
        .ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;

        let ct_flagship = pending_sortie_battle(store, profile_id)
            .and_then(|s| s.friendly.first().map(|f| f.ship.api_ship_id))
            .and_then(|sid| codex.manifest.find_ship(sid))
            .is_some_and(|m| m.api_stype == 21);

        store.with_pending_result_mut(profile_id, |snapshot| {
            snapshot.win_rank = night.outcome.win_rank.clone();
            snapshot.mvp = night.outcome.mvp;
            snapshot.get_exp =
                calculate_battle_admiral_exp(snapshot.get_base_exp, &snapshot.win_rank);
            if let Some(updated) = pending_sortie_battle(store, profile_id) {
                snapshot.friendly_nowhps = updated.friendly.iter().map(|f| f.hp().max(0)).collect();
                let friend_ships = updated
                    .friendly
                    .iter()
                    .cloned()
                    .map(|ship| BattleShipInput {
                        ship: ship.ship,
                        slot_items: ship.slot_items,
                        effect_list: ship.effect_list,
                    })
                    .collect::<Vec<_>>();
                let (ship_exp, ship_lvup) = calculate_sortie_ship_exp(
                    &friend_ships,
                    snapshot.get_base_exp,
                    snapshot.mvp,
                    &snapshot.friendly_nowhps,
                    ct_flagship,
                );
                snapshot.get_ship_exp = ship_exp;
                snapshot.get_exp_lvup = ship_lvup;
            }
        });

        let current = pending_sortie_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        Ok(build_sortie_night_battle_response(current.deck_id, &current, night.packet))
    }

    async fn sortie_sp_midnight_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<SortieNightBattleResponse, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let store = self.sortie_store();
        let tx = db.begin().await?;

        let mut active = store.get_active_sortie(profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "active sortie not found for profile {profile_id}",
            ))
        })?;
        if active.pending_battle_cell_id.is_some() {
            return Err(GameplayError::WrongType("sortie battle already pending".to_string()));
        }

        let profile = find_profile(&tx, profile_id).await?;
        let catalog = active_map_catalog(codex);
        let definition = catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
        })?;
        let stage = definition.stage(&active.stage_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "stage `{}` not found for map {}",
                active.stage_id, active.map_id,
            ))
        })?;

        let fleet_ships = get_fleet_ships_impl(&tx, profile_id, active.deck_id).await?;
        if fleet_ships.is_empty() {
            return Err(GameplayError::WrongType(format!(
                "fleet {} has no ships for sortie battle",
                active.deck_id,
            )));
        }

        let friend_ships = build_sortie_friend_ships(&tx, &fleet_ships).await?;
        let enemy_fleet = resolve_sortie_enemy_fleet(active.map_id, stage, active.current_cell_id);
        let enemy_composition = active
            .locked_enemy_composition
            .clone()
            .or_else(|| select_random_enemy_composition(&enemy_fleet))
            .unwrap_or_else(|| fallback_enemy_composition(active.current_cell_id));
        let (enemy_ships, enemy_level, enemy_rank, enemy_deck_name) =
            build_sortie_enemy_ships(codex, definition, &enemy_fleet, &enemy_composition)?;

        let enemy_formation_id = enemy_fleet.formations.first().copied().unwrap_or(1);
        let (day_session, night_session) = simulate_and_store_sortie_sp_midnight_battle(
            store,
            codex,
            SortieBattleInput {
                profile_id,
                deck_id: active.deck_id,
                map_id: active.map_id,
                cell_id: active.current_cell_id,
                context: BattleContext {
                    mode: BattleMode::Sortie,
                    battle_type: BattleType::Normal,
                    is_sortie: true,
                    friendly_formation_id: formation_id,
                    enemy_formation_id,
                    engagement: engagement_for_cell(active.map_id, active.current_cell_id),
                    friend_ships: friend_ships.clone(),
                    enemy_ships: enemy_ships.clone(),
                    rng_seed: None,
                },
            },
            enemy_formation_id,
        );

        let base_exp = calculate_sortie_base_exp(active.map_level, active.current_cell_id);
        let get_exp = calculate_battle_admiral_exp(base_exp, &night_session.outcome.win_rank);
        let friendly_nowhps: Vec<i64> = pending_sortie_battle(store, profile_id)
            .map(|s| s.friendly.iter().map(|f| f.hp().max(0)).collect())
            .unwrap_or_default();
        let ct_flagship = friend_ships
            .first()
            .and_then(|s| codex.manifest.find_ship(s.ship.api_ship_id))
            .is_some_and(|m| m.api_stype == 21);
        let (ship_exp, ship_lvup) = calculate_sortie_ship_exp(
            &friend_ships,
            base_exp,
            night_session.outcome.mvp,
            &friendly_nowhps,
            ct_flagship,
        );
        store.insert_pending_result(
            profile_id,
            SortieBattleResultSnapshot {
                friendly_ship_ids: day_session.friendly_ship_ids.clone(),
                enemy_ship_ids: day_session.enemy_ship_ids.clone(),
                friendly_nowhps,
                enemy_ship_types: day_session
                    .enemy_ship_ids
                    .iter()
                    .map(|&id| codex.find::<ApiMstShip>(&id).map(|m| m.api_stype).unwrap_or(0))
                    .collect(),
                enemy_nowhps: night_session.packet.enemy_nowhps.clone(),
                win_rank: night_session.outcome.win_rank.clone(),
                get_exp,
                member_lv: profile.hq_level,
                member_exp: profile.experience,
                get_base_exp: base_exp,
                mvp: night_session.outcome.mvp,
                get_ship_exp: ship_exp,
                get_exp_lvup: ship_lvup,
                quest_name: active.map_name.clone(),
                quest_level: active.map_level,
                enemy_level,
                enemy_rank,
                enemy_deck_name,
            },
        );

        active.pending_battle_cell_id = Some(active.current_cell_id);
        store.insert_active_sortie(profile_id, active);

        let current = pending_sortie_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;

        tx.commit().await?;
        Ok(build_sortie_night_battle_response(current.deck_id, &current, night_session.packet))
    }

    async fn sortie_goback_port(
        &self,
        profile_id: i64,
    ) -> Result<SortieGobackPortResponse, GameplayError> {
        let store = self.sortie_store();
        let removed = store.remove_active_sortie(profile_id);
        if removed.is_none() {
            return Err(GameplayError::EntryNotFound(format!(
                "active sortie not found for profile {profile_id}",
            )));
        }

        clear_pending_sortie_runtime_state(store, profile_id);

        Ok(SortieGobackPortResponse::default())
    }
}

async fn sortie_battle_impl(
    store: &SortieStore,
    codex: &Codex,
    db: &emukc_db::sea_orm::DatabaseConnection,
    profile_id: i64,
    formation_id: i64,
    battle_type: BattleType,
) -> Result<SortieBattleResponse, GameplayError> {
    let tx = db.begin().await?;

    let mut active = store.get_active_sortie(profile_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!("active sortie not found for profile {profile_id}",))
    })?;
    if active.pending_battle_cell_id.is_some() {
        return Err(GameplayError::WrongType("sortie battle already pending".to_string()));
    }

    let profile = find_profile(&tx, profile_id).await?;
    if profile.combined_type > 0 {
        return Err(GameplayError::WrongType(
            "combined sortie battle is not implemented yet".to_string(),
        ));
    }

    let catalog = active_map_catalog(codex);
    let definition = catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
    })?;
    let stage = definition.stage(&active.stage_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!(
            "stage `{}` not found for map {}",
            active.stage_id, active.map_id,
        ))
    })?;
    let current_cell = stage.cell(active.current_cell_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!(
            "cell {} not found in map {}",
            active.current_cell_id, active.map_id,
        ))
    })?;
    if current_cell.event_kind != 1 {
        return Err(GameplayError::WrongType(format!(
            "cell {} is not a battle cell",
            current_cell.cell_no,
        )));
    }

    let fleet_ships = get_fleet_ships_impl(&tx, profile_id, active.deck_id).await?;
    if fleet_ships.is_empty() {
        return Err(GameplayError::WrongType(format!(
            "fleet {} has no ships for sortie battle",
            active.deck_id,
        )));
    }

    let friend_ships = build_sortie_friend_ships(&tx, &fleet_ships).await?;
    let enemy_fleet = resolve_sortie_enemy_fleet(active.map_id, stage, current_cell.cell_no);
    let enemy_composition = active
        .locked_enemy_composition
        .clone()
        .or_else(|| select_random_enemy_composition(&enemy_fleet))
        .unwrap_or_else(|| fallback_enemy_composition(current_cell.cell_no));
    let (enemy_ships, enemy_level, enemy_rank, enemy_deck_name) =
        build_sortie_enemy_ships(codex, definition, &enemy_fleet, &enemy_composition)?;

    let session = simulate_and_store_sortie_day_battle(
        store,
        codex,
        SortieBattleInput {
            profile_id,
            deck_id: active.deck_id,
            map_id: active.map_id,
            cell_id: active.current_cell_id,
            context: BattleContext {
                mode: BattleMode::Sortie,
                battle_type,
                is_sortie: true,
                friendly_formation_id: formation_id,
                enemy_formation_id: enemy_fleet.formations.first().copied().unwrap_or(1),
                engagement: engagement_for_cell(active.map_id, active.current_cell_id),
                friend_ships: friend_ships.clone(),
                enemy_ships: enemy_ships.clone(),
                rng_seed: None,
            },
        },
    );

    let base_exp = calculate_sortie_base_exp(active.map_level, active.current_cell_id);
    let get_exp = calculate_battle_admiral_exp(base_exp, &session.outcome.win_rank);
    let friendly_nowhps: Vec<i64> = session.friendly.iter().map(|f| f.hp().max(0)).collect();
    let ct_flagship = friend_ships
        .first()
        .and_then(|s| codex.manifest.find_ship(s.ship.api_ship_id))
        .is_some_and(|m| m.api_stype == 21);
    let (ship_exp, ship_lvup) = calculate_sortie_ship_exp(
        &friend_ships,
        base_exp,
        session.outcome.mvp,
        &friendly_nowhps,
        ct_flagship,
    );
    let response = build_sortie_battle_response(
        active.deck_id,
        friend_ships,
        enemy_ships,
        session.packet.clone(),
    );
    store.insert_pending_result(
        profile_id,
        SortieBattleResultSnapshot {
            friendly_ship_ids: session.friendly_ship_ids.clone(),
            enemy_ship_ids: session.enemy_ship_ids.clone(),
            friendly_nowhps,
            enemy_ship_types: session
                .enemy_ship_ids
                .iter()
                .map(|&id| codex.find::<ApiMstShip>(&id).map(|m| m.api_stype).unwrap_or(0))
                .collect(),
            enemy_nowhps: session.packet.enemy_nowhps.clone(),
            win_rank: session.outcome.win_rank.clone(),
            get_exp,
            member_lv: profile.hq_level,
            member_exp: profile.experience,
            get_base_exp: base_exp,
            mvp: session.outcome.mvp,
            get_ship_exp: ship_exp,
            get_exp_lvup: ship_lvup,
            quest_name: active.map_name.clone(),
            quest_level: active.map_level,
            enemy_level,
            enemy_rank,
            enemy_deck_name,
        },
    );

    active.pending_battle_cell_id = Some(active.current_cell_id);
    store.insert_active_sortie(profile_id, active);

    tx.commit().await?;
    Ok(response)
}

pub(crate) fn split_map_id(map_id: i64) -> (i64, i64) {
    (map_id / 10, map_id % 10)
}

fn build_sortie_cell_data(map_id: i64, stage: &MapStageDefinition) -> Vec<SortieCellData> {
    stage
        .cells
        .iter()
        .map(|cell| SortieCellData {
            master_cell_id: cell.master_cell_id.unwrap_or(map_id * 100 + cell.cell_no),
            cell_no: cell.cell_no,
            color_no: cell.color_no,
            passed: cell.cell_no != 0,
            distance: cell.distance,
        })
        .collect()
}

fn default_sortie_airsearch() -> SortieAirSearch {
    SortieAirSearch {
        plane_type: 0,
        result: 0,
    }
}

fn build_enemy_deck_preview(composition: &EnemyComposition) -> Vec<SortieEnemyDeckPreview> {
    if composition.ship_ids.is_empty() {
        return Vec::new();
    }

    let api_kind = match composition.ship_ids.len() {
        0..=3 => 0,
        4 => 1,
        _ => 2,
    };

    vec![SortieEnemyDeckPreview {
        kind: api_kind,
        ship_ids: composition.ship_ids.iter().copied().take(3).collect(),
    }]
}

fn select_locked_enemy_composition(
    map_id: i64,
    stage: &MapStageDefinition,
    cell_no: i64,
) -> Option<EnemyComposition> {
    let current = stage.cell(cell_no)?;
    if current.event_kind != 1 {
        return None;
    }

    let enemy_fleet = resolve_sortie_enemy_fleet(map_id, stage, cell_no);
    select_random_enemy_composition(&enemy_fleet)
        .or_else(|| Some(fallback_enemy_composition(cell_no)))
}

fn sortie_bosscomp(stage: &MapStageDefinition) -> bool {
    stage.enemy_fleets.contains_key(&stage.boss_cell_no)
}

/// `KanColle` `event_id` values for non-battle cells:
/// 0 = start, 1 = no event, 2 = resource obtain, 3 = maelstrom (渦潮),
/// 4 = normal battle, 5 = boss, 6 = imaginary (気のせい), 7 = air battle.
///
/// Resolve resource acquisition or maelstrom loss for the given cell.
/// Only `event_kind`=0 cells produce effects; battle cells are handled elsewhere.
async fn resolve_non_battle_node_effect<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    cell: &MapCellDefinition,
    fleet_ships: &[ship::Model],
) -> Result<(Option<Vec<SortieItemGet>>, Option<SortieHappening>), GameplayError>
where
    C: ConnectionTrait,
{
    if cell.event_kind != 0 {
        return Ok((None, None));
    }

    match cell.event_id {
        2 => {
            // Resource acquisition node: award a resource based on map area.
            // Resource type cycles by color_no: 2=fuel, 3=ammo, 4=steel, 5=bauxite.
            let resource_type = match cell.color_no {
                2 => 1_i64, // green → fuel
                3 => 2,     // red → ammo
                6 => 3,     // grey → steel
                _ => 4,     // yellow/etc → bauxite
            };
            // Amount is proportional to fleet size (5-15 per ship).
            let base_amount = (fleet_ships.len() as i64) * 10;
            let amount = (base_amount + (cell.cell_no % 5) * 3).max(5);
            let category = MaterialCategory::from_id(resource_type);
            let _ = add_material_impl(c, codex, profile_id, &[(category, amount)]).await?;
            Ok((
                Some(vec![SortieItemGet {
                    resource_type,
                    amount,
                }]),
                None,
            ))
        }
        3 => {
            // Maelstrom (渦潮): lose fuel or ammo.
            // Type: fuel (api_type=1) or ammo (api_type=2), determined by color_no.
            let resource_type = if cell.color_no == 4 {
                2
            } else {
                1
            }; // purple=ammo, else=fuel
            let mat = get_mat_impl(c, profile_id).await?;
            let stock = if resource_type == 1 {
                mat.fuel
            } else {
                mat.ammo
            };
            // Base loss is ~20-40% of current stock (simplified).
            let base_loss = (stock * 3 / 10).max(1);
            // Radar (電探, type3=12/13/93) reduces loss by ~50%.
            let slot_ids: Vec<i64> = fleet_ships
                .iter()
                .flat_map(|s| [s.slot_1, s.slot_2, s.slot_3, s.slot_4, s.slot_5])
                .filter(|&id| id > 0)
                .collect();
            let has_radar = if slot_ids.is_empty() {
                false
            } else {
                slot_item::Entity::find()
                    .filter(slot_item::Column::Id.is_in(slot_ids))
                    .filter(slot_item::Column::Type3.is_in([12_i64, 13, 93]))
                    .count(c)
                    .await?
                    > 0
            };
            let (actual_loss, radar_reduced) = if has_radar {
                ((base_loss + 1) / 2, true)
            } else {
                (base_loss, false)
            };
            let final_loss = actual_loss.min(stock);
            if final_loss > 0 {
                let category = MaterialCategory::from_id(resource_type);
                let _ = deduct_material_impl(c, profile_id, &[(category, final_loss)]).await?;
            }
            Ok((
                None,
                Some(SortieHappening {
                    resource_type,
                    amount: final_loss,
                    radar_reduced,
                }),
            ))
        }
        _ => {
            // event_id 0 (start), 1 (nothing), 6 (imaginary) — no effect
            Ok((None, None))
        }
    }
}

async fn build_fleet_route_context<C>(
    c: &C,
    codex: &Codex,
    fleet_ships: &[ship::Model],
) -> Result<FleetRouteContext, GameplayError>
where
    C: ConnectionTrait,
{
    let slot_ids = fleet_ships
        .iter()
        .flat_map(|ship| {
            [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
        })
        .filter(|slot_id| *slot_id > 0)
        .collect::<Vec<_>>();
    let slot_items = if slot_ids.is_empty() {
        Vec::new()
    } else {
        find_slot_items_by_id_impl(c, &slot_ids).await?
    };
    let slot_item_types = slot_items
        .into_iter()
        .map(|item| (item.id, (item.type3, item.mst_id)))
        .collect::<BTreeMap<_, _>>();
    let mut ship_ids = BTreeSet::new();
    let mut ship_type_counts = BTreeMap::<i64, i64>::new();
    let mut ship_entries = Vec::with_capacity(fleet_ships.len());
    let mut min_speed = i64::MAX;
    let mut los_total = 0;
    let mut total_drums = 0;
    let mut flagship_ship_id = None;
    let mut flagship_ship_type = None;

    for (idx, ship) in fleet_ships.iter().enumerate() {
        ship_ids.insert(ship.mst_id);
        if let Some(mst) = codex.manifest.find_ship(ship.mst_id) {
            *ship_type_counts.entry(mst.api_stype).or_default() += 1;
            if idx == 0 {
                flagship_ship_id = Some(ship.mst_id);
                flagship_ship_type = Some(mst.api_stype);
            }
            let mut entry = FleetRouteShipEntry {
                ship_id: ship.mst_id,
                ship_type: mst.api_stype,
                speed: ship.speed,
                slotitem_types: BTreeSet::new(),
            };
            for slot_id in
                [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
            {
                let Some((type3, mst_id)) = slot_item_types.get(&slot_id).copied() else {
                    continue;
                };
                entry.slotitem_types.insert(type3);
                if mst_id == DRUM_CANISTER_MST_ID {
                    total_drums += 1;
                }
            }
            ship_entries.push(entry);
        }
        min_speed = min_speed.min(ship.speed);
        los_total += ship.los_now;
    }

    Ok(FleetRouteContext {
        fleet_size: fleet_ships.len() as i64,
        visited_cell_ids: BTreeSet::new(),
        ship_ids,
        flagship_ship_id,
        flagship_ship_type,
        ship_type_counts,
        ship_entries,
        min_speed: if min_speed == i64::MAX {
            0
        } else {
            min_speed
        },
        los_total,
        total_drums,
    })
}

async fn build_sortie_friend_ships<C>(
    c: &C,
    friend_ships: &[emukc_db::entity::profile::ship::Model],
) -> Result<Vec<BattleShipInput>, GameplayError>
where
    C: ConnectionTrait,
{
    let mut result = Vec::with_capacity(friend_ships.len());
    for ship in friend_ships {
        let slot_ids =
            [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
                .into_iter()
                .filter(|slot_id| *slot_id > 0)
                .collect::<Vec<_>>();
        let slot_items = find_slot_items_by_id_impl(c, &slot_ids).await?;
        let slot_items = slot_items.into_iter().map(std::convert::Into::into).collect();

        result.push(BattleShipInput {
            ship: (*ship).into(),
            slot_items,
            effect_list: vec![],
        });
    }

    Ok(result)
}

fn build_sortie_enemy_ships(
    codex: &Codex,
    definition: &MapDefinition,
    enemy_fleet: &EnemyFleetDefinition,
    composition: &EnemyComposition,
) -> Result<(Vec<BattleShipInput>, i64, String, String), GameplayError> {
    let enemy_level = (definition.level.max(1) * 5 + enemy_fleet.cell_no).max(1);
    let enemy_rank = UserHQRank::RearAdmiral.get_name().to_string();
    let enemy_deck_name = format!("{}海域敵艦隊", definition.name);
    let ship_ids = if composition.ship_ids.is_empty() {
        vec![412]
    } else {
        composition.ship_ids.clone()
    };

    let enemy_ships = ship_ids
        .into_iter()
        .map(|ship_id| build_sortie_enemy_ship(codex, ship_id, enemy_level))
        .collect::<Result<Vec<_>, GameplayError>>()?;

    Ok((enemy_ships, enemy_level, enemy_rank, enemy_deck_name))
}

fn build_sortie_enemy_ship(
    codex: &Codex,
    ship_id: i64,
    enemy_level: i64,
) -> Result<BattleShipInput, GameplayError> {
    if let Some((mut api_ship, slot_items)) = codex.new_enemy_ship(ship_id) {
        let exp_now = level::ship_level_required_exp(enemy_level.min(99));
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        api_ship.api_lv = enemy_level;
        api_ship.api_exp = [exp_now, next_exp, 0];
        return Ok(BattleShipInput {
            ship: api_ship,
            slot_items,
            effect_list: vec![0],
        });
    }

    if let Some((mut api_ship, slot_items)) = codex.new_ship(ship_id) {
        warn!(ship_id, "enemy bootstrap data missing; using ship_extra fallback for sortie enemy",);
        let exp_now = level::ship_level_required_exp(enemy_level.min(99));
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        api_ship.api_lv = enemy_level;
        api_ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut api_ship, &slot_items)?;
        for (idx, slot_item) in slot_items.iter().take(5).enumerate() {
            api_ship.api_slot[idx] = slot_item.api_slotitem_id;
        }
        return Ok(BattleShipInput {
            ship: api_ship,
            slot_items,
            effect_list: vec![0],
        });
    }

    let mst =
        codex.manifest.find_ship(ship_id).ok_or_else(|| {
            warn!(
                ship_id,
                "enemy bootstrap data missing and no manifest entry found for sortie enemy",
            );
            GameplayError::ManifestNotFound(ship_id)
        })?;
    Ok(build_manifest_only_sortie_enemy_ship(mst, ship_id, enemy_level))
}

#[derive(Debug)]
struct ManifestOnlyEnemyStats {
    sortno: i64,
    hp: [i64; 2],
    firepower: [i64; 2],
    torpedo: [i64; 2],
    aa: [i64; 2],
    armor: [i64; 2],
    asw: [i64; 2],
    luck: [i64; 2],
    range: i64,
    backs: i64,
    fuel: i64,
    bull: i64,
    missing_fields: Vec<&'static str>,
}

fn build_manifest_only_sortie_enemy_ship(
    mst: &ApiMstShip,
    ship_id: i64,
    enemy_level: i64,
) -> BattleShipInput {
    let fallback = manifest_only_enemy_stats(mst);
    if fallback.missing_fields.is_empty() {
        warn!(ship_id, "enemy bootstrap data missing; using manifest-only sortie enemy fallback",);
    } else {
        warn!(
            ship_id,
            missing_fields = ?fallback.missing_fields,
            "enemy bootstrap data missing; using degraded manifest-only sortie enemy fallback",
        );
    }
    let exp_now = level::ship_level_required_exp(enemy_level.min(99));
    let (_, next_exp) = level::exp_to_ship_level(exp_now);
    let hp = fallback.hp;
    let api_ship = KcApiShip {
        api_id: 0,
        api_sortno: fallback.sortno,
        api_ship_id: ship_id,
        api_lv: enemy_level,
        api_exp: [exp_now, next_exp, 0],
        api_nowhp: hp[0].max(1),
        api_maxhp: hp[0].max(1),
        api_soku: mst.api_soku,
        api_leng: fallback.range,
        api_slot: [-1; 5],
        api_onslot: [0; 5],
        api_slot_ex: 0,
        api_kyouka: [0; 7],
        api_backs: fallback.backs,
        api_fuel: fallback.fuel,
        api_bull: fallback.bull,
        api_slotnum: mst.api_slot_num,
        api_ndock_time: 0,
        api_ndock_item: [0, 0],
        api_srate: 0,
        api_cond: 49,
        api_karyoku: fallback.firepower,
        api_raisou: fallback.torpedo,
        api_taiku: fallback.aa,
        api_soukou: fallback.armor,
        api_kaihi: [0, 0],
        api_taisen: fallback.asw,
        api_sakuteki: [0, 0],
        api_lucky: fallback.luck,
        api_locked: 0,
        api_locked_equip: 0,
        api_sally_area: 0,
        api_sp_effect_items: None,
    };

    BattleShipInput {
        ship: api_ship,
        slot_items: Vec::<KcApiSlotItem>::new(),
        effect_list: vec![0],
    }
}

fn manifest_only_enemy_stats(mst: &ApiMstShip) -> ManifestOnlyEnemyStats {
    let mut missing_fields = Vec::new();
    let _ = manifest_onslot_or_default(mst.api_maxeq, "api_maxeq", &mut missing_fields);
    ManifestOnlyEnemyStats {
        sortno: mst.api_sortno.unwrap_or(mst.api_sort_id),
        hp: manifest_pair_or_default(mst.api_taik, [1, 1], "api_taik", &mut missing_fields),
        firepower: manifest_pair_or_default(mst.api_houg, [0, 0], "api_houg", &mut missing_fields),
        torpedo: manifest_pair_or_default(mst.api_raig, [0, 0], "api_raig", &mut missing_fields),
        aa: manifest_pair_or_default(mst.api_tyku, [0, 0], "api_tyku", &mut missing_fields),
        armor: manifest_pair_or_default(mst.api_souk, [0, 0], "api_souk", &mut missing_fields),
        asw: manifest_single_pair_or_default(mst.api_tais, [0, 0], "api_tais", &mut missing_fields),
        luck: manifest_pair_or_default(mst.api_luck, [0, 0], "api_luck", &mut missing_fields),
        range: mst.api_leng.unwrap_or(-1),
        backs: mst.api_backs.unwrap_or(-1),
        fuel: mst.api_fuel_max.unwrap_or(0),
        bull: mst.api_bull_max.unwrap_or(0),
        missing_fields,
    }
}

fn manifest_pair_or_default(
    value: Option<[i64; 2]>,
    default: [i64; 2],
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 2] {
    value.unwrap_or_else(|| {
        missing_fields.push(field);
        default
    })
}

fn manifest_single_pair_or_default(
    value: Option<[i64; 1]>,
    default: [i64; 2],
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 2] {
    value.map(|[stat]| [stat, stat]).unwrap_or_else(|| {
        missing_fields.push(field);
        default
    })
}

fn manifest_onslot_or_default(
    value: Option<[i64; 5]>,
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 5] {
    value.unwrap_or_else(|| {
        missing_fields.push(field);
        [0; 5]
    })
}

fn resolve_sortie_enemy_fleet(
    map_id: i64,
    variant: &MapVariantDefinition,
    cell_no: i64,
) -> EnemyFleetDefinition {
    if let Some(enemy_fleet) = variant.enemy_fleets.get(&cell_no) {
        return enemy_fleet.clone();
    }

    warn!(
        map_id,
        cell_no, "missing enemy fleet definition for sortie cell; using fallback composition",
    );
    fallback_enemy_fleet(cell_no)
}

fn fallback_enemy_fleet(cell_no: i64) -> EnemyFleetDefinition {
    EnemyFleetDefinition {
        cell_no,
        battle_kind: 1,
        formations: vec![1],
        compositions: vec![fallback_enemy_composition(cell_no)],
    }
}

fn fallback_enemy_composition(cell_no: i64) -> EnemyComposition {
    EnemyComposition {
        comp_id: format!("fallback:{cell_no}"),
        weight: 1,
        ship_ids: vec![412],
        formation: Some(1),
        raw_ship_names: Vec::new(),
    }
}

fn select_random_enemy_composition(enemy_fleet: &EnemyFleetDefinition) -> Option<EnemyComposition> {
    if enemy_fleet.compositions.is_empty() {
        return None;
    }

    let total_weight = enemy_fleet
        .compositions
        .iter()
        .map(|composition| composition.weight.max(1) as u64)
        .sum::<u64>();
    if total_weight == 0 {
        return enemy_fleet.compositions.first().cloned();
    }

    let roll = rng::u64(0..total_weight);
    select_enemy_composition_for_roll(enemy_fleet, roll).cloned()
}

fn select_enemy_composition_for_roll(
    enemy_fleet: &EnemyFleetDefinition,
    mut roll: u64,
) -> Option<&EnemyComposition> {
    for composition in &enemy_fleet.compositions {
        let weight = composition.weight.max(1) as u64;
        if roll < weight {
            return Some(composition);
        }
        roll -= weight;
    }

    enemy_fleet.compositions.last()
}

fn clear_pending_sortie_runtime_state(store: &SortieStore, profile_id: i64) {
    store.take_pending_result(profile_id);
    let _ = take_sortie_day_battle_result(store, profile_id);
}

fn build_sortie_battle_response(
    deck_id: i64,
    friend_ships: Vec<BattleShipInput>,
    enemy_ships: Vec<BattleShipInput>,
    packet: BattlePacket,
) -> SortieBattleResponse {
    SortieBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: friend_ships.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_f_maxhps: friend_ships.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_fParam: friend_ships
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_ship_ke: enemy_ships.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy_ships.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: enemy_ships.iter().map(|ship| ship.ship.api_nowhp).collect(),
        api_e_maxhps: enemy_ships.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_eSlot: enemy_ships.iter().map(enemy_slot_ids).collect(),
        api_eParam: enemy_ships
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_e_effect_list: enemy_ships
            .iter()
            .map(|ship| {
                if ship.effect_list.is_empty() {
                    vec![0]
                } else {
                    ship.effect_list.clone()
                }
            })
            .collect(),
        api_smoke_type: packet.smoke_type,
        api_balloon_cell: packet.balloon_cell,
        api_atoll_cell: packet.atoll_cell,
        api_midnight_flag: packet.midnight_flag,
        api_search: packet.search,
        api_stage_flag: packet.stage_flag,
        api_kouku: packet.kouku,
        api_opening_taisen_flag: packet.opening_taisen_flag,
        api_opening_taisen: packet.opening_taisen,
        api_opening_flag: packet.opening_flag,
        api_opening_atack: packet.opening_attack,
        api_hourai_flag: packet.hourai_flag,
        api_hougeki1: packet.hougeki1,
        api_hougeki2: packet.hougeki2,
        api_hougeki3: packet.hougeki3,
        api_raigeki: packet.raigeki,
    }
}

fn build_sortie_night_battle_response(
    deck_id: i64,
    session: &crate::game::battle::sortie::SortieBattleSession,
    packet: crate::game::battle::core::NightBattlePacket,
) -> SortieNightBattleResponse {
    SortieNightBattleResponse {
        api_deck_id: deck_id,
        api_formation: packet.formation,
        api_f_nowhps: packet.friendly_nowhps,
        api_f_maxhps: packet.friendly_maxhps,
        api_fParam: session
            .friendly
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_ship_ke: session.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: session.enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: packet.enemy_nowhps,
        api_e_maxhps: packet.enemy_maxhps,
        api_eSlot: session
            .enemy
            .iter()
            .map(|ship| {
                enemy_slot_ids(&BattleShipInput {
                    ship: ship.ship.clone(),
                    slot_items: ship.slot_items.clone(),
                    effect_list: ship.effect_list.clone(),
                })
            })
            .collect(),
        api_eParam: session
            .enemy
            .iter()
            .map(|ship| {
                [
                    ship.ship.api_karyoku[0],
                    ship.ship.api_raisou[0],
                    ship.ship.api_taiku[0],
                    ship.ship.api_soukou[0],
                ]
            })
            .collect(),
        api_smoke_type: 0,
        api_balloon_cell: 0,
        api_atoll_cell: 0,
        api_touch_plane: packet.touch_plane,
        api_flare_pos: packet.flare_pos,
        api_hougeki: packet.hougeki,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::battle::{
        core::{BattleMode, BattleType},
        sortie::{
            pending_sortie_battle, simulate_and_store_sortie_day_battle,
            simulate_and_store_sortie_sp_midnight_battle,
        },
    };
    use crate::prelude::*;
    use emukc_bootstrap::prelude::build_final_map_catalog_from_repo_assets;
    use emukc_db::{
        entity::profile::map_record,
        prelude::new_mem_db,
        sea_orm::{
            ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
        },
    };
    use emukc_model::{
        codex::Codex,
        prelude::{ApiMstShip, Kc3rdEnemyShip, Kc3rdEnemyShipSlotInfo},
    };
    use std::collections::HashMap;

    fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items).unwrap();
        BattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
        }
    }

    fn weaken_for_midnight(mut ship: BattleShipInput) -> BattleShipInput {
        ship.ship.api_karyoku[0] = 1;
        ship.ship.api_raisou[0] = 0;
        ship.ship.api_soukou[0] = 200;
        ship
    }

    fn enemy_test_codex() -> Codex {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let enemy_ship_id = 19991;
        let enemy_slot_id = 1519;
        codex.manifest.api_mst_ship.push(ApiMstShip {
            api_id: enemy_ship_id,
            api_name: "enemy-test".to_string(),
            api_yomi: "enemy-test".to_string(),
            api_stype: 7,
            api_ctype: 1,
            api_soku: 10,
            api_slot_num: 2,
            api_sort_id: enemy_ship_id,
            api_sortno: Some(enemy_ship_id),
            api_taik: Some([45, 45]),
            api_houg: Some([35, 35]),
            api_raig: Some([10, 10]),
            api_tyku: Some([40, 40]),
            api_souk: Some([20, 20]),
            api_tais: Some([30]),
            api_luck: Some([5, 5]),
            api_maxeq: Some([18, 6, 0, 0, 0]),
            api_leng: Some(2),
            api_backs: Some(4),
            api_fuel_max: Some(0),
            api_bull_max: Some(0),
            ..ApiMstShip::default()
        });
        codex.enemy_ship_extra.insert(
            enemy_ship_id,
            Kc3rdEnemyShip {
                api_id: enemy_ship_id,
                name: "enemy-test".to_string(),
                yomi: "enemy-test".to_string(),
                stype: 7,
                ctype: 1,
                hp: 45,
                firepower: 35,
                torpedo: 10,
                aa: 40,
                armor: 20,
                evasion: 12,
                asw: 30,
                los: 18,
                luck: 5,
                speed: 10,
                range: 2,
                rarity: 4,
                backs: 4,
                slot_num: 2,
                maxeq: [18, 6, 0, 0, 0],
                slots: vec![
                    Kc3rdEnemyShipSlotInfo {
                        item_id: enemy_slot_id,
                        onslot: 18,
                    },
                    Kc3rdEnemyShipSlotInfo {
                        item_id: 525,
                        onslot: 6,
                    },
                ],
            },
        );
        codex
    }

    fn manifest_only_test_codex(mst: ApiMstShip) -> Codex {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        codex.manifest.api_mst_ship.retain(|ship| ship.api_id != mst.api_id);
        codex.ship_extra.remove(&mst.api_id);
        codex.enemy_ship_extra.remove(&mst.api_id);
        codex.manifest.api_mst_ship.push(mst);
        codex
    }

    fn successful_boss_snapshot() -> SortieBattleResultSnapshot {
        SortieBattleResultSnapshot {
            friendly_ship_ids: vec![],
            enemy_ship_ids: vec![],
            friendly_nowhps: vec![],
            enemy_ship_types: vec![],
            enemy_nowhps: vec![],
            win_rank: "S".to_string(),
            get_exp: 0,
            member_lv: 0,
            member_exp: 0,
            get_base_exp: 0,
            mvp: 0,
            get_ship_exp: vec![],
            get_exp_lvup: vec![],
            quest_name: String::new(),
            quest_level: 0,
            enemy_level: 0,
            enemy_rank: String::new(),
            enemy_deck_name: String::new(),
        }
    }

    #[test]
    fn build_sortie_enemy_ship_prefers_enemy_bootstrap_stats_and_slots() {
        let codex = enemy_test_codex();
        let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
        assert_eq!(enemy.ship.api_ship_id, 19991);
        assert_eq!(enemy.ship.api_nowhp, 45);
        assert_eq!(enemy.ship.api_karyoku, [35, 35]);
        assert_eq!(enemy.ship.api_taiku, [40, 40]);
        assert_eq!(enemy.ship.api_taisen, [30, 30]);
        assert_eq!(enemy.ship.api_onslot, [18, 6, 0, 0, 0]);
        assert_eq!(enemy_slot_ids(&enemy), [1519, 525, -1, -1, -1]);
    }

    #[test]
    fn build_sortie_enemy_ship_drops_enemy_slots_missing_from_manifest() {
        let mut codex = enemy_test_codex();
        let enemy_extra = codex.enemy_ship_extra.get_mut(&19991).unwrap();
        enemy_extra.slots[0].item_id = 999999;

        let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
        assert_eq!(enemy.ship.api_onslot, [0, 6, 0, 0, 0]);
        assert_eq!(enemy_slot_ids(&enemy), [-1, 525, -1, -1, -1]);
    }

    #[test]
    fn build_sortie_enemy_ship_uses_enemy_bootstrap_when_manifest_entry_is_missing() {
        let mut codex = enemy_test_codex();
        codex.manifest.api_mst_ship.retain(|ship| ship.api_id != 19991);
        assert!(codex.manifest.find_ship(19991).is_none());
        assert!(codex.new_ship(19991).is_none());

        let (bootstrap_ship, bootstrap_slots) = codex.new_enemy_ship(19991).unwrap();
        assert_eq!(bootstrap_ship.api_sortno, 19991);
        assert_eq!(bootstrap_ship.api_fuel, 0);
        assert_eq!(bootstrap_ship.api_bull, 0);
        assert_eq!(bootstrap_ship.api_onslot, [18, 6, 0, 0, 0]);
        assert_eq!(
            bootstrap_slots.iter().map(|slot| slot.api_slotitem_id).collect::<Vec<_>>(),
            vec![1519, 525]
        );

        let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
        assert_eq!(enemy.ship.api_ship_id, 19991);
        assert_eq!(enemy.ship.api_sortno, 19991);
        assert_eq!(enemy.ship.api_fuel, 0);
        assert_eq!(enemy.ship.api_bull, 0);
        assert_eq!(enemy.ship.api_nowhp, 45);
        assert_eq!(enemy.ship.api_karyoku, [35, 35]);
        assert_eq!(enemy.ship.api_taiku, [40, 40]);
        assert_eq!(enemy.ship.api_taisen, [30, 30]);
        assert_eq!(enemy.ship.api_onslot, [18, 6, 0, 0, 0]);
        assert_eq!(enemy_slot_ids(&enemy), [1519, 525, -1, -1, -1]);
    }

    #[test]
    fn build_sortie_enemy_ship_falls_back_to_ship_extra_data_when_enemy_bootstrap_is_missing() {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship_id = 518;
        codex.enemy_ship_extra.remove(&ship_id);
        assert!(codex.new_enemy_ship(ship_id).is_none());

        let expected = sample_ship(&codex, ship_id, 55);
        let enemy = build_sortie_enemy_ship(&codex, ship_id, 55).unwrap();
        assert_eq!(enemy.ship.api_ship_id, ship_id);
        assert_eq!(enemy.ship.api_lv, 55);
        assert_eq!(enemy.ship.api_nowhp, expected.ship.api_nowhp);
        assert_eq!(enemy.ship.api_karyoku, expected.ship.api_karyoku);
        assert_eq!(enemy.ship.api_kaihi, expected.ship.api_kaihi);
        assert_eq!(enemy.ship.api_taisen, expected.ship.api_taisen);
        assert_eq!(enemy.ship.api_lucky, expected.ship.api_lucky);
        assert_eq!(enemy.ship.api_onslot, expected.ship.api_onslot);
        assert_eq!(enemy_slot_ids(&enemy), enemy_slot_ids(&expected));
    }

    #[test]
    fn build_sortie_enemy_ship_keeps_common_abyssals_buildable_without_enemy_bootstrap() {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        for ship_id in [1501, 1505, 1538] {
            codex.enemy_ship_extra.remove(&ship_id);
            assert!(codex.new_enemy_ship(ship_id).is_none());
            assert!(codex.new_ship(ship_id).is_none());

            let mst = codex.manifest.find_ship(ship_id).unwrap();
            let enemy = build_sortie_enemy_ship(&codex, ship_id, 45).unwrap();
            assert_eq!(enemy.ship.api_ship_id, ship_id);
            assert_eq!(enemy.ship.api_lv, 45);
            assert_eq!(enemy.ship.api_sortno, mst.api_sortno.unwrap_or(mst.api_sort_id));
            assert_eq!(enemy.ship.api_slotnum, mst.api_slot_num);
            assert_eq!(enemy.ship.api_nowhp, mst.api_taik.unwrap_or([1, 1])[0].max(1));
            assert_eq!(enemy.ship.api_karyoku, mst.api_houg.unwrap_or([0, 0]));
            assert_eq!(enemy.ship.api_taiku, mst.api_tyku.unwrap_or([0, 0]));
            assert_eq!(
                enemy.ship.api_taisen,
                mst.api_tais.map(|[stat]| [stat, stat]).unwrap_or([0, 0]),
            );
            assert_eq!(enemy.ship.api_lucky, mst.api_luck.unwrap_or([0, 0]));
            assert_eq!(enemy.ship.api_onslot, [0; 5]);
            assert!(enemy.slot_items.is_empty());
        }
    }

    #[test]
    fn build_sortie_enemy_ship_manifest_fallback_uses_available_manifest_stats() {
        let ship_id = 29991;
        let codex = manifest_only_test_codex(ApiMstShip {
            api_id: ship_id,
            api_name: "enemy-manifest-only".to_string(),
            api_yomi: "enemy-manifest-only".to_string(),
            api_stype: 7,
            api_ctype: 1,
            api_soku: 10,
            api_slot_num: 2,
            api_sort_id: ship_id,
            api_taik: Some([45, 45]),
            api_houg: Some([35, 35]),
            api_raig: Some([10, 10]),
            api_tyku: Some([40, 40]),
            api_souk: Some([20, 20]),
            api_tais: Some([30]),
            api_luck: Some([5, 5]),
            api_maxeq: Some([18, 6, 0, 0, 0]),
            api_leng: Some(2),
            api_backs: Some(4),
            api_fuel_max: Some(0),
            api_bull_max: Some(0),
            ..ApiMstShip::default()
        });

        let enemy = build_sortie_enemy_ship(&codex, ship_id, 45).unwrap();
        assert_eq!(enemy.ship.api_ship_id, ship_id);
        assert_eq!(enemy.ship.api_sortno, ship_id);
        assert_eq!(enemy.ship.api_nowhp, 45);
        assert_eq!(enemy.ship.api_karyoku, [35, 35]);
        assert_eq!(enemy.ship.api_raisou, [10, 10]);
        assert_eq!(enemy.ship.api_taiku, [40, 40]);
        assert_eq!(enemy.ship.api_soukou, [20, 20]);
        assert_eq!(enemy.ship.api_taisen, [30, 30]);
        assert_eq!(enemy.ship.api_lucky, [5, 5]);
        assert_eq!(enemy.ship.api_onslot, [0; 5]);
        assert!(enemy.slot_items.is_empty());
    }

    #[tokio::test]
    async fn sortie_midnight_battle_updates_pending_snapshot() {
        use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
        let store = &*GLOBAL_SORTIE_STORE;
        store.clear();

        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = (db, codex.clone());
        let profile_id = 42;

        let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
        let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));
        let session = simulate_and_store_sortie_day_battle(
            store,
            &codex,
            SortieBattleInput {
                profile_id,
                deck_id: 1,
                map_id: 11,
                cell_id: 1,
                context: BattleContext {
                    mode: BattleMode::Sortie,
                    battle_type: BattleType::Normal,
                    is_sortie: true,
                    friendly_formation_id: 1,
                    enemy_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    friend_ships: vec![friend.clone()],
                    enemy_ships: vec![enemy.clone()],
                    rng_seed: Some(1),
                },
            },
        );

        assert_eq!(session.packet.midnight_flag, 1);
        store.insert_pending_result(
            profile_id,
            SortieBattleResultSnapshot {
                friendly_ship_ids: session.friendly_ship_ids.clone(),
                enemy_ship_ids: session.enemy_ship_ids.clone(),
                friendly_nowhps: session.friendly.iter().map(|f| f.hp().max(0)).collect(),
                enemy_ship_types: session
                    .enemy_ship_ids
                    .iter()
                    .map(|&id| codex.find::<ApiMstShip>(&id).map(|m| m.api_stype).unwrap_or(0))
                    .collect(),
                enemy_nowhps: session.packet.enemy_nowhps.clone(),
                win_rank: session.outcome.win_rank.clone(),
                get_exp: 0,
                member_lv: 1,
                member_exp: 0,
                get_base_exp: 30,
                mvp: session.outcome.mvp,
                get_ship_exp: vec![],
                get_exp_lvup: vec![],
                quest_name: "test".to_string(),
                quest_level: 1,
                enemy_level: 1,
                enemy_rank: "Test".to_string(),
                enemy_deck_name: "Test".to_string(),
            },
        );

        let response = context.sortie_midnight_battle(profile_id).await.unwrap();
        assert_eq!(response.api_deck_id, 1);
        assert!(response.api_hougeki.is_some());

        let updated_snapshot = store.take_pending_result(profile_id).unwrap();
        assert!(!updated_snapshot.win_rank.is_empty());
        assert!(updated_snapshot.mvp >= 1);

        let stored = pending_sortie_battle(store, profile_id).unwrap();
        assert_eq!(stored.packet.midnight_flag, 0);

        let _ = take_sortie_day_battle_result(store, profile_id);
        store.clear();
    }

    #[tokio::test]
    async fn sortie_sp_midnight_battle_runs_night_only() {
        use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
        let store = &*GLOBAL_SORTIE_STORE;
        store.clear();

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let profile_id = 84;

        let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
        let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));

        let (day_session, night_session) = simulate_and_store_sortie_sp_midnight_battle(
            store,
            &codex,
            SortieBattleInput {
                profile_id,
                deck_id: 1,
                map_id: 11,
                cell_id: 1,
                context: BattleContext {
                    mode: BattleMode::Sortie,
                    battle_type: BattleType::Normal,
                    is_sortie: true,
                    friendly_formation_id: 1,
                    enemy_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    friend_ships: vec![friend.clone()],
                    enemy_ships: vec![enemy.clone()],
                    rng_seed: Some(1),
                },
            },
            1,
        );

        // Day packet should have no combat phases (sp_midnight skips day battle)
        assert!(day_session.packet.kouku.is_none());
        assert!(day_session.packet.hougeki1.is_none());
        assert!(day_session.packet.opening_taisen.is_none());
        assert_eq!(day_session.packet.hourai_flag, [0, 0, 0, 0]);

        // Night battle should have run
        assert!(night_session.packet.hougeki.is_some());
        assert_eq!(night_session.profile_id, profile_id);

        // The stored session should have been updated with night results
        let stored = pending_sortie_battle(store, profile_id).unwrap();
        assert_eq!(stored.packet.midnight_flag, 0); // no further midnight allowed

        clear_pending_sortie_runtime_state(store, profile_id);
    }

    #[test]
    fn weighted_enemy_composition_selection_uses_weights() {
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 3,
            battle_kind: 1,
            formations: vec![1],
            compositions: vec![
                EnemyComposition {
                    comp_id: "light".to_string(),
                    weight: 1,
                    ship_ids: vec![501],
                    formation: Some(1),
                    raw_ship_names: Vec::new(),
                },
                EnemyComposition {
                    comp_id: "heavy".to_string(),
                    weight: 3,
                    ship_ids: vec![502],
                    formation: Some(1),
                    raw_ship_names: Vec::new(),
                },
            ],
        };

        assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 0).unwrap().comp_id, "light",);
        assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 1).unwrap().comp_id, "heavy",);
        assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 3).unwrap().comp_id, "heavy",);
    }

    #[test]
    fn fallback_enemy_fleet_is_only_used_when_catalog_data_is_missing() {
        let mut variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 5,
            cells: vec![],
            routing_rules: HashMap::new().into_iter().collect(),
            enemy_fleets: HashMap::new().into_iter().collect(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };
        variant.enemy_fleets.insert(
            2,
            EnemyFleetDefinition {
                cell_no: 2,
                battle_kind: 1,
                formations: vec![2],
                compositions: vec![EnemyComposition {
                    comp_id: "real".to_string(),
                    weight: 1,
                    ship_ids: vec![501, 502],
                    formation: Some(2),
                    raw_ship_names: Vec::new(),
                }],
            },
        );

        let real = resolve_sortie_enemy_fleet(11, &variant, 2);
        assert_eq!(real.formations, vec![2]);
        assert_eq!(real.compositions[0].ship_ids, vec![501, 502]);

        let fallback = resolve_sortie_enemy_fleet(11, &variant, 7);
        assert_eq!(fallback.compositions[0].ship_ids, vec![412]);
    }

    #[test]
    fn eligible_sortie_ship_drops_skip_limited_and_non_victory_results() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 3,
            cells: vec![],
            routing_rules: BTreeMap::new(),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::from([(
                1,
                vec![
                    emukc_model::codex::map::ShipDropDefinition {
                        ship_id: 1,
                        raw_ship_name: "睦月".to_string(),
                        tags: Vec::new(),
                    },
                    emukc_model::codex::map::ShipDropDefinition {
                        ship_id: 2,
                        raw_ship_name: "如月".to_string(),
                        tags: vec!["limited".to_string()],
                    },
                    emukc_model::codex::map::ShipDropDefinition {
                        ship_id: 999999,
                        raw_ship_name: "missing".to_string(),
                        tags: Vec::new(),
                    },
                ],
            )]),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };

        let eligible = eligible_sortie_ship_drops(&codex, &variant, 1, "S");
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].ship_id, 1);
        assert!(eligible_sortie_ship_drops(&codex, &variant, 1, "C").is_empty());
    }

    #[test]
    fn route_predicate_matches_ship_set_variants() {
        fn route_entry(
            ship_id: i64,
            ship_type: i64,
            speed: i64,
            slotitem_types: &[i64],
        ) -> FleetRouteShipEntry {
            FleetRouteShipEntry {
                ship_id,
                ship_type,
                speed,
                slotitem_types: slotitem_types.iter().copied().collect(),
            }
        }

        let context = FleetRouteContext {
            fleet_size: 3,
            visited_cell_ids: BTreeSet::new(),
            ship_ids: BTreeSet::from([526, 6001, 6002]),
            flagship_ship_id: Some(526),
            flagship_ship_type: Some(7),
            ship_type_counts: BTreeMap::from([(2, 2), (7, 1)]),
            ship_entries: vec![
                route_entry(526, 7, 10, &[]),
                route_entry(6001, 2, 10, &[]),
                route_entry(6002, 2, 10, &[]),
            ],
            min_speed: 10,
            los_total: 20,
            total_drums: 0,
        };

        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::ContainsShipSet {
                    ship_types: vec![1],
                    ship_ids: vec![526],
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::OnlyShipSet {
                    ship_types: vec![2],
                    ship_ids: vec![526],
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::ShipSetCount {
                    ship_types: vec![2],
                    ship_ids: vec![526],
                    op: RouteOperator::Eq,
                    value: 3,
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::FlagshipShipId {
                    ship_ids: vec![526],
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn route_predicate_matches_visited_equipment_and_speed_qualified_predicates() {
        let context = FleetRouteContext {
            fleet_size: 4,
            visited_cell_ids: BTreeSet::from([1, 4]),
            ship_ids: BTreeSet::from([9001, 9002, 9003, 9004]),
            flagship_ship_id: Some(9001),
            flagship_ship_type: Some(3),
            ship_type_counts: BTreeMap::from([(3, 1), (8, 2), (11, 1)]),
            ship_entries: vec![
                FleetRouteShipEntry {
                    ship_id: 9001,
                    ship_type: 3,
                    speed: 10,
                    slotitem_types: BTreeSet::from([12]),
                },
                FleetRouteShipEntry {
                    ship_id: 9002,
                    ship_type: 8,
                    speed: 5,
                    slotitem_types: BTreeSet::new(),
                },
                FleetRouteShipEntry {
                    ship_id: 9003,
                    ship_type: 8,
                    speed: 5,
                    slotitem_types: BTreeSet::new(),
                },
                FleetRouteShipEntry {
                    ship_id: 9004,
                    ship_type: 11,
                    speed: 10,
                    slotitem_types: BTreeSet::new(),
                },
            ],
            min_speed: 5,
            los_total: 20,
            total_drums: 0,
        };

        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNode {
                    cell_nos: vec![4],
                    visited: true,
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::VisitedNode {
                    cell_nos: vec![7],
                    visited: false,
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::EquipmentCount {
                    slotitem_types: vec![12, 13, 93],
                    op: RouteOperator::Eq,
                    value: 1,
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::FlagshipShipType {
                    ship_types: vec![3],
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
        assert!(matches!(
            route_predicate_matches(
                &RoutePredicate::ShipSetSpeedCount {
                    ship_types: vec![8],
                    ship_ids: vec![],
                    speed_op: RouteOperator::Lte,
                    speed_class: SpeedClass::Slow,
                    op: RouteOperator::Gte,
                    value: 2,
                },
                &context,
            ),
            crate::game::map_route::RoutePredicateEval::Matched
        ));
    }

    #[test]
    fn route_rules_prefer_executable_predicates_over_static_next_cells() {
        let current = MapCellDefinition {
            cell_no: 1,
            color_no: 4,
            event_id: 4,
            event_kind: 1,
            next_cells: vec![2, 3],
            node_label: None,
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 3,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::from([(
                1,
                vec![
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 2,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::ContainsShipType {
                            ship_types: vec![13],
                        },
                        raw_text: "潜水艦を含む".to_string(),
                    },
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 3,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: "それ以外".to_string(),
                    },
                ],
            )]),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };
        let context = FleetRouteContext {
            fleet_size: 4,
            visited_cell_ids: BTreeSet::new(),
            ship_ids: BTreeSet::new(),
            flagship_ship_id: None,
            flagship_ship_type: None,
            ship_type_counts: BTreeMap::from([(2, 4)]),
            ship_entries: vec![
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
            ],
            min_speed: 10,
            los_total: 20,
            total_drums: 0,
        };

        let next = evaluate_route_destination(&current, &variant, &context, None).unwrap();
        assert_eq!(next, 3);
    }

    #[test]
    fn route_rules_use_unique_unconditional_fallback_when_predicate_is_unknown() {
        let current = MapCellDefinition {
            cell_no: 1,
            color_no: 4,
            event_id: 4,
            event_kind: 1,
            next_cells: vec![2, 3],
            node_label: None,
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 3,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::from([(
                1,
                vec![
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 2,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Unknown {
                            raw_text: "ランダム".to_string(),
                        },
                        raw_text: "ランダム".to_string(),
                    },
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 3,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: "それ以外".to_string(),
                    },
                ],
            )]),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };

        let next =
            evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
                .unwrap();
        assert_eq!(next, 3);
    }

    #[test]
    fn weighted_route_selection_uses_weights() {
        let weights = BTreeMap::from([(2, 45_u64), (3, 55_u64)]);
        assert_eq!(select_route_target_for_roll(&weights, 0), Some(2));
        assert_eq!(select_route_target_for_roll(&weights, 44), Some(2));
        assert_eq!(select_route_target_for_roll(&weights, 45), Some(3));
        assert_eq!(select_route_target_for_roll(&weights, 99), Some(3));
    }

    #[test]
    fn selected_route_is_accepted_when_all_rules_are_unknown() {
        let current = MapCellDefinition {
            cell_no: 1,
            color_no: 4,
            event_id: 4,
            event_kind: 1,
            next_cells: vec![2, 3],
            node_label: None,
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 3,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::from([(
                1,
                vec![
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 2,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Unknown {
                            raw_text: "能動分岐".to_string(),
                        },
                        raw_text: "能動分岐".to_string(),
                    },
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 3,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Unknown {
                            raw_text: "能動分岐".to_string(),
                        },
                        raw_text: "能動分岐".to_string(),
                    },
                ],
            )]),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };

        let next =
            evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), Some(3))
                .unwrap();
        assert_eq!(next, 3);
    }

    #[test]
    fn fallback_rule_does_not_compete_with_matching_specific_rule() {
        let current = MapCellDefinition {
            cell_no: 1,
            color_no: 4,
            event_id: 4,
            event_kind: 1,
            next_cells: vec![2, 3],
            node_label: None,
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 3,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::from([(
                1,
                vec![
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 2,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::ContainsShipType {
                            ship_types: vec![13],
                        },
                        raw_text: "潜水艦を含む".to_string(),
                    },
                    RouteRule {
                        from_cell_no: 1,
                        to_cell_no: 3,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: "それ以外".to_string(),
                    },
                ],
            )]),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };
        let context = FleetRouteContext {
            fleet_size: 4,
            visited_cell_ids: BTreeSet::new(),
            ship_ids: BTreeSet::new(),
            flagship_ship_id: None,
            flagship_ship_type: None,
            ship_type_counts: BTreeMap::from([(13, 1)]),
            ship_entries: vec![
                FleetRouteShipEntry {
                    ship_id: 1601,
                    ship_type: 13,
                    speed: 10,
                    slotitem_types: BTreeSet::new(),
                },
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
                FleetRouteShipEntry::default(),
            ],
            min_speed: 10,
            los_total: 20,
            total_drums: 0,
        };

        let next = evaluate_route_destination(&current, &variant, &context, None).unwrap();
        assert_eq!(next, 2);
    }

    #[test]
    fn cell_zero_uses_explicit_start_rules_before_static_next_cells() {
        let current = MapCellDefinition {
            cell_no: 0,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells: vec![1, 2],
            node_label: Some("Start".to_string()),
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 2,
            cells: vec![
                current.clone(),
                MapCellDefinition {
                    cell_no: 1,
                    color_no: 4,
                    event_id: 4,
                    event_kind: 1,
                    next_cells: vec![],
                    node_label: Some("A".to_string()),
                    master_cell_id: None,
                    distance: None,
                },
                MapCellDefinition {
                    cell_no: 2,
                    color_no: 5,
                    event_id: 5,
                    event_kind: 1,
                    next_cells: vec![],
                    node_label: Some("C".to_string()),
                    master_cell_id: None,
                    distance: None,
                },
            ],
            routing_rules: BTreeMap::from([(
                0,
                vec![RouteRule {
                    from_cell_no: 0,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: "出撃".to_string(),
                }],
            )]),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };

        let next =
            evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
                .unwrap();
        assert_eq!(next, 2);
    }

    #[test]
    fn ambiguous_cell_zero_without_rules_is_rejected() {
        let current = MapCellDefinition {
            cell_no: 0,
            color_no: 0,
            event_id: 0,
            event_kind: 0,
            next_cells: vec![1, 2],
            node_label: Some("Start".to_string()),
            master_cell_id: None,
            distance: None,
        };
        let variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 2,
            cells: vec![current.clone()],
            routing_rules: BTreeMap::new(),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: vec!["missing_start_routes".to_string()],
        };

        let error =
            evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
                .unwrap_err();
        assert!(error.to_string().contains("explicit start routing rules"));
    }

    #[tokio::test]
    async fn first_gauge_clear_switches_map_variant_without_finishing_map() {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = (db, codex);
        let account = context.sign_up("variant-switch", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "variant-admin").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let profile_id = session.profile.id;
        let now = Utc::now();
        if let Ok(record) = find_map_record_impl(&context.0, profile_id, 73).await {
            let mut am = record.into_active_model();
            am.cleared = ActiveValue::Set(false);
            am.unlocked = ActiveValue::Set(true);
            am.last_cleared_at = ActiveValue::Set(None);
            am.last_reset_at = ActiveValue::Set(Some(now));
            am.defeat_count = ActiveValue::Set(Some(2));
            am.current_hp = ActiveValue::Set(None);
            am.gauge_index = ActiveValue::Set(1);
            assign_stage_id(&mut am, Some("pre_p_unlock".to_string()));
            am.selected_rank = ActiveValue::Set(map_record::SelectedRank::NotSet);
            am.event_state = ActiveValue::Set(None);
            am.update(&context.0).await.unwrap();
        } else {
            map_record::ActiveModel {
                id: ActiveValue::NotSet,
                profile_id: ActiveValue::Set(profile_id),
                map_id: ActiveValue::Set(73),
                cleared: ActiveValue::Set(false),
                last_cleared_at: ActiveValue::Set(None),
                last_reset_at: ActiveValue::Set(Some(now)),
                defeat_count: ActiveValue::Set(Some(2)),
                current_hp: ActiveValue::Set(None),
                gauge_index: ActiveValue::Set(1),
                stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
                selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
                event_state: ActiveValue::Set(None),
                unlocked: ActiveValue::Set(true),
            }
            .insert(&context.0)
            .await
            .unwrap();
        }

        let definition = context.1.maps.map_definition(73).unwrap().clone();
        assert_eq!(definition.default_variant, "pre_p_unlock");
        assert_eq!(definition.gauge_count, Some(2));
        let variant = definition.variant("pre_p_unlock").unwrap().clone();
        assert_eq!(variant.required_defeat_count, Some(3));
        assert_eq!(variant.clear_to_variant_key.as_deref(), Some("post_p_unlock"));
        let snapshot = successful_boss_snapshot();

        assert_eq!(
            apply_sortie_map_result(&context.0, profile_id, &definition, &variant, true, &snapshot)
                .await
                .unwrap(),
            0
        );

        let record = map_record::Entity::find()
            .filter(map_record::Column::ProfileId.eq(profile_id))
            .filter(map_record::Column::MapId.eq(73))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap();
        assert!(!record.cleared);
        assert_eq!(record.defeat_count, Some(0));
        assert_eq!(record.gauge_index, 2);
        assert_eq!(record.stage_id.as_deref(), Some("post_p_unlock"));
        assert!(record.last_cleared_at.is_none());
    }

    #[tokio::test]
    async fn start_sortie_returns_post_p_unlock_layout_after_first_gauge_clear() {
        let db = new_mem_db().await.unwrap();
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        codex.maps =
            build_final_map_catalog_from_repo_assets("../../.data/temp", &codex.manifest).unwrap();
        let context = (db, codex);
        let account = context.sign_up("variant-layout", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "variant-layout-admin").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let profile_id = session.profile.id;
        let now = Utc::now();
        if let Ok(record) = find_map_record_impl(&context.0, profile_id, 73).await {
            let mut am = record.into_active_model();
            am.cleared = ActiveValue::Set(false);
            am.unlocked = ActiveValue::Set(true);
            am.last_cleared_at = ActiveValue::Set(None);
            am.last_reset_at = ActiveValue::Set(Some(now));
            am.defeat_count = ActiveValue::Set(Some(2));
            am.current_hp = ActiveValue::Set(None);
            am.gauge_index = ActiveValue::Set(1);
            assign_stage_id(&mut am, Some("pre_p_unlock".to_string()));
            am.selected_rank = ActiveValue::Set(map_record::SelectedRank::NotSet);
            am.event_state = ActiveValue::Set(None);
            am.update(&context.0).await.unwrap();
        } else {
            map_record::ActiveModel {
                id: ActiveValue::NotSet,
                profile_id: ActiveValue::Set(profile_id),
                map_id: ActiveValue::Set(73),
                cleared: ActiveValue::Set(false),
                last_cleared_at: ActiveValue::Set(None),
                last_reset_at: ActiveValue::Set(Some(now)),
                defeat_count: ActiveValue::Set(Some(2)),
                current_hp: ActiveValue::Set(None),
                gauge_index: ActiveValue::Set(1),
                stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
                selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
                event_state: ActiveValue::Set(None),
                unlocked: ActiveValue::Set(true),
            }
            .insert(&context.0)
            .await
            .unwrap();
        }

        let definition = context.1.maps.map_definition(73).unwrap().clone();
        let variant = definition.variant("pre_p_unlock").unwrap().clone();
        let snapshot = successful_boss_snapshot();
        apply_sortie_map_result(&context.0, profile_id, &definition, &variant, true, &snapshot)
            .await
            .unwrap();

        let ship = context.add_ship(profile_id, 951).await.unwrap();
        context
            .update_fleet_ships(profile_id, 1, &[ship.api_id, -1, -1, -1, -1, -1])
            .await
            .unwrap();

        let response = context.start_sortie(profile_id, 1, 7, 3, 1).await.unwrap();
        let cell_nos = response.cell_data.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

        assert!(cell_nos.iter().any(|cell_no| *cell_no > 16));
        assert!(cell_nos.contains(&25));
        assert_eq!(response.cell_data.first().map(|cell| cell.cell_no), Some(0));
        assert_eq!(response.cell_data.last().map(|cell| cell.cell_no), Some(25));
    }

    #[tokio::test]
    async fn hp_gauge_clear_advances_to_next_gauge_before_marking_map_cleared() {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = (db, codex);
        let account = context.sign_up("hp-gauge", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "hp-gauge-admin").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let profile_id = session.profile.id;
        let now = Utc::now();
        let definition = MapDefinition {
            map_id: 99011,
            maparea_id: 99,
            mapinfo_no: 11,
            name: "hp gauge".to_string(),
            level: 1,
            sally_flag: vec![],
            is_event: true,
            reset_policy: Default::default(),
            airbase_count: None,
            gauge_type: Some(2),
            gauge_count: Some(2),
            required_defeat_count: None,
            max_hp: Some(1),
            default_variant: String::new(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::from([(
                String::new(),
                MapStageDefinition {
                    variant_key: String::new(),
                    ..Default::default()
                },
            )]),
        };
        let stage = definition.variant("").unwrap().clone();
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(definition.map_id),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(Some(1)),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(None),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(Some(1)),
            unlocked: ActiveValue::Set(true),
        }
        .insert(&context.0)
        .await
        .unwrap();

        assert_eq!(
            apply_sortie_map_result(
                &context.0,
                profile_id,
                &definition,
                &stage,
                true,
                &successful_boss_snapshot(),
            )
            .await
            .unwrap(),
            0
        );

        let record = map_record::Entity::find()
            .filter(map_record::Column::ProfileId.eq(profile_id))
            .filter(map_record::Column::MapId.eq(definition.map_id))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap();
        assert!(!record.cleared);
        assert_eq!(record.current_hp, Some(1));
        assert_eq!(record.gauge_index, 2);
        assert_eq!(record.event_state, Some(1));
        assert!(record.last_cleared_at.is_none());
    }

    #[tokio::test]
    async fn hp_gauge_clear_switches_stage_before_marking_map_cleared() {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = (db, codex);
        let account = context.sign_up("hp-stage", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "hp-stage-admin").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let profile_id = session.profile.id;
        let now = Utc::now();
        let definition = MapDefinition {
            map_id: 99012,
            maparea_id: 99,
            mapinfo_no: 12,
            name: "hp stage".to_string(),
            level: 1,
            sally_flag: vec![],
            is_event: true,
            reset_policy: Default::default(),
            airbase_count: None,
            gauge_type: Some(2),
            gauge_count: Some(2),
            required_defeat_count: None,
            max_hp: Some(1),
            default_variant: "pre".to_string(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::from([
                (
                    "pre".to_string(),
                    MapStageDefinition {
                        variant_key: "pre".to_string(),
                        clear_to_variant_key: Some("post".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "post".to_string(),
                    MapStageDefinition {
                        variant_key: "post".to_string(),
                        ..Default::default()
                    },
                ),
            ]),
        };
        let stage = definition.variant("pre").unwrap().clone();
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(definition.map_id),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(Some(1)),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("pre".to_string())),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(Some(1)),
            unlocked: ActiveValue::Set(true),
        }
        .insert(&context.0)
        .await
        .unwrap();

        assert_eq!(
            apply_sortie_map_result(
                &context.0,
                profile_id,
                &definition,
                &stage,
                true,
                &successful_boss_snapshot(),
            )
            .await
            .unwrap(),
            0
        );

        let record = map_record::Entity::find()
            .filter(map_record::Column::ProfileId.eq(profile_id))
            .filter(map_record::Column::MapId.eq(definition.map_id))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap();
        assert!(!record.cleared);
        assert_eq!(record.current_hp, Some(1));
        assert_eq!(record.gauge_index, 2);
        assert_eq!(record.stage_id.as_deref(), Some("post"));
        assert_eq!(record.event_state, Some(1));
        assert!(record.last_cleared_at.is_none());
    }

    #[tokio::test]
    async fn final_hp_gauge_clear_marks_map_cleared() {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = (db, codex);
        let account = context.sign_up("hp-final", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "hp-final-admin").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let profile_id = session.profile.id;
        let now = Utc::now();
        let definition = MapDefinition {
            map_id: 99013,
            maparea_id: 99,
            mapinfo_no: 13,
            name: "hp final".to_string(),
            level: 1,
            sally_flag: vec![],
            is_event: true,
            reset_policy: Default::default(),
            airbase_count: None,
            gauge_type: Some(2),
            gauge_count: Some(2),
            required_defeat_count: None,
            max_hp: Some(1),
            default_variant: String::new(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::from([(
                String::new(),
                MapStageDefinition {
                    variant_key: String::new(),
                    ..Default::default()
                },
            )]),
        };
        let stage = definition.variant("").unwrap().clone();
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(definition.map_id),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(Some(1)),
            gauge_index: ActiveValue::Set(2),
            stage_id: ActiveValue::Set(None),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(Some(1)),
            unlocked: ActiveValue::Set(true),
        }
        .insert(&context.0)
        .await
        .unwrap();

        assert_eq!(
            apply_sortie_map_result(
                &context.0,
                profile_id,
                &definition,
                &stage,
                true,
                &successful_boss_snapshot(),
            )
            .await
            .unwrap(),
            1
        );

        let record = map_record::Entity::find()
            .filter(map_record::Column::ProfileId.eq(profile_id))
            .filter(map_record::Column::MapId.eq(definition.map_id))
            .one(&context.0)
            .await
            .unwrap()
            .unwrap();
        assert!(record.cleared);
        assert_eq!(record.current_hp, Some(0));
        assert_eq!(record.gauge_index, 2);
        assert_eq!(record.event_state, Some(2));
        assert!(record.last_cleared_at.is_some());
    }

    #[tokio::test]
    async fn clearing_map_1_1_unlocks_dependents_via_cascade() {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = (db, codex);
        let account = context.sign_up("cascade-test", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "cascade-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        let profile_id = session.profile.id;

        let catalog = active_map_catalog(&context.1);
        let deps = catalog.dependents_of(11);
        assert!(!deps.is_empty(), "1-1 should have dependents");

        // Verify dependents start locked
        for &dep_id in &deps {
            let rec = find_map_record_impl(&context.0, profile_id, dep_id).await.unwrap();
            assert!(!rec.unlocked, "dependent {dep_id} should start locked");
        }

        // Simulate Boss win on 1-1 through the actual cascade
        let definition = catalog.as_ref().map_definition(11).unwrap();
        let stage = definition.stage(&String::new()).unwrap();
        let snapshot = successful_boss_snapshot();

        let first_clear = apply_sortie_map_result(
            &context.0, profile_id, definition, stage, true, // boss cell
            &snapshot,
        )
        .await
        .unwrap();
        assert_eq!(first_clear, 1, "first clear should return 1");

        let unlocked = check_and_unlock_dependencies_impl(&context.0, &context.1, profile_id, 11)
            .await
            .unwrap();
        assert!(!unlocked.is_empty(), "should unlock at least one map");

        // Verify dependents are now unlocked
        for &dep_id in &deps {
            let rec = find_map_record_impl(&context.0, profile_id, dep_id).await.unwrap();
            assert!(rec.unlocked, "dependent {dep_id} should be unlocked after clearing 1-1");
        }
    }
}

fn enemy_slot_ids(ship: &BattleShipInput) -> [i64; 5] {
    if ship.ship.api_slot.iter().any(|slot| *slot > 0) {
        let mut slots = [-1; 5];
        for (idx, slot) in ship.ship.api_slot.iter().take(5).enumerate() {
            if *slot > 0 {
                slots[idx] = *slot;
            }
        }
        return slots;
    }
    let mut slots = [-1; 5];
    for (idx, slot_item) in ship.slot_items.iter().take(5).enumerate() {
        slots[idx] = slot_item.api_slotitem_id;
    }
    slots
}

fn engagement_for_cell(map_id: i64, cell_id: i64) -> EngagementType {
    match (map_id + cell_id).rem_euclid(4) {
        1 => EngagementType::HeadOn,
        2 => EngagementType::TAdvantage,
        3 => EngagementType::TDisadvantage,
        _ => EngagementType::SameCourse,
    }
}
