//! Battle simulation orchestrators.
//!
//! This module provides the top-level `simulate_day` and `simulate_night` entry
//! points that compose the individual phase simulations (kouku, OASW, torpedo,
//! shelling, night hougeki) into complete battle simulations.

use emukc_model::codex::Codex;

use crate::combined::{CombinedFleetRole, CombinedType};
use crate::config::{BattleFlow, BattlePhaseKind};
use crate::random::BattleRng;
use crate::simulation::shelling::ShellingRound;
use crate::state::{BattleState, CombinedLayout};
use crate::targeting::{any_alive, can_closing_torpedo, can_opening_torpedo, fleet_has_bb_class};
use crate::types::{
    AirState, BattleContext, BattleHougeki, BattleSimulation, NightBattleInput,
    NightBattleSimulation,
};

pub(crate) mod air_base;
pub(crate) mod air_raid;
pub(crate) mod asw;
pub(crate) mod day_attack;
pub(crate) mod day_cutin;
pub(crate) mod kouku;
pub(crate) mod night;
pub(crate) mod shelling;
pub(crate) mod special_attack;
pub(crate) mod torpedo;

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

    let has_bb =
        fleet_has_bb_class(codex, &state.friendly) || fleet_has_bb_class(codex, &state.enemy);
    state.set_has_bb_class_at_start(has_bb);

    // A combined fleet reorders the shelling tail and scopes several phases to
    // one deck, so it gets its own orchestrator.
    if let Some(layout) = state.combined() {
        simulate_day_combined(codex, &mut state, rng, layout);
        return state.finalize_day();
    }

    // Likewise an enemy combined fleet, which is fought one deck at a time.
    if let Some(escort_start) = state.enemy_escort_start() {
        simulate_day_enemy_combined(codex, &mut state, rng, escort_start);
        return state.finalize_day();
    }

    // Gated on there being an air corps at all, so a battle without one draws
    // exactly the random numbers it always did.
    if !state.air_corps.is_empty() {
        execute_air_base_attacks(codex, &mut state, rng);
    }

    for &phase in flow.phases {
        match phase {
            BattlePhaseKind::Kouku => execute_kouku(codex, &mut state, rng),
            BattlePhaseKind::OpeningAsw => execute_opening_asw(codex, &mut state, rng),
            BattlePhaseKind::OpeningTorpedo => execute_opening_torpedo(codex, &mut state, rng),
            BattlePhaseKind::Shelling1 => execute_shelling(codex, &mut state, rng, true),
            BattlePhaseKind::Shelling2 => execute_shelling(codex, &mut state, rng, false),
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
            let friendly_deck = match deck {
                CombinedFleetRole::Main => 0..layout.escort_start,
                CombinedFleetRole::Escort => layout.escort_start..state.friendly.len(),
            };
            let enemy_deck = 0..state.enemy.len();
            let hougeki =
                shelling_round(codex, state, rng, friendly_deck, enemy_deck, !is_second_round);
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
        // gated away — but only for a battle type that has one at all. A
        // レーダー射撃マス is shelling and nothing else.
        if round == torpedo_after_round && runs(BattlePhaseKind::ClosingTorpedo) {
            execute_closing_torpedo(codex, state, rng);
        }
    }
}

/// One round of shelling between part of each fleet, the first kind of round
/// (`by_range`) or the second. `None` when either part has nobody left.
///
/// Both sides fire inside every round, ship by ship, into one `BattleHougeki`
/// that tells them apart by `api_at_eflag`.
fn shelling_round(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    friendly_deck: std::ops::Range<usize>,
    enemy_deck: std::ops::Range<usize>,
    by_range: bool,
) -> Option<BattleHougeki> {
    if !any_alive(&state.friendly[friendly_deck.clone()])
        || !any_alive(&state.enemy[enemy_deck.clone()])
    {
        return None;
    }
    let air_state =
        state.kouku().and_then(|k| AirState::from_api_disp_seiku(k.api_stage1.api_disp_seiku));
    let round = ShellingRound {
        friendly_deck,
        enemy_deck,
        by_range,
        friendly_formation: state.friendly_formation_id(),
        enemy_formation: state.enemy_formation_id(),
        engagement: state.engagement(),
        air_state: air_state.as_ref(),
    };
    shelling::simulate_shelling_round(
        codex,
        rng,
        &mut state.friendly,
        &mut state.enemy,
        &round,
        &mut state.special_attack_used,
    )
}

