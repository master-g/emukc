//! Sortie battle orchestration — build context → call `emukc_battle` → persist.

use emukc_battle::{
    BattlePacket, BattleRng, BattleRuntimeShip, EngagementType, NightBattleInput,
    NightBattlePacket, execute_day, execute_night, execute_sp_midnight,
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

/// Run the night battle that follows `session`'s day battle, store the updated
/// session and return it with the night packet.
///
/// The formation and engagement carry over from the day packet. 連合艦隊: only
/// 第2艦隊 fights at night (R5); 第1艦隊 stays in the session untouched — it
/// still has to be reported, repaired and paid experience.
pub fn run_night_battle(
    store: &SortieStore,
    codex: &Codex,
    mut session: SortieBattleSession,
    rng: &mut impl BattleRng,
) -> (SortieBattleSession, SortieNightBattleSession) {
    use emukc_battle::AirState;

    let [friendly_formation_id, enemy_formation_id, engagement] = session.packet.formation;
    let air_state = session
        .packet
        .kouku
        .as_ref()
        .and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));
    let simulation = execute_night(
        codex,
        NightBattleInput {
            friendly: session.night_fleet().to_vec(),
            enemy: session.enemy.clone(),
            friendly_formation_id,
            enemy_formation_id,
            engagement: EngagementType::from_api_id(engagement)
                .unwrap_or(EngagementType::SameCourse),
            air_state,
        },
        rng,
    );
    session.absorb_night(&simulation);
    store.insert_pending_battle(session.profile_id, session.clone());

    let night = SortieNightBattleSession {
        profile_id: session.profile_id,
        packet: simulation.packet,
        outcome: simulation.outcome,
    };
    (session, night)
}

/// Run a night-start (`sp_midnight`) battle — no preceding day battle.
///
/// Runs the night simulation on the setup fleets and stores the outcome as the
/// pending session, so `sortie_battle_result` (`enemy_nowhps`) and
/// `sortie_midnight_battle` (`formation`) read it the same way as after a day battle.
///
/// A combined fleet fights it with 第2艦隊 alone, exactly as it would a night
/// battle following a day one. 第1艦隊 still enters the stored session ahead of
/// it, because everything downstream — the result snapshot, the settlement, the
/// response — reads both decks out of that one vector.
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

    let sp = execute_sp_midnight(codex, context, rng);
    let mut session = SortieBattleSession {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        friendly_ship_ids: sp
            .main_deck
            .iter()
            .chain(&sp.night.friendly)
            .map(|s| s.ship.api_id)
            .collect(),
        enemy_ship_ids: sp.night.enemy.iter().map(|s| s.ship.api_ship_id).collect(),
        packet: night_start_packet(&sp.night.packet, &sp.main_deck),
        friendly: sp.main_deck,
        enemy: Vec::new(),
        outcome: sp.night.outcome.clone(),
    };
    session.absorb_night(&sp.night);
    store.insert_pending_battle(profile_id, session.clone());

    let night_session = SortieNightBattleSession {
        profile_id,
        packet: sp.night.packet,
        outcome: sp.night.outcome,
    };

    (session, night_session)
}

/// The day-packet view of a night-start battle before the night is absorbed: no
/// day phase ran, so only `formation` and 第1艦隊's `nowhps` carry values.
///
/// `main_deck` is 第1艦隊 when the fleet is combined and empty otherwise; its
/// ships never fought, so they report the HP they entered the node with.
/// [`SortieBattleSession::absorb_night`] appends the night fleet after them.
fn night_start_packet(night: &NightBattlePacket, main_deck: &[BattleRuntimeShip]) -> BattlePacket {
    BattlePacket {
        formation: night.formation,
        friendly_nowhps: main_deck.iter().map(|s| s.hp().max(0)).collect(),
        enemy_nowhps: Vec::new(),
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
