//! Battle simulation orchestrators.
//!
//! This module provides the top-level `simulate_day` and `simulate_night` entry
//! points that compose the individual phase simulations (kouku, OASW, torpedo,
//! shelling, night hougeki) into complete battle simulations.

use emukc_model::codex::Codex;

use crate::combined::{CombinedFleetRole, CombinedType};
use crate::config::{BattleFlow, BattlePhaseKind};
use crate::random::BattleRng;
use crate::state::{BattleState, CombinedLayout};
use crate::targeting::{any_alive, can_closing_torpedo, can_opening_torpedo, fleet_has_bb_class};
use crate::types::{
    AirState, BattleContext, BattleHougeki, BattlePhase, BattleRuntimeShip, BattleSimulation,
    NightBattleInput, NightBattleSimulation, ShellingParams,
};

pub(crate) mod asw;
pub(crate) mod day_cutin;
pub(crate) mod kouku;
pub(crate) mod night;
pub(crate) mod shelling;
pub(crate) mod special_attack;
pub(crate) mod torpedo;

/// Returns the fleet speed value (minimum `api_soku` among alive ships).
/// `KanColle` speed values: 5=slow, 10=fast, 15=fast+, 20=fastest.
fn fleet_speed(fleet: &[BattleRuntimeShip]) -> i64 {
    fleet.iter().filter(|s| s.is_alive()).map(|s| s.ship.api_soku).min().unwrap_or(0)
}

/// Returns true if the friendly fleet is faster (enemy attacks first in shelling).
fn enemy_shells_first(friendly: &[BattleRuntimeShip], enemy: &[BattleRuntimeShip]) -> bool {
    fleet_speed(enemy) > fleet_speed(friendly)
}

/// Simulate a full day battle.
///
/// Selects the phase flow based on [`BattleType`](crate::types::BattleType), then
/// dispatches each phase in order. Runtime preconditions (planes, torpedo-capable ships,
/// alive counts) are checked within each phase arm.
///
/// The `rng` parameter is consumed sequentially across all phases (kouku → OASW →
/// opening torpedo → shelling → closing torpedo), so the same seed produces a
/// deterministic full battle result. Callers must NOT share the RNG instance across
/// separate battle simulations if determinism is required.
pub(crate) fn simulate_day(
    codex: &Codex,
    context: BattleContext,
    rng: &mut impl BattleRng,
) -> BattleSimulation {
    let mut state = BattleState::from_context(context);
    let flow = BattleFlow::for_battle_type(state.battle_type());
    let enemy_first = enemy_shells_first(&state.friendly, &state.enemy);

    let has_bb =
        fleet_has_bb_class(codex, &state.friendly) || fleet_has_bb_class(codex, &state.enemy);
    state.set_has_bb_class_at_start(has_bb);

    // A combined fleet reorders the shelling tail and scopes several phases to
    // one deck, so it gets its own orchestrator. The single-fleet arm below is
    // untouched, which is what guarantees the frozen golden transcript and every
    // existing seed still produce the same RNG stream.
    if let Some(layout) = state.combined() {
        simulate_day_combined(codex, &mut state, rng, layout, enemy_first);
        return state.finalize_day();
    }

    for &phase in flow.phases {
        match phase {
            BattlePhaseKind::Kouku => execute_kouku(codex, &mut state, rng),
            BattlePhaseKind::OpeningAsw => execute_opening_asw(codex, &mut state, rng),
            BattlePhaseKind::OpeningTorpedo => execute_opening_torpedo(codex, &mut state, rng),
            BattlePhaseKind::Shelling1 => execute_shelling1(codex, &mut state, rng, enemy_first),
            BattlePhaseKind::Shelling2 => execute_shelling2(codex, &mut state, rng, enemy_first),
            BattlePhaseKind::ClosingTorpedo => execute_closing_torpedo(codex, &mut state, rng),
        }
    }

    state.finalize_day()
}

