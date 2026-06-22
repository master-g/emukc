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
pub struct SortieBattleResultSnapshot {
    pub friendly_ship_ids: Vec<i64>,
    pub enemy_ship_ids: Vec<i64>,
    pub friendly_nowhps: Vec<i64>,
    pub enemy_ship_types: Vec<i64>,
    pub enemy_nowhps: Vec<i64>,
    pub win_rank: String,
    pub get_exp: i64,
    pub member_lv: i64,
    pub member_exp: i64,
    pub get_base_exp: i64,
    pub mvp: i64,
    pub get_ship_exp: Vec<i64>,
    pub get_exp_lvup: Vec<Vec<i64>>,
    pub quest_name: String,
    pub quest_level: i64,
    pub enemy_level: i64,
    pub enemy_rank: String,
    pub enemy_deck_name: String,
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
    friend_ships: &[emukc_battle::BattleShipInput],
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
        let gain = if friendly_nowhps.get(idx).copied().unwrap_or(1) <= 0
            || (!ship.married && ship.ship.api_lv >= 99)
        {
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
        let level_cap = level::ship_level_cap(ship.married);
        let mut lvup_vec = build_exp_lvup_vector(ship.ship.api_exp[0], new_exp);
        if level_cap < 180 {
            let cap_threshold = level::ship_level_required_exp(level_cap + 1);
            lvup_vec.retain(|&exp| exp < cap_threshold);
        }
        lvup.push(lvup_vec);
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
    let stage = definition.stage(&active.stage_id).ok_or_else(|| {
        GameplayError::EntryNotFound(format!(
            "stage `{}` not found for map {}",
            active.stage_id, active.map_id
        ))
    })?;
    Ok(QuestActionEvent::SortieBattleCompleted {
        maparea_id: definition.maparea_id,
        mapinfo_no: definition.mapinfo_no,
        boss_cell: active
            .pending_battle_cell_id
            .is_some_and(|id| stage.boss_cell_nos().contains(&id)),
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
        let mut api_ship: emukc_model::kc2::KcApiShip = ship_model.into();

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
                let raw_exp = ship_model.exp_now + gain;
                let (ship_level, next_exp) = level::exp_to_ship_level(raw_exp);
                let level_cap = level::ship_level_cap(ship_model.married);
                let ship_level = ship_level.min(level_cap);

                let (next_exp, progress, new_ship_exp) = if ship_level >= level_cap {
                    let cap_exp = level::ship_level_required_exp(level_cap);
                    (0, 0, cap_exp)
                } else {
                    let current_level_exp = level::ship_level_required_exp(ship_level);
                    let progress = if next_exp > current_level_exp {
                        ((raw_exp - current_level_exp) * 100 / (next_exp - current_level_exp))
                            .clamp(0, 99)
                    } else {
                        0
                    };
                    (next_exp, progress, raw_exp)
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
        tracing::debug!(
            map_id = definition.map_id,
            is_boss_cell,
            win_rank = %snapshot.win_rank,
            "apply_sortie_map_result: skipped (not boss or bad rank)"
        );
        return Ok(0);
    }

    let record = find_map_record_impl(c, profile_id, definition.map_id).await?;
    let now = Utc::now();
    let was_cleared = record.cleared;
    let current_hp = record.current_hp;
    let current_gauge_index = record.gauge_index;
    let previous_defeat_count = record.defeat_count.unwrap_or_default();
    let next_gauge_index = current_gauge_index + 1;
    tracing::debug!(
        map_id = definition.map_id,
        was_cleared,
        max_hp = ?definition.max_hp,
        current_hp = ?current_hp,
        gauge_index = current_gauge_index,
        required_defeat = ?stage.required_defeat_count.or(definition.required_defeat_count),
        "apply_sortie_map_result: record state"
    );
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
        codex::map::{MapDefinition, MapStageDefinition, MapVariantDefinition, ShipDropDefinition},
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
        use emukc_model::codex::map::{MapCellDefinition, MapVariantDefinition};

        let definition = MapDefinition {
            maparea_id: 1,
            mapinfo_no: 2,
            default_variant: String::new(),
            variants: {
                let mut v = std::collections::BTreeMap::new();
                v.insert(
                    String::new(),
                    MapVariantDefinition {
                        boss_cell_no: 3,
                        cells: vec![MapCellDefinition {
                            cell_no: 3,
                            event_id: 5,
                            node_label: Some("C".to_string()),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                );
                v
            },
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
    fn build_sortie_quest_event_marks_boss_cell_via_label_equivalence() {
        // Mirror of map 1-2 node E: two cells (3, 4) share label "C",
        // boss_cell_no=3. Player reaches cell 4 (non-canonical) — must still be boss.
        use emukc_model::codex::map::{MapCellDefinition, MapVariantDefinition};

        let definition = MapDefinition {
            maparea_id: 1,
            mapinfo_no: 2,
            default_variant: String::new(),
            variants: {
                let mut v = std::collections::BTreeMap::new();
                v.insert(
                    String::new(),
                    MapVariantDefinition {
                        boss_cell_no: 3,
                        cells: vec![
                            MapCellDefinition {
                                cell_no: 3,
                                event_id: 5,
                                node_label: Some("C".to_string()),
                                ..Default::default()
                            },
                            MapCellDefinition {
                                cell_no: 4,
                                event_id: 5,
                                node_label: Some("C".to_string()),
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                );
                v
            },
            ..Default::default()
        };
        let active = ActiveSortieState {
            deck_id: 3,
            map_id: 12,
            map_name: "1-2".to_string(),
            map_level: 1,
            stage_id: String::new(),
            current_cell_id: 4,
            boss_cell_id: 3,
            pending_battle_cell_id: Some(4),
            visited_cell_ids: BTreeSet::new(),
            locked_enemy_composition: None,
        };

        let event = build_sortie_quest_event(&definition, &active, &snapshot("S")).unwrap();
        match event {
            QuestActionEvent::SortieBattleCompleted {
                boss_cell,
                ..
            } => {
                assert!(boss_cell, "cell 4 shares boss label and must be recognized as boss");
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

    // --- gauge progression tests ---

    fn gauge_map_definition(gauge_count: i64, max_hp: i64) -> MapDefinition {
        MapDefinition {
            map_id: 99_001,
            maparea_id: 99,
            mapinfo_no: 1,
            gauge_count: Some(gauge_count),
            max_hp: Some(max_hp),
            ..Default::default()
        }
    }

    fn gauge_stage() -> MapStageDefinition {
        MapStageDefinition {
            cells: vec![],
            ..Default::default()
        }
    }

    async fn insert_test_profile(db: &emukc_db::sea_orm::DbConn) -> i64 {
        use emukc_db::entity::{profile, user};
        let account = user::account::ActiveModel {
            name: ActiveValue::Set("gauge-test".into()),
            secret: ActiveValue::Set(String::new()),
            create_time: ActiveValue::Set(emukc_time::chrono::Utc::now()),
            last_login: ActiveValue::Set(emukc_time::chrono::Utc::now()),
            ..Default::default()
        };
        let account = account.insert(db).await.unwrap();
        let prof = profile::default_active_model(account.uid, "gauge-tester");
        let prof = prof.insert(db).await.unwrap();
        prof.id
    }

    async fn insert_gauge_record(db: &emukc_db::sea_orm::DbConn, profile_id: i64, map_id: i64) {
        use emukc_db::entity::profile::map_record;
        let record = map_record::ActiveModel {
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(map_id),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(None),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(Some(1)),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(None),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
            ..Default::default()
        };
        record.insert(db).await.unwrap();
    }

    async fn get_record(
        db: &emukc_db::sea_orm::DbConn,
        profile_id: i64,
        map_id: i64,
    ) -> emukc_db::entity::profile::map_record::Model {
        find_map_record_impl(db, profile_id, map_id).await.unwrap()
    }

    async fn get_gauge_index(db: &emukc_db::sea_orm::DbConn, profile_id: i64, map_id: i64) -> i64 {
        get_record(db, profile_id, map_id).await.gauge_index
    }

    async fn is_cleared(db: &emukc_db::sea_orm::DbConn, profile_id: i64, map_id: i64) -> bool {
        get_record(db, profile_id, map_id).await.cleared
    }

    #[tokio::test]
    async fn gauge_advances_after_boss_kill_with_remaining_gauges() {
        let db = emukc_db::prelude::new_mem_db().await.unwrap();
        let pid = insert_test_profile(&db).await;
        let definition = gauge_map_definition(2, 2);
        let stage = gauge_stage();
        insert_gauge_record(&db, pid, definition.map_id).await;

        let snap = snapshot("S");
        let result =
            apply_sortie_map_result(&db, pid, &definition, &stage, true, &snap).await.unwrap();
        assert_eq!(result, 0, "gauge advance should not report first-clear");

        let idx = get_gauge_index(&db, pid, definition.map_id).await;
        assert_eq!(idx, 2, "gauge_index should advance from 1 to 2");
        assert!(!is_cleared(&db, pid, definition.map_id).await);
    }

    #[tokio::test]
    async fn final_gauge_clears_map() {
        let db = emukc_db::prelude::new_mem_db().await.unwrap();
        let pid = insert_test_profile(&db).await;
        let definition = gauge_map_definition(2, 1);
        let stage = gauge_stage();

        // Insert record already at gauge 2 with 1 HP remaining
        use emukc_db::entity::profile::map_record;
        let record = map_record::ActiveModel {
            profile_id: ActiveValue::Set(pid),
            map_id: ActiveValue::Set(definition.map_id),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(None),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(Some(1)),
            gauge_index: ActiveValue::Set(2),
            stage_id: ActiveValue::Set(None),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
            ..Default::default()
        };
        record.insert(&db).await.unwrap();

        let snap = snapshot("S");
        let result =
            apply_sortie_map_result(&db, pid, &definition, &stage, true, &snap).await.unwrap();
        assert_eq!(result, 1, "final gauge clear should report first-clear");

        let cleared = is_cleared(&db, pid, definition.map_id).await;
        assert!(cleared, "map should be marked cleared after last gauge");
    }

    #[tokio::test]
    async fn single_gauge_map_clears_on_boss_kill() {
        let db = emukc_db::prelude::new_mem_db().await.unwrap();
        let pid = insert_test_profile(&db).await;
        let definition = gauge_map_definition(1, 1);
        let stage = gauge_stage();
        insert_gauge_record(&db, pid, definition.map_id).await;

        let snap = snapshot("S");
        let result =
            apply_sortie_map_result(&db, pid, &definition, &stage, true, &snap).await.unwrap();
        assert_eq!(result, 1, "single-gauge clear should report first-clear");
        assert!(is_cleared(&db, pid, definition.map_id).await);
    }

    #[tokio::test]
    async fn non_boss_does_not_advance_gauge() {
        let db = emukc_db::prelude::new_mem_db().await.unwrap();
        let pid = insert_test_profile(&db).await;
        let definition = gauge_map_definition(2, 2);
        let stage = gauge_stage();
        insert_gauge_record(&db, pid, definition.map_id).await;

        let snap = snapshot("S");
        let result =
            apply_sortie_map_result(&db, pid, &definition, &stage, false, &snap).await.unwrap();
        assert_eq!(result, 0);

        let idx = get_gauge_index(&db, pid, definition.map_id).await;
        assert_eq!(idx, 1, "gauge_index should not change on non-boss cell");
    }
}
