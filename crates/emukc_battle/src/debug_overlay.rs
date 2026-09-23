//! Debug overlay: applies god_mode and one_hit_kill to a completed
//! [`BattleSimulation`] by overriding HP and everything derived from it.
//!
//! # Flow
//!
//! 1. Simulate normally (no debug branching in simulation code)
//! 2. Derive the debug HP directly from post-simulation ship state
//! 3. Override the packet HP arrays and rebuild the per-phase damage arrays
//!    so the client animation agrees with them
//! 4. Override the runtime ships and the outcome (win rank, MVP)
//! 5. Recompute `can_midnight` from the overridden alive sets (day only)

use crate::BattleRuntimeShip;
use crate::simulation::day_attack::DayAttackKind;
use crate::types::{
    BattleHougeki, BattleNightHougeki, BattleOutcome, BattlePacket, BattleSimulation, DamageCell,
    NightBattleSimulation, SiListId,
};

/// Debug HP overrides for one battle, indexed by fleet position.
struct DebugHp {
    /// Friendly HP after applying the debug policy.
    friendly: Vec<i64>,
    /// Enemy HP after applying the debug policy.
    enemy: Vec<i64>,
}

/// Post-simulation HP clamped into the range the overlay reports: never above
/// entry HP, never below 0, and exactly 0 once the ship is sunk.
fn clamped_hp(ship: &BattleRuntimeShip) -> i64 {
    if ship.is_sunk() {
        return 0;
    }
    (ship.entry_hp - (ship.entry_hp - ship.hp()).max(0)).max(0)
}

/// Derive the debug HP overrides directly from post-simulation ship state.
///
/// `god_mode` pins every friendly to its entry HP; `one_hit_kill` sinks every
/// enemy. A side not covered by an active flag keeps its clamped simulation HP.
fn debug_hp(
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
    god_mode: bool,
    one_hit_kill: bool,
) -> DebugHp {
    DebugHp {
        friendly: friendly
            .iter()
            .map(|ship| {
                if god_mode {
                    ship.entry_hp
                } else {
                    clamped_hp(ship)
                }
            })
            .collect(),
        enemy: enemy
            .iter()
            .map(|ship| {
                if one_hit_kill {
                    0
                } else {
                    clamped_hp(ship)
                }
            })
            .collect(),
    }
}

/// Apply the debug policy to a day battle simulation result.
///
/// If neither flag is set, the simulation is returned unchanged. Otherwise the
/// debug HP is derived from the post-simulation ships and the packet's HP
/// arrays, per-phase damage arrays, ships and outcome are overridden.
pub(crate) fn apply_day_debug(
    mut sim: BattleSimulation,
    god_mode: bool,
    one_hit_kill: bool,
) -> BattleSimulation {
    if !god_mode && !one_hit_kill {
        return sim;
    }

    let debug = debug_hp(&sim.friendly, &sim.enemy, god_mode, one_hit_kill);

    // Capture finishing-volley inputs from the real post-simulation ship state,
    // *before* any override mutates HP. Decouples the volley synthesis from the
    // order of `rebuild_*_packet_arrays` relative to `override_ships`.
    let finishing = one_hit_kill.then(|| FinishingVolley::capture(&sim.friendly, &sim.enemy));

    override_day_packet(&mut sim.packet, &debug);
    rebuild_day_packet_arrays(&mut sim.packet, god_mode, finishing.as_ref());
    override_ships(&mut sim.friendly, &mut sim.enemy, &debug);
    override_outcome(&mut sim.outcome, &sim.friendly, &sim.enemy);
    recompute_midnight(&mut sim.outcome, &mut sim.packet, &sim.friendly, &sim.enemy);

    sim
}

