//! Debug overlay: applies god_mode and one_hit_kill transforms to a
//! completed [`BattleSimulation`] by diffing HP and overriding the result.
//!
//! This is the integration layer between the simulation, the [event
//! transforms](crate::transforms), and the [reducer](crate::reducer).
//!
//! # Flow
//!
//! 1. Simulate normally (no debug branching in simulation code)
//! 2. Derive events from HP diff (entry HP → final HP)
//! 3. Apply transforms (god_mode filters friendly damage, one_hit_kill sinks enemies)
//! 4. Reduce transformed events to get modified HP
//! 5. Override the simulation packet's HP arrays and outcome

use crate::BattleRuntimeShip;
use crate::event::{BattleEvent, EventLog, Phase, ShipRef, Side};
use crate::reducer::{DerivedState, InitialState, reduce};
use crate::transforms::{god_mode_transform, one_hit_kill_transform};
use crate::types::{
    BattleHougeki, BattleNightHougeki, BattleOutcome, BattlePacket, BattleSimulation,
    NightBattleSimulation,
};

/// Derive a damage event log from HP diff between entry and final state.
///
/// Each ship's total damage = `entry_hp - final_hp`. We emit one `Damage`
/// event per ship that took damage, plus a `Sunk` event if HP reached 0.
///
/// This is lossy (no per-phase breakdown) but sufficient for transform
/// application — transforms only care about total damage per ship and
/// which ships are sunk.
fn derive_events_from_ships(
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) -> EventLog {
    let mut events = Vec::new();

    for (i, ship) in friendly.iter().enumerate() {
        let damage = ship.entry_hp - ship.hp();
        if damage > 0 {
            events.push(BattleEvent::Damage {
                target: ShipRef(Side::Friendly, i),
                raw: damage,
                dealt: damage,
                phase: Phase::Shelling1,
            });
        }
        if ship.is_sunk() {
            events.push(BattleEvent::Sunk {
                target: ShipRef(Side::Friendly, i),
            });
        }
    }

    for (i, ship) in enemy.iter().enumerate() {
        let damage = ship.entry_hp - ship.hp();
        if damage > 0 {
            events.push(BattleEvent::Damage {
                target: ShipRef(Side::Enemy, i),
                raw: damage,
                dealt: damage,
                phase: Phase::Shelling1,
            });
        }
        if ship.is_sunk() {
            events.push(BattleEvent::Sunk {
                target: ShipRef(Side::Enemy, i),
            });
        }
    }

    events
}

/// Build initial state from ship entry HP values.
fn initial_state_from_ships(
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) -> InitialState {
    InitialState::new(
        friendly.iter().map(|s| s.entry_hp).collect(),
        enemy.iter().map(|s| s.entry_hp).collect(),
    )
}

/// Run the shared derive → transform → reduce pipeline.
///
/// Returns the initial state (for reference) and the derived state
/// after applying debug transforms.
fn run_debug_transforms(
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
    god_mode: bool,
    one_hit_kill: bool,
) -> DerivedState {
    let initial = initial_state_from_ships(friendly, enemy);
    let mut events = derive_events_from_ships(friendly, enemy);

    if god_mode {
        events = god_mode_transform(events);
    }
    if one_hit_kill {
        events = one_hit_kill_transform(events, enemy.len());
    }

    reduce(&events, &initial)
}

/// Apply debug transforms to a day battle simulation result.
///
/// If neither flag is set, the simulation is returned unchanged.
/// Otherwise, events are derived from HP diff, transforms are applied,
/// and the packet's HP arrays and outcome are overridden.
pub fn apply_day_debug(
    mut sim: BattleSimulation,
    god_mode: bool,
    one_hit_kill: bool,
) -> BattleSimulation {
    if !god_mode && !one_hit_kill {
        return sim;
    }

    let derived = run_debug_transforms(&sim.friendly, &sim.enemy, god_mode, one_hit_kill);

    override_day_packet(&mut sim.packet, &derived);
    override_ships(&mut sim.friendly, &mut sim.enemy, &derived);
    rebuild_day_packet_arrays(&mut sim.packet, god_mode, one_hit_kill, &sim.friendly, &sim.enemy);
    override_outcome(&mut sim.outcome, &sim.friendly, &sim.enemy);
    recompute_midnight(&mut sim.outcome, &mut sim.packet, &sim.friendly, &sim.enemy);

    sim
}

