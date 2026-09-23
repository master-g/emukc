//! Shared pre-battle resolution for the sortie day-start and night-start entries.
//!
//! `sortie_battle_impl` and `Ctx::sortie_sp_midnight_battle` resolve the same
//! state (active sortie, profile, stage, both fleets) and enforce the same guards
//! before they diverge on which simulation to run.

use std::collections::BTreeMap;

use emukc_battle::{
    BattleContext, BattleShipInput, BattleType, CombinedSetup, CombinedType, EngagementType,
    combined_formation_min_escort_size,
};
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
        slot_item::find_slot_items_by_id_impl,
        sortie_result::{
            SortieBattleResultSnapshot, SortieDeckRewards, calculate_sortie_base_exp,
            calculate_sortie_deck_rewards,
        },
        sortie_store::SortieStore,
    },
};

use super::{
    ActiveSortieState,
    enemy_ship::{EnemyEncounter, build_enemy_encounter},
};

/// The escort deck of a combined fleet is always fleet 2
/// (`docs/apilist.txt:3008`: `api_deck_id` is 1 by specification, and the client
/// has no way to nominate a different escort).
const ESCORT_DECK_ID: i64 = 2;

/// 第2艦隊's ships. A profile whose second fleet is not unlocked yet reads as
/// missing rather than empty; both mean the same thing to a combined sortie.
pub(super) async fn escort_fleet_ships_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<Vec<profile::ship::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    match get_fleet_ships_impl(c, profile_id, ESCORT_DECK_ID).await {
        Ok(models) => Ok(models),
        Err(GameplayError::EntryNotFound(_)) => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

/// Which battle endpoint the client called.
///
/// The client picks the URL from its own `api_combined_flag`, so a mismatch
/// means the two sides disagree about the fleet: the packet would be ordered for
/// one shape and rendered as another, silently crediting hits to the wrong
/// ships. Each entry states what it serves and the setup rejects the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SortieBattleEndpoint {
    /// `api_req_sortie/*` and `api_req_battle_midnight/sp_midnight` — a single
    /// fleet.
    Single,
    /// `api_req_combined_battle/battle` — 空母機動部隊 or 輸送護衛部隊.
    Combined,
    /// `api_req_combined_battle/battle_water` — 水上打撃部隊.
    CombinedWater,
    /// `api_req_combined_battle/{airbattle,ld_airbattle,ld_shooting,sp_midnight}`
    /// — any of the three combined types.
    ///
    /// These four have no `_water` twin, unlike the two shelling entries above:
    /// their packets carry no shelling round order for the two sides to disagree
    /// about, so the client sends every combined fleet to the same URL.
    CombinedAnyType,
}

/// Everything a sortie battle entry needs before it picks a simulation.
pub(super) struct SortieBattleSetup {
    pub active: ActiveSortieState,
    pub profile: profile::Model,
    /// 第1艦隊 when `combined_type` is set, otherwise the whole sortie fleet.
    pub friend_ships: Vec<BattleShipInput>,
    /// 第2艦隊; empty for a single fleet.
    pub escort_ships: Vec<BattleShipInput>,
    /// `None` for a single fleet.
    pub combined_type: Option<CombinedType>,
    pub enemy: EnemyEncounter,
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
    let combined_type = if profile.combined_type == 0 {
        None
    } else {
        Some(CombinedType::from_api_id(profile.combined_type).ok_or_else(|| {
            GameplayError::WrongType(format!(
                "unknown combined fleet type {}",
                profile.combined_type,
            ))
        })?)
    };

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

    // 第2艦隊 sorties with 第1艦隊 but is a separate fleet row; an empty one means
    // the player disbanded it without clearing `combined_type`.
    let escort_ships = if combined_type.is_some() {
        let escort_models = escort_fleet_ships_impl(c, profile_id).await?;
        if escort_models.is_empty() {
            return Err(GameplayError::WrongType(
                "combined sortie battle needs ships in fleet 2".to_string(),
            ));
        }
        build_sortie_friend_ships(c, &escort_models).await?
    } else {
        Vec::new()
    };

    let enemy = build_enemy_encounter(
        codex,
        definition,
        stage,
        current_cell.cell_no,
        active.locked_enemy_composition.as_ref(),
    )?;