/// Apply the debug policy to a night battle simulation result.
pub(crate) fn apply_night_debug(
    mut sim: NightBattleSimulation,
    god_mode: bool,
    one_hit_kill: bool,
) -> NightBattleSimulation {
    if !god_mode && !one_hit_kill {
        return sim;
    }

    let debug = debug_hp(&sim.friendly, &sim.enemy, god_mode, one_hit_kill);

    // Capture finishing-volley inputs from the real ship state before override
    // (see `apply_day_debug`).
    let finishing = one_hit_kill.then(|| FinishingVolley::capture(&sim.friendly, &sim.enemy));

    override_night_packet(&mut sim.packet, &debug);
    rebuild_night_packet_arrays(&mut sim.packet, god_mode, finishing.as_ref());
    override_ships(&mut sim.friendly, &mut sim.enemy, &debug);
    override_outcome(&mut sim.outcome, &sim.friendly, &sim.enemy);

    sim
}

fn override_day_packet(packet: &mut BattlePacket, debug: &DebugHp) {
    packet.friendly_nowhps = debug.friendly.clone();
    packet.enemy_nowhps = debug.enemy.clone();
}

/// Zero all friendly-directed damage in a [`BattleHougeki`] when `god_mode` is active.
/// `api_at_eflag[i]==1` means enemy attacking → friendly defender.
fn zero_friendly_hougeki_damage(hougeki: &mut BattleHougeki) {
    for (i, &flag) in hougeki.api_at_eflag.iter().enumerate() {
        if flag == 1 {
            hougeki.api_damage[i] = vec![DamageCell::Plain(0); hougeki.api_damage[i].len()];
        }
    }
}

/// Zero all friendly-directed damage in a `BattleNightHougeki`.
fn zero_friendly_night_hougeki_damage(hougeki: &mut BattleNightHougeki) {
    for (i, &flag) in hougeki.api_at_eflag.iter().enumerate() {
        if flag == 1 {
            hougeki.api_damage[i] = vec![DamageCell::Plain(0); hougeki.api_damage[i].len()];
        }
    }
}

/// Finishing-volley inputs captured from the real post-simulation ship state
/// **before** any debug override mutates HP.
///
/// `one_hit_kill` synthesizes a finishing volley dealing each still-alive
/// enemy's remaining HP. That calculation must read the *real* simulation HP,
/// not the overridden (zeroed) HP. Capturing the inputs up front decouples the
/// synthesis from the order of [`rebuild_day_packet_arrays`] /
/// [`rebuild_night_packet_arrays`] relative to [`override_ships`]: a reorder can
/// no longer silently drop the volley, because the synthesis no longer reads the
/// ships that `override_ships` mutates.
struct FinishingVolley {
    /// First alive friendly ship index — the synthetic attacker. `None` if all
    /// friendlies are dead (no volley is synthesized).
    attacker_idx: Option<usize>,
    /// `(enemy_index, remaining_hp)` for each still-alive enemy.
    targets: Vec<(usize, i64)>,
}

impl FinishingVolley {
    fn capture(friendly: &[BattleRuntimeShip], enemy: &[BattleRuntimeShip]) -> Self {
        Self {
            attacker_idx: friendly.iter().position(BattleRuntimeShip::is_alive),
            targets: enemy
                .iter()
                .enumerate()
                .filter(|(_, ship)| ship.is_alive())
                .map(|(i, ship)| (i, ship.hp()))
                .collect(),
        }
    }
}

