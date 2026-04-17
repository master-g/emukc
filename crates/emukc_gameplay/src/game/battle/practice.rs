#![allow(non_snake_case)]

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use serde::Serialize;

use emukc_model::{
    codex::Codex, kc2::level, profile::practice::Rival, thirdparty::FleetShipSnapshot,
};

use crate::{
    err::GameplayError,
    game::battle::core::{
        AirState, BattleContext, BattleHougeki, BattleKouku, BattleMode, BattleNightHougeki,
        BattleOpeningAttack, BattleRaigeki, BattleRuntimeShip, BattleShipInput, BattleType,
        EngagementType, NightBattlePacket, simulate_day_battle_v1, simulate_night_battle_v1,
    },
};

static PENDING_PRACTICE_BATTLES: LazyLock<Mutex<HashMap<i64, PracticeBattleSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub type PracticeBattleShipInput = BattleShipInput;

#[derive(Debug, Clone)]
pub struct PracticeBattleInput {
    pub profile_id: i64,
    pub deck_id: i64,
    pub formation_id: i64,
    pub enemy_id: i64,
    pub member_lv: i64,
    pub member_exp: i64,
    pub friend_ships: Vec<PracticeBattleShipInput>,
    pub enemy_ships: Vec<PracticeBattleShipInput>,
    pub rival: Rival,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleResponse {
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
    pub api_e_effect_list: Vec<Vec<i64>>,
    pub api_smoke_type: i64,
    pub api_balloon_cell: i64,
    pub api_atoll_cell: i64,
    pub api_midnight_flag: i64,
    pub api_search: [i64; 2],
    pub api_stage_flag: [i64; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_kouku: Option<BattleKouku>,
    pub api_opening_taisen_flag: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_opening_taisen: Option<BattleHougeki>,
    pub api_opening_flag: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_opening_atack: Option<BattleOpeningAttack>,
    pub api_hourai_flag: [i64; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki1: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki2: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_hougeki3: Option<BattleHougeki>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_raigeki: Option<BattleRaigeki>,
}

#[derive(Debug, Clone)]
pub struct PracticeBattleResultSnapshot {
    pub deck_id: i64,
    pub enemy_id: i64,
    pub friendly_ship_ids: Vec<i64>,
    pub friendly_fleet_snapshot: Vec<FleetShipSnapshot>,
    pub enemy_ship_ids: Vec<i64>,
    pub win_rank: String,
    pub get_exp: i64,
    pub member_lv: i64,
    pub member_exp: i64,
    pub get_base_exp: i64,
    pub mvp: i64,
    pub get_ship_exp: Vec<i64>,
    pub get_exp_lvup: Vec<Vec<i64>>,
    pub did_night_battle: bool,
    pub enemy_level: i64,
    pub enemy_rank: String,
    pub enemy_deck_name: String,
}

#[derive(Debug, Clone)]
pub struct PracticeBattleSession {
    pub profile_id: i64,
    pub deck_id: i64,
    pub enemy_id: i64,
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub formation: [i64; 3],
    pub outcome: crate::game::battle::core::BattleOutcome,
    pub air_state: Option<AirState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleResultResponse {
    pub api_ship_id: Vec<i64>,
    pub api_win_rank: String,
    pub api_get_exp: i64,
    pub api_member_lv: i64,
    pub api_member_exp: i64,
    pub api_get_base_exp: i64,
    pub api_mvp: i64,
    pub api_get_ship_exp: Vec<i64>,
    pub api_get_exp_lvup: Vec<Vec<i64>>,
    pub api_enemy_info: PracticeBattleEnemyInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeBattleEnemyInfo {
    pub api_user_name: String,
    pub api_level: i64,
    pub api_rank: String,
    pub api_deck_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeNightBattleResponse {
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

pub fn simulate_practice_day_battle(
    codex: &Codex,
    input: PracticeBattleInput,
) -> Result<(PracticeBattleResponse, PracticeBattleResultSnapshot), GameplayError> {
    let friendly_nowhps =
        input.friend_ships.iter().map(|ship| ship.ship.api_nowhp).collect::<Vec<_>>();
    let friendly_maxhps =
        input.friend_ships.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
    let enemy_nowhps = input.enemy_ships.iter().map(|ship| ship.ship.api_nowhp).collect::<Vec<_>>();
    let enemy_maxhps = input.enemy_ships.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
    let simulation = simulate_day_battle_v1(
        codex,
        BattleContext {
            mode: BattleMode::Practice,
            battle_type: BattleType::Normal,
            is_sortie: false,
            friendly_formation_id: input.formation_id,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships: input.friend_ships,
            enemy_ships: input.enemy_ships,
            rng_seed: None,
        },
    );

    let base_exp = calculate_practice_base_exp(&input.rival);
    let get_exp = calculate_admiral_exp(base_exp, &simulation.outcome.win_rank);
    let (ship_exp, ship_lvup) =
        calculate_practice_ship_exp(&simulation.friendly, base_exp, simulation.outcome.mvp);

    let air_state = simulation
        .packet
        .kouku
        .as_ref()
        .and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));

    let response = PracticeBattleResponse {
        api_deck_id: input.deck_id,
        api_formation: simulation.packet.formation,
        api_f_nowhps: friendly_nowhps,
        api_f_maxhps: friendly_maxhps,
        api_fParam: simulation
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
        api_ship_ke: simulation.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: simulation.enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: enemy_nowhps,
        api_e_maxhps: enemy_maxhps,
        api_eSlot: simulation.enemy.iter().map(enemy_slot_ids).collect(),
        api_eParam: simulation
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
        api_e_effect_list: simulation
            .enemy
            .iter()
            .map(|ship| {
                if ship.effect_list.is_empty() {
                    vec![0]
                } else {
                    ship.effect_list.clone()
                }
            })
            .collect(),
        api_smoke_type: simulation.packet.smoke_type,
        api_balloon_cell: simulation.packet.balloon_cell,
        api_atoll_cell: simulation.packet.atoll_cell,
        api_midnight_flag: simulation.packet.midnight_flag,
        api_search: simulation.packet.search,
        api_stage_flag: simulation.packet.stage_flag,
        api_kouku: simulation.packet.kouku,
        api_opening_taisen_flag: simulation.packet.opening_taisen_flag,
        api_opening_taisen: simulation.packet.opening_taisen,
        api_opening_flag: simulation.packet.opening_flag,
        api_opening_atack: simulation.packet.opening_attack,
        api_hourai_flag: simulation.packet.hourai_flag,
        api_hougeki1: simulation.packet.hougeki1,
        api_hougeki2: simulation.packet.hougeki2,
        api_hougeki3: simulation.packet.hougeki3,
        api_raigeki: simulation.packet.raigeki,
    };

    let snapshot = PracticeBattleResultSnapshot {
        deck_id: input.deck_id,
        enemy_id: input.enemy_id,
        friendly_ship_ids: simulation.friendly.iter().map(|ship| ship.ship.api_id).collect(),
        friendly_fleet_snapshot: simulation
            .friendly
            .iter()
            .enumerate()
            .map(|(idx, ship)| FleetShipSnapshot {
                mst_id: ship.ship.api_ship_id,
                level: ship.ship.api_lv,
                position: idx as i64 + 1,
            })
            .collect(),
        enemy_ship_ids: simulation.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        win_rank: simulation.outcome.win_rank,
        get_exp,
        member_lv: input.member_lv,
        member_exp: input.member_exp,
        get_base_exp: base_exp,
        mvp: simulation.outcome.mvp,
        get_ship_exp: ship_exp,
        get_exp_lvup: ship_lvup,
        did_night_battle: false,
        enemy_level: input.rival.level,
        enemy_rank: input.rival.rank.get_name().to_string(),
        enemy_deck_name: input.rival.details.deck_name,
    };

    PENDING_PRACTICE_BATTLES.lock().unwrap().insert(
        input.profile_id,
        PracticeBattleSession {
            profile_id: input.profile_id,
            deck_id: input.deck_id,
            enemy_id: input.enemy_id,
            friendly: simulation.friendly,
            enemy: simulation.enemy,
            formation: response.api_formation,
            outcome: crate::game::battle::core::BattleOutcome {
                win_rank: snapshot.win_rank.clone(),
                mvp: snapshot.mvp,
                can_midnight: response.api_midnight_flag > 0,
            },
            air_state,
        },
    );

    Ok((response, snapshot))
}

pub fn build_practice_battle_result_response(
    snapshot: PracticeBattleResultSnapshot,
) -> PracticeBattleResultResponse {
    PracticeBattleResultResponse {
        api_ship_id: snapshot.enemy_ship_ids,
        api_win_rank: snapshot.win_rank,
        api_get_exp: snapshot.get_exp,
        api_member_lv: snapshot.member_lv,
        api_member_exp: snapshot.member_exp,
        api_get_base_exp: snapshot.get_base_exp,
        api_mvp: snapshot.mvp,
        api_get_ship_exp: snapshot.get_ship_exp,
        api_get_exp_lvup: snapshot.get_exp_lvup,
        api_enemy_info: PracticeBattleEnemyInfo {
            api_user_name: String::new(),
            api_level: snapshot.enemy_level,
            api_rank: snapshot.enemy_rank,
            api_deck_name: snapshot.enemy_deck_name,
        },
    }
}

pub fn simulate_practice_night_battle(
    codex: &Codex,
    profile_id: i64,
) -> Option<(PracticeNightBattleResponse, PracticeBattleResultSnapshot)> {
    let mut sessions = PENDING_PRACTICE_BATTLES.lock().unwrap();
    let session = sessions.get_mut(&profile_id)?;
    if !session.outcome.can_midnight {
        return None;
    }
    let simulation = simulate_night_battle_v1(
        codex,
        session.friendly.clone(),
        session.enemy.clone(),
        session.formation[0],
        session.formation[1],
        EngagementType::from_api_id(session.formation[2]).unwrap_or(EngagementType::SameCourse),
        session.air_state.as_ref(),
    );
    session.friendly = simulation.friendly.clone();
    session.enemy = simulation.enemy.clone();
    session.outcome = simulation.outcome.clone();

    let response = build_practice_night_battle_response(session, &simulation.packet);
    let snapshot = PracticeBattleResultSnapshot {
        deck_id: session.deck_id,
        enemy_id: session.enemy_id,
        friendly_ship_ids: session.friendly.iter().map(|ship| ship.ship.api_id).collect(),
        friendly_fleet_snapshot: session
            .friendly
            .iter()
            .enumerate()
            .map(|(idx, ship)| FleetShipSnapshot {
                mst_id: ship.ship.api_ship_id,
                level: ship.ship.api_lv,
                position: idx as i64 + 1,
            })
            .collect(),
        enemy_ship_ids: session.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        win_rank: simulation.outcome.win_rank.clone(),
        get_exp: 0,
        member_lv: 0,
        member_exp: 0,
        get_base_exp: 0,
        mvp: simulation.outcome.mvp,
        get_ship_exp: vec![],
        get_exp_lvup: vec![],
        did_night_battle: true,
        enemy_level: session.enemy.first().map(|ship| ship.ship.api_lv).unwrap_or_default(),
        enemy_rank: String::new(),
        enemy_deck_name: String::new(),
    };
    Some((response, snapshot))
}

pub fn clear_pending_practice_battle(profile_id: i64) {
    PENDING_PRACTICE_BATTLES.lock().unwrap().remove(&profile_id);
}

pub fn pending_practice_battle(profile_id: i64) -> Option<PracticeBattleSession> {
    PENDING_PRACTICE_BATTLES.lock().unwrap().get(&profile_id).cloned()
}

fn enemy_slot_ids(ship: &BattleRuntimeShip) -> [i64; 5] {
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

fn enemy_slot_ids_from_input(ship: &PracticeBattleShipInput) -> [i64; 5] {
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

fn build_practice_night_battle_response(
    session: &PracticeBattleSession,
    packet: &NightBattlePacket,
) -> PracticeNightBattleResponse {
    PracticeNightBattleResponse {
        api_deck_id: session.deck_id,
        api_formation: packet.formation,
        api_f_nowhps: packet.friendly_nowhps.clone(),
        api_f_maxhps: packet.friendly_maxhps.clone(),
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
        api_e_nowhps: packet.enemy_nowhps.clone(),
        api_e_maxhps: packet.enemy_maxhps.clone(),
        api_eSlot: session
            .enemy
            .iter()
            .map(|ship| {
                enemy_slot_ids_from_input(&PracticeBattleShipInput {
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
        api_hougeki: packet.hougeki.clone(),
    }
}

fn calculate_practice_base_exp(rival: &Rival) -> i64 {
    (rival.level.max(1) * 9).clamp(100, 1200)
}

pub(crate) fn calculate_admiral_exp(base_exp: i64, win_rank: &str) -> i64 {
    match win_rank {
        "S" => (base_exp as f64 * 0.12).round() as i64,
        "A" => (base_exp as f64 * 0.1).round() as i64,
        "B" => (base_exp as f64 * 0.08).round() as i64,
        "C" => (base_exp as f64 * 0.05).round() as i64,
        _ => (base_exp as f64 * 0.03).round() as i64,
    }
}

pub(crate) fn calculate_practice_ship_exp(
    friendly: &[BattleRuntimeShip],
    base_exp: i64,
    mvp_idx: i64,
) -> (Vec<i64>, Vec<Vec<i64>>) {
    let mut exp = vec![-1];
    let mut lvup = Vec::with_capacity(friendly.len());

    for (idx, ship) in friendly.iter().enumerate() {
        let gain = if idx as i64 + 1 == mvp_idx {
            base_exp * 2
        } else if idx == 0 {
            ((base_exp as f64) * 1.5).floor() as i64
        } else {
            base_exp
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

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_model::codex::Codex;

    fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> PracticeBattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items).unwrap();
        PracticeBattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
        }
    }

    #[test]
    fn practice_battle_core_generates_packet_and_result() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let input = PracticeBattleInput {
            profile_id: 1,
            deck_id: 1,
            formation_id: 1,
            enemy_id: 1,
            member_lv: 120,
            member_exp: 123456,
            friend_ships: vec![sample_ship(&codex, 89, 99), sample_ship(&codex, 79, 80)],
            enemy_ships: vec![sample_ship(&codex, 412, 185)],
            rival: Rival {
                id: 1,
                index: 1,
                name: "Enemy".to_string(),
                comment: String::new(),
                level: 120,
                rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
                flag: emukc_model::profile::practice::RivalFlag::Gold,
                status: emukc_model::profile::practice::RivalStatus::Untouched,
                medals: 10,
                details: emukc_model::profile::practice::RivalDetail {
                    deck_name: "演習".to_string(),
                    ..Default::default()
                },
            },
        };

        let (battle, result) = simulate_practice_day_battle(&codex, input).unwrap();
        assert_eq!(battle.api_deck_id, 1);
        assert_eq!(battle.api_formation, [1, 1, 1]);
        assert_eq!(battle.api_f_nowhps.len(), 2);
        assert_eq!(battle.api_ship_ke.len(), 1);
        assert_eq!(result.enemy_ship_ids.len(), 1);
        assert_eq!(result.member_lv, 120);
    }

    #[test]
    fn practice_day_battle_enables_midnight_when_both_sides_survive() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut friend = sample_ship(&codex, 79, 1);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, 412, 99);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let input = PracticeBattleInput {
            profile_id: 1,
            deck_id: 1,
            formation_id: 5,
            enemy_id: 1,
            member_lv: 120,
            member_exp: 123456,
            friend_ships: vec![friend],
            enemy_ships: vec![enemy],
            rival: Rival {
                id: 1,
                index: 1,
                name: "Enemy".to_string(),
                comment: String::new(),
                level: 120,
                rank: emukc_model::kc2::UserHQRank::MarshalAdmiral,
                flag: emukc_model::profile::practice::RivalFlag::Gold,
                status: emukc_model::profile::practice::RivalStatus::Untouched,
                medals: 10,
                details: emukc_model::profile::practice::RivalDetail {
                    deck_name: "演習".to_string(),
                    ..Default::default()
                },
            },
        };

        let (battle, _) = simulate_practice_day_battle(&codex, input).unwrap();
        assert_eq!(battle.api_midnight_flag, 1);
    }

    #[test]
    fn exp_lvup_vector_keeps_pre_gain_exp_and_future_thresholds() {
        let before = 48_802;
        let after = 49_880;
        assert_eq!(build_exp_lvup_vector(before, after), vec![48_802, 49_600, 52_800]);
    }
}