/// Simulate a day battle for a friendly combined fleet against a single enemy
/// fleet.
///
/// Phase order comes from `docs/battle/combined-fleet-reference.md` §Phase
/// order, and which packet field carries which deck comes from its §Protocol
/// fields (ultimately `docs/apilist.txt`). The two shapes are mirror images:
///
/// - 空母機動 / 輸送護衛: deck 2 shelling → deck 2 torpedo → deck 1 shelling ×2,
///   landing in `hougeki1` → `raigeki` → `hougeki2` → `hougeki3`.
/// - 水上打撃: deck 1 shelling ×2 → deck 2 shelling → deck 2 torpedo, landing in
///   `hougeki1` → `hougeki2` → `hougeki3` → `raigeki`.
///
/// Deck 1 takes no part in the opening ASW, opening torpedo or closing torpedo
/// phases; that is enforced per ship inside those phase functions rather than by
/// slicing here, because the enemy still fires at *both* decks in them.
fn simulate_day_combined(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    layout: CombinedLayout,
    enemy_first: bool,
) {
    let flow = BattleFlow::for_battle_type(state.battle_type());
    let runs = |kind: BattlePhaseKind| flow.phases.contains(&kind);

    // Aerial combat draws on both decks' planes, and the opening phases already
    // filter deck 1 per ship, so all three reuse the single-fleet executors.
    if runs(BattlePhaseKind::Kouku) {
        execute_kouku(codex, state, rng);
    }
    if runs(BattlePhaseKind::OpeningAsw) {
        execute_opening_asw(codex, state, rng);
    }
    if runs(BattlePhaseKind::OpeningTorpedo) {
        execute_opening_torpedo(codex, state, rng);
    }

    if !runs(BattlePhaseKind::Shelling1) {
        return;
    }

    // The round order differs only in which deck fires when; the packet slots
    // are always filled 1, 2, 3 in the order the rounds happen.
    let rounds: [CombinedFleetRole; 3] = match layout.combined_type {
        CombinedType::CarrierTaskForce | CombinedType::TransportEscort => {
            [CombinedFleetRole::Escort, CombinedFleetRole::Main, CombinedFleetRole::Main]
        }
        CombinedType::SurfaceTaskForce => {
            [CombinedFleetRole::Main, CombinedFleetRole::Main, CombinedFleetRole::Escort]
        }
    };
    // 水上打撃 closes with the torpedo phase; the other two slot it between deck
    // 2's shelling and deck 1's.
    let torpedo_after_round = match layout.combined_type {
        CombinedType::CarrierTaskForce | CombinedType::TransportEscort => 0,
        CombinedType::SurfaceTaskForce => 2,
    };

    for (round, deck) in rounds.into_iter().enumerate() {
        // A deck's second consecutive round is the 2巡目, gated on a battleship
        // being present exactly as the single-fleet path gates `hougeki2`.
        let is_second_round = round > 0 && deck == rounds[round - 1];
        if !is_second_round || state.has_bb_class_at_start() {
            let hougeki = execute_combined_shelling(codex, state, rng, layout, deck, enemy_first);
            let happened = hougeki.is_some();
            match round {
                0 => state.set_hougeki1(hougeki),
                1 => state.set_hougeki2(hougeki),
                _ => state.set_hougeki3(hougeki),
            }
            if happened {
                state.set_hourai_flag(round, 1);
            }
        }

        // The torpedo phase keeps its slot even when the round before it was
        // gated away.
        if round == torpedo_after_round {
            execute_closing_torpedo(codex, state, rng);
        }
    }
}

/// One combined shelling round: the designated friendly deck and the enemy each
/// fire once, ordered by fleet speed.
///
/// This is where a combined round departs from the single-fleet path. There,
/// `hougeki1` and `hougeki2` each hold **one** side's attacks and the two sides
/// alternate across the rounds. A combined battle cannot use that scheme: with
/// three rounds and a fixed deck per round (see `docs/apilist.txt`), alternating
/// would cost one of the decks its shelling entirely. So both sides fire inside
/// every round and are merged into one `BattleHougeki`, told apart by
/// `api_at_eflag` — which is what the client expects in either case. The
/// single-fleet path keeps its own scheme untouched.
fn execute_combined_shelling(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    layout: CombinedLayout,
    deck: CombinedFleetRole,
    enemy_first: bool,
) -> Option<BattleHougeki> {
    let deck_range = match deck {
        CombinedFleetRole::Main => 0..layout.escort_start,
        CombinedFleetRole::Escort => layout.escort_start..state.friendly.len(),
    };
    if !any_alive(&state.friendly[deck_range.clone()]) || !any_alive(&state.enemy) {
        return None;
    }

    let friendly_form = state.friendly_formation_id();
    let enemy_form = state.enemy_formation_id();
    let eng = state.engagement();
    let air_state =
        state.kouku().and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));

    // `simulate_shelling_side` numbers its attackers from the start of the slice
    // it is handed, and deck 2's slice starts at `escort_start` — so its output
    // has to be lifted back into the whole-fleet space the rest of the packet
    // speaks. Deck 1's slice starts at 0 and the shift is a no-op.
    let deck_offset = deck_range.start;
    let friendly_turn = |state: &mut BattleState, rng: &mut _| {
        let mut round = shelling::simulate_shelling_side(
            codex,
            rng,
            &mut state.friendly[deck_range.clone()],
            &mut state.enemy,
            &ShellingParams {
                attacker_is_enemy: false,
                formation_id: friendly_form,
                defender_formation_id: enemy_form,
                engagement: eng,
                phase: BattlePhase::DayShelling,
                air_state: air_state.as_ref(),
            },
        );
        if let Some(round) = round.as_mut() {
            shift_friendly_attackers(round, deck_offset);
        }
        round
    };
    let enemy_turn = |state: &mut BattleState, rng: &mut _| {
        // The enemy fires at the whole friendly force, not just the deck whose
        // round this is.
        shelling::simulate_shelling_side(
            codex,
            rng,
            &mut state.enemy,
            &mut state.friendly,
            &ShellingParams {
                attacker_is_enemy: true,
                formation_id: enemy_form,
                defender_formation_id: friendly_form,
                engagement: eng,
                phase: BattlePhase::DayShelling,
                air_state: air_state.as_ref(),
            },
        )
    };

    let (first, second) = if enemy_first {
        let e = enemy_turn(state, rng);
        let f = friendly_turn(state, rng);
        (e, f)
    } else {
        let f = friendly_turn(state, rng);
        let e = enemy_turn(state, rng);
        (f, e)
    };

    merge_hougeki(first, second)
}

