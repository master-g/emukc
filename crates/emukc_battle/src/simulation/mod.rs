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
    BattleContext, BattlePhase, BattleRuntimeShip, BattleSimulation, NightBattleInput,
    NightBattleSimulation, ShellingParams,
};

pub(crate) mod asw;
pub(crate) mod kouku;
pub(crate) mod night;
pub(crate) mod shelling;
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
pub fn simulate_day(
    codex: &Codex,
    context: BattleContext,
    rng: &mut impl BattleRng,
) -> BattleSimulation {
    let mut state = BattleState::from_context(context);
    let flow = BattleFlow::for_battle_type(state.battle_type());
    let enemy_first = enemy_shells_first(&state.friendly, &state.enemy);

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
        let has_attack = attack.is_some();
        state.set_opening_attack(attack);
        if has_attack {
            state.set_hourai_flag(0, 1);
        }
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
        let hougeki = if enemy_first {
            shelling::simulate_shelling_side(
                codex,
                rng,
                &mut state.enemy,
                &mut state.friendly,
                &ShellingParams {
                    attacker_is_enemy: true,
                    formation_id: enemy_form,
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
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
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
                },
            )
        };
        let has_hougeki = hougeki.is_some();
        state.set_hougeki1(hougeki);
        if has_hougeki {
            state.set_hourai_flag(1, 1);
        }
    }
}

fn execute_shelling2(
    codex: &Codex,
    state: &mut BattleState,
    rng: &mut impl BattleRng,
    enemy_first: bool,
) {
    if any_alive(&state.friendly) && any_alive(&state.enemy) {
        let friendly_form = state.friendly_formation_id();
        let enemy_form = state.enemy_formation_id();
        let eng = state.engagement();
        let hougeki = if enemy_first {
            shelling::simulate_shelling_side(
                codex,
                rng,
                &mut state.friendly,
                &mut state.enemy,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: friendly_form,
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
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
                    engagement: eng,
                    phase: BattlePhase::DayShelling,
                },
            )
        };
        let has_hougeki = hougeki.is_some();
        state.set_hougeki2(hougeki);
        if has_hougeki {
            state.set_hourai_flag(2, 1);
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
pub fn simulate_night(
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
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![dd],
                enemy_ships: vec![enemy1, enemy2],
            },
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
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships,
                enemy_ships: vec![enemy],
            },
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
}