    Ok(SortieBattleSetup {
        active,
        profile,
        friend_ships,
        escort_ships,
        combined_type,
        enemy,
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
                enemy_formation_id: self.enemy.formation_id,
                engagement: engagement_for_cell(self.active.map_id, self.active.current_cell_id),
                friend_ships: self.friend_ships.clone(),
                enemy_ships: self.enemy.ships.clone(),
                combined: self.combined_type.map(|combined_type| CombinedSetup {
                    combined_type,
                    escort_ships: self.escort_ships.clone(),
                }),
            },
        }
    }

    /// Reject a call whose endpoint does not match the player's fleet shape.
    pub(super) fn validate_endpoint(
        &self,
        endpoint: SortieBattleEndpoint,
    ) -> Result<(), GameplayError> {
        let ok = matches!(
            (endpoint, self.combined_type),
            (SortieBattleEndpoint::Single, None)
                | (
                    SortieBattleEndpoint::Combined,
                    Some(CombinedType::CarrierTaskForce | CombinedType::TransportEscort),
                )
                | (SortieBattleEndpoint::CombinedWater, Some(CombinedType::SurfaceTaskForce))
                | (SortieBattleEndpoint::CombinedAnyType, Some(_))
        );
        if ok {
            return Ok(());
        }
        Err(GameplayError::WrongType(format!(
            "{endpoint:?} does not serve combined fleet type {:?}",
            self.combined_type,
        )))
    }

    /// Reject a 警戒航行序列 the escort deck is too small for (R2).
    ///
    /// Eligibility depends on 第2艦隊 only. Formation ids outside 11..=14 are not
    /// this check's business: a night-start cell enters with one of the six
    /// normal formations even when the fleet is combined.
    pub(super) fn validate_formation(&self, formation_id: i64) -> Result<(), GameplayError> {
        let Some(minimum) = combined_formation_min_escort_size(formation_id) else {
            return Ok(());
        };
        if self.combined_type.is_none() {
            return Err(GameplayError::WrongType(format!(
                "formation {formation_id} is a 警戒航行序列 and needs a combined fleet",
            )));
        }
        if self.escort_ships.len() < minimum {
            return Err(GameplayError::WrongType(format!(
                "formation {formation_id} needs at least {minimum} ships in fleet 2, found {}",
                self.escort_ships.len(),
            )));
        }
        Ok(())
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
        let rewards = calculate_sortie_deck_rewards(
            &session.friendly,
            &friendly_nowhps,
            self.combined_type.map(|_| self.friend_ships.len()),
            base_exp,
            ct_flagship,
            codex.game_cfg.exp.ct_exp_boost,
        );
        let SortieDeckRewards {
            mvp,
            mvp_combined,
            get_ship_exp,
            get_exp_lvup,
            get_ship_exp_combined,
            get_exp_lvup_combined,
        } = rewards;
        SortieBattleResultSnapshot {
            friendly_ship_ids: session.friendly_ship_ids.clone(),
            enemy_ship_ids: session.enemy_ship_ids.clone(),
            friendly_nowhps,
            mvp_combined,
            get_ship_exp_combined,
            get_exp_lvup_combined,
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
            mvp,
            get_ship_exp,
            get_exp_lvup,
            quest_name: self.active.map_name.clone(),
            quest_level: self.active.map_level,
            enemy_level: self.enemy.level,
            enemy_rank: self.enemy.rank.clone(),
            enemy_deck_name: self.enemy.deck_name.clone(),
        }
    }
}

async fn build_sortie_friend_ships<C>(
    c: &C,
    friend_ships: &[profile::ship::Model],
) -> Result<Vec<BattleShipInput>, GameplayError>
where
    C: ConnectionTrait,
{
    let all_slot_ids: Vec<i64> = friend_ships
        .iter()
        .flat_map(|ship| {
            [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
        })
        .filter(|slot_id| *slot_id > 0)
        .collect();

    let all_slot_items = if all_slot_ids.is_empty() {
        BTreeMap::new()
    } else {
        find_slot_items_by_id_impl(c, &all_slot_ids)
            .await?
            .into_iter()
            .map(|item| (item.id, item))
            .collect::<BTreeMap<_, _>>()
    };

    let mut result = Vec::with_capacity(friend_ships.len());
    for ship in friend_ships {
        let slot_items =
            [ship.slot_1, ship.slot_2, ship.slot_3, ship.slot_4, ship.slot_5, ship.slot_ex]
                .into_iter()
                .filter(|slot_id| *slot_id > 0)
                .filter_map(|slot_id| all_slot_items.get(&slot_id).cloned())
                .map(std::convert::Into::into)
                .collect();

        result.push(BattleShipInput {
            ship: (*ship).into(),
            slot_items,
            effect_list: vec![],
            married: ship.married,
        });
    }

    Ok(result)
}

fn engagement_for_cell(map_id: i64, cell_id: i64) -> EngagementType {
    match (map_id + cell_id).rem_euclid(4) {
        1 => EngagementType::HeadOn,
        2 => EngagementType::TAdvantage,
        3 => EngagementType::TDisadvantage,
        _ => EngagementType::SameCourse,
    }
}