/// Lift every friendly attacker index in a round by `offset`.
///
/// Only the attacker end moves: a friendly attack's `api_df_list` holds enemy
/// indices, and an enemy attack was simulated against the whole friendly force,
/// so both are already in the right space.
fn shift_friendly_attackers(round: &mut BattleHougeki, offset: usize) {
    if offset == 0 {
        return;
    }
    let BattleHougeki {
        api_at_eflag,
        api_at_list,
        ..
    } = round;
    for (entry, &eflag) in api_at_eflag.iter().enumerate() {
        if eflag == 0
            && let Some(attacker) = api_at_list.get_mut(entry)
        {
            *attacker += offset as i64;
        }
    }
}

/// Concatenate two shelling rounds into one `BattleHougeki`, preserving order.
/// Every field is a parallel per-attack array, so a concatenation is all the
/// merge needs; `api_at_eflag` already records which side each attack came from.
fn merge_hougeki(
    first: Option<BattleHougeki>,
    second: Option<BattleHougeki>,
) -> Option<BattleHougeki> {
    match (first, second) {
        (Some(mut a), Some(b)) => {
            a.api_at_eflag.extend(b.api_at_eflag);
            a.api_at_list.extend(b.api_at_list);
            a.api_at_type.extend(b.api_at_type);
            a.api_df_list.extend(b.api_df_list);
            a.api_si_list.extend(b.api_si_list);
            a.api_cl_list.extend(b.api_cl_list);
            a.api_damage.extend(b.api_damage);
            Some(a)
        }
        (only @ Some(_), None) | (None, only) => only,
    }
}

fn execute_kouku(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if kouku::has_any_air_combat_planes(codex, &state.friendly)
        || kouku::has_any_air_combat_planes(codex, &state.enemy)
    {
        let kouku = kouku::simulate_kouku(codex, &mut state.friendly, &mut state.enemy, rng);
        state.set_stage_flag([1, 1, 1]);
        state.set_kouku(kouku);
    }
}

fn execute_opening_asw(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    let friendly_form = state.friendly_formation_id();
    let enemy_form = state.enemy_formation_id();
    let eng = state.engagement();
    let taisen = asw::simulate_opening_taisen(
        codex,
        rng,
        &mut state.friendly,
        &mut state.enemy,
        friendly_form,
        enemy_form,
        eng,
    );
    let has_taisen = taisen.is_some();
    state.set_opening_taisen(taisen);
    state.set_opening_taisen_flag(has_taisen);
}

fn execute_opening_torpedo(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if can_opening_torpedo(codex, &state.friendly) || can_opening_torpedo(codex, &state.enemy) {
        let friendly_form = state.friendly_formation_id();
        let enemy_form = state.enemy_formation_id();
        let eng = state.engagement();
        let attack = torpedo::simulate_opening_torpedo(
            codex,
            rng,
            &mut state.friendly,
            &mut state.enemy,
            friendly_form,
            enemy_form,
            eng,
        );
        state.set_opening_attack(attack);
        // `opening_attack` is advertised via the scalar `api_opening_flag`
        // (set in `to_packet` from `opening_attack.is_some()`). It must NOT also
        // touch `api_hourai_flag[0]` — that slot belongs to `api_hougeki1`
        // per the client-derived battle rules.
    }
}

