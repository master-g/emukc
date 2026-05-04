//! Sortie battle orchestration — build context → call `emukc_battle` → persist.

use emukc_battle::{
    BattleOutcome, BattlePacket, BattleRuntimeShip, EngagementType, NightBattleInput, simulate_day,
    simulate_night,
};
use emukc_model::codex::Codex;
use emukc_model::kc2::KcSortieResultRank;

use super::super::{repository::SortieRepository, rng::CryptoRng};
use super::{
    SortieBattleInput, SortieBattleSession, SortieNightBattleSession, build_sortie_session,
};

/// Run a day battle, store the session, return it.
pub fn run_day_battle(
    store: &dyn SortieRepository,
    codex: &Codex,
    input: SortieBattleInput,
) -> SortieBattleSession {
    let SortieBattleInput {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        context,
    } = input;
    let mut rng = CryptoRng;
    let simulation = simulate_day(codex, context, &mut rng);
    let session = build_sortie_session(profile_id, deck_id, map_id, cell_id, simulation);
    store.insert_pending_battle(session.profile_id, session.clone());
    session
}

/// Remove and return a pending day battle session.
pub fn take_day_battle_result(
    store: &dyn SortieRepository,
    profile_id: i64,
) -> Option<SortieBattleSession> {
    store.take_pending_battle(profile_id)
}

/// Check whether a pending battle session exists.
pub fn pending_battle(
    store: &dyn SortieRepository,
    profile_id: i64,
) -> Option<SortieBattleSession> {
    store.get_pending_battle(profile_id)
}

/// Run a night battle following a day battle, update the stored session.
pub fn run_night_battle(
    store: &dyn SortieRepository,
    codex: &Codex,
    profile_id: i64,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
) -> Option<SortieNightBattleSession> {
    use emukc_battle::AirState;

    let mut session = store.get_pending_battle(profile_id)?;
    let air_state = session
        .packet
        .kouku
        .as_ref()
        .and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));
    let mut rng = CryptoRng;
    let simulation = simulate_night(
        codex,
        NightBattleInput {
            friendly: session.friendly.clone(),
            enemy: session.enemy.clone(),
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            air_state,
        },
        &mut rng,
    );
    session.friendly = simulation.friendly.clone();
    session.enemy = simulation.enemy.clone();
    session.outcome = simulation.outcome.clone();
    session.packet.friendly_nowhps = simulation.packet.friendly_nowhps.clone();
    session.packet.enemy_nowhps = simulation.packet.enemy_nowhps.clone();
    session.packet.midnight_flag = 0;
    store.insert_pending_battle(profile_id, session);

    Some(SortieNightBattleSession {
        profile_id,
        packet: simulation.packet,
        outcome: simulation.outcome,
    })
}

/// Run a night-start (`sp_midnight`) battle — no preceding day battle.
///
/// Constructs a minimal day session (no combat phases), stores it, then
/// immediately runs the night simulation and updates the stored session.
pub fn run_sp_midnight_battle(
    store: &dyn SortieRepository,
    codex: &Codex,
    input: SortieBattleInput,
    enemy_formation_id: i64,
) -> (SortieBattleSession, SortieNightBattleSession) {
    let SortieBattleInput {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        context,
    } = input;

    let friendly_formation_id = context.friendly_formation_id;
    let engagement = context.engagement;
    let friendly: Vec<BattleRuntimeShip> =
        context.friend_ships.into_iter().map(|s| BattleRuntimeShip::new(s, true, true)).collect();
    let enemy: Vec<BattleRuntimeShip> =
        context.enemy_ships.into_iter().map(|s| BattleRuntimeShip::new(s, false, true)).collect();

    // Create a minimal day session to anchor the night battle
    let day_session = SortieBattleSession {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        friendly_ship_ids: friendly.iter().map(|s| s.ship.api_id).collect(),
        enemy_ship_ids: enemy.iter().map(|s| s.ship.api_ship_id).collect(),
        friendly: friendly.clone(),
        enemy: enemy.clone(),
        packet: BattlePacket {
            formation: [friendly_formation_id, enemy_formation_id, engagement.api_id()],
            friendly_nowhps: friendly.iter().map(BattleRuntimeShip::hp).collect(),
            enemy_nowhps: enemy.iter().map(BattleRuntimeShip::hp).collect(),
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: 1,
            search: [1, 1],
            stage_flag: [0, 0, 0],
            kouku: None,
            opening_taisen_flag: 0,
            opening_taisen: None,
            opening_flag: 0,
            opening_attack: None,
            hourai_flag: [0, 0, 0, 0],
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
        },
        outcome: BattleOutcome {
            win_rank: KcSortieResultRank::D,
            mvp: 0,
            can_midnight: true,
        },
    };
    store.insert_pending_battle(profile_id, day_session.clone());

    // Run night battle using the stored session
    let mut rng = CryptoRng;
    let night = simulate_night(
        codex,
        NightBattleInput {
            friendly,
            enemy,
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            air_state: None,
        },
        &mut rng,
    );

    // Update the stored day session with night results
    if let Some(mut stored) = store.take_pending_battle(profile_id) {
        stored.friendly = night.friendly.clone();
        stored.enemy = night.enemy.clone();
        stored.outcome = night.outcome.clone();
        stored.packet.friendly_nowhps = night.packet.friendly_nowhps.clone();
        stored.packet.enemy_nowhps = night.packet.enemy_nowhps.clone();
        stored.packet.midnight_flag = 0;
        store.insert_pending_battle(profile_id, stored);
    }

    let night_session = SortieNightBattleSession {
        profile_id,
        packet: night.packet,
        outcome: night.outcome,
    };

    (day_session, night_session)
}