/// Apply debug transforms to a night battle simulation result.
pub fn apply_night_debug(
    mut sim: NightBattleSimulation,
    god_mode: bool,
    one_hit_kill: bool,
) -> NightBattleSimulation {
    if !god_mode && !one_hit_kill {
        return sim;
    }

    let derived = run_debug_transforms(&sim.friendly, &sim.enemy, god_mode, one_hit_kill);

    override_night_packet(&mut sim.packet, &derived);
    override_ships(&mut sim.friendly, &mut sim.enemy, &derived);
    rebuild_night_packet_arrays(&mut sim.packet, god_mode, one_hit_kill, &sim.friendly, &sim.enemy);
    override_outcome(&mut sim.outcome, &sim.friendly, &sim.enemy);

    sim
}

fn override_day_packet(packet: &mut BattlePacket, derived: &DerivedState) {
    packet.friendly_nowhps = derived.friendly_hp.clone();
    packet.enemy_nowhps = derived.enemy_hp.clone();
}

/// Zero all friendly-directed damage in a [`BattleHougeki`] when `god_mode` is active.
/// `api_at_eflag[i]==1` means enemy attacking → friendly defender.
fn zero_friendly_hougeki_damage(hougeki: &mut BattleHougeki) {
    for (i, &flag) in hougeki.api_at_eflag.iter().enumerate() {
        if flag == 1 {
            hougeki.api_damage[i] = vec![0; hougeki.api_damage[i].len()];
        }
    }
}

/// Zero all friendly-directed damage in a `BattleNightHougeki`.
fn zero_friendly_night_hougeki_damage(hougeki: &mut BattleNightHougeki) {
    for (i, &flag) in hougeki.api_at_eflag.iter().enumerate() {
        if flag == 1 {
            hougeki.api_damage[i] = vec![0; hougeki.api_damage[i].len()];
        }
    }
}

/// Rebuild day packet per-phase damage arrays for consistency with overridden HP.
///
/// - **`god_mode`**: zero every friendly-directed damage entry so cumulative
///   friendly damage is 0, matching `friendly_nowhps == entry_hp`.
/// - **`one_hit_kill`**: synthesize a finishing volley in `hougeki3` dealing
///   exactly each still-alive enemy's remaining HP.
fn rebuild_day_packet_arrays(
    packet: &mut BattlePacket,
    god_mode: bool,
    one_hit_kill: bool,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) {
    if god_mode {
        // Zero friendly damage in aerial combat.
        if let Some(kouku) = &mut packet.kouku {
            kouku.api_stage3.api_fdam.fill(0);
        }
        // Zero friendly damage in opening torpedo.
        if let Some(opening) = &mut packet.opening_attack {
            opening.api_fdam.fill(0);
        }
        // Zero friendly damage in closing torpedo.
        if let Some(raigeki) = &mut packet.raigeki {
            raigeki.api_fdam.fill(0);
        }
        // Zero friendly damage in all hougeki phases.
        if let Some(h) = &mut packet.opening_taisen {
            zero_friendly_hougeki_damage(h);
        }
        if let Some(h) = &mut packet.hougeki1 {
            zero_friendly_hougeki_damage(h);
        }
        if let Some(h) = &mut packet.hougeki2 {
            zero_friendly_hougeki_damage(h);
        }
        if let Some(h) = &mut packet.hougeki3 {
            zero_friendly_hougeki_damage(h);
        }
    }

    if one_hit_kill {
        synthesize_day_finishing_volley(packet, friendly, enemy);
    }
}