fn execute_shelling1(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    enemy_first: bool,
) {
    if any_alive(&state.friendly) && any_alive(&state.enemy) {
        let friendly_form = state.friendly_formation_id();
        let enemy_form = state.enemy_formation_id();
        let eng = state.engagement();
        let air_state =
            state.kouku().and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));
        let hougeki = if enemy_first {
            shelling::simulate_shelling_side(
                codex,
                rng,
                &mut state.enemy,
                &mut state.friendly,
                &ShellingParams {
                    attacker_is_enemy: true,
                    formation_id: enemy_form,
                    defender_formation_id: friendly_form,
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
                    air_state: air_state.as_ref(),
                },
            )
        } else {
            shelling::simulate_shelling_side(
                codex,
                rng,
                &mut state.friendly,
                &mut state.enemy,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: friendly_form,
                    defender_formation_id: enemy_form,
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
                    air_state: air_state.as_ref(),
                },
            )
        };
        let has_hougeki = hougeki.is_some();
        state.set_hougeki1(hougeki);
        if has_hougeki {
            state.set_hourai_flag(0, 1);
        }
    }
}

fn execute_shelling2(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    enemy_first: bool,
) {
    if !state.has_bb_class_at_start() {
        return;
    }
    if any_alive(&state.friendly) && any_alive(&state.enemy) {
        let friendly_form = state.friendly_formation_id();
        let enemy_form = state.enemy_formation_id();
        let eng = state.engagement();
        let air_state =
            state.kouku().and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));
        // Shelling2 reverses the attack order: the side that went second in
        // Shelling1 attacks first here (KanColle alternating rule).
        let hougeki = if enemy_first {
            shelling::simulate_shelling_side(
                codex,
                rng,
                &mut state.friendly,
                &mut state.enemy,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: friendly_form,
                    defender_formation_id: enemy_form,
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
                    air_state: air_state.as_ref(),
                },
            )
        } else {
            shelling::simulate_shelling_side(
                codex,
                rng,
                &mut state.enemy,
                &mut state.friendly,
                &ShellingParams {
                    attacker_is_enemy: true,
                    formation_id: enemy_form,
                    defender_formation_id: friendly_form,
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
                    air_state: air_state.as_ref(),
                },
            )
        };
        let has_hougeki = hougeki.is_some();
        state.set_hougeki2(hougeki);
        if has_hougeki {
            state.set_hourai_flag(1, 1);
        }
    }
}

fn execute_closing_torpedo(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if any_alive(&state.friendly)
        && any_alive(&state.enemy)
        && (can_closing_torpedo(codex, &state.friendly) || can_closing_torpedo(codex, &state.enemy))
    {
        let friendly_form = state.friendly_formation_id();
        let enemy_form = state.enemy_formation_id();
        let eng = state.engagement();
        if let Some(round) = torpedo::simulate_raigeki(
            codex,
            rng,
            &mut state.friendly,
            &mut state.enemy,
            friendly_form,
            enemy_form,
            eng,
        ) {
            state.set_raigeki(Some(round));
            state.set_hourai_flag(3, 1);
        }
    }
}