/// Rebuild day packet per-phase damage arrays for consistency with overridden HP.
///
/// - **`god_mode`**: zero every friendly-directed damage entry so cumulative
///   friendly damage is 0, matching `friendly_nowhps == entry_hp`.
/// - **`one_hit_kill`** (`finishing` is `Some`): synthesize a finishing volley
///   in `hougeki3` dealing exactly each still-alive enemy's remaining HP.
fn rebuild_day_packet_arrays(
    packet: &mut BattlePacket,
    god_mode: bool,
    finishing: Option<&FinishingVolley>,
) {
    if god_mode {
        // Zero friendly damage in aerial combat.
        if let Some(kouku) = &mut packet.kouku {
            kouku.api_stage3.api_fdam.fill(DamageCell::Plain(0));
        }
        // Zero friendly damage in opening torpedo. The client animates each
        // friendly's HP from the per-attacker enemy-torpedo entries
        // (`api_eydam_list_items`), not the `api_fdam` summary, so those must be
        // zeroed too — otherwise a god_mode friendly still visibly takes (and can
        // be taiha'd by) opening-torpedo damage mid-animation.
        if let Some(opening) = &mut packet.opening_attack {
            opening.api_fdam.fill(DamageCell::Plain(0));
            for cells in opening.api_eydam_list_items.iter_mut().flatten() {
                cells.iter_mut().for_each(|cell| *cell = DamageCell::Plain(0));
            }
        }
        // Zero friendly damage in closing torpedo. `api_eydam` is the per-attacker
        // enemy torpedo damage the client animates from (same reason as opening).
        if let Some(raigeki) = &mut packet.raigeki {
            raigeki.api_fdam.fill(DamageCell::Plain(0));
            raigeki.api_eydam.iter_mut().for_each(|cell| *cell = DamageCell::Plain(0));
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

    if let Some(finishing) = finishing {
        synthesize_day_finishing_volley(packet, finishing);
    }
}

/// Synthesize a `hougeki3` finishing volley that deals exactly each
/// still-alive enemy's remaining HP. This ensures cumulative enemy
/// damage reaches each enemy's entry HP, consistent with `enemy_nowhps==0`.
///
/// Consumes a [`FinishingVolley`] captured from the real ship state before
/// override, so this is independent of when it runs relative to `override_ships`.
fn synthesize_day_finishing_volley(packet: &mut BattlePacket, finishing: &FinishingVolley) {
    // The flagship is always alive in sortie (sinking protection); for practice
    // any alive friendly is the attacker. None alive → nothing to synthesize.
    let Some(attacker_idx) = finishing.attacker_idx else {
        return;
    };
    if finishing.targets.is_empty() {
        return;
    }

    // Append to the existing hougeki3 if present, otherwise open it.
    if packet.hougeki3.is_none() {
        packet.hourai_flag[2] = 1;
    }
    let h3 = packet.hougeki3.get_or_insert_with(BattleHougeki::default);
    for &(enemy_idx, remaining_hp) in &finishing.targets {
        h3.record_day_attack(
            DayAttackKind::DebugInjected,
            false,
            attacker_idx,
            vec![enemy_idx as i64],
            vec![remaining_hp.into()],
        );
    }
}

/// Rebuild night packet per-phase damage arrays.
fn rebuild_night_packet_arrays(
    packet: &mut crate::NightBattlePacket,
    god_mode: bool,
    finishing: Option<&FinishingVolley>,
) {
    if god_mode && let Some(h) = &mut packet.hougeki {
        zero_friendly_night_hougeki_damage(h);
    }

    if let Some(finishing) = finishing {
        synthesize_night_finishing_volley(packet, finishing);
    }
}

/// Synthesize finishing attacks in the night hougeki. Consumes a pre-override
/// [`FinishingVolley`] snapshot (see [`synthesize_day_finishing_volley`]).
fn synthesize_night_finishing_volley(
    packet: &mut crate::NightBattlePacket,
    finishing: &FinishingVolley,
) {
    let Some(attacker_idx) = finishing.attacker_idx else {
        return;
    };
    if finishing.targets.is_empty() {
        return;
    }

    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut n_mother_list = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut sp_list = Vec::new();
    let mut damage = Vec::new();

    for &(enemy_idx, remaining_hp) in &finishing.targets {
        at_eflag.push(0);
        at_list.push(attacker_idx as i64);
        n_mother_list.push(0);
        df_list.push(vec![enemy_idx as i64]);
        si_list.push(vec![SiListId::Num(-1)]);
        cl_list.push(vec![1]);
        sp_list.push(0);
        damage.push(vec![remaining_hp.into()]);
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

/// After the debug override, recompute `can_midnight` via conjunction:
/// the original value already encodes the `battle_type` gate; the override
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

fn override_night_packet(packet: &mut crate::NightBattlePacket, debug: &DebugHp) {
    packet.friendly_nowhps = debug.friendly.clone();
    packet.enemy_nowhps = debug.enemy.clone();
}

fn override_ships(
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    debug: &DebugHp,
) {
    for (i, ship) in friendly.iter_mut().enumerate() {
        if i < debug.friendly.len() {
            ship.set_hp_for_debug(debug.friendly[i]);
        }
    }
    for (i, ship) in enemy.iter_mut().enumerate() {
        if i < debug.enemy.len() {
            ship.set_hp_for_debug(debug.enemy[i]);
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

    /// Minimal day packet with no per-phase arrays; HP arrays supplied by caller.
    fn day_packet(friendly_nowhps: Vec<i64>, enemy_nowhps: Vec<i64>) -> BattlePacket {
        BattlePacket {
            formation: [1, 1, 1],
            friendly_nowhps,
            enemy_nowhps,
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

    #[test]
    fn one_hit_kill_win_rank_is_ss() {
        // All enemies sunk, no friendly damage → SS rank.
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
        assert_eq!(result.outcome.win_rank, KcSortieResultRank::S);
    }

    #[test]
    fn god_mode_revives_practice_friendly() {
        // Practice battle: is_sortie=false, no sinking protection.
        // Friendly sunk (hp=0), god_mode should revive to entry HP.
        let mut sunk_ship = make_test_ship_ctx(0, 30, 0, 40, true, false);
        sunk_ship.entry_hp = 30;
        let sim = BattleSimulation {
            friendly: vec![sunk_ship],
            enemy: vec![make_test_ship_ctx(0, 0, 0, 40, false, false)],
            packet: BattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![0],
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
                win_rank: KcSortieResultRank::B,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, true, false);
        assert_eq!(result.friendly[0].hp(), 30, "god_mode revives practice-friendly to entry HP");
    }

    #[test]
    fn apply_night_debug_god_mode_restores_friendly() {
        let mut damaged = make_ship(10, 40, true);
        damaged.entry_hp = 30;
        let sim = NightBattleSimulation {
            friendly: vec![damaged],
            enemy: vec![make_ship(0, 40, false)],
            packet: crate::NightBattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![10],
                friendly_maxhps: vec![40],
                enemy_nowhps: vec![0],
                enemy_maxhps: vec![40],
                touch_plane: [-1, -1],
                flare_pos: [-1, -1],
                hougeki: None,
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_night_debug(sim, true, false);
        assert_eq!(result.friendly[0].hp(), 30, "god_mode restores friendly HP in night");
        assert_eq!(result.packet.friendly_nowhps[0], 30);
    }

    #[test]
    fn apply_night_debug_one_hit_kill_sinks_enemies() {
        let sim = NightBattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![make_ship(30, 40, false)],
            packet: crate::NightBattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![30],
                friendly_maxhps: vec![40],
                enemy_nowhps: vec![30],
                enemy_maxhps: vec![40],
                touch_plane: [-1, -1],
                flare_pos: [-1, -1],
                hougeki: None,
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::D,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_night_debug(sim, false, true);
        assert_eq!(result.enemy[0].hp(), 0, "one_hit_kill sinks enemies in night");
        assert_eq!(result.packet.enemy_nowhps[0], 0);
        assert!(result.packet.hougeki.is_some(), "finishing volley synthesized in night hougeki");
    }

    /// R1 regression: the finishing volley deals each still-alive enemy's real
    /// remaining HP. The original convention-only ordering (live-ship read +
    /// `rebuild` before `override`) would, on reorder, read already-zeroed
    /// enemies and synthesize an empty volley — this test would then fail. The
    /// pre-override snapshot makes the synthesis order-independent.
    #[test]
    fn one_hit_kill_synthesizes_finishing_volley() {
        let sim = BattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![make_ship(25, 40, false), make_ship(12, 40, false)],
            packet: day_packet(vec![30], vec![25, 12]),
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::D,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, false, true);
        let h3 = result.packet.hougeki3.expect("finishing volley synthesized");
        assert_eq!(h3.api_at_eflag.len(), 2, "one entry per alive enemy");
        assert_eq!(h3.api_damage[0], vec![DamageCell::Plain(25)], "enemy 0 remaining HP");
        assert_eq!(h3.api_damage[1], vec![DamageCell::Plain(12)], "enemy 1 remaining HP");
        assert_eq!(h3.api_df_list[0], vec![0], "targets enemy index 0");
        assert_eq!(h3.api_df_list[1], vec![1], "targets enemy index 1");
        assert_eq!(result.packet.hourai_flag[2], 1, "hougeki3 phase flagged on");
    }

    /// R2: the synthetic `hougeki3` field shape matches the real client
    /// `BattleHougeki` payload decoded from `~/Downloads/kcsapi/battle*.txt`:
    /// all seven arrays length-aligned, `api_at_eflag == 0` (friendly→enemy),
    /// `api_at_type == 0` (normal attack), and `api_si_list` int sentinel.
    #[test]
    fn synthetic_finishing_volley_shape_matches_client() {
        let sim = BattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![make_ship(25, 40, false), make_ship(12, 40, false)],
            packet: day_packet(vec![30], vec![25, 12]),
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::D,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, false, true);
        let h3 = result.packet.hougeki3.expect("finishing volley synthesized");

        // All seven arrays are length-aligned (the captures are ALL-EQUAL).
        let n = h3.api_at_eflag.len();
        assert_eq!(h3.api_at_list.len(), n);
        assert_eq!(h3.api_at_type.len(), n);
        assert_eq!(h3.api_df_list.len(), n);
        assert_eq!(h3.api_si_list.len(), n);
        assert_eq!(h3.api_cl_list.len(), n);
        assert_eq!(h3.api_damage.len(), n);

        for i in 0..n {
            assert_eq!(h3.api_at_eflag[i], 0, "friendly→enemy direction");
            assert_eq!(h3.api_at_type[i], 0, "normal attack type");
            assert_eq!(h3.api_si_list[i], vec![SiListId::Num(-1)], "int -1 sentinel");
            assert_eq!(h3.api_df_list[i].len(), 1, "single defender per attack");
            assert_eq!(h3.api_cl_list[i], vec![1], "hit flag");
            assert_eq!(h3.api_damage[i].len(), 1, "single damage cell per attack");
        }

        // The -1 sentinel renders as a JSON integer, not a string (normal
        // attacks use int si_list; cf. plan 2026-06-24-001).
        assert_eq!(serde_json::to_string(&h3.api_si_list[0]).unwrap(), "[-1]");
    }

    /// `god_mode` alone must not synthesize a finishing volley.
    #[test]
    fn god_mode_synthesizes_no_finishing_volley() {
        let mut damaged = make_ship(10, 40, true);
        damaged.entry_hp = 30;
        let sim = BattleSimulation {
            friendly: vec![damaged],
            enemy: vec![make_ship(25, 40, false)],
            packet: day_packet(vec![10], vec![25]),
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::B,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, true, false);
        assert!(result.packet.hougeki3.is_none(), "god_mode adds no finishing volley");
    }

    /// god_mode must zero the per-attacker enemy torpedo damage the client
    /// animates HP from, not just the `api_fdam` summary. Regression: opening /
    /// closing torpedo `api_eydam` survived god_mode, so a friendly still visibly
    /// took (and could be taiha'd by) torpedo damage mid-animation even though the
    /// summary and final HP were restored.
    #[test]
    fn god_mode_zeros_enemy_torpedo_damage() {
        let mut damaged = make_ship(10, 40, true);
        damaged.entry_hp = 30;

        let mut packet = day_packet(vec![10], vec![0]);

        // Enemy closing torpedo deals 20 to the friendly ship.
        let mut raigeki = BattleRaigeki::blank(1);
        raigeki.api_fdam[0] = DamageCell::Plain(20);
        raigeki.api_eydam[0] = DamageCell::Plain(20);
        raigeki.api_erai[0] = 0;
        raigeki.api_ecl[0] = 1;
        packet.raigeki = Some(raigeki);

        // Enemy opening torpedo deals 15 to the friendly ship.
        let mut opening = BattleOpeningAttack::blank(1);
        opening.api_fdam[0] = DamageCell::Plain(15);
        opening.api_eydam_list_items[0] = Some(vec![DamageCell::Plain(15)]);
        opening.api_erai_list_items[0] = Some(vec![0]);
        opening.api_ecl_list_items[0] = Some(vec![1]);
        packet.opening_attack = Some(opening);

        let sim = BattleSimulation {
            friendly: vec![damaged],
            enemy: vec![make_ship(0, 40, false)],
            packet,
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::B,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_day_debug(sim, true, false);

        let raigeki = result.packet.raigeki.expect("raigeki retained");
        assert_eq!(raigeki.api_fdam, vec![DamageCell::Plain(0)], "closing torpedo summary zeroed");
        assert_eq!(
            raigeki.api_eydam,
            vec![DamageCell::Plain(0)],
            "closing torpedo per-attacker enemy damage zeroed"
        );
        // Only the damage value is neutralized — the attack structure is kept,
        // mirroring the shelling phase (`zero_friendly_hougeki_damage` leaves
        // `api_at_eflag` / `api_df_list` intact). The client still plays the
        // torpedo hit, for zero damage. Pins the target / hit flags against
        // drift: clearing them would desync the animation, and a future refactor
        // must zero `api_eydam` without touching `api_erai` / `api_ecl`.
        assert_eq!(raigeki.api_erai, vec![0], "closing torpedo target flag preserved");
        assert_eq!(raigeki.api_ecl, vec![1], "closing torpedo hit flag preserved");

        let opening = result.packet.opening_attack.expect("opening retained");
        assert_eq!(opening.api_fdam, vec![DamageCell::Plain(0)], "opening torpedo summary zeroed");
        assert_eq!(
            opening.api_eydam_list_items[0],
            Some(vec![DamageCell::Plain(0)]),
            "opening torpedo per-attacker enemy damage zeroed"
        );
        assert_eq!(
            opening.api_erai_list_items[0],
            Some(vec![0]),
            "opening torpedo target flag preserved"
        );
        assert_eq!(
            opening.api_ecl_list_items[0],
            Some(vec![1]),
            "opening torpedo hit flag preserved"
        );

        assert_eq!(result.friendly[0].hp(), 30, "god_mode restores friendly HP");
    }

    /// Night finishing volley deals each alive enemy's remaining HP.
    #[test]
    fn night_one_hit_kill_volley_damage_matches_remaining_hp() {
        let sim = NightBattleSimulation {
            friendly: vec![make_ship(30, 40, true)],
            enemy: vec![make_ship(25, 40, false), make_ship(12, 40, false)],
            packet: crate::NightBattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![30],
                friendly_maxhps: vec![40],
                enemy_nowhps: vec![25, 12],
                enemy_maxhps: vec![40, 40],
                touch_plane: [-1, -1],
                flare_pos: [-1, -1],
                hougeki: None,
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::D,
                mvp: 0,
                can_midnight: false,
            },
        };
        let result = apply_night_debug(sim, false, true);
        let h = result.packet.hougeki.expect("night finishing volley synthesized");
        assert_eq!(h.api_at_eflag, vec![0, 0], "both attacks friendly→enemy");
        assert_eq!(h.api_damage[0], vec![DamageCell::Plain(25)]);
        assert_eq!(h.api_damage[1], vec![DamageCell::Plain(12)]);
    }
}