/// Synthesize a `hougeki3` finishing volley that deals exactly each
/// still-alive enemy's remaining HP. This ensures cumulative enemy
/// damage reaches each enemy's entry HP, consistent with `enemy_nowhps==0`.
fn synthesize_day_finishing_volley(
    packet: &mut BattlePacket,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) {
    // Find first alive friendly as attacker (flagship is always alive in sortie
    // due to sinking protection; for practice, find any alive friendly).
    let attacker_idx = friendly.iter().position(BattleRuntimeShip::is_alive);
    let Some(attacker_idx) = attacker_idx else {
        return; // All friendlies dead — skip.
    };

    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut at_type = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut damage = Vec::new();

    for (i, ship) in enemy.iter().enumerate() {
        if ship.is_alive() {
            let remaining_hp = ship.hp();
            at_eflag.push(0); // friendly attacking
            at_list.push(attacker_idx as i64);
            at_type.push(0);
            df_list.push(vec![i as i64]);
            si_list.push(vec![-1]);
            cl_list.push(vec![1]);
            damage.push(vec![remaining_hp]);
        }
    }

    if at_eflag.is_empty() {
        return;
    }

    // Merge into existing hougeki3 if present, otherwise create new.
    if let Some(h3) = &mut packet.hougeki3 {
        h3.api_at_eflag.extend(at_eflag);
        h3.api_at_list.extend(at_list);
        h3.api_at_type.extend(at_type);
        h3.api_df_list.extend(df_list);
        h3.api_si_list.extend(si_list);
        h3.api_cl_list.extend(cl_list);
        h3.api_damage.extend(damage);
    } else {
        packet.hougeki3 = Some(BattleHougeki {
            api_at_eflag: at_eflag,
            api_at_list: at_list,
            api_at_type: at_type,
            api_df_list: df_list,
            api_si_list: si_list,
            api_cl_list: cl_list,
            api_damage: damage,
        });
        packet.hourai_flag[2] = 1;
    }
}

/// Rebuild night packet per-phase damage arrays.
fn rebuild_night_packet_arrays(
    packet: &mut crate::NightBattlePacket,
    god_mode: bool,
    one_hit_kill: bool,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) {
    if god_mode && let Some(h) = &mut packet.hougeki {
        zero_friendly_night_hougeki_damage(h);
    }

    if one_hit_kill {
        synthesize_night_finishing_volley(packet, friendly, enemy);
    }
}

/// Synthesize finishing attacks in the night hougeki.
fn synthesize_night_finishing_volley(
    packet: &mut crate::NightBattlePacket,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) {
    let attacker_idx = friendly.iter().position(BattleRuntimeShip::is_alive);
    let Some(attacker_idx) = attacker_idx else {
        return;
    };

    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut n_mother_list = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut sp_list = Vec::new();
    let mut damage = Vec::new();

    for (i, ship) in enemy.iter().enumerate() {
        if ship.is_alive() {
            let remaining_hp = ship.hp();
            at_eflag.push(0);
            at_list.push(attacker_idx as i64);
            n_mother_list.push(0);
            df_list.push(vec![i as i64]);
            si_list.push(vec![-1]);
            cl_list.push(vec![1]);
            sp_list.push(0);
            damage.push(vec![remaining_hp]);
        }
    }

    if at_eflag.is_empty() {
        return;
    }

    if let Some(h) = &mut packet.hougeki {
        h.api_at_eflag.extend(at_eflag);
        h.api_at_list.extend(at_list);
        h.api_n_mother_list.extend(n_mother_list);
        h.api_df_list.extend(df_list);
        h.api_si_list.extend(si_list);
        h.api_cl_list.extend(cl_list);
        h.api_sp_list.extend(sp_list);
        h.api_damage.extend(damage);
    } else {
        packet.hougeki = Some(BattleNightHougeki {
            api_at_eflag: at_eflag,
            api_at_list: at_list,
            api_n_mother_list: n_mother_list,
            api_df_list: df_list,
            api_si_list: si_list,
            api_cl_list: cl_list,
            api_sp_list: sp_list,
            api_damage: damage,
        });
    }
}

/// After debug transforms, recompute `can_midnight` via conjunction:
/// the original value already encodes the `battle_type` gate; transforms
/// can only reduce the alive set on the gating side.
fn recompute_midnight(
    outcome: &mut BattleOutcome,
    packet: &mut BattlePacket,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) {
    let new_can_midnight = outcome.can_midnight
        && crate::targeting::any_alive(friendly)
        && crate::targeting::any_alive(enemy);
    outcome.can_midnight = new_can_midnight;
    packet.midnight_flag = i64::from(new_can_midnight);
}