/// Simulate a night battle.
pub(crate) fn simulate_night(
    codex: &Codex,
    input: NightBattleInput,
    rng: &mut impl BattleRng,
) -> NightBattleSimulation {
    let NightBattleInput {
        mut friendly,
        mut enemy,
        friendly_formation_id,
        enemy_formation_id,
        engagement,
        air_state,
        ..
    } = input;
    let entry_friendly_nowhps = friendly.iter().map(|ship| ship.hp().max(0)).collect::<Vec<_>>();
    let entry_friendly_maxhps = friendly.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
    let entry_enemy_nowhps = enemy.iter().map(|ship| ship.hp().max(0)).collect::<Vec<_>>();
    let entry_enemy_maxhps = enemy.iter().map(|ship| ship.ship.api_maxhp).collect::<Vec<_>>();
    let hougeki = night::simulate_night_hougeki(
        codex,
        rng,
        &mut friendly,
        &mut enemy,
        &crate::types::NightBattleParams {
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            air_state: air_state.as_ref(),
        },
    );

    // Build a minimal state for finalization
    let state = BattleState::for_night(
        friendly,
        enemy,
        friendly_formation_id,
        enemy_formation_id,
        engagement,
    );

    state.finalize_night(
        entry_friendly_nowhps,
        entry_friendly_maxhps,
        entry_enemy_nowhps,
        entry_enemy_maxhps,
        hougeki,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::SeededRng;
    use crate::test_utils::*;
    use crate::types::{BattleContext, BattleType};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::{KcShipType, KcSlotItemType3};

    #[test]
    fn sortie_day_battle_enables_midnight_when_both_sides_survive() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut friend = sample_ship(&codex, 79, 1);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, 412, 99);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert_eq!(simulation.packet.midnight_flag, 1);
        assert!(simulation.outcome.can_midnight);
    }

    #[test]
    fn fighter_only_carrier_participates_in_air_combat_but_deals_no_bombing_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let fighter_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);

        let mut carrier = sample_ship(&codex, carrier_mst, 50);
        carrier.slot_items = vec![slotitem_with_mst_id(fighter_id)];
        carrier.ship.api_onslot = [18, 0, 0, 0, 0];
        let enemy = sample_ship(&codex, dd_mst, 50);

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, false, vec![carrier], vec![enemy]),
            &mut rng,
        );

        let kouku = simulation.packet.kouku.unwrap();
        // Fighter-only carrier participates in air combat (api_plane_from includes it)
        // but deals no bombing damage in Stage 3.
        assert_eq!(kouku.api_plane_from[0], vec![1]);
        assert_eq!(kouku.api_stage3.api_edam.iter().sum::<i64>(), 0);
    }

    #[test]
    fn airbattle_mode_skips_shelling_and_torpedo() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let friend = sample_ship(&codex, bb_mst, 99);
        let enemy = sample_ship(&codex, dd_mst, 50);

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::AirBattle, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki1.is_none(), "airbattle should skip shelling");
        assert!(simulation.packet.hougeki2.is_none());
        assert!(simulation.packet.raigeki.is_none(), "airbattle should skip closing torpedo");
        assert!(
            simulation.packet.opening_attack.is_none(),
            "airbattle should skip opening torpedo"
        );
        assert_eq!(simulation.packet.hourai_flag, [0, 0, 0, 0]);
    }

    #[test]
    fn airbattle_mode_still_runs_kouku() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        let mut carrier = sample_ship(&codex, cvl_mst, 50);
        carrier.slot_items = vec![slotitem_with_mst_id(bomber_id)];
        carrier.ship.api_onslot = [18, 0, 0, 0, 0];

        let enemy = sample_ship(&codex, dd_mst, 50);

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::AirBattle, true, vec![carrier], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.kouku.is_some(), "airbattle should still run kouku");
        assert_eq!(simulation.packet.stage_flag, [1, 1, 1]);
    }

    #[test]
    fn faster_enemy_fleet_shells_first() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly: slow ship (soku=5)
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_soku = 5;
        friend.ship.api_karyoku[0] = 50;
        friend.ship.api_soukou[0] = 200;

        // Enemy: fast ship (soku=10)
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_soku = 10;
        enemy.ship.api_karyoku[0] = 50;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(42);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        // When enemy is faster, shelling1 should be enemy attacking, shelling2 should be friendly
        let h1 = simulation.packet.hougeki1.unwrap();
        assert_eq!(h1.api_at_eflag[0], 1, "enemy should attack first in shelling1");
        if let Some(h2) = &simulation.packet.hougeki2 {
            assert_eq!(h2.api_at_eflag[0], 0, "friendly should attack second in shelling2");
        }
    }

    #[test]
    fn equal_speed_friendly_shells_first() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Both fast
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_soku = 10;
        friend.ship.api_karyoku[0] = 50;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_soku = 10;
        enemy.ship.api_karyoku[0] = 50;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(42);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        // Equal speed: friendly goes first (default)
        let h1 = simulation.packet.hougeki1.unwrap();
        assert_eq!(h1.api_at_eflag[0], 0, "friendly should attack first on equal speed");
    }

    #[test]
    fn fleet_speed_returns_min_alive() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship1 = sample_ship(&codex, dd_mst, 99);
        ship1.ship.api_soku = 10;
        let mut ship2 = sample_ship(&codex, dd_mst, 99);
        ship2.ship.api_soku = 5;
        let mut ship3 = sample_ship(&codex, dd_mst, 99);
        ship3.ship.api_soku = 15;
        ship3.ship.api_nowhp = 0; // sunk

        let fleet: Vec<BattleRuntimeShip> = vec![
            BattleRuntimeShip::from(ship1),
            BattleRuntimeShip::from(ship2),
            BattleRuntimeShip::from(ship3),
        ];
        assert_eq!(fleet_speed(&fleet), 5, "fleet speed is min of alive ships");
    }

    #[test]
    fn day_battle_display_damage_consistent_under_protection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly DD at low HP but NOT taiha (HP > 25% max), zero armor
        let mut dd = sample_ship(&codex, dd_mst, 50);
        dd.ship.api_soukou[0] = 0;
        dd.ship.api_nowhp = 8;
        dd.ship.api_maxhp = 30;
        let dd_hp_before = dd.ship.api_nowhp;

        // Enemy DDs with high firepower
        let mut enemy1 = sample_ship(&codex, dd_mst, 99);
        enemy1.ship.api_karyoku[0] = 200;
        enemy1.ship.api_soukou[0] = 0;
        let mut enemy2 = sample_ship(&codex, dd_mst, 99);
        enemy2.ship.api_karyoku[0] = 200;
        enemy2.ship.api_soukou[0] = 0;

        let mut rng = SeededRng::new(42);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![dd], vec![enemy1, enemy2]),
            &mut rng,
        );

        // Verify damage was actually dealt
        let dd_hp_after = simulation.friendly[0].hp();
        let dd_actual_lost = dd_hp_before - dd_hp_after;
        assert!(dd_actual_lost > 0, "enemy (karyoku=200) must deal damage to zero-armor DD");

        // The DD must survive (sinking protection)
        assert!(
            simulation.friendly[0].hp() > 0,
            "friendly DD must survive day battle under sinking protection"
        );

        // The DD's actual HP loss should be achievable without sinking
        assert!(
            dd_actual_lost < dd_hp_before,
            "DD actual HP loss ({dd_actual_lost}) must be less than entry HP ({dd_hp_before})"
        );
    }

    #[test]
    fn day_battle_all_friendly_survive_under_protection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly ships at low HP but NOT taiha (HP > 25% max)
        // taiha threshold: entry_hp * 4 <= maxhp → must have entry_hp > maxhp/4
        // maxhp varies by DD; use entry_hp high enough to be above taiha threshold
        let friend_ships: Vec<_> = (0..3)
            .map(|_| {
                let mut s = sample_ship(&codex, dd_mst, 50);
                s.ship.api_soukou[0] = 0;
                s.ship.api_nowhp = s.ship.api_maxhp / 4 + 1; // just above taiha
                s
            })
            .collect();

        // Record entry HP before battle
        let entry_hps: Vec<i64> = friend_ships.iter().map(|s| s.ship.api_nowhp).collect();

        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 200;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(42);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, friend_ships, vec![enemy]),
            &mut rng,
        );

        let mut any_damage = false;
        for (i, ship) in simulation.friendly.iter().enumerate() {
            assert!(
                ship.hp() > 0,
                "friendly ship {i} must survive day battle under sinking protection (non-taiha entry)"
            );
            if ship.hp() < entry_hps[i] {
                any_damage = true;
            }
        }
        assert!(any_damage, "enemy (karyoku=200) must deal at least some damage");
    }

    #[test]
    fn shelling2_no_bb_no_second_round() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, dd_mst, 50);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki2.is_none(), "no BB on either side → no shelling2");
        assert_eq!(simulation.packet.hourai_flag[2], 0);
    }

    #[test]
    fn shelling2_friendly_bb_triggers_second_round() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, bb_mst, 50);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki2.is_some(), "friendly BB → shelling2 runs");
        assert_eq!(simulation.packet.hourai_flag[1], 1);
    }

    #[test]
    fn shelling2_enemy_bb_triggers_second_round() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, dd_mst, 50);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, bb_mst, 50);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki2.is_some(), "enemy BB → shelling2 runs");
    }

    #[test]
    fn shelling2_cvl_no_bb_no_second_round() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, cvl_mst, 50);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki2.is_none(), "CVL is not BB-class → no shelling2");
    }

    #[test]
    fn shelling2_fbb_triggers_second_round() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fbb_mst = first_ship_mst_by_type(&codex, KcShipType::FBB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, fbb_mst, 50);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki2.is_some(), "FBB → shelling2 runs");
    }

    #[test]
    fn shelling2_bbv_triggers_second_round() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, bbv_mst, 50);
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        assert!(simulation.packet.hougeki2.is_some(), "BBV → shelling2 runs");
    }

    #[test]
    fn shelling2_fires_after_enemy_bb_sunk_in_shelling1() {
        // has_bb_class_at_start is a battle-start snapshot: even if the BB is sunk
        // during Shelling1, Shelling2 still executes.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly DD with high firepower to kill enemy BB in one hit
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 300;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 200;

        // Enemy BB with zero armor → vulnerable to one-shot kill
        let mut enemy_bb = sample_ship(&codex, bb_mst, 1);
        enemy_bb.ship.api_soukou[0] = 0;
        enemy_bb.ship.api_raisou[0] = 0;

        // Enemy DD with high armor → survives Shelling1
        let mut enemy_dd = sample_ship(&codex, dd_mst, 50);
        enemy_dd.ship.api_karyoku[0] = 1;
        enemy_dd.ship.api_raisou[0] = 0;
        enemy_dd.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(1);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(
                BattleType::Normal,
                false,
                vec![friend],
                vec![enemy_bb, enemy_dd],
            ),
            &mut rng,
        );

        // Enemy BB must be dead (no sinking protection for enemy side)
        assert!(
            simulation.enemy[0].hp() <= 0,
            "enemy BB should be sunk after Shelling1 (zero armor, high firepower hit)"
        );
        // But Shelling2 still fires — the snapshot was taken at battle start
        assert!(
            simulation.packet.hougeki2.is_some(),
            "Shelling2 fires even after enemy BB is sunk (battle-start snapshot)"
        );
        assert_eq!(simulation.packet.hourai_flag[1], 1);
    }

    #[test]
    fn closing_torpedo_rejects_chuha_dd_through_pipeline() {
        // A DD damaged to chūha in Shelling1 should be excluded from closing torpedo.
        // Enemy is faster → enemy fires in Shelling1. Friendly DD has zero armor and
        // high maxhp (200), so the ~150 damage from enemy karyoku=200 leaves it at
        // chūha (~50/200) without triggering sinking protection (damage < current HP).
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly DD: high raisou, zero armor, inflated maxhp to avoid sinking protection
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_raisou[0] = 80;
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_maxhp = 200;
        friend.ship.api_nowhp = 200;

        // Enemy DD: fast (speed=15), extreme firepower, high raisou
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_soku = 15;
        enemy.ship.api_karyoku[0] = 200;
        enemy.ship.api_raisou[0] = 80;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(7);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        // Friendly DD must survive at chūha
        let friendly_hp = simulation.friendly[0].hp();
        let friendly_maxhp = simulation.friendly[0].ship.api_maxhp;
        assert!(friendly_hp > 0, "DD should survive: hp={}", friendly_hp);
        assert!(
            friendly_hp * 2 <= friendly_maxhp,
            "DD should be chūha: hp={}, maxhp={}",
            friendly_hp,
            friendly_maxhp,
        );

        // Closing torpedo fires (enemy DD participates)
        let raigeki = simulation.packet.raigeki.as_ref();
        assert!(raigeki.is_some(), "closing torpedo should fire (enemy DD is healthy)");
        let r = raigeki.unwrap();
        // Friendly DD (index 0) did not fire → api_frai[0] == -1
        assert_eq!(r.api_frai[0], -1, "chūha DD should not participate in closing torpedo");
        // Enemy DD (index 0) did fire → api_erai[0] has a valid target
        assert!(r.api_erai[0] >= 0, "enemy DD should fire in closing torpedo");
    }

    #[test]
    fn closing_torpedo_accepts_shoha_dd_through_pipeline() {
        // Regression: DD at shōha (> 50% HP) should still participate in closing torpedo.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly DD: high raisou, some armor to stay shōha
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_raisou[0] = 80;
        friend.ship.api_karyoku[0] = 1;
        friend.ship.api_soukou[0] = 200;

        // Enemy DD: low firepower, high raisou
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_raisou[0] = 80;
        enemy.ship.api_soukou[0] = 200;

        let mut rng = SeededRng::new(7);
        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
            &mut rng,
        );

        let friendly_hp = simulation.friendly[0].hp();
        let friendly_maxhp = simulation.friendly[0].ship.api_maxhp;

        // karyoku=1 vs soukou=200 guarantees scratch damage → DD stays shōha
        assert!(
            friendly_hp * 2 > friendly_maxhp,
            "DD should be shōha for this regression guard: hp={}, maxhp={}",
            friendly_hp,
            friendly_maxhp,
        );

        let raigeki = simulation.packet.raigeki.as_ref();
        assert!(raigeki.is_some(), "closing torpedo should fire");
        let r = raigeki.unwrap();
        assert!(r.api_frai[0] >= 0, "shōha DD should participate in closing torpedo");
    }
}