/// Simulate a day battle for a friendly single fleet against an enemy combined
/// fleet (`ec_battle`).
///
/// Phase order from `docs/battle/combined-fleet-reference.md` §Friendly single
/// vs enemy combined: the opening phases, then shelling against the enemy
/// escort fleet, the torpedo phase, shelling against the enemy main fleet, and
/// — battleship condition — one more round against everything. They land in
/// `hougeki1` → `raigeki` → `hougeki2` → `hougeki3`.
///
/// Both enemy decks take part in the opening phases; the closing torpedo is the
/// escort fleet's alone, which `torpedo::simulate_raigeki` enforces per ship.
fn simulate_day_enemy_combined(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    escort_start: usize,
) {
    let flow = BattleFlow::for_battle_type(state.battle_type());
    let runs = |kind: BattlePhaseKind| flow.phases.contains(&kind);

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

    let enemy_len = state.enemy.len();
    let rounds = [escort_start..enemy_len, 0..escort_start, 0..enemy_len];
    for (round, deck) in rounds.into_iter().enumerate() {
        if round < 2 || state.has_bb_class_at_start() {
            let friendly_deck = 0..state.friendly.len();
            let hougeki = shelling_round(codex, state, rng, friendly_deck, deck, round < 2);
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
        if round == 0 && runs(BattlePhaseKind::ClosingTorpedo) {
            execute_closing_torpedo(codex, state, rng);
        }
    }
}

/// Fly every air corps sent against this cell, one attack for each time it was
/// pointed here, in air corps order. Losses carry from one attack to the next.
fn execute_air_base_attacks(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    let mut air_corps = std::mem::take(&mut state.air_corps);
    for corps in &mut air_corps {
        for _ in 0..corps.waves {
            if !any_alive(&state.enemy) {
                break;
            }
            let attack = air_base::simulate_air_base_attack(codex, corps, &mut state.enemy, rng);
            state.air_base_attack.push(attack);
        }
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

/// A single fleet's shelling round: the first, ordered by range, or the second,
/// which is fought down the line and only when a battleship was there to begin
/// with.
fn execute_shelling(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    first_round: bool,
) {
    if !first_round && !state.has_bb_class_at_start() {
        return;
    }
    let (friendly_deck, enemy_deck) = (0..state.friendly.len(), 0..state.enemy.len());
    let hougeki = shelling_round(codex, state, rng, friendly_deck, enemy_deck, first_round);
    let slot = usize::from(!first_round);
    if hougeki.is_some() {
        state.set_hourai_flag(slot, 1);
    }
    if first_round {
        state.set_hougeki1(hougeki);
    } else {
        state.set_hougeki2(hougeki);
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
        assert_eq!(kouku.api_stage3.api_edam.iter().map(|d| d.amount()).sum::<i64>(), 0);
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

    /// A ship nobody can sink, so that a round is fought to its end.
    fn armoured(codex: &Codex, ship_type: KcShipType) -> crate::BattleShipInput {
        let mut ship = sample_ship(codex, first_ship_mst_by_type(codex, ship_type), 99);
        ship.ship.api_karyoku[0] = 50;
        ship.ship.api_soukou[0] = 400;
        ship
    }

    #[test]
    fn both_sides_shell_in_turn_with_no_battleship_about() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fleet = || vec![armoured(&codex, KcShipType::DD); 3];

        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, fleet(), fleet()),
            &mut SeededRng::new(42),
        );

        let round = simulation.packet.hougeki1.unwrap();
        assert_eq!(round.api_at_eflag, [0, 1, 0, 1, 0, 1], "friendly first, then ship by ship");
        assert!(simulation.packet.hougeki2.is_none(), "no battleship, no second round");
    }

    #[test]
    fn the_first_round_goes_by_range_and_the_second_down_the_line() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut short = armoured(&codex, KcShipType::DD);
        short.ship.api_leng = 1;
        short.slot_items.clear();
        let mut long = armoured(&codex, KcShipType::BB);
        long.ship.api_leng = 4;

        let simulation = simulate_day(
            &codex,
            BattleContext::head_on(
                BattleType::Normal,
                true,
                vec![short, long],
                vec![armoured(&codex, KcShipType::DD); 2],
            ),
            &mut SeededRng::new(42),
        );

        let friendly_order = |round: &BattleHougeki| -> Vec<i64> {
            round
                .api_at_eflag
                .iter()
                .zip(&round.api_at_list)
                .filter(|(eflag, _)| **eflag == 0)
                .map(|(_, attacker)| *attacker)
                .collect()
        };
        assert_eq!(friendly_order(simulation.packet.hougeki1.as_ref().unwrap()), [1, 0]);
        assert_eq!(friendly_order(simulation.packet.hougeki2.as_ref().unwrap()), [0, 1]);
    }

    #[test]
    fn a_ship_sunk_before_its_turn_does_not_fire() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut friend = armoured(&codex, KcShipType::DD);
        friend.ship.api_karyoku[0] = 500;
        let mut enemy = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 1);
        enemy.ship.api_kaihi[0] = 0;
        enemy.ship.api_lucky[0] = 0;

        // Whatever the seed lands on, the enemy never fires after it is sunk.
        for seed in 0..20 {
            let simulation = simulate_day(
                &codex,
                BattleContext::head_on(
                    BattleType::Normal,
                    true,
                    vec![friend.clone()],
                    vec![enemy.clone()],
                ),
                &mut SeededRng::new(seed),
            );
            let round = simulation.packet.hougeki1.unwrap();
            let sunk = round.api_cl_list[0][0] > 0;
            assert_eq!(
                round.api_at_eflag.len(),
                if sunk {
                    1
                } else {
                    2
                },
                "seed {seed}"
            );
        }
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

        let mut rng = SeededRng::new(45);
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

        // Enemy DD: extreme firepower, high raisou
        let mut enemy = sample_ship(&codex, dd_mst, 99);
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
    /// 敵連合: each shelling round is fought against the deck it belongs to, and
    /// the packet speaks the client's index space — the enemy escort fleet at
    /// 6..=11 even though the main fleet here holds only three ships.
    #[test]
    fn enemy_combined_rounds_target_one_deck_each_in_client_indices() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        // Nobody can sink anybody, so every round and the torpedo phase happen.
        let tank = |mst: i64| {
            let mut ship = sample_ship(&codex, mst, 50);
            ship.ship.api_karyoku[0] = 1;
            ship.ship.api_raisou[0] = 1;
            ship.ship.api_soukou[0] = 300;
            ship
        };

        let context = BattleContext {
            battle_type: BattleType::Normal,
            is_sortie: true,
            friendly_formation_id: 1,
            enemy_formation_id: 13,
            engagement: EngagementType::SameCourse,
            friend_ships: vec![tank(bb), tank(dd)],
            enemy_ships: vec![tank(dd); 3],
            enemy_escort_ships: vec![tank(dd); 2],
            air_corps: Vec::new(),
            combined: None,
        };
        let sim = super::simulate_day(&codex, context, &mut SeededRng::new(7));

        assert_eq!(sim.enemy.len(), 5, "both decks stay in one vector");
        assert_eq!(sim.packet.enemy_nowhps.len(), 5);
        assert!(sim.enemy[..3].iter().all(crate::types::BattleRuntimeShip::is_main_deck));
        assert!(sim.enemy[3..].iter().all(crate::types::BattleRuntimeShip::is_escort_deck));

        // Every enemy index a round mentions, on either end of an attack.
        let enemy_indices = |round: &BattleHougeki| -> Vec<i64> {
            round
                .api_at_eflag
                .iter()
                .enumerate()
                .flat_map(|(entry, &eflag)| {
                    if eflag == 1 {
                        vec![round.api_at_list[entry]]
                    } else {
                        round.api_df_list[entry].clone()
                    }
                })
                .collect()
        };
        let vs_escort = enemy_indices(sim.packet.hougeki1.as_ref().expect("round vs escort"));
        let vs_main = enemy_indices(sim.packet.hougeki2.as_ref().expect("round vs main"));
        let vs_all = enemy_indices(sim.packet.hougeki3.as_ref().expect("battleship round"));
        assert!(vs_escort.iter().all(|i| (6..8).contains(i)), "{vs_escort:?}");
        assert!(vs_main.iter().all(|i| (0..3).contains(i)), "{vs_main:?}");
        assert!(vs_all.iter().all(|i| (0..3).contains(i) || (6..8).contains(i)), "{vs_all:?}");
        assert!(vs_all.iter().any(|i| *i >= 6) && vs_all.iter().any(|i| *i < 3), "{vs_all:?}");
        assert_eq!(sim.packet.hourai_flag, [1, 1, 1, 1]);

        let raigeki = sim.packet.raigeki.as_ref().expect("closing torpedo");
        assert_eq!(raigeki.api_erai.len(), 12, "enemy arrays span both decks");
        assert_eq!(raigeki.api_edam.len(), 12);
        assert_eq!(raigeki.api_frai.len(), 2, "friendly arrays are the fleet's own size");
        assert!(
            raigeki.api_erai[..6].iter().all(|t| *t == -1),
            "the enemy main fleet does not close with torpedoes: {:?}",
            raigeki.api_erai
        );
        assert!(raigeki.api_erai[6..8].iter().any(|t| *t >= 0), "{:?}", raigeki.api_erai);
        assert!(
            raigeki.api_frai.iter().all(|t| *t < 3 || (6..8).contains(t)),
            "{:?}",
            raigeki.api_frai
        );
    }

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
            enemy_escort_ships: Vec::new(),
            air_corps: Vec::new(),
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

    /// Deck 2 is shelled as a sub-slice, whose attackers are numbered from 0. By the time the packet leaves the simulation they must
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
