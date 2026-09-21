//! Sortie battle orchestration — build context → call `emukc_battle` → persist.

use emukc_battle::{
    BattlePacket, BattleRng, BattleRuntimeShip, EngagementType, NightBattleInput,
    NightBattlePacket, execute_day, execute_night,
};
use emukc_model::codex::Codex;

use super::{
    SortieBattleInput, SortieBattleSession, SortieNightBattleSession, build_sortie_session,
};
use crate::game::SortieStore;

/// Run a day battle, store the session, return it.
pub fn run_day_battle(
    store: &SortieStore,
    codex: &Codex,
    input: SortieBattleInput,
    rng: &mut impl BattleRng,
) -> SortieBattleSession {
    let SortieBattleInput {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        context,
    } = input;
    let simulation = execute_day(codex, context, rng);
    let session = build_sortie_session(profile_id, deck_id, map_id, cell_id, simulation);
    store.insert_pending_battle(session.profile_id, session.clone());
    session
}

/// Remove and return a pending day battle session.
pub fn take_day_battle_result(store: &SortieStore, profile_id: i64) -> Option<SortieBattleSession> {
    store.take_pending_battle(profile_id)
}

/// Check whether a pending battle session exists.
pub fn pending_battle(store: &SortieStore, profile_id: i64) -> Option<SortieBattleSession> {
    store.get_pending_battle(profile_id)
}

/// Run a night battle following a day battle, update the stored session.
pub fn run_night_battle(
    store: &SortieStore,
    codex: &Codex,
    profile_id: i64,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
    rng: &mut impl BattleRng,
) -> Option<SortieNightBattleSession> {
    use emukc_battle::AirState;

    let mut session = store.get_pending_battle(profile_id)?;
    let air_state = session
        .packet
        .kouku
        .as_ref()
        .and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));
    // 連合艦隊: only 第2艦隊 fights at night (R5). 第1艦隊 stays in the session
    // untouched — it still has to be reported, repaired and paid experience.
    let escort_start = escort_deck_start(&session.friendly);
    let simulation = execute_night(
        codex,
        NightBattleInput {
            friendly: session.friendly[escort_start..].to_vec(),
            enemy: session.enemy.clone(),
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            air_state,
        },
        rng,
    );
    session.friendly.truncate(escort_start);
    session.friendly.extend(simulation.friendly.iter().cloned());
    session.enemy = simulation.enemy.clone();
    session.outcome = simulation.outcome.clone();
    session.packet.friendly_nowhps.truncate(escort_start);
    session.packet.friendly_nowhps.extend(simulation.packet.friendly_nowhps.iter().copied());
    session.packet.enemy_nowhps = simulation.packet.enemy_nowhps.clone();
    session.packet.midnight_flag = 0;
    store.insert_pending_battle(profile_id, session);

    Some(SortieNightBattleSession {
        profile_id,
        packet: simulation.packet,
        outcome: simulation.outcome,
    })
}

/// Where 第2艦隊 starts in a session's friendly vector, or 0 for a single fleet.
///
/// The ships carry their own deck tag, so the boundary is recoverable from the
/// session alone — nothing has to store it alongside.
pub fn escort_deck_start(friendly: &[BattleRuntimeShip]) -> usize {
    friendly.iter().position(BattleRuntimeShip::is_escort_deck).unwrap_or(0)
}

/// Run a night-start (`sp_midnight`) battle — no preceding day battle.
///
/// Runs the night simulation on the setup fleets and stores the outcome as the
/// pending session, so `sortie_battle_result` (`enemy_nowhps`) and
/// `sortie_midnight_battle` (`formation`) read it the same way as after a day battle.
pub fn run_sp_midnight_battle(
    store: &SortieStore,
    codex: &Codex,
    input: SortieBattleInput,
    rng: &mut impl BattleRng,
) -> (SortieBattleSession, SortieNightBattleSession) {
    let SortieBattleInput {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        context,
    } = input;
    let enemy_formation_id = context.enemy_formation_id;

    let night = execute_night(
        codex,
        NightBattleInput {
            friendly: context
                .friend_ships
                .into_iter()
                .map(|s| BattleRuntimeShip::new(s, true, true))
                .collect(),
            enemy: context
                .enemy_ships
                .into_iter()
                .map(|s| BattleRuntimeShip::new(s, false, true))
                .collect(),
            friendly_formation_id: context.friendly_formation_id,
            enemy_formation_id,
            engagement: context.engagement,
            air_state: None,
        },
        rng,
    );

    let session = SortieBattleSession {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        friendly_ship_ids: night.friendly.iter().map(|s| s.ship.api_id).collect(),
        enemy_ship_ids: night.enemy.iter().map(|s| s.ship.api_ship_id).collect(),
        friendly: night.friendly,
        enemy: night.enemy,
        packet: night_start_packet(&night.packet),
        outcome: night.outcome.clone(),
    };
    store.insert_pending_battle(profile_id, session.clone());

    let night_session = SortieNightBattleSession {
        profile_id,
        packet: night.packet,
        outcome: night.outcome,
    };

    (session, night_session)
}

/// The day-packet view of a night-start battle: no day phase ran, so only the
/// fields later readers consume (`formation` and both `nowhps`) carry values.
fn night_start_packet(night: &NightBattlePacket) -> BattlePacket {
    BattlePacket {
        formation: night.formation,
        friendly_nowhps: night.friendly_nowhps.clone(),
        enemy_nowhps: night.enemy_nowhps.clone(),
        smoke_type: 0,
        balloon_cell: 0,
        atoll_cell: 0,
        midnight_flag: 0,
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
    }
}