#[cfg(test)]
mod combined_tests {
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;

    use crate::combined::CombinedType;
    use crate::random::SeededRng;
    use crate::test_utils::{first_ship_mst_by_type, sample_ship};
    use crate::types::{
        BattleContext, BattleHougeki, BattleSimulation, BattleType, CombinedSetup, EngagementType,
    };

    /// Deck 1 is submarines and deck 2 destroyers, so "which deck shelled" is
    /// directly observable: `can_shell_day_ship` rejects submarines, therefore a
    /// round with friendly attacks in it can only be deck 2's.
    fn combined_sim(codex: &Codex, combined_type: CombinedType) -> BattleSimulation {
        let ss = first_ship_mst_by_type(codex, KcShipType::SS);
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);

        let context = BattleContext {
            battle_type: BattleType::Normal,
            is_sortie: true,
            // 第一警戒航行序列; the enemy keeps a normal formation.
            friendly_formation_id: 11,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships: vec![sample_ship(codex, ss, 99), sample_ship(codex, ss, 99)],
            enemy_ships: vec![sample_ship(codex, dd, 50), sample_ship(codex, dd, 50)],
            combined: Some(CombinedSetup {
                combined_type,
                escort_ships: vec![sample_ship(codex, dd, 99), sample_ship(codex, dd, 99)],
            }),
        };

