mod enemy_ship;
mod route_context;

use enemy_ship::{
    build_sortie_enemy_ships, fallback_enemy_composition, resolve_sortie_enemy_fleet,
    select_random_enemy_composition,
};
use route_context::{build_fleet_route_context, build_sortie_friend_ships, engagement_for_cell};

use std::collections::BTreeSet;

use async_trait::async_trait;
use emukc_crypto::rng;
use emukc_db::entity::profile::{item::slot_item, ship};
use emukc_db::sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait, entity::prelude::*};
#[cfg(test)]
use emukc_model::codex::map::{RouteOperator, RoutePredicate, RouteRule, SpeedClass};
use emukc_model::{
    codex::{
        Codex,
        map::{EnemyComposition, MapCellDefinition, MapStageDefinition, split_map_id},
    },
    kc2::{MaterialCategory, start2::ApiMstShip},
    thirdparty::QuestActionEvent,
};
use serde::Serialize;

use crate::{err::GameplayError, gameplay::HasContext};

use super::battle::repository::SortieRepository;

#[cfg(test)]
use super::map_progress::assign_stage_id;
#[cfg(test)]
use super::map_route::{route_predicate_matches, select_route_target_for_roll};
#[cfg(test)]
use super::sortie_result::eligible_sortie_ship_drops;
use emukc_battle::{
    BattleContext, BattleNightHougeki, BattleShipInput, BattleType, EngagementType,
};
#[cfg(test)]
use enemy_ship::{build_sortie_enemy_ship, select_enemy_composition_for_roll};

use super::{
    basic::find_profile,
    battle::{
        practice::PracticeBattleResponse,
        sortie::{
            SortieBattleInput, build_day_response, build_night_response, pending_battle,
            run_day_battle, run_night_battle, run_sp_midnight_battle, take_day_battle_result,
        },
    },
    fleet::get_fleet_ships_impl,
    map::{
        active_map_catalog, check_and_unlock_dependencies_impl, ensure_map_records_impl,
        find_map_definition, find_map_record_impl, refresh_all_map_records_impl,
    },
    map_progress::resolve_record_stage_id,
    map_route::{
        cell_has_routing_outgoing, evaluate_route_candidate_count, evaluate_route_destination,
    },
    material::add_material_impl,
    quest::update::update_quest_progress_for_action,
    sortie_result::{
        SortieBattleResultSnapshot, apply_sortie_map_result, build_sortie_quest_event,
        calculate_battle_admiral_exp, calculate_sortie_base_exp, calculate_sortie_ship_exp,
        try_grant_sortie_ship_drop, update_sortie_result_stats,
    },
};

pub use super::sortie_result::{SortieBattleResultEnemyInfo, SortieBattleResultResponse};

pub type SortieBattleResponse = PracticeBattleResponse;

#[derive(Debug, Clone)]
pub struct ActiveSortieState {
    pub deck_id: i64,
    pub map_id: i64,
    pub map_name: String,
    pub map_level: i64,
    pub stage_id: String,
    pub current_cell_id: i64,
    pub boss_cell_id: i64,
    pub pending_battle_cell_id: Option<i64>,
    pub visited_cell_ids: BTreeSet<i64>,
    pub locked_enemy_composition: Option<EnemyComposition>,
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

    /// Clear any stale sortie state for a profile without erroring if none exists.
    async fn clear_sortie_state_if_any(&self, profile_id: i64);
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

