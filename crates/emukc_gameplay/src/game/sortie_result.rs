use emukc_crypto::rng;
use emukc_db::{
    entity::profile::ship,
    sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel},
};
use emukc_model::{
    codex::{
        Codex,
        map::{MapDefinition, MapStageDefinition, MapVariantDefinition},
    },
    kc2::{KcSortieResultRank, level},
    thirdparty::QuestActionEvent,
};
use emukc_time::chrono::Utc;
use serde::Serialize;

use crate::err::GameplayError;

use super::{
    basic::find_profile,
    map::find_map_record_impl,
    map_progress::assign_stage_id,
    ship::{add_ship_impl, update_ship_impl},
    sortie::ActiveSortieState,
};

#[derive(Debug, Clone)]
pub(super) struct SortieBattleResultSnapshot {
    pub(super) friendly_ship_ids: Vec<i64>,
    pub(super) enemy_ship_ids: Vec<i64>,
    pub(super) friendly_nowhps: Vec<i64>,
    pub(super) enemy_ship_types: Vec<i64>,
    pub(super) enemy_nowhps: Vec<i64>,
    pub(super) win_rank: String,
    pub(super) get_exp: i64,
    pub(super) member_lv: i64,
    pub(super) member_exp: i64,
    pub(super) get_base_exp: i64,
    pub(super) mvp: i64,
    pub(super) get_ship_exp: Vec<i64>,
    pub(super) get_exp_lvup: Vec<Vec<i64>>,
    pub(super) quest_name: String,
    pub(super) quest_level: i64,
    pub(super) enemy_level: i64,
    pub(super) enemy_rank: String,
    pub(super) enemy_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieBattleResultEnemyInfo {
    pub api_level: i64,
    pub api_rank: String,
    pub api_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieBattleResultGetShip {
    pub api_ship_id: i64,
    pub api_ship_type: String,
    pub api_ship_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_getmes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SortieBattleResultResponse {
    pub api_ship_id: Vec<i64>,
    pub api_win_rank: String,
    pub api_get_exp: i64,
    pub api_mvp: i64,
    pub api_member_lv: i64,
    pub api_member_exp: i64,
    pub api_get_base_exp: i64,
    pub api_get_ship_exp: Vec<i64>,
    pub api_get_exp_lvup: Vec<Vec<i64>>,
    pub api_dests: i64,
    pub api_destsf: i64,
    pub api_quest_name: String,
    pub api_quest_level: i64,
    pub api_enemy_info: SortieBattleResultEnemyInfo,
    pub api_first_clear: i64,
    pub api_get_flag: [i64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_get_ship: Option<SortieBattleResultGetShip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_next_map_ids: Option<Vec<i64>>,
}

pub(super) fn calculate_sortie_base_exp(map_level: i64, cell_id: i64) -> i64 {
    (map_level.max(1) * 25 + cell_id * 10).clamp(30, 1200)
}

pub(super) fn calculate_battle_admiral_exp(base_exp: i64, win_rank: &str) -> i64 {
    match win_rank {
        "S" => (base_exp as f64 * 0.12).round() as i64,
        "A" => (base_exp as f64 * 0.1).round() as i64,
        "B" => (base_exp as f64 * 0.08).round() as i64,
        "C" => (base_exp as f64 * 0.05).round() as i64,
        _ => (base_exp as f64 * 0.03).round() as i64,
    }
}

pub(super) fn calculate_sortie_ship_exp(
    friend_ships: &[super::battle::core::BattleShipInput],
    base_exp: i64,
    mvp_idx: i64,
    friendly_nowhps: &[i64],
    ct_flagship: bool,
    ct_exp_boost: f64,
) -> (Vec<i64>, Vec<Vec<i64>>) {
    let mut exp = vec![-1];
    let mut lvup = Vec::with_capacity(friend_ships.len());
    let ct_mult = if ct_flagship {
        ct_exp_boost
    } else {
        1.0
    };

    for (idx, ship) in friend_ships.iter().enumerate() {
        // Sunk ships (HP <= 0) or unmarried ships at level 99+ do not receive experience
        let gain = if friendly_nowhps.get(idx).copied().unwrap_or(1) <= 0 {
            0
        } else if !ship.married && ship.ship.api_lv >= 99 {
            0
        } else if idx as i64 + 1 == mvp_idx {
            (base_exp as f64 * 2.0 * ct_mult).floor() as i64
        } else if idx == 0 {
            (base_exp as f64 * 1.5 * ct_mult).floor() as i64
        } else {
            (base_exp as f64 * ct_mult).floor() as i64
        };
        exp.push(gain);

        let new_exp = ship.ship.api_exp[0] + gain;
        lvup.push(build_exp_lvup_vector(ship.ship.api_exp[0], new_exp));
    }

    (exp, lvup)
}

fn build_exp_lvup_vector(before_exp: i64, after_exp: i64) -> Vec<i64> {
    let mut result = vec![before_exp];
    let (_, mut next_exp) = level::exp_to_ship_level(before_exp);
    if next_exp <= 0 {
        result.push(-1);
        return result;
    }
    result.push(next_exp);

    while next_exp > 0 && after_exp >= next_exp {
        let (_, candidate_next) = level::exp_to_ship_level(next_exp);
        if candidate_next <= 0 {
            result.push(-1);
            break;
        }
        if candidate_next == next_exp {
            break;
        }
        result.push(candidate_next);
        next_exp = candidate_next;
    }

    result
}

pub(super) fn build_sortie_quest_event(
    definition: &MapDefinition,
    active: &ActiveSortieState,
    snapshot: &SortieBattleResultSnapshot,
) -> Result<QuestActionEvent, GameplayError> {
    Ok(QuestActionEvent::SortieBattleCompleted {
        maparea_id: definition.maparea_id,
        mapinfo_no: definition.mapinfo_no,
        boss_cell: active.pending_battle_cell_id == Some(active.boss_cell_id),
        win_rank: parse_sortie_result_rank(&snapshot.win_rank)?,
        fleet_id: active.deck_id,
    })
}

fn parse_sortie_result_rank(win_rank: &str) -> Result<KcSortieResultRank, GameplayError> {
    match win_rank {
        "S" => Ok(KcSortieResultRank::S),
        "A" => Ok(KcSortieResultRank::A),
        "B" => Ok(KcSortieResultRank::B),
        "C" => Ok(KcSortieResultRank::C),
        "D" => Ok(KcSortieResultRank::D),
        "E" => Ok(KcSortieResultRank::E),
        _ => Err(GameplayError::WrongType(format!("unexpected sortie result rank `{win_rank}`",))),
    }
}

pub(super) fn eligible_sortie_ship_drops<'a>(
    codex: &Codex,
    variant: &'a MapVariantDefinition,
    cell_no: i64,
    win_rank: &str,
) -> Vec<&'a emukc_model::codex::map::ShipDropDefinition> {
    if !matches!(win_rank, "S" | "A" | "B") {
        return Vec::new();
    }

    variant
        .ship_drops(cell_no)
        .map(|drops| {
            drops
                .iter()
                .filter(|drop| {
                    !drop.tags.iter().any(|tag| tag == "limited")
                        && codex.new_ship(drop.ship_id).is_some()
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) async fn try_grant_sortie_ship_drop<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    variant: &MapVariantDefinition,
    cell_no: i64,
    win_rank: &str,
) -> Result<Option<SortieBattleResultGetShip>, GameplayError>
where
    C: ConnectionTrait,
{
    let candidates = eligible_sortie_ship_drops(codex, variant, cell_no, win_rank);
    if candidates.is_empty() {
        return Ok(None);
    }

    let selected = { candidates[rng::usize(0..candidates.len())] };
    let mst = codex
        .manifest
        .find_ship(selected.ship_id)
        .ok_or(GameplayError::ManifestNotFound(selected.ship_id))?;
    let response = SortieBattleResultGetShip {
        api_ship_id: mst.api_id,
        api_ship_type: codex
            .manifest
            .find_ship_type(mst.api_stype)
            .map(|ship_type| ship_type.api_name.clone())
            .unwrap_or_default(),
        api_ship_name: mst.api_name.clone(),
        api_getmes: mst.api_getmes.clone(),
    };

    match add_ship_impl(c, codex, profile_id, selected.ship_id).await {
        Ok(_) => Ok(Some(response)),
        Err(GameplayError::CapacityExceeded(limit)) => {
            warn!(
                profile_id,
                ship_id = selected.ship_id,
                limit,
                "skipping sortie ship drop because ship capacity is full"
            );
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(super) async fn update_sortie_result_stats<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    mut snapshot: SortieBattleResultSnapshot,
) -> Result<SortieBattleResultSnapshot, GameplayError>
where
    C: ConnectionTrait,
{
    let profile = find_profile(c, profile_id).await?;
    let mut am = profile.into_active_model();
    let current_exp = am.experience.take().unwrap_or_default();
    let new_exp = current_exp + snapshot.get_exp;
    let (hq_level, _) = level::exp_to_hq_level(new_exp);
    if matches!(snapshot.win_rank.as_str(), "S" | "A" | "B") {
        am.sortie_wins = ActiveValue::Set(am.sortie_wins.take().unwrap_or_default() + 1);
    } else {
        am.sortie_loses = ActiveValue::Set(am.sortie_loses.take().unwrap_or_default() + 1);
    }
    am.experience = ActiveValue::Set(new_exp);
    am.hq_level = ActiveValue::Set(hq_level);
    let updated_profile = am.update(c).await?;

    for (idx, ship_id) in snapshot.friendly_ship_ids.iter().copied().enumerate() {
        let ship_model = ship::Entity::find_by_id(ship_id).one(c).await?.ok_or_else(|| {
            GameplayError::EntryNotFound(format!("ship with id {ship_id} not found"))
        })?;
        let mut api_ship: emukc_model::kc2::KcApiShip = ship_model.clone().into();

        // Apply battle damage: update HP from battle result.
        let final_hp = snapshot.friendly_nowhps.get(idx).copied().unwrap_or(1);
        api_ship.api_nowhp = final_hp.max(0);
        let is_sunk = final_hp <= 0;

        // Sunk ships: save HP=0 but skip resource consumption and EXP
        if !is_sunk {
            // Consume fuel and ammo: 20% of max per battle node.
            if let Ok(mst) =
                codex.find::<emukc_model::kc2::start2::ApiMstShip>(&api_ship.api_ship_id)
            {
                let fuel_max = mst.api_fuel_max.unwrap_or(0);
                let ammo_max = mst.api_bull_max.unwrap_or(0);
                let fuel_cost = (fuel_max * 2 / 10).max(1);
                let ammo_cost = (ammo_max * 2 / 10).max(1);
                api_ship.api_fuel = (api_ship.api_fuel - fuel_cost).max(0);
                api_ship.api_bull = (api_ship.api_bull - ammo_cost).max(0);
            }

            // Apply EXP gain.
            let gain = snapshot.get_ship_exp.get(idx + 1).copied().unwrap_or(-1);
            if gain > 0 {
                let new_ship_exp = ship_model.exp_now + gain;
                let (ship_level, next_exp) = level::exp_to_ship_level(new_ship_exp);
                let current_level_exp = level::ship_level_required_exp(ship_level);
                let progress = if next_exp > current_level_exp {
                    ((new_ship_exp - current_level_exp) * 100 / (next_exp - current_level_exp))
                        .clamp(0, 99)
                } else {
                    0
                };
                api_ship.api_lv = ship_level;
                api_ship.api_exp = [new_ship_exp, next_exp, progress];
            }
        }

        update_ship_impl(c, codex, &api_ship).await?;
    }

    snapshot.member_lv = updated_profile.hq_level;
    snapshot.member_exp = updated_profile.experience;
    Ok(snapshot)
}

pub(super) async fn apply_sortie_map_result<C>(
    c: &C,
    profile_id: i64,
    definition: &MapDefinition,
    stage: &MapStageDefinition,
    is_boss_cell: bool,
    snapshot: &SortieBattleResultSnapshot,
) -> Result<i64, GameplayError>
where
    C: ConnectionTrait,
{
    if !is_boss_cell || !matches!(snapshot.win_rank.as_str(), "S" | "A" | "B") {
        return Ok(0);
    }

    let record = find_map_record_impl(c, profile_id, definition.map_id).await?;
    let now = Utc::now();
    let was_cleared = record.cleared;
    let current_hp = record.current_hp;
    let current_gauge_index = record.gauge_index;
    let previous_defeat_count = record.defeat_count.unwrap_or_default();
    let next_gauge_index = current_gauge_index + 1;
    let mut am = record.into_active_model();

    if let Some(max_hp) = definition.max_hp {
        // Shared stage/gauge state stays in place, but event-specific API expansion is not part of
        // the current non-event-map roadmap.
        let next_hp = (current_hp.unwrap_or(max_hp) - 1).max(0);
        let stage_cleared = next_hp <= 0;
        if !stage_cleared {
            am.current_hp = ActiveValue::Set(Some(next_hp));
            am.event_state = ActiveValue::Set(Some(1));
            am.update(c).await?;
            return Ok(0);
        }

        if let Some(next_stage_id) = stage.clear_to_variant_key.clone() {
            assign_stage_id(&mut am, Some(next_stage_id));
            am.gauge_index = ActiveValue::Set(next_gauge_index);
            am.current_hp = ActiveValue::Set(Some(max_hp));
            am.cleared = ActiveValue::Set(false);
            am.last_cleared_at = ActiveValue::Set(None);
            am.event_state = ActiveValue::Set(Some(1));
            am.update(c).await?;
            return Ok(0);
        }

        if definition.gauge_count.unwrap_or(1) > current_gauge_index {
            am.gauge_index = ActiveValue::Set(next_gauge_index);
            am.current_hp = ActiveValue::Set(Some(max_hp));
            am.cleared = ActiveValue::Set(false);
            am.last_cleared_at = ActiveValue::Set(None);
            am.event_state = ActiveValue::Set(Some(1));
            am.update(c).await?;
            return Ok(0);
        }

        am.current_hp = ActiveValue::Set(Some(0));
        am.event_state = ActiveValue::Set(Some(2));
        am.cleared = ActiveValue::Set(true);
        am.last_cleared_at = ActiveValue::Set(Some(now));
        am.update(c).await?;
        return Ok(i64::from(!was_cleared));
    }

    if let Some(required) = stage.required_defeat_count.or(definition.required_defeat_count) {
        let next_defeat = previous_defeat_count + 1;
        let stage_cleared = next_defeat >= required;
        am.defeat_count = ActiveValue::Set(Some(next_defeat.min(required)));
        if stage_cleared && let Some(next_variant_key) = stage.clear_to_variant_key.clone() {
            assign_stage_id(&mut am, Some(next_variant_key));
            am.defeat_count = ActiveValue::Set(Some(0));
            am.gauge_index = ActiveValue::Set(next_gauge_index);
            am.cleared = ActiveValue::Set(false);
            am.last_cleared_at = ActiveValue::Set(None);
            am.update(c).await?;
            return Ok(0);
        }
        if stage_cleared {
            am.cleared = ActiveValue::Set(true);
            am.last_cleared_at = ActiveValue::Set(Some(now));
        }
        am.update(c).await?;
        return Ok(i64::from(!was_cleared && stage_cleared));
    }

    if !was_cleared {
        am.cleared = ActiveValue::Set(true);
        am.last_cleared_at = ActiveValue::Set(Some(now));
        am.update(c).await?;
        return Ok(1);
    }

    am.update(c).await?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use emukc_model::{
        codex::map::{MapDefinition, MapVariantDefinition, ShipDropDefinition},
        thirdparty::QuestActionEvent,
    };

    use super::*;

    fn snapshot(win_rank: &str) -> SortieBattleResultSnapshot {
        SortieBattleResultSnapshot {
            friendly_ship_ids: vec![],
            enemy_ship_ids: vec![],
            friendly_nowhps: vec![],
            enemy_ship_types: vec![],
            enemy_nowhps: vec![],
            win_rank: win_rank.to_string(),
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
    fn build_sortie_quest_event_marks_boss_cells() {
        let definition = MapDefinition {
            maparea_id: 1,
            mapinfo_no: 2,
            ..Default::default()
        };
        let active = ActiveSortieState {
            deck_id: 3,
            map_id: 12,
            map_name: "1-2".to_string(),
            map_level: 1,
            stage_id: String::new(),
            current_cell_id: 3,
            boss_cell_id: 3,
            pending_battle_cell_id: Some(3),
            visited_cell_ids: BTreeSet::new(),
            locked_enemy_composition: None,
        };

        let event = build_sortie_quest_event(&definition, &active, &snapshot("A")).unwrap();

        match event {
            QuestActionEvent::SortieBattleCompleted {
                maparea_id,
                mapinfo_no,
                boss_cell,
                win_rank,
                fleet_id,
            } => {
                assert_eq!(maparea_id, 1);
                assert_eq!(mapinfo_no, 2);
                assert!(boss_cell);
                assert_eq!(win_rank, emukc_model::kc2::KcSortieResultRank::A);
                assert_eq!(fleet_id, 3);
            }
            other => panic!("unexpected quest event: {other:?}"),
        }
    }

    #[test]
    fn eligible_sortie_ship_drops_keep_only_non_limited_known_ships() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let variant = MapVariantDefinition {
            ship_drops: BTreeMap::from([(
                1,
                vec![
                    ShipDropDefinition {
                        ship_id: 1,
                        raw_ship_name: "睦月".to_string(),
                        tags: Vec::new(),
                    },
                    ShipDropDefinition {
                        ship_id: 2,
                        raw_ship_name: "如月".to_string(),
                        tags: vec!["limited".to_string()],
                    },
                    ShipDropDefinition {
                        ship_id: 999_999,
                        raw_ship_name: "unknown".to_string(),
                        tags: Vec::new(),
                    },
                ],
            )]),
            ..Default::default()
        };

        let drops = eligible_sortie_ship_drops(&codex, &variant, 1, "S");

        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].ship_id, 1);
    }
}