        super::simulate_day(codex, context, &mut SeededRng::new(42))
    }

    /// Number of attacks the friendly side made in a shelling round.
    /// `api_at_eflag` is 0 for a friendly attacker, 1 for an enemy one.
    fn friendly_attacks(round: Option<&BattleHougeki>) -> usize {
        round.map_or(0, |h| h.api_at_eflag.iter().filter(|&&flag| flag == 0).count())
    }

    /// Number of friendly ships that launched an opening torpedo. Each entry of
    /// `api_frai_list_items` is that attacker's target list, `None` when it did
    /// not fire.
    fn opening_friendly_shots(sim: &BattleSimulation) -> usize {
        sim.packet
            .opening_attack
            .as_ref()
            .map_or(0, |o| o.api_frai_list_items.iter().filter(|targets| targets.is_some()).count())
    }

    /// Deck 2 is shelled as a sub-slice, so `simulate_shelling_side` numbers its
    /// attackers from 0. By the time the packet leaves the simulation they must
    /// be at 6 or above: the client reads anything below 6 as 第1艦隊 and would
    /// credit every deck 2 hit to the wrong ship.
    #[test]
    fn escort_deck_attacks_reach_the_packet_in_client_index_space() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let sim = combined_sim(&codex, CombinedType::CarrierTaskForce);

        let round = sim.packet.hougeki1.as_ref().expect("deck 2 shells in hougeki1");
        let attackers = round
            .api_at_eflag
            .iter()
            .zip(round.api_at_list.iter())
            .filter(|(eflag, _)| **eflag == 0)
            .map(|(_, attacker)| *attacker)
            .collect::<Vec<_>>();

        assert!(!attackers.is_empty(), "deck 2's destroyers must shell");
        for index in attackers {
            assert!(index >= 6, "deck 2 attacker {index} must sit at packet index 6 or above");
        }
    }

    /// 空母機動部隊: deck 2 shells first (`hougeki1`), deck 1 follows
    /// (`hougeki2`). Neither fleet here holds a battleship, so the 2巡目
    /// (`hougeki3`) is gated away exactly as it is for a single fleet.
    #[test]
    fn carrier_task_force_shells_escort_deck_first() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let sim = combined_sim(&codex, CombinedType::CarrierTaskForce);

        assert!(
            friendly_attacks(sim.packet.hougeki1.as_ref()) > 0,
            "hougeki1 is deck 2's round; its destroyers must shell"
        );
        assert_eq!(
            friendly_attacks(sim.packet.hougeki2.as_ref()),
            0,
            "hougeki2 is deck 1's round, and submarines cannot shell"
        );
        assert!(
            sim.packet.hougeki3.is_none(),
            "no battleship on either side, so the second round is skipped"
        );
    }

    /// 水上打撃部隊 is the mirror image: deck 1 takes `hougeki1`/`hougeki2` and
    /// deck 2 drops to `hougeki3`. Same fleets, same seed — only the order moves.
    #[test]
    fn surface_task_force_shells_main_deck_first() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let sim = combined_sim(&codex, CombinedType::SurfaceTaskForce);

        assert_eq!(
            friendly_attacks(sim.packet.hougeki1.as_ref()),
            0,
            "hougeki1 is deck 1's round, and submarines cannot shell"
        );
        assert!(
            sim.packet.hougeki2.is_none(),
            "hougeki2 is deck 1's 2巡目; no battleship, so it is skipped"
        );
        assert!(
            friendly_attacks(sim.packet.hougeki3.as_ref()) > 0,
            "hougeki3 is deck 2's round; its destroyers must shell"
        );
    }

    /// R5: deck 1 takes no part in the opening torpedo phase. These submarines
    /// would all fire if they were a single fleet — `opening_torpedo_fires_for_a_
    /// single_fleet_of_the_same_ships` below proves they can.
    #[test]
    fn main_deck_does_not_open_with_torpedoes() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        for ty in [CombinedType::CarrierTaskForce, CombinedType::SurfaceTaskForce] {
            let sim = combined_sim(&codex, ty);
            assert_eq!(
                opening_friendly_shots(&sim),
                0,
                "{ty:?}: deck 1 must not open with torpedoes"
            );
        }
    }

    /// The control for the test above: the very same submarines, sortied as an
    /// ordinary single fleet, do open with torpedoes. Without this the previous
    /// test would also pass if opening torpedoes were broken outright.
    #[test]
    fn opening_torpedo_fires_for_a_single_fleet_of_the_same_ships() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        let context = BattleContext::head_on(
            BattleType::Normal,
            true,
            vec![sample_ship(&codex, ss, 99), sample_ship(&codex, ss, 99)],
            vec![sample_ship(&codex, dd, 50), sample_ship(&codex, dd, 50)],
        );
        let sim = super::simulate_day(&codex, context, &mut SeededRng::new(42));

        assert!(
            opening_friendly_shots(&sim) > 0,
            "single-fleet submarines must open with torpedoes"
        );
    }
}