        let profile = find_profile(&tx, profile_id).await?;
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
        let source_cell = select_start_source_cell(stage).map_err(|err| {
            GameplayError::EntryNotFound(format!("{} for map {}", err, definition.map_id))
        })?;
        let mut route_context =
            build_fleet_route_context(&tx, codex, &fleet_ships, profile.hq_level).await?;
        route_context.visited_cell_ids.insert(source_cell.cell_no);
        let first_cell = evaluate_route_destination(source_cell, stage, &route_context, None)?;
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
            visited_cell_ids: BTreeSet::from([source_cell.cell_no, first_cell]),
            locked_enemy_composition: locked_enemy_composition.clone(),
        };
        tx.commit().await?;
        self.sortie_store()
            .with_profile_lock(profile_id, async {
                clear_pending_sortie_runtime_state(self.sortie_store(), profile_id);
                let _ = self.sortie_store().insert_active(profile_id, active);
            })
            .await;

        let _ = formation_id;
        let start_candidate_count =
            evaluate_route_candidate_count(source_cell, stage, &route_context);
        Ok(SortieStartResponse {
            cell_data: build_sortie_cell_data(definition.map_id, stage),
            rashin_flg: start_candidate_count > 1,
            rashin_id: if start_candidate_count > 1 {
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
            has_next: cell_has_routing_outgoing(current_cell.cell_no, stage),
            boss_cell_no: stage.boss_cell_no,
            bosscomp: sortie_bosscomp(stage),
            from_cell_no: source_cell.cell_no,
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
        self.sortie_store()
            .with_profile_lock(profile_id, async {
                let codex = self.codex();
                let db = self.db();
                let store = self.sortie_store();
                let mut active = store.get_active(profile_id).ok_or_else(|| {
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
                let definition =
                    catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
                        GameplayError::EntryNotFound(format!(
                            "map definition {} not found",
                            active.map_id
                        ))
                    })?;

                // Defense-in-depth: refresh stage from DB in case sortie_battle_result
                // missed the update after a gauge-clear transition.
                let stage_refreshed =
                    refresh_sortie_stage(db, codex, profile_id, &mut active).await?;
                if !stage_refreshed {
                    store.remove_active(profile_id);
                    return Err(GameplayError::WrongType(format!(
                        "cell {} no longer exists in refreshed stage for map {}",
                        active.current_cell_id, active.map_id,
                    )));
                }
                let _ = store.insert_active(profile_id, active.clone());

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
                if !cell_has_routing_outgoing(active.current_cell_id, stage) {
                    return Err(GameplayError::WrongType(format!(
                        "cell {} has no next route",
                        current.cell_no,
                    )));
                }

                let tx = db.begin().await?;
                let fleet_ships = get_fleet_ships_impl(&tx, profile_id, active.deck_id).await?;
                let hq_level = find_profile(&tx, profile_id).await?.hq_level;
                let mut route_context =
                    build_fleet_route_context(&tx, codex, &fleet_ships, hq_level).await?;
                tx.commit().await?;
                route_context.visited_cell_ids = active.visited_cell_ids.clone();

                let next_cell_id =
                    evaluate_route_destination(current, stage, &route_context, selected_cell_id)?;
                let next = stage.cell(next_cell_id).ok_or_else(|| {
                    GameplayError::EntryNotFound(format!("cell {next_cell_id} not found"))
                })?;
                let locked_enemy_composition =
                    select_locked_enemy_composition(active.map_id, stage, next.cell_no);

                if let Some(mut state) = store.get_active(profile_id) {
                    state.current_cell_id = next_cell_id;
                    state.visited_cell_ids.insert(next_cell_id);
                    state.locked_enemy_composition = locked_enemy_composition.clone();
                    let _ = store.insert_active(profile_id, state);
                }

                // Resolve non-battle node effects (resource gain / maelstrom loss).
                let tx = db.begin().await?;
                let (itemget, happening) =
                    resolve_non_battle_node_effect(&tx, codex, profile_id, next, &fleet_ships)
                        .await?;
                tx.commit().await?;

                let (maparea_id, mapinfo_no) = split_map_id(active.map_id);
                let next_candidate_count =
                    evaluate_route_candidate_count(current, stage, &route_context);
                Ok(SortieNextResponse {
                    rashin_flg: next_candidate_count > 1,
                    rashin_id: if next_candidate_count > 1 {
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
                    has_next: cell_has_routing_outgoing(next.cell_no, stage),
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
            })
            .await
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
        let session = take_day_battle_result(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        let mut active = store.get_active(profile_id).ok_or_else(|| {
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
            if hp <= 0
                && let Some(&stype) = snapshot.enemy_ship_types.get(i)
            {
                let sink_event = QuestActionEvent::EnemyShipSunk {
                    ship_stype: stype,
                };
                update_quest_progress_for_action(&tx, codex, profile_id, &sink_event).await?;
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

        // Refresh stage identity from DB before deciding sortie fate.
        // apply_sortie_map_result may have changed stage_id via gauge clear.
        // Serialize the in-memory state mutation to prevent TOCTOU races.
        self.sortie_store()
            .with_profile_lock(profile_id, async {
                let store = self.sortie_store();
                let stage_refreshed = refresh_sortie_stage(db, codex, profile_id, &mut active).await?;
                if !stage_refreshed {
                    tracing::debug!(
                        "active sortie removed: stage no longer contains current cell after gauge clear"
                    );
                    store.remove_active(profile_id);
                    return Ok(SortieBattleResultResponse {
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
                        api_destsf: i64::from(
                            session.packet.enemy_nowhps.first().copied().unwrap_or(1) <= 0,
                        ),
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
                    });
                }
                let stage = definition.stage(&active.stage_id).ok_or_else(|| {
                    GameplayError::EntryNotFound(format!(
                        "stage `{}` not found for map {}",
                        active.stage_id, active.map_id,
                    ))
                })?;
                let current_cell = stage.cell(pending_cell_id).ok_or_else(|| {
                    GameplayError::EntryNotFound(format!("cell {pending_cell_id} not found"))
                })?;

                let should_finish_sortie = current_cell.cell_no == active.boss_cell_id
                    || !cell_has_routing_outgoing(current_cell.cell_no, stage);
                if should_finish_sortie {
                    store.remove_active(profile_id);
                } else {
                    active.pending_battle_cell_id = None;
                    let _ = store.insert_active(profile_id, active);
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
                    api_destsf: i64::from(
                        session.packet.enemy_nowhps.first().copied().unwrap_or(1) <= 0,
                    ),
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
            })
            .await
    }

    async fn sortie_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<SortieNightBattleResponse, GameplayError> {
        let codex = self.codex();
        let store = self.sortie_store();
        let pending = pending_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        if !pending.outcome.can_midnight {
            return Err(GameplayError::WrongType(
                "night battle is not available for this sortie battle".to_string(),
            ));
        }

        let night = run_night_battle(
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

        let ct_flagship = pending_battle(store, profile_id)
            .and_then(|s| s.friendly.first().map(|f| f.ship.api_ship_id))
            .and_then(|sid| codex.manifest.find_ship(sid))
            .is_some_and(|m| m.api_stype == 21);

        if let Some(mut snapshot) = store.take_pending_result(profile_id) {
            snapshot.win_rank = night.outcome.win_rank.to_string();
            snapshot.mvp = night.outcome.mvp;
            snapshot.get_exp =
                calculate_battle_admiral_exp(snapshot.get_base_exp, &snapshot.win_rank);
            if let Some(updated) = pending_battle(store, profile_id) {
                snapshot.friendly_nowhps = updated.friendly.iter().map(|f| f.hp().max(0)).collect();
                let friend_ships = updated
                    .friendly
                    .iter()
                    .cloned()
                    .map(|ship| BattleShipInput {
                        ship: ship.ship,
                        slot_items: ship.slot_items,
                        effect_list: ship.effect_list,
                        married: ship.married,
                    })
                    .collect::<Vec<_>>();
                let (ship_exp, ship_lvup) = calculate_sortie_ship_exp(
                    &friend_ships,
                    snapshot.get_base_exp,
                    snapshot.mvp,
                    &snapshot.friendly_nowhps,
                    ct_flagship,
                    codex.game_cfg.exp.ct_exp_boost,
                );
                snapshot.get_ship_exp = ship_exp;
                snapshot.get_exp_lvup = ship_lvup;
            }
            store.insert_pending_result(profile_id, snapshot);
        }

        let current = pending_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        Ok(build_night_response(current.deck_id, &current, night.packet))
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

        let mut active = store.get_active(profile_id).ok_or_else(|| {
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
        let (day_session, night_session) = run_sp_midnight_battle(
            store,
            codex,
            SortieBattleInput {
                profile_id,
                deck_id: active.deck_id,
                map_id: active.map_id,
                cell_id: active.current_cell_id,
                context: BattleContext {
                    battle_type: BattleType::Normal,
                    is_sortie: true,
                    friendly_formation_id: formation_id,
                    enemy_formation_id,
                    engagement: engagement_for_cell(active.map_id, active.current_cell_id),
                    friend_ships: friend_ships.clone(),
                    enemy_ships: enemy_ships.clone(),
                },
            },
            enemy_formation_id,
        );

        let base_exp = calculate_sortie_base_exp(active.map_level, active.current_cell_id);
        let get_exp =
            calculate_battle_admiral_exp(base_exp, &night_session.outcome.win_rank.to_string());
        let friendly_nowhps: Vec<i64> = pending_battle(store, profile_id)
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
            codex.game_cfg.exp.ct_exp_boost,
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
                win_rank: night_session.outcome.win_rank.to_string(),
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

        let current = pending_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;

        tx.commit().await?;
        let _ = store.insert_active(profile_id, active);
        Ok(build_night_response(current.deck_id, &current, night_session.packet))
    }

    async fn sortie_goback_port(
        &self,
        profile_id: i64,
    ) -> Result<SortieGobackPortResponse, GameplayError> {
        let store = self.sortie_store();
        let removed = store.remove_active(profile_id);
        if removed.is_none() {
            return Err(GameplayError::EntryNotFound(format!(
                "active sortie not found for profile {profile_id}",
            )));
        }

        clear_pending_sortie_runtime_state(store, profile_id);

        Ok(SortieGobackPortResponse::default())
    }

    async fn clear_sortie_state_if_any(&self, profile_id: i64) {
        let store = self.sortie_store();
        clear_pending_sortie_runtime_state(store, profile_id);
    }
}

async fn refresh_sortie_stage(
    db: &emukc_db::sea_orm::DatabaseConnection,
    codex: &Codex,
    profile_id: i64,
    active: &mut ActiveSortieState,
) -> Result<bool, GameplayError> {
    let catalog = active_map_catalog(codex);
    let definition = catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
    })?;
    let record = find_map_record_impl(db, profile_id, active.map_id).await?;

    let new_stage_id = resolve_record_stage_id(definition, &record).unwrap_or_default();

    if new_stage_id != active.stage_id {
        let new_stage = definition.stage(&new_stage_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "stage `{new_stage_id}` not found for map {}",
                active.map_id,
            ))
        })?;
        if new_stage.cell(active.current_cell_id).is_none() {
            return Ok(false);
        }
        active.stage_id = new_stage_id;
        active.boss_cell_id = new_stage.boss_cell_no;
    }
    Ok(true)
}

async fn sortie_battle_impl(
    store: &crate::game::sortie_store::SortieStore,
    codex: &Codex,
    db: &emukc_db::sea_orm::DatabaseConnection,
    profile_id: i64,
    formation_id: i64,
    battle_type: BattleType,
) -> Result<SortieBattleResponse, GameplayError> {
    store
        .with_profile_lock(profile_id, async {
            let tx = db.begin().await?;

            let mut active = store.get_active(profile_id).ok_or_else(|| {
                GameplayError::EntryNotFound(format!(
                    "active sortie not found for profile {profile_id}",
                ))
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
            let enemy_fleet =
                resolve_sortie_enemy_fleet(active.map_id, stage, current_cell.cell_no);
            let enemy_composition = active
                .locked_enemy_composition
                .clone()
                .or_else(|| select_random_enemy_composition(&enemy_fleet))
                .unwrap_or_else(|| fallback_enemy_composition(current_cell.cell_no));
            let (enemy_ships, enemy_level, enemy_rank, enemy_deck_name) =
                build_sortie_enemy_ships(codex, definition, &enemy_fleet, &enemy_composition)?;

            let session = run_day_battle(
                store,
                codex,
                SortieBattleInput {
                    profile_id,
                    deck_id: active.deck_id,
                    map_id: active.map_id,
                    cell_id: active.current_cell_id,
                    context: BattleContext {
                        battle_type,
                        is_sortie: true,
                        friendly_formation_id: formation_id,
                        enemy_formation_id: enemy_fleet.formations.first().copied().unwrap_or(1),
                        engagement: engagement_for_cell(active.map_id, active.current_cell_id),
                        friend_ships: friend_ships.clone(),
                        enemy_ships: enemy_ships.clone(),
                    },
                },
            );

            let base_exp = calculate_sortie_base_exp(active.map_level, active.current_cell_id);
            let get_exp =
                calculate_battle_admiral_exp(base_exp, &session.outcome.win_rank.to_string());
            let friendly_nowhps: Vec<i64> =
                session.friendly.iter().map(|f| f.hp().max(0)).collect();
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
                codex.game_cfg.exp.ct_exp_boost,
            );
            let response = build_day_response(
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
                    win_rank: session.outcome.win_rank.to_string(),
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

            tx.commit().await?;
            let _ = store.insert_active(profile_id, active);
            Ok(response)
        })
        .await
}

fn build_sortie_cell_data(map_id: i64, stage: &MapStageDefinition) -> Vec<SortieCellData> {
    stage
        .cells
        .iter()
        .map(|cell| SortieCellData {
            master_cell_id: cell.master_cell_id.unwrap_or(map_id * 100 + cell.cell_no),
            cell_no: cell.cell_no,
            color_no: cell.color_no,
            passed: false,
            distance: cell.distance,
        })
        .collect()
}

fn start_source_cells(stage: &MapStageDefinition) -> Vec<&MapCellDefinition> {
    let incoming = stage
        .cells
        .iter()
        .flat_map(|cell| cell.next_cells.iter().copied())
        .collect::<BTreeSet<_>>();
    let roots = stage
        .cells
        .iter()
        .filter(|cell| {
            !incoming.contains(&cell.cell_no) && cell_has_routing_outgoing(cell.cell_no, stage)
        })
        .collect::<Vec<_>>();
    if !roots.is_empty() {
        return roots;
    }

    stage
        .cell(0)
        .filter(|cell| cell_has_routing_outgoing(cell.cell_no, stage))
        .into_iter()
        .collect()
}

fn select_start_source_cell(stage: &MapStageDefinition) -> Result<&MapCellDefinition, String> {
    let sources = start_source_cells(stage);
    match sources.as_slice() {
        [] => Err("start source cell not found".to_string()),
        [only] => Ok(*only),
        many => Ok(many[rng::usize(0..many.len())]),
    }
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
            // Current assets infer the drained resource from the node appearance.
            // Capture-backed calibration can tighten this later if needed.
            let resource_type = if cell.color_no == 4 {
                2
            } else {
                1
            }; // purple=ammo, else=fuel
            let slot_ids: Vec<i64> = fleet_ships
                .iter()
                .flat_map(|s| [s.slot_1, s.slot_2, s.slot_3, s.slot_4, s.slot_5])
                .filter(|&id| id > 0)
                .collect();
            let radar_item_ids = if slot_ids.is_empty() {
                std::collections::BTreeSet::new()
            } else {
                slot_item::Entity::find()
                    .filter(slot_item::Column::Id.is_in(slot_ids))
                    .filter(slot_item::Column::Type3.is_in([12_i64, 13, 93]))
                    .all(c)
                    .await?
                    .into_iter()
                    .map(|item| item.id)
                    .collect::<std::collections::BTreeSet<_>>()
            };
            let radar_ship_count = fleet_ships
                .iter()
                .filter(|ship| {
                    [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5]
                        .into_iter()
                        .filter(|slot_id| *slot_id > 0)
                        .any(|slot_id| radar_item_ids.contains(&slot_id))
                })
                .count();
            let radar_reduction = match radar_ship_count {
                0 => 0.0,
                1 => 0.25,
                2 => 0.40,
                3 => 0.50,
                4 => 0.55,
                5 => 0.58,
                _ => 0.60,
            };
            let mut total_loss = 0;
            for ship_model in fleet_ships {
                let stock = if resource_type == 1 {
                    ship_model.fuel
                } else {
                    ship_model.ammo
                };
                let ship_loss = ((stock as f64) * 0.30 * (1.0 - radar_reduction)).floor() as i64;
                if ship_loss <= 0 {
                    continue;
                }

                let mut am = ship_model.clone().into_active_model();
                if resource_type == 1 {
                    am.fuel = ActiveValue::Set((ship_model.fuel - ship_loss).max(0));
                } else {
                    am.ammo = ActiveValue::Set((ship_model.ammo - ship_loss).max(0));
                }
                am.update(c).await?;
                total_loss += ship_loss;
            }
            Ok((
                None,
                Some(SortieHappening {
                    resource_type,
                    amount: total_loss,
                    radar_reduced: radar_ship_count > 0,
                }),
            ))
        }
        _ => {
            // event_id 0 (start), 1 (nothing), 6 (imaginary) — no effect
            Ok((None, None))
        }
    }
}

fn clear_pending_sortie_runtime_state(store: &dyn SortieRepository, profile_id: i64) {
    store.remove_active(profile_id);
    store.take_pending_result(profile_id);
    let _ = take_day_battle_result(store, profile_id);
}

#[cfg(test)]
#[path = "../sortie_tests.rs"]
mod tests;
