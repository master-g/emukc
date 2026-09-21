mod enemy_ship;
mod route_context;
mod setup;

use enemy_ship::{
    fallback_enemy_composition, resolve_sortie_enemy_fleet, select_random_enemy_composition,
};
use route_context::build_fleet_route_context;
use setup::{SortieBattleEndpoint, resolve_sortie_battle_setup_impl};

use std::collections::BTreeSet;

use emukc_crypto::rng;
use emukc_db::entity::profile::{item::slot_item, ship};
use emukc_db::sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait, entity::prelude::*};
use emukc_model::{
    codex::{
        Codex,
        map::{EnemyComposition, MapCellDefinition, MapStageDefinition, split_map_id},
    },
    kc2::MaterialCategory,
};
use serde::Serialize;

use crate::{err::GameplayError, gameplay::Ctx};

use emukc_battle::{BattleType, EngagementType};

use super::{
    basic::find_profile,
    battle::{
        response::{
            DayBattleResponse, NightBattleResponse, build_day_response, build_night_response,
        },
        rng::ProductionRng,
        sortie::{
            escort_deck_start, pending_battle, run_day_battle, run_night_battle,
            run_sp_midnight_battle, take_day_battle_result,
        },
    },
    fleet::get_fleet_ships_impl,
    map::{
        active_map_catalog, ensure_map_records_impl, find_map_definition, find_map_record_impl,
        refresh_all_map_records_impl,
    },
    map_progress::resolve_record_stage_id,
    map_route::{cell_has_routing_outgoing, evaluate_route_destination},
    material::add_material_impl,
    quest::observe::observe,
    ship::exp::calculate_admiral_exp,
    sortie_result::{calculate_sortie_deck_rewards, settle_sortie_battle_impl},
    sortie_store::SortieStore,
};

pub use super::sortie_result::SortieBattleResultResponse;

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

#[derive(Debug, Clone, Default, Serialize)]
pub struct SortieGobackPortResponse {}

impl Ctx {
    pub async fn start_sortie(
        &self,
        profile_id: i64,
        deck_id: i64,
        maparea_id: i64,
        mapinfo_no: i64,
    ) -> Result<SortieStartResponse, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
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
        self.sortie_store
            .as_ref()
            .with_profile_lock(profile_id, async {
                clear_pending_sortie_runtime_state(self.sortie_store.as_ref(), profile_id);
                let _ = self.sortie_store.as_ref().insert_active(profile_id, active);
            })
            .await;

        // rashin_flg keys on whether the departing cell is a physical branch node
        // (out-degree > 1), not the fleet-resolved candidate count. See
        // docs/solutions/architecture-patterns/sortie-compass-rashin-flag.md.
        let departing_is_branch = source_cell.next_cells.len() > 1;
        Ok(SortieStartResponse {
            cell_data: build_sortie_cell_data(definition.map_id, stage),
            rashin_flg: departing_is_branch,
            rashin_id: if departing_is_branch {
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

    pub async fn next_sortie(
        &self,
        profile_id: i64,
        selected_cell_id: Option<i64>,
    ) -> Result<SortieNextResponse, GameplayError> {
        self.sortie_store
            .as_ref()
            .with_profile_lock(profile_id, async {
                let codex = self.codex.as_ref();
                let db = self.db.as_ref();
                let store = self.sortie_store.as_ref();
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
                // rashin_flg keys on the departing cell's physical out-degree
                // (branch node), not the fleet-resolved candidate count.
                let departing_is_branch = current.next_cells.len() > 1;
                Ok(SortieNextResponse {
                    rashin_flg: departing_is_branch,
                    rashin_id: if departing_is_branch {
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

    pub async fn sortie_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store.as_ref(),
            self.codex.as_ref(),
            self.db.as_ref(),
            profile_id,
            formation_id,
            BattleType::Normal,
            SortieBattleEndpoint::Single,
        )
        .await
    }

    pub async fn sortie_airbattle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store.as_ref(),
            self.codex.as_ref(),
            self.db.as_ref(),
            profile_id,
            formation_id,
            BattleType::AirBattle,
            SortieBattleEndpoint::Single,
        )
        .await
    }

    pub async fn sortie_ld_airbattle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store.as_ref(),
            self.codex.as_ref(),
            self.db.as_ref(),
            profile_id,
            formation_id,
            BattleType::LdAirBattle,
            SortieBattleEndpoint::Single,
        )
        .await
    }

    pub async fn sortie_ld_shooting(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store.as_ref(),
            self.codex.as_ref(),
            self.db.as_ref(),
            profile_id,
            formation_id,
            BattleType::LdShooting,
            SortieBattleEndpoint::Single,
        )
        .await
    }

    /// `api_req_combined_battle/battle` — 空母機動部隊 or 輸送護衛部隊.
    pub async fn sortie_combined_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store.as_ref(),
            self.codex.as_ref(),
            self.db.as_ref(),
            profile_id,
            formation_id,
            BattleType::Normal,
            SortieBattleEndpoint::Combined,
        )
        .await
    }

