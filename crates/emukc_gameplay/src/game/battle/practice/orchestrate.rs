//! Practice battle orchestration — build context → call `emukc_battle` → build responses.

use emukc_battle::{
    AirState, BattleContext, BattleOutcome, BattleType, EngagementType, NightBattleInput,
    simulate_day, simulate_night,
};
use emukc_model::codex::Codex;
use emukc_model::kc2::KcSortieResultRank;

use crate::err::GameplayError;

use super::super::rng::CryptoRng;
use super::exp::{calculate_admiral_exp, calculate_ship_exp};
use super::response::{build_night_response, calculate_base_exp, enemy_slot_ids};
use super::{
    PracticeBattleInput, PracticeBattleResponse, PracticeBattleResultSnapshot,
    PracticeBattleSession, PracticeNightBattleResponse,
};

/// Run a practice day battle and produce response + result snapshot.
pub fn run_day_battle(
    codex: &Codex,
    input: PracticeBattleInput,
) -> Result<(PracticeBattleResponse, PracticeBattleResultSnapshot), GameplayError> {
    let friendly_nowhps =
        input.friend_ships.iter().map(|ship| ship.ship.api_nowhp).collect::<Vec<_>>();
    let friendly_maxhps =
        input.friend_ships.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
    let enemy_nowhps = input.enemy_ships.iter().map(|ship| ship.ship.api_nowhp).collect::<Vec<_>>();
    let enemy_maxhps = input.enemy_ships.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
    let mut rng = CryptoRng;
    let simulation = simulate_day(
        codex,
        BattleContext {
            battle_type: BattleType::Normal,
            is_sortie: false,
            friendly_formation_id: input.formation_id,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships: input.friend_ships,
            enemy_ships: input.enemy_ships,
        },
        &mut rng,
    );

    let base_exp = calculate_base_exp(&input.rival);
    let get_exp = calculate_admiral_exp(base_exp, &simulation.outcome.win_rank.to_string());
    let ct_flagship = simulation
        .friendly
        .first()
        .and_then(|s| codex.manifest.find_ship(s.ship.api_ship_id))
        .is_some_and(|m| m.api_stype == 21);
    let (ship_exp, ship_lvup) = calculate_ship_exp(
        &simulation.friendly,
        base_exp,
        simulation.outcome.mvp,
        ct_flagship,
        codex.game_cfg.exp.ct_exp_boost,
        codex.game_cfg.exp.practice_exp_boost,
    );

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
            .map(|(idx, ship)| emukc_model::thirdparty::FleetShipSnapshot {
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

    super::PENDING_PRACTICE_BATTLES.lock().unwrap().insert(
        input.profile_id,
        PracticeBattleSession {
            profile_id: input.profile_id,
            deck_id: input.deck_id,
            enemy_id: input.enemy_id,
            friendly: simulation.friendly,
            enemy: simulation.enemy,
            formation: response.api_formation,
            outcome: BattleOutcome {
                win_rank: snapshot.win_rank,
                mvp: snapshot.mvp,
                can_midnight: response.api_midnight_flag > 0,
            },
            air_state,
        },
    );

    Ok((response, snapshot))
}

/// Run a practice night battle and produce response + updated snapshot.
pub fn run_night_battle(
    codex: &Codex,
    profile_id: i64,
) -> Option<(PracticeNightBattleResponse, PracticeBattleResultSnapshot)> {
    let mut sessions = super::PENDING_PRACTICE_BATTLES.lock().unwrap();
    let session = sessions.get_mut(&profile_id)?;
    if !session.outcome.can_midnight {
        return None;
    }
    let mut rng = CryptoRng;
    let simulation = simulate_night(
        codex,
        NightBattleInput {
            friendly: session.friendly.clone(),
            enemy: session.enemy.clone(),
            friendly_formation_id: session.formation[0],
            enemy_formation_id: session.formation[1],
            engagement: EngagementType::from_api_id(session.formation[2])
                .unwrap_or(EngagementType::SameCourse),
            air_state: session.air_state,
        },
        &mut rng,
    );
    session.friendly = simulation.friendly.clone();
    session.enemy = simulation.enemy.clone();
    session.outcome = simulation.outcome.clone();

    let response = build_night_response(session, &simulation.packet);
    let snapshot = PracticeBattleResultSnapshot {
        deck_id: session.deck_id,
        enemy_id: session.enemy_id,
        friendly_ship_ids: session.friendly.iter().map(|ship| ship.ship.api_id).collect(),
        friendly_fleet_snapshot: session
            .friendly
            .iter()
            .enumerate()
            .map(|(idx, ship)| emukc_model::thirdparty::FleetShipSnapshot {
                mst_id: ship.ship.api_ship_id,
                level: ship.ship.api_lv,
                position: idx as i64 + 1,
            })
            .collect(),
        enemy_ship_ids: session.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        win_rank: simulation.outcome.win_rank,
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
