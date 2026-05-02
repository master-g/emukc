//! Battle simulation orchestrators.
//!
//! This module provides the top-level `simulate_day` and `simulate_night` entry
//! points that compose the individual phase simulations (kouku, OASW, torpedo,
//! shelling, night hougeki) into complete battle simulations.

use emukc_model::codex::Codex;

use crate::config::{BattleFlow, BattlePhaseKind};
use crate::random::BattleRng;
use crate::state::BattleState;
use crate::targeting::{any_alive, can_closing_torpedo, can_opening_torpedo};
use crate::types::{
    BattleContext, BattlePhase, BattleRuntimeShip, BattleSimulation, EngagementType,
    NightBattleSimulation, ShellingParams,
};

pub(crate) mod asw;
pub(crate) mod kouku;
pub(crate) mod night;
pub(crate) mod shelling;
pub(crate) mod torpedo;

/// Simulate a full day battle.
///
/// Selects the phase flow based on [`BattleType`](crate::types::BattleType), then
/// dispatches each phase in order. Runtime preconditions (planes, torpedo-capable ships,
/// alive counts) are checked within each phase arm.
pub fn simulate_day(
    codex: &Codex,
    context: BattleContext,
    rng: &mut impl BattleRng,
) -> BattleSimulation {
    let mut state = BattleState::from_context(context);
    let flow = BattleFlow::for_battle_type(state.battle_type);

    for &phase in flow.phases {
        match phase {
            BattlePhaseKind::Kouku => execute_kouku(codex, &mut state, rng),
            BattlePhaseKind::OpeningAsw => execute_opening_asw(codex, &mut state, rng),
            BattlePhaseKind::OpeningTorpedo => execute_opening_torpedo(codex, &mut state, rng),
            BattlePhaseKind::Shelling1 => execute_shelling1(codex, &mut state, rng),
            BattlePhaseKind::Shelling2 => execute_shelling2(codex, &mut state, rng),
            BattlePhaseKind::ClosingTorpedo => execute_closing_torpedo(codex, &mut state, rng),
        }
    }

    state.finalize_day()
}

fn execute_kouku(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if kouku::has_any_air_combat_planes(codex, &state.friendly)
        || kouku::has_any_air_combat_planes(codex, &state.enemy)
    {
        state.stage_flag = [1, 1, 1];
        state.kouku =
            Some(kouku::simulate_kouku(codex, &mut state.friendly, &mut state.enemy, rng));
    }
}

fn execute_opening_asw(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    state.opening_taisen = asw::simulate_opening_taisen(
        codex,
        rng,
        &mut state.friendly,
        &mut state.enemy,
        state.friendly_formation_id,
        state.enemy_formation_id,
        state.engagement,
    );
    state.opening_taisen_flag = i64::from(state.opening_taisen.is_some());
}

fn execute_opening_torpedo(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if can_opening_torpedo(codex, &state.friendly) || can_opening_torpedo(codex, &state.enemy) {
        state.opening_attack = torpedo::simulate_opening_torpedo(
            codex,
            rng,
            &mut state.friendly,
            &mut state.enemy,
            state.friendly_formation_id,
            state.enemy_formation_id,
            state.engagement,
        );
        if state.opening_attack.is_some() {
            state.hourai_flag[0] = 1;
        }
    }
}

fn execute_shelling1(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    state.hougeki1 = shelling::simulate_shelling_side(
        codex,
        rng,
        &mut state.friendly,
        &mut state.enemy,
        &ShellingParams {
            attacker_is_enemy: false,
            formation_id: state.friendly_formation_id,
            engagement: state.engagement,
            phase: BattlePhase::DayShelling,
        },
    );
    if state.hougeki1.is_some() {
        state.hourai_flag[0] = 1;
    }
}

fn execute_shelling2(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if any_alive(&state.friendly) && any_alive(&state.enemy) {
        state.hougeki2 = shelling::simulate_shelling_side(
            codex,
            rng,
            &mut state.enemy,
            &mut state.friendly,
            &ShellingParams {
                attacker_is_enemy: true,
                formation_id: state.enemy_formation_id,
                engagement: state.engagement,
                phase: BattlePhase::DayShelling,
            },
        );
        if state.hougeki2.is_some() {
            state.hourai_flag[1] = 1;
        }
    }
}

fn execute_closing_torpedo(codex: &Codex, state: &mut BattleState, rng: &mut impl BattleRng) {
    if any_alive(&state.friendly)
        && any_alive(&state.enemy)
        && (can_closing_torpedo(codex, &state.friendly) || can_closing_torpedo(codex, &state.enemy))
        && let Some(round) = torpedo::simulate_raigeki(
            codex,
            rng,
            &mut state.friendly,
            &mut state.enemy,
            state.friendly_formation_id,
            state.enemy_formation_id,
            state.engagement,
        )
    {
        state.raigeki = Some(round);
        state.hourai_flag[3] = 1;
    }
}

/// Simulate a night battle.
pub fn simulate_night(
    codex: &Codex,
    mut friendly: Vec<BattleRuntimeShip>,
    mut enemy: Vec<BattleRuntimeShip>,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
    air_state: Option<&crate::types::AirState>,
    rng: &mut impl BattleRng,
) -> NightBattleSimulation {
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
            air_state,
        },
    );

    // Build a minimal state for finalization
    let state = BattleState {
        friendly,
        enemy,
        is_sortie: false,
        battle_type: crate::types::BattleType::Normal,
        friendly_formation_id,
        enemy_formation_id,
        engagement,
        kouku: None,
        opening_attack: None,
        opening_taisen: None,
        hougeki1: None,
        hougeki2: None,
        raigeki: None,
        stage_flag: [0, 0, 0],
        hourai_flag: [0, 0, 0, 0],
        opening_taisen_flag: 0,
    };

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
    use crate::types::{BattleContext, BattleType, EngagementType};
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
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![friend],
                enemy_ships: vec![enemy],
            },
            &mut rng,
        );

        assert_eq!(simulation.packet.midnight_flag, 1);
        assert!(simulation.outcome.can_midnight);
    }

    #[test]
    fn fighter_only_carrier_does_not_launch_airstrike_damage() {
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
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: false,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![carrier],
                enemy_ships: vec![enemy],
            },
            &mut rng,
        );

        let kouku = simulation.packet.kouku.unwrap();
        assert!(kouku.api_plane_from[0].is_empty());
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
            BattleContext {
                battle_type: BattleType::AirBattle,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![friend],
                enemy_ships: vec![enemy],
            },
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
            BattleContext {
                battle_type: BattleType::AirBattle,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![carrier],
                enemy_ships: vec![enemy],
            },
            &mut rng,
        );

        assert!(simulation.packet.kouku.is_some(), "airbattle should still run kouku");
        assert_eq!(simulation.packet.stage_flag, [1, 1, 1]);
    }
}