    /// `api_req_combined_battle/battle_water` — 水上打撃部隊.
    ///
    /// Same simulation as [`sortie_combined_battle`](Self::sortie_combined_battle);
    /// the shelling order comes from the player's combined type, and the two
    /// endpoints exist so the client can render the one it expects.
    pub async fn sortie_combined_battle_water(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<DayBattleResponse, GameplayError> {
        sortie_battle_impl(
            self.sortie_store.as_ref(),
            self.codex.as_ref(),
            self.db.as_ref(),
            profile_id,
            formation_id,
            BattleType::Normal,
            SortieBattleEndpoint::CombinedWater,
        )
        .await
    }

    pub async fn sortie_battle_result(
        &self,
        profile_id: i64,
    ) -> Result<SortieBattleResultResponse, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
        let store = self.sortie_store.as_ref();
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
        let catalog = active_map_catalog(codex);
        let definition = catalog.as_ref().map_definition(active.map_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!("map definition {} not found", active.map_id))
        })?;

        let settlement = settle_sortie_battle_impl(
            &tx,
            codex,
            profile_id,
            definition,
            &active,
            snapshot,
            &session.packet.enemy_nowhps,
        )
        .await?;

        observe(&tx, codex, profile_id, &settlement.outcomes).await?;

        tx.commit().await?;

        // Refresh stage identity from DB before deciding sortie fate.
        // apply_sortie_map_result may have changed stage_id via gauge clear.
        // Serialize the in-memory state mutation to prevent TOCTOU races.
        store
            .with_profile_lock(profile_id, async {
                let stage_refreshed = refresh_sortie_stage(db, codex, profile_id, &mut active).await?;
                if !stage_refreshed {
                    tracing::debug!(
                        "active sortie removed: stage no longer contains current cell after gauge clear"
                    );
                    store.remove_active(profile_id);
                } else {
                    let stage = definition.stage(&active.stage_id).ok_or_else(|| {
                        GameplayError::EntryNotFound(format!(
                            "stage `{}` not found for map {}",
                            active.stage_id, active.map_id,
                        ))
                    })?;
                    let current_cell = stage.cell(settlement.cell_no).ok_or_else(|| {
                        GameplayError::EntryNotFound(format!("cell {} not found", settlement.cell_no))
                    })?;

                    let should_finish_sortie = stage
                        .boss_cell_nos()
                        .contains(&current_cell.cell_no)
                        || !cell_has_routing_outgoing(current_cell.cell_no, stage);
                    if should_finish_sortie {
                        store.remove_active(profile_id);
                    } else {
                        active.pending_battle_cell_id = None;
                        let _ = store.insert_active(profile_id, active);
                    }
                }

                Ok(settlement.into())
            })
            .await
    }

    pub async fn sortie_midnight_battle(
        &self,
        profile_id: i64,
    ) -> Result<NightBattleResponse, GameplayError> {
        let codex = self.codex.as_ref();
        let store = self.sortie_store.as_ref();
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

        let mut rng = ProductionRng;
        let night = run_night_battle(
            store,
            codex,
            profile_id,
            pending.packet.formation[0],
            pending.packet.formation[1],
            EngagementType::from_api_id(pending.packet.formation[2])
                .unwrap_or(EngagementType::SameCourse),
            &mut rng,
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
            snapshot.get_exp = calculate_admiral_exp(snapshot.get_base_exp, &snapshot.win_rank);
            if let Some(updated) = pending_battle(store, profile_id) {
                snapshot.friendly_nowhps = updated.friendly.iter().map(|f| f.hp().max(0)).collect();
                // Rescored over both decks: a combined night battle only moved
                // 第2艦隊's HP and damage, but 第1艦隊's share of the node is
                // still owed and its MVP still has to come from 第1艦隊 alone.
                let escort_start = escort_deck_start(&updated.friendly);
                let rewards = calculate_sortie_deck_rewards(
                    &updated.friendly,
                    &snapshot.friendly_nowhps,
                    (escort_start > 0).then_some(escort_start),
                    snapshot.get_base_exp,
                    ct_flagship,
                    codex.game_cfg.exp.ct_exp_boost,
                );
                snapshot.mvp = rewards.mvp;
                snapshot.mvp_combined = rewards.mvp_combined;
                snapshot.get_ship_exp = rewards.get_ship_exp;
                snapshot.get_exp_lvup = rewards.get_exp_lvup;
                snapshot.get_ship_exp_combined = rewards.get_ship_exp_combined;
                snapshot.get_exp_lvup_combined = rewards.get_exp_lvup_combined;
            }
            store.insert_pending_result(profile_id, snapshot);
        }

        let current = pending_battle(store, profile_id).ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "sortie battle session not found for profile {profile_id}",
            ))
        })?;
        // Only 第2艦隊 fought, so the packet's friendly arrays are its alone.
        let escort_start = escort_deck_start(&current.friendly);
        let response = build_night_response(
            current.deck_id,
            &current.friendly[escort_start..],
            &current.enemy,
            night.packet,
        );
        Ok(if escort_start == 0 {
            response
        } else {
            response.with_main_deck(&current.friendly[..escort_start])
        })
    }

    pub async fn sortie_sp_midnight_battle(
        &self,
        profile_id: i64,
        formation_id: i64,
    ) -> Result<NightBattleResponse, GameplayError> {
        let codex = self.codex.as_ref();
        let db = self.db.as_ref();
        let store = self.sortie_store.as_ref();

        // Same write set as `sortie_battle_impl`, so it takes the same lock.
        store
            .with_profile_lock(profile_id, async {
                let tx = db.begin().await?;

                // A combined night-start cell is `api_req_combined_battle/sp_midnight`,
                // which this build does not serve; running the single-fleet path
                // would silently drop 第2艦隊 from the battle and the result. The
                // check reads the profile directly so it does not also depend on
                // whether fleet 2 is in a sortie-ready state.
                if find_profile(&tx, profile_id).await?.combined_type > 0 {
                    return Err(GameplayError::WrongType(
                        "combined night-start battle is not implemented".to_string(),
                    ));
                }
                let setup = resolve_sortie_battle_setup_impl(&tx, codex, store, profile_id).await?;
                let mut rng = ProductionRng;
                let (session, night_session) = run_sp_midnight_battle(
                    store,
                    codex,
                    setup.battle_input(BattleType::Normal, formation_id),
                    &mut rng,
                );
                store.insert_pending_result(profile_id, setup.result_snapshot(codex, &session));

                let mut active = setup.active;
                active.pending_battle_cell_id = Some(active.current_cell_id);

                tx.commit().await?;
                let _ = store.insert_active(profile_id, active);
                Ok(build_night_response(
                    session.deck_id,
                    &session.friendly,
                    &session.enemy,
                    night_session.packet,
                ))
            })
            .await
    }

    pub async fn sortie_goback_port(
        &self,
        profile_id: i64,
    ) -> Result<SortieGobackPortResponse, GameplayError> {
        let store = self.sortie_store.as_ref();
        let removed = store.remove_active(profile_id);
        if removed.is_none() {
            return Err(GameplayError::EntryNotFound(format!(
                "active sortie not found for profile {profile_id}",
            )));
        }

        clear_pending_sortie_runtime_state(store, profile_id);

        Ok(SortieGobackPortResponse::default())
    }

    /// Clear any stale sortie state for a profile without erroring if none exists.
    pub async fn clear_sortie_state_if_any(&self, profile_id: i64) {
        let store = self.sortie_store.as_ref();
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
    endpoint: SortieBattleEndpoint,
) -> Result<DayBattleResponse, GameplayError> {
    store
        .with_profile_lock(profile_id, async {
            let tx = db.begin().await?;

            let setup = resolve_sortie_battle_setup_impl(&tx, codex, store, profile_id).await?;
            setup.validate_endpoint(endpoint)?;
            setup.validate_formation(formation_id)?;
            let mut rng = ProductionRng;
            let session = run_day_battle(
                store,
                codex,
                setup.battle_input(battle_type, formation_id),
                &mut rng,
            );
            let mut response = build_day_response(
                setup.active.deck_id,
                &setup.friend_ships,
                &setup.enemy_ships,
                session.packet.clone(),
            );
            if setup.combined_type.is_some() {
                response = response.with_escort_deck(&setup.escort_ships);
            }
            store.insert_pending_result(profile_id, setup.result_snapshot(codex, &session));

            let mut active = setup.active;
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
    stage.start_source_cells()
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
///
/// The `c` parameter must be a transaction connection when the maelstrom branch (`event_id` 3)
/// is reachable. Per-ship resource deductions are applied individually; a non-transaction
/// connection risks partial state on failure.
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

                let mut am = ship_model.into_active_model();
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

fn clear_pending_sortie_runtime_state(store: &SortieStore, profile_id: i64) {
    store.remove_active(profile_id);
    store.take_pending_result(profile_id);
    let _ = take_day_battle_result(store, profile_id);
}

#[cfg(test)]
mod tests;
