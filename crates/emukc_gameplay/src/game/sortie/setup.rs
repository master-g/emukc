//! Shared pre-battle resolution for the sortie day-start and night-start entries.
//!
//! `sortie_battle_impl` and `Ctx::sortie_sp_midnight_battle` resolve the same
//! state (active sortie, profile, stage, both fleets) and enforce the same guards
//! before they diverge on which simulation to run.

use emukc_battle::{BattleContext, BattleShipInput, BattleType};
use emukc_db::entity::profile;
use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::{codex::Codex, kc2::start2::ApiMstShip};

use crate::{
    err::GameplayError,
    game::{
        basic::find_profile,
        battle::sortie::{SortieBattleInput, SortieBattleSession},
        fleet::get_fleet_ships_impl,
        map::active_map_catalog,
        ship::exp::calculate_admiral_exp,
        sortie_result::{
            SortieBattleResultSnapshot, calculate_sortie_base_exp, calculate_sortie_ship_exp,
        },
        sortie_store::SortieStore,
    },
};

use super::{
    ActiveSortieState,
    enemy_ship::{
        build_sortie_enemy_ships, fallback_enemy_composition, resolve_sortie_enemy_fleet,
        select_random_enemy_composition,
    },
    route_context::{build_sortie_friend_ships, engagement_for_cell},
};

/// Everything a sortie battle entry needs before it picks a simulation.
pub(super) struct SortieBattleSetup {
    pub active: ActiveSortieState,
    pub profile: profile::Model,
    pub friend_ships: Vec<BattleShipInput>,
    pub enemy_ships: Vec<BattleShipInput>,
    pub enemy_formation_id: i64,
    pub enemy_level: i64,
    pub enemy_rank: String,
    pub enemy_deck_name: String,
}

/// Resolve the active sortie into battle-ready fleets, applying every guard both
/// entries share: an active sortie without a pending battle, a single (non-combined)
/// fleet, a battle cell, and a non-empty deck.
pub(super) async fn resolve_sortie_battle_setup_impl<C>(
    c: &C,
    codex: &Codex,
    store: &SortieStore,
    profile_id: i64,
) -> Result<SortieBattleSetup, GameplayError>
where
    C: ConnectionTrait,
{
    let active = store.get_active(profile_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!("active sortie not found for profile {profile_id}",))
    })?;
    if active.pending_battle_cell_id.is_some() {
        return Err(GameplayError::WrongType("sortie battle already pending".to_string()));
    }

    let profile = find_profile(c, profile_id).await?;
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

    let fleet_ships = get_fleet_ships_impl(c, profile_id, active.deck_id).await?;
    if fleet_ships.is_empty() {
        return Err(GameplayError::WrongType(format!(
            "fleet {} has no ships for sortie battle",
            active.deck_id,
        )));
    }

    let friend_ships = build_sortie_friend_ships(c, &fleet_ships).await?;
    let enemy_fleet = resolve_sortie_enemy_fleet(active.map_id, stage, current_cell.cell_no);
    let enemy_composition = active
        .locked_enemy_composition
        .clone()
        .or_else(|| select_random_enemy_composition(&enemy_fleet))
        .unwrap_or_else(|| fallback_enemy_composition(current_cell.cell_no));
    let (enemy_ships, enemy_level, enemy_rank, enemy_deck_name) =
        build_sortie_enemy_ships(codex, definition, &enemy_fleet, &enemy_composition)?;

    Ok(SortieBattleSetup {
        active,
        profile,
        friend_ships,
        enemy_ships,
        enemy_formation_id: enemy_fleet.formations.first().copied().unwrap_or(1),
        enemy_level,
        enemy_rank,
        enemy_deck_name,
    })
}

impl SortieBattleSetup {
    /// The simulation input for this setup; only the battle type and the player's
    /// formation differ between entries.
    pub(super) fn battle_input(
        &self,
        battle_type: BattleType,
        formation_id: i64,
    ) -> SortieBattleInput {
        SortieBattleInput {
            profile_id: self.profile.id,
            deck_id: self.active.deck_id,
            map_id: self.active.map_id,
            cell_id: self.active.current_cell_id,
            context: BattleContext {
                battle_type,
                is_sortie: true,
                friendly_formation_id: formation_id,
                enemy_formation_id: self.enemy_formation_id,
                engagement: engagement_for_cell(self.active.map_id, self.active.current_cell_id),
                friend_ships: self.friend_ships.clone(),
                enemy_ships: self.enemy_ships.clone(),
                combined: None,
            },
        }
    }

    /// The pending result snapshot for a finished session, read later by
    /// `sortie_battle_result`.
    pub(super) fn result_snapshot(
        &self,
        codex: &Codex,
        session: &SortieBattleSession,
    ) -> SortieBattleResultSnapshot {
        let base_exp =
            calculate_sortie_base_exp(self.active.map_level, self.active.current_cell_id);
        let win_rank = session.outcome.win_rank.to_string();
        let friendly_nowhps: Vec<i64> = session.friendly.iter().map(|f| f.hp().max(0)).collect();
        let ct_flagship = self
            .friend_ships
            .first()
            .and_then(|s| codex.manifest.find_ship(s.ship.api_ship_id))
            .is_some_and(|m| m.api_stype == 21);
        let (get_ship_exp, get_exp_lvup) = calculate_sortie_ship_exp(
            &self.friend_ships,
            base_exp,
            session.outcome.mvp,
            &friendly_nowhps,
            ct_flagship,
            codex.game_cfg.exp.ct_exp_boost,
        );
        SortieBattleResultSnapshot {
            friendly_ship_ids: session.friendly_ship_ids.clone(),
            enemy_ship_ids: session.enemy_ship_ids.clone(),
            friendly_nowhps,
            enemy_ship_types: session
                .enemy_ship_ids
                .iter()
                .map(|&id| codex.find::<ApiMstShip>(&id).map(|m| m.api_stype).unwrap_or(0))
                .collect(),
            get_exp: calculate_admiral_exp(base_exp, &win_rank),
            win_rank,
            member_lv: self.profile.hq_level,
            member_exp: self.profile.experience,
            get_base_exp: base_exp,
            mvp: session.outcome.mvp,
            get_ship_exp,
            get_exp_lvup,
            quest_name: self.active.map_name.clone(),
            quest_level: self.active.map_level,
            enemy_level: self.enemy_level,
            enemy_rank: self.enemy_rank.clone(),
            enemy_deck_name: self.enemy_deck_name.clone(),
        }
    }
}