fn override_night_packet(packet: &mut crate::NightBattlePacket, derived: &DerivedState) {
    packet.friendly_nowhps = derived.friendly_hp.clone();
    packet.enemy_nowhps = derived.enemy_hp.clone();
}

fn override_ships(
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    derived: &DerivedState,
) {
    for (i, ship) in friendly.iter_mut().enumerate() {
        if i < derived.friendly_hp.len() {
            ship.set_hp_for_debug(derived.friendly_hp[i]);
        }
    }
    for (i, ship) in enemy.iter_mut().enumerate() {
        if i < derived.enemy_hp.len() {
            ship.set_hp_for_debug(derived.enemy_hp[i]);
        }
    }
}

fn override_outcome(
    outcome: &mut BattleOutcome,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) {
    outcome.win_rank = crate::outcome::calculate_win_rank(friendly, enemy);
    outcome.mvp = crate::outcome::calculate_mvp(friendly);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::make_test_ship_ctx;
    use crate::types::*;
    use emukc_model::kc2::KcSortieResultRank;

    fn make_ship(hp: i64, max_hp: i64, is_friendly: bool) -> BattleRuntimeShip {
        make_test_ship_ctx(hp, hp, hp, max_hp, is_friendly, true)
    }

    #[test]
    fn no_debug_flags_returns_unchanged() {
        let sim = BattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![make_ship(0, 40, false)],
            packet: BattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![30],
                enemy_nowhps: vec![0],
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
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim.clone(), false, false);
        assert_eq!(result.friendly[0].hp(), 30);
        assert_eq!(result.enemy[0].hp(), 0);
    }

    #[test]
    fn god_mode_restores_friendly_hp() {
        // Ship took 20 damage (30→10), god_mode should restore to entry (30)
        let mut damaged_ship = make_ship(10, 40, true);
        damaged_ship.entry_hp = 30;
        let sim = BattleSimulation {
            friendly: vec![damaged_ship],
            enemy: vec![make_ship(0, 40, false)],
            packet: BattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![10],
                enemy_nowhps: vec![0],
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
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, true, false);
        assert_eq!(result.friendly[0].hp(), 30, "god_mode restores friendly HP");
        assert_eq!(result.packet.friendly_nowhps[0], 30);
    }

    #[test]
    fn one_hit_kill_sinks_all_enemies() {
        // Enemy has 30 HP (not sunk), one_hit_kill should sink it
        let enemy_ship = make_ship(30, 40, false);
        let sim = BattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![enemy_ship],
            packet: BattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![30],
                enemy_nowhps: vec![30],
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
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::D,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, false, true);
        assert_eq!(result.enemy[0].hp(), 0, "one_hit_kill sinks all enemies");
        assert_eq!(result.packet.enemy_nowhps[0], 0);
    }

    #[test]
    fn both_flags_compose() {
        // Friendly took damage, enemy alive → god_mode restores friendly,
        // one_hit_kill sinks enemy
        let mut damaged = make_ship(10, 40, true);
        damaged.entry_hp = 30;
        let sim = BattleSimulation {
            friendly: vec![damaged],
            enemy: vec![make_ship(30, 40, false)],
            packet: BattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![10],
                enemy_nowhps: vec![30],
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
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::D,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, true, true);
        assert_eq!(result.friendly[0].hp(), 30, "god_mode restores friendly");
        assert_eq!(result.enemy[0].hp(), 0, "one_hit_kill sinks enemy");
    }

    #[test]
    fn one_hit_kill_clears_midnight_flag() {
        // Battle originally had can_midnight=true, midnight_flag=1.
        // one_hit_kill sinks all enemies → no midnight possible.
        let sim = BattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![make_ship(30, 40, false)],
            packet: BattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![30],
                enemy_nowhps: vec![30],
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
        let result = apply_day_debug(sim, false, true);
        assert!(!result.outcome.can_midnight, "one_hit_kill clears can_midnight");
        assert_eq!(result.packet.midnight_flag, 0, "midnight_flag is 0");
    }
}
