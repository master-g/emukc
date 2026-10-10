//! Aerial combat (kouku) phase simulation.
//!
//! Implements the three-stage aerial combat model:
//! Stage 1: Fighter combat (air superiority), Stage 2: Anti-air fire,
//! Stage 3: Bombing damage (dive bombing + torpedo bombing).

use emukc_model::{
    codex::Codex,
    kc2::{KcApiSlotItem, KcShipType, KcSlotItemType3, start2::ApiMstSlotitem},
};

use super::air_base::{ENEMY_FLAT_SHOT, fleet_anti_air, weighted_anti_air};
use crate::accuracy::{HitOutcome, PROFICIENCY_EXP, plane_proficiency, roll_strike};
use crate::combined::CombinedFleetRole;
use crate::damage::{apply_cap, calculate_defense_power, resolve_damage};
use crate::random::BattleRng;
use crate::targeting::{is_air_combat_type, is_airstrike_attack_type, ship_type};
use crate::types::{
    AirState, AirstrikeOutput, BattleKouku, BattleKoukuStage1, BattleKoukuStage2,
    BattleKoukuStage3, BattleRuntimeShip, DamageCell,
};

// ---------------------------------------------------------------------------
// Fighter power & plane count helpers
// ---------------------------------------------------------------------------

fn is_fighter_power_type(slotitem_type: i64) -> bool {
    matches!(
        KcSlotItemType3::n(slotitem_type),
        Some(
            KcSlotItemType3::CarrierBasedFighter
                | KcSlotItemType3::CarrierBasedDiveBomber
                | KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::SeaBasedBomber
                | KcSlotItemType3::SeaplaneFighter
                | KcSlotItemType3::JetFighter
                | KcSlotItemType3::JetFighterBomber
                | KcSlotItemType3::JetAttacker
        )
    )
}

pub(crate) fn calculate_fighter_power(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
    ships
        .iter()
        .flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
        .filter_map(|(slot_item, onslot)| {
            if onslot <= 0 {
                return None;
            }
            let mst = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()?;
            if !is_fighter_power_type(mst.api_type[2]) {
                return None;
            }
            let aa = mst.api_tyku.max(0) as f64;
            let bonus = proficiency_fighter_power(mst.api_type[2], slot_item.api_alv, false);
            Some((aa * (onslot as f64).sqrt() + bonus).floor() as i64)
        })
        .sum()
}

/// What each proficiency level adds to a fighter's 制空値.
const FIGHTER_PROFICIENCY: [f64; 8] = [0.0, 0.0, 2.0, 5.0, 9.0, 14.0, 14.0, 22.0];
/// The same for a seaplane bomber.
const SEAPLANE_BOMBER_PROFICIENCY: [f64; 8] = [0.0, 0.0, 1.0, 1.0, 1.0, 3.0, 3.0, 6.0];

/// 制空値 a slot gains from its proficiency (`setProficiency`'s `APbonus`):
/// the root of a tenth of its experience, plus a step by level for the
/// aircraft that fight for the air. Bombers get the root alone; anything else
/// gets it only when it flies from a land base.
pub(crate) fn proficiency_fighter_power(type3: i64, alv: Option<i64>, land_base: bool) -> f64 {
    let level = alv.unwrap_or(0).clamp(0, 7) as usize;
    let root = (PROFICIENCY_EXP[level] * 0.1).sqrt();
    match KcSlotItemType3::n(type3) {
        Some(
            KcSlotItemType3::CarrierBasedFighter
            | KcSlotItemType3::SeaplaneFighter
            | KcSlotItemType3::LocalFighter
            | KcSlotItemType3::JetFighter,
        ) => root + FIGHTER_PROFICIENCY[level],
        Some(KcSlotItemType3::SeaBasedBomber) => root + SEAPLANE_BOMBER_PROFICIENCY[level],
        Some(
            KcSlotItemType3::CarrierBasedTorpedoBomber
            | KcSlotItemType3::CarrierBasedDiveBomber
            | KcSlotItemType3::JetFighterBomber,
        ) => root,
        _ if land_base => root,
        _ => 0.0,
    }
}

pub(crate) fn total_plane_count(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
    ships
        .iter()
        .flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
        .filter(|(slot_item, onslot)| {
            *onslot > 0
                && codex
                    .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
                    .ok()
                    .is_some_and(|mst| is_air_combat_type(mst.api_type[2]))
        })
        .map(|(_, onslot)| onslot)
        .sum()
}

pub(crate) fn has_any_air_combat_planes(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
    total_plane_count(codex, ships) > 0
}

/// Ship types that can participate in aerial combat (launch planes).
const AIR_COMBAT_SHIP_TYPES: &[KcShipType] = &[
    KcShipType::CVL, // 軽空母
    KcShipType::CV,  // 正規空母
    KcShipType::CVB, // 装甲空母
    KcShipType::BBV, // 航空戦艦
    KcShipType::CAV, // 航空巡洋艦
    KcShipType::AV,  // 水上機母艦
];

pub(crate) fn attack_plane_from(codex: &Codex, ships: &[BattleRuntimeShip]) -> Vec<i64> {
    ships
        .iter()
        .enumerate()
        .filter_map(|(idx, ship)| {
            let stype = ship_type(codex, ship);
            if !stype.is_some_and(|st| AIR_COMBAT_SHIP_TYPES.contains(&st)) {
                return None;
            }
            let has_plane =
                ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(slot_item, onslot)| {
                    onslot > 0
                        && codex
                            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
                            .ok()
                            .is_some_and(|mst| is_air_combat_type(mst.api_type[2]))
                });
            has_plane.then_some(idx as i64 + 1)
        })
        .collect()
}

fn first_touch_plane(codex: &Codex, ships: &[BattleRuntimeShip]) -> Option<i64> {
    ships.iter().flat_map(|ship| ship.slot_items.iter()).find_map(|slot_item| {
        codex
            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
            .ok()
            .filter(|mst| {
                matches!(
                    KcSlotItemType3::n(mst.api_type[2]),
                    Some(KcSlotItemType3::CarrierBasedRecon | KcSlotItemType3::CarrierBasedRecon2)
                )
            })
            .map(|mst| mst.api_id)
    })
}

/// Find the ship index with the highest total bombing power (for damage attribution).
fn best_bomber_index(codex: &Codex, ships: &[BattleRuntimeShip]) -> Option<usize> {
    ships
        .iter()
        .enumerate()
        .map(|(idx, ship)| {
            let power: f64 = ship
                .slot_items
                .iter()
                .zip(ship.ship.api_onslot)
                .filter_map(|(si, onslot)| {
                    if onslot <= 0 {
                        return None;
                    }
                    let mst = codex.find::<ApiMstSlotitem>(&si.api_slotitem_id).ok()?;
                    if !is_airstrike_attack_type(mst.api_type[2]) {
                        return None;
                    }
                    let is_torpedo = KcSlotItemType3::n(mst.api_type[2])
                        == Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
                    let stat = if is_torpedo {
                        mst.api_raig.max(0) as f64
                    } else {
                        mst.api_baku.max(0) as f64
                    };
                    Some(stat * (onslot as f64).sqrt())
                })
                .sum();
            (idx, power)
        })
        .filter(|(_, power)| *power > 0.0)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
}

// ---------------------------------------------------------------------------
// Plane losses, slot by slot
// ---------------------------------------------------------------------------

/// The share of her anti-air fire a ship keeps when a combined fleet is in
/// the fight: 0.48 in an escort deck, 0.8 in a main deck, and 0.8 too for a
/// single fleet under an enemy combined fleet's aircraft (`getAAShotProp`,
/// `getAAShotFlat` and `forceCF` in `kcsim.js`).
fn combined_fire_share(defender: &BattleRuntimeShip, attackers_combined: bool) -> f64 {
    if defender.is_escort_deck() || defender.enemy_deck == Some(CombinedFleetRole::Escort) {
        0.48
    } else if defender.is_main_deck() || defender.enemy_deck.is_some() || attackers_combined {
        0.8
    } else {
        1.0
    }
}

/// Stage 1: every slot of `ships` that fights for the air loses its own share
/// (`kcsim.js` `AADefenceFighters`). Returns how many aircraft that was.
fn fight_for_the_air(
    codex: &Codex,
    rng: &mut impl BattleRng,
    ships: &mut [BattleRuntimeShip],
    air_state: AirState,
    friendly: bool,
) -> i64 {
    let mut total = 0;
    for ship in ships.iter_mut().filter(|ship| ship.is_alive()) {
        for (slot, item) in ship.slot_items.iter().enumerate().take(5) {
            let count = ship.ship.api_onslot[slot];
            if count <= 0 {
                continue;
            }
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
                continue;
            };
            if !is_fighter_power_type(mst.api_type[2]) {
                continue;
            }
            let jet = match KcSlotItemType3::n(mst.api_type[2]) {
                Some(
                    KcSlotItemType3::JetFighter
                    | KcSlotItemType3::JetFighterBomber
                    | KcSlotItemType3::JetAttacker,
                ) => 0.6,
                _ => 1.0,
            };
            // The source divides the enemy's tenths last, after the count.
            let lost = if friendly {
                let (always, more) = air_state.stage1_friendly_slot_loss();
                let share = always + rng.roll_range(0, more + 1) as f64 / 1000.0;
                count as f64 * share * jet
            } else {
                let below = air_state.stage1_enemy_slot_loss();
                let tenths =
                    0.35 * rng.roll_range(0, below) as f64 + 0.65 * rng.roll_range(0, below) as f64;
                count as f64 * tenths * jet / 10.0
            };
            let lost = (lost.floor() as i64).min(count);
            ship.ship.api_onslot[slot] = count - lost;
            total += lost;
        }
    }
    total
}

/// Stage 2: every slot of `attackers` that comes to strike is fired on by one
/// of `defenders` still afloat (`kcsim.js` `AADefenceBombersAndAirstrike`).
/// Returns how many aircraft flew in and how many were shot down.
// ponytail: no anti-air cut-in, formation modifier or 改修; add them to the
// fixed shot when they are modelled.
fn fly_through_anti_air(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &[BattleRuntimeShip],
) -> (i64, i64) {
    let fleet_aa = fleet_anti_air(codex, defenders) as f64;
    let attackers_combined = attackers.iter().any(|ship| ship.enemy_deck.is_some());
    let (mut flew, mut shot) = (0, 0);
    for ship in attackers.iter_mut().filter(|ship| ship.is_alive()) {
        for (slot, item) in ship.slot_items.iter().enumerate().take(5) {
            let count = ship.ship.api_onslot[slot];
            if count <= 0 {
                continue;
            }
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
                continue;
            };
            if !is_airstrike_attack_type(mst.api_type[2]) {
                continue;
            }
            let afloat: Vec<&BattleRuntimeShip> =
                defenders.iter().filter(|defender| defender.is_alive()).collect();
            let Some(pick) = rng.choose_index(afloat.len()) else {
                return (flew, shot);
            };
            let defender = afloat[pick];
            let kept = combined_fire_share(defender, attackers_combined);
            let ship_aa = weighted_anti_air(codex, defender) as f64 * kept;
            // A friendly fleet's fire counts for 1/1.3 as much at a rate of 0.2,
            // and always takes one aircraft more (`getAAShotFlat`, `shotFix`).
            let (fleet_fire, rate, at_least) = if defender.is_friendly {
                ((fleet_aa / 1.3).floor() * kept, 0.2, 1)
            } else {
                (fleet_aa * kept, ENEMY_FLAT_SHOT, 0)
            };
            let mut lost = at_least;
            if rng.roll_range(0, 2) == 0 {
                lost += (count as f64 * ship_aa / 200.0).floor() as i64;
            }
            if rng.roll_range(0, 2) == 0 {
                lost += ((ship_aa + fleet_fire) * rate).floor() as i64;
            }
            let lost = lost.min(count);
            ship.ship.api_onslot[slot] = count - lost;
            flew += count;
            shot += lost;
        }
    }
    (flew, shot)
}

// ---------------------------------------------------------------------------
// Plane loss application
// ---------------------------------------------------------------------------

/// Take `lostcount` aircraft from `ships`, largest slot first. The carrier air
/// phase rolls each slot on its own instead; this is what is left for the
/// enemy's aircraft under an air corps attack.
pub(super) fn apply_plane_losses(
    codex: &Codex,
    ships: &mut [BattleRuntimeShip],
    mut lostcount: i64,
) {
    while lostcount > 0 {
        let mut best_slot: Option<(usize, usize, i64)> = None;
        for (ship_idx, ship) in ships.iter().enumerate() {
            for (slot_idx, slot_item) in ship.slot_items.iter().enumerate().take(5) {
                let onslot = ship.ship.api_onslot[slot_idx];
                if onslot <= 0 {
                    continue;
                }
                let Some(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()
                else {
                    continue;
                };
                if !is_air_combat_type(mst.api_type[2]) {
                    continue;
                }
                if best_slot.is_none_or(|(_, _, current)| onslot > current) {
                    best_slot = Some((ship_idx, slot_idx, onslot));
                }
            }
        }

        let Some((ship_idx, slot_idx, _)) = best_slot else {
            break;
        };
        ships[ship_idx].ship.api_onslot[slot_idx] -= 1;
        lostcount -= 1;
    }
}

// ---------------------------------------------------------------------------
// Single-slot airstrike damage
// ---------------------------------------------------------------------------

/// 連合艦隊補正 of an airstrike: only a friendly strike on an enemy combined
/// fleet has one (−10 against its main fleet, −20 against its escort fleet).
fn aerial_correction(defender: &BattleRuntimeShip) -> f64 {
    defender.enemy_deck.map_or(0.0, |role| {
        crate::combined::combined_correction_vs_enemy_combined(
            crate::combined::CombinedAttackClass::AntiAir,
            role,
            true,
        ) as f64
    })
}

/// The share of carrier strikes that would land on a target that cannot
/// dodge, in percent (`kcsim.js` `airstrike`).
pub(super) const AIRSTRIKE_HIT_PERCENT: f64 = 95.0;

/// Calculate airstrike damage for a single bomber slot.
///
/// Uses bomb/torpedo stat × √(onslot) + 25, capped at 170.
fn calculate_single_slot_airstrike_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    slot_item: &KcApiSlotItem,
    onslot: i64,
    defender: &BattleRuntimeShip,
    outcome: HitOutcome,
    critical_damage: f64,
) -> i64 {
    if onslot <= 0 {
        return 0;
    }
    let Ok(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id) else {
        return 0;
    };
    if !is_airstrike_attack_type(mst.api_type[2]) {
        return 0;
    }
    let is_torpedo_bomber =
        KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
    let stat = if is_torpedo_bomber {
        mst.api_raig.max(0) as f64
    } else {
        mst.api_baku.max(0) as f64
    };
    let bomb_power = stat * (onslot as f64).sqrt();
    if bomb_power <= 0.0 {
        return 0;
    }
    let raw_power = bomb_power + 25.0 + aerial_correction(defender);
    let capped = outcome.power_with(apply_cap(raw_power, 170.0) as f64, critical_damage);
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped, defense, defender.hp())
}

// ---------------------------------------------------------------------------
// Airstrike phase execution
// ---------------------------------------------------------------------------

fn execute_airstrike_phase(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    is_enemy_side: bool,
    output: &mut AirstrikeOutput,
) {
    // Phase 1: Dive bombing — iterate per bomber slot (non-torpedo types)
    for (ship_idx, ship) in attackers.iter_mut().enumerate() {
        let planes = plane_proficiency(codex, ship);
        for (slot_idx, slot_item) in ship.slot_items.iter().enumerate() {
            let onslot = ship.ship.api_onslot.get(slot_idx).copied().unwrap_or(0);
            if onslot <= 0 {
                continue;
            }
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id) else {
                continue;
            };
            let Some(type3) = KcSlotItemType3::n(mst.api_type[2]) else {
                continue;
            };
            if !is_airstrike_attack_type(mst.api_type[2]) {
                continue;
            }
            if type3 == KcSlotItemType3::CarrierBasedTorpedoBomber {
                continue;
            }

            let alive_targets: Vec<usize> = defenders
                .iter()
                .enumerate()
                .filter(|(_, s)| s.is_alive())
                .map(|(i, _)| i)
                .collect();
            if alive_targets.is_empty() {
                continue;
            }
            let target_idx = alive_targets[rng
                .choose_index(alive_targets.len())
                .expect("alive_targets non-empty by construction")];
            let outcome =
                roll_strike(codex, rng, &defenders[target_idx], AIRSTRIKE_HIT_PERCENT, 1.0, planes);
            let damage = calculate_single_slot_airstrike_damage(
                codex,
                rng,
                slot_item,
                onslot,
                &defenders[target_idx],
                outcome,
                planes.critical_damage,
            );
            if outcome == HitOutcome::Critical {
                output.cl_flags[target_idx] = 1;
            }
            if damage > 0 {
                let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
                // display_damage returns dealt for friendly defenders (sinking protection),
                // raw for enemy defenders. Must NOT accumulate raw_dmg directly.
                let display =
                    crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
                output.damage[target_idx] += display;
            }
            // A strike that misses is still flown at its target: the client
            // draws the run from the flag and the miss from the zero.
            if damage > 0 || outcome == HitOutcome::Miss {
                output.bak_targets[ship_idx] = target_idx as i64;
                output.bak_flags[target_idx] = 1;
            }
        }
    }

    // Phase 2: Torpedo bombing — iterate per torpedo bomber slot
    for (ship_idx, ship) in attackers.iter_mut().enumerate() {
        let planes = plane_proficiency(codex, ship);
        for (slot_idx, slot_item) in ship.slot_items.iter().enumerate() {
            let onslot = ship.ship.api_onslot.get(slot_idx).copied().unwrap_or(0);
            if onslot <= 0 {
                continue;
            }
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id) else {
                continue;
            };
            if KcSlotItemType3::n(mst.api_type[2])
                != Some(KcSlotItemType3::CarrierBasedTorpedoBomber)
            {
                continue;
            }

            let alive_targets: Vec<usize> = defenders
                .iter()
                .enumerate()
                .filter(|(_, s)| s.is_alive())
                .map(|(i, _)| i)
                .collect();
            if alive_targets.is_empty() {
                continue;
            }
            let target_idx = alive_targets[rng
                .choose_index(alive_targets.len())
                .expect("alive_targets non-empty by construction")];
            let outcome =
                roll_strike(codex, rng, &defenders[target_idx], AIRSTRIKE_HIT_PERCENT, 1.0, planes);
            let damage = calculate_single_slot_airstrike_damage(
                codex,
                rng,
                slot_item,
                onslot,
                &defenders[target_idx],
                outcome,
                planes.critical_damage,
            );
            if outcome == HitOutcome::Critical {
                output.cl_flags[target_idx] = 1;
            }
            if damage > 0 {
                let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
                // display_damage returns dealt for friendly defenders (sinking protection),
                // raw for enemy defenders. Must NOT accumulate raw_dmg directly.
                let display =
                    crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
                output.damage[target_idx] += display;
            }
            if damage > 0 || outcome == HitOutcome::Miss {
                output.rai_targets[ship_idx] = target_idx as i64;
                output.rai_flags[target_idx] = 1;
            }
        }
    }

    // Attribute total damage to best bomber ship (for statistics)
    if !is_enemy_side && let Some(best_idx) = best_bomber_index(codex, attackers) {
        let total: i64 = output.damage.iter().sum();
        attackers[best_idx].damage_dealt += total;
    }
}

// ---------------------------------------------------------------------------
// Full kouku simulation
// ---------------------------------------------------------------------------

pub(crate) fn simulate_kouku(
    codex: &Codex,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    rng: &mut impl BattleRng,
) -> BattleKouku {
    let friend_planes = total_plane_count(codex, friendly);
    let enemy_planes = total_plane_count(codex, enemy);

    let friend_fighter_power = calculate_fighter_power(codex, friendly);
    let enemy_fighter_power = calculate_fighter_power(codex, enemy);
    let air_state = AirState::from_power(friend_fighter_power, enemy_fighter_power);

    // Stage 1: the fight for the air costs every slot that flies in it.
    let stage1_f_lost = fight_for_the_air(codex, rng, friendly, air_state, true);
    let stage1_e_lost = fight_for_the_air(codex, rng, enemy, air_state, false);

    // Stage 2: each slot that comes to strike is fired on by one ship.
    let (stage2_f_count, stage2_f_lost) = fly_through_anti_air(codex, rng, friendly, enemy);
    let (stage2_e_count, stage2_e_lost) = fly_through_anti_air(codex, rng, enemy, friendly);

    // Stage 3: bombing damage
    let mut api_edam = vec![0i64; enemy.len()];
    let mut api_fdam = vec![0i64; friendly.len()];
    let mut api_erai = vec![-1i64; enemy.len()];
    let mut api_ebak = vec![-1i64; enemy.len()];
    let mut api_frai = vec![-1i64; friendly.len()];
    let mut api_fbak = vec![-1i64; friendly.len()];
    let mut api_erai_flag = vec![0i64; enemy.len()];
    let mut api_ebak_flag = vec![0i64; enemy.len()];
    let mut api_frai_flag = vec![0i64; friendly.len()];
    let mut api_fbak_flag = vec![0i64; friendly.len()];
    let mut api_ecl_flag = vec![0i64; enemy.len()];
    let mut api_fcl_flag = vec![0i64; friendly.len()];

    // Stage 3: Per-slot bombing — split into dive bombing and torpedo bombing phases
    // Each bomber slot independently selects a random alive target.
    execute_airstrike_phase(
        codex,
        rng,
        friendly,
        enemy,
        false,
        &mut AirstrikeOutput {
            damage: &mut api_edam,
            bak_targets: &mut api_fbak,
            rai_targets: &mut api_frai,
            bak_flags: &mut api_ebak_flag,
            rai_flags: &mut api_erai_flag,
            cl_flags: &mut api_ecl_flag,
        },
    );
    execute_airstrike_phase(
        codex,
        rng,
        enemy,
        friendly,
        true,
        &mut AirstrikeOutput {
            damage: &mut api_fdam,
            bak_targets: &mut api_ebak,
            rai_targets: &mut api_erai,
            bak_flags: &mut api_fbak_flag,
            rai_flags: &mut api_frai_flag,
            cl_flags: &mut api_fcl_flag,
        },
    );

    BattleKouku {
        api_plane_from: [attack_plane_from(codex, friendly), attack_plane_from(codex, enemy)],
        api_stage1: BattleKoukuStage1 {
            api_f_count: friend_planes,
            api_f_lostcount: stage1_f_lost,
            api_e_count: enemy_planes,
            api_e_lostcount: stage1_e_lost,
            api_disp_seiku: air_state.api_disp_seiku(),
            api_touch_plane: [
                first_touch_plane(codex, friendly).unwrap_or(-1),
                first_touch_plane(codex, enemy).unwrap_or(-1),
            ],
        },
        api_stage2: BattleKoukuStage2 {
            api_f_count: stage2_f_count,
            api_f_lostcount: stage2_f_lost,
            api_e_count: stage2_e_count,
            api_e_lostcount: stage2_e_lost,
        },
        api_stage3: BattleKoukuStage3 {
            api_frai,
            api_erai,
            api_fbak,
            api_ebak,
            api_frai_flag,
            api_erai_flag,
            api_fbak_flag,
            api_ebak_flag,
            // The client reads these as the critical flag (hit type = flag + 1).
            api_fcl_flag,
            api_ecl_flag,
            api_fdam: api_fdam.into_iter().map(DamageCell::Plain).collect(),
            api_edam: api_edam.into_iter().map(DamageCell::Plain).collect(),
            api_f_sp_list: vec![None; friendly.len()],
            api_e_sp_list: vec![None; enemy.len()],
        },
        // A combined battle splits deck 2 out of `api_stage3` afterwards, in
        // `combined_packet::split_kouku_stage3`; the airstrike itself treats the
        // two decks as one fleet.
        api_stage3_combined: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::BattleRuntimeShip;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    #[test]
    fn fighter_power_calculates_from_equipment_aa_and_slot_count() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fighter_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        let fighter_mst = codex.manifest.find_slotitem(fighter_mst_id).unwrap();
        let aa = fighter_mst.api_tyku;

        let mut ship_input =
            sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 50);
        ship_input.ship.api_onslot = [18, 0, 0, 0, 0];
        ship_input.slot_items = vec![slotitem_with_mst_id(fighter_mst_id)];

        let ships = vec![BattleRuntimeShip::from(ship_input)];
        let power = calculate_fighter_power(&codex, &ships);
        let expected = (aa as f64 * (18.0_f64).sqrt()).floor() as i64;
        assert_eq!(power, expected);
    }

    /// One ship flying 18 of `mst_id` at proficiency `alv`.
    fn fighter_power_at(codex: &Codex, mst_id: i64, alv: i64) -> i64 {
        let mut ship = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::CVL), 50);
        ship.ship.api_onslot = [18, 0, 0, 0, 0];
        let mut item = slotitem_with_mst_id(mst_id);
        item.api_alv = Some(alv);
        ship.slot_items = vec![item];
        calculate_fighter_power(codex, &[BattleRuntimeShip::from(ship)])
    }

    #[test]
    fn proficiency_adds_to_fighter_power_inside_the_slot_floor() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let root = 18.0_f64.sqrt();
        // (kind, level, what the level adds): a fighter's step, a seaplane
        // bomber's smaller one, and the root alone for a torpedo bomber.
        for (kind, alv, bonus) in [
            (KcSlotItemType3::CarrierBasedFighter, 7, 12.0_f64.sqrt() + 22.0),
            (KcSlotItemType3::CarrierBasedFighter, 2, 2.5_f64.sqrt() + 2.0),
            (KcSlotItemType3::CarrierBasedFighter, 1, 1.0),
            (KcSlotItemType3::SeaBasedBomber, 7, 12.0_f64.sqrt() + 6.0),
            (KcSlotItemType3::CarrierBasedTorpedoBomber, 7, 12.0_f64.sqrt()),
            (KcSlotItemType3::CarrierBasedFighter, 0, 0.0),
        ] {
            let mst_id = first_slotitem_mst_by_type(&codex, kind);
            let aa = codex.manifest.find_slotitem(mst_id).unwrap().api_tyku.max(0) as f64;
            assert_eq!(
                fighter_power_at(&codex, mst_id, alv),
                (aa * root + bonus).floor() as i64,
                "{kind:?} at level {alv}"
            );
        }
    }

    #[test]
    fn proficiency_fighter_power_follows_the_source_figures() {
        let fighter = KcSlotItemType3::CarrierBasedFighter as i64;
        // 対空 10 on 18 aircraft is 42.43; level 7 adds sqrt(12) + 22 = 25.46.
        let base = 10.0 * 18.0_f64.sqrt();
        assert_eq!(base.floor() as i64, 42);
        assert_eq!((base + proficiency_fighter_power(fighter, Some(7), false)).floor() as i64, 67);
        assert_eq!(proficiency_fighter_power(fighter, None, false), 0.0);
        // A land attacker has none aboard a ship and the root alone from a base.
        let land = KcSlotItemType3::LandBasedAttacker as i64;
        assert_eq!(proficiency_fighter_power(land, Some(7), false), 0.0);
        assert_eq!(proficiency_fighter_power(land, Some(7), true), 12.0_f64.sqrt());
        let local = KcSlotItemType3::LocalFighter as i64;
        assert_eq!(proficiency_fighter_power(local, Some(7), true), 12.0_f64.sqrt() + 22.0);
    }

    #[test]
    fn kouku_stage1_reports_nonzero_losses_when_planes_present() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let mut friend = sample_ship(&codex, cvl_mst, 50);
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, cvl_mst, 50);
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        assert!(kouku.api_stage1.api_f_count > 0);
        assert!(kouku.api_stage1.api_e_count > 0);
        let total_f_lost = kouku.api_stage1.api_f_lostcount + kouku.api_stage2.api_f_lostcount;
        let total_e_lost = kouku.api_stage1.api_e_lostcount + kouku.api_stage2.api_e_lostcount;
        assert!(total_f_lost + total_e_lost > 0 || kouku.api_stage1.api_f_count == 0);
    }

    #[test]
    fn kouku_does_not_wipe_all_enemy_planes_unconditionally() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let mut friend = sample_ship(&codex, bb_mst, 50);
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.ship.api_taiku[0] = 10;

        let mut enemy = sample_ship(&codex, cvl_mst, 50);
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        let remaining_enemy_planes = total_plane_count(&codex, &enemies);
        assert!(remaining_enemy_planes > 0, "enemy planes should not be fully wiped");
        assert!(kouku.api_stage2.api_e_lostcount < kouku.api_stage2.api_e_count);
    }

    #[test]
    fn kouku_air_state_reflects_fighter_power_balance() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, cvl_mst, 50);
        let fighter_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        friend.ship.api_onslot = [24, 0, 0, 0, 0];
        friend.slot_items = vec![slotitem_with_mst_id(fighter_mst_id)];
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let friendly_fp =
            calculate_fighter_power(&codex, &[BattleRuntimeShip::from(friend.clone())]);
        assert!(friendly_fp > 0, "CVL with fighter should have positive fighter power");

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);
        assert_eq!(kouku.api_stage1.api_disp_seiku, 1); // supremacy
    }

    /// Skilled bombers score criticals and flag their target; green ones never
    /// do, and the proficiency costs no extra draw.
    #[test]
    fn skilled_bombers_score_criticals_and_flag_the_target() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);

        let fly = |alv: Option<i64>, seed: u64| {
            let mut friend = sample_ship(&codex, cvl_mst, 50);
            let mut item = slotitem_with_mst_id(bomber);
            item.api_alv = alv;
            friend.ship.api_onslot = [18, 0, 0, 0, 0];
            friend.slot_items = vec![item];
            let mut enemy = sample_ship(&codex, dd_mst, 50);
            enemy.ship.api_nowhp = 9999;
            enemy.ship.api_maxhp = 9999;
            let mut friendly = vec![BattleRuntimeShip::from(friend)];
            let mut enemies = vec![BattleRuntimeShip::from(enemy)];
            let mut rng = crate::random::SeededRng::new(seed);
            let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);
            (kouku.api_stage3.api_ecl_flag[0], rng.roll_range(0, 1000))
        };

        let mut criticals = 0;
        for seed in 0..200 {
            let (green_flag, green_next) = fly(None, seed);
            let (skilled_flag, skilled_next) = fly(Some(7), seed);
            assert_eq!(green_flag, 0, "seed {seed}: a green squadron scored a critical");
            assert_eq!(green_next, skilled_next, "seed {seed}: proficiency moved the stream");
            criticals += skilled_flag;
        }
        // 8 in a hundred of the strikes that are flown.
        assert!((5..=30).contains(&criticals), "{criticals} criticals in 200 strikes");
    }

    /// Losses are rolled slot by slot: a small slot loses its own share and can
    /// be emptied, which taking from the largest slot first never did.
    #[test]
    fn every_slot_takes_its_own_losses() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cv_mst = first_ship_mst_by_type(&codex, KcShipType::CV);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let fighter = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        let fly = |seed: u64, friendly_side: bool| {
            let mut carrier = sample_ship(&codex, cv_mst, 50);
            carrier.slot_items = vec![
                slotitem_with_mst_id(bomber),
                slotitem_with_mst_id(bomber),
                slotitem_with_mst_id(bomber),
            ];
            carrier.ship.api_onslot = [30, 12, 3, 0, 0];
            // The other side holds the air, so every slot loses a quarter or more.
            let mut screen = sample_ship(&codex, cv_mst, 50);
            screen.slot_items = vec![slotitem_with_mst_id(fighter)];
            screen.ship.api_onslot = [90, 0, 0, 0, 0];
            screen.ship.api_nowhp = 9999;
            screen.ship.api_maxhp = 9999;
            // An escort with anti-air to spare: her fixed shot alone takes five.
            let mut escort = sample_ship(&codex, dd_mst, 50);
            escort.ship.api_taiku[0] = 900;
            let (mut friendly, mut enemies) = if friendly_side {
                (
                    vec![BattleRuntimeShip::new(carrier, true, true)],
                    vec![
                        BattleRuntimeShip::new(screen, false, true),
                        BattleRuntimeShip::new(escort, false, true),
                    ],
                )
            } else {
                (
                    vec![
                        BattleRuntimeShip::new(screen, true, true),
                        BattleRuntimeShip::new(escort, true, true),
                    ],
                    vec![BattleRuntimeShip::new(carrier, false, true)],
                )
            };
            let mut rng = crate::random::SeededRng::new(seed);
            let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);
            let left = if friendly_side {
                friendly[0].ship.api_onslot
            } else {
                enemies[0].ship.api_onslot
            };
            (left, kouku)
        };

        let mut small_slot_emptied = 0;
        for seed in 0..50 {
            let (left, kouku) = fly(seed, true);
            assert_eq!(kouku.api_stage1.api_disp_seiku, 4, "the enemy holds the air");
            assert!(left[0] <= 30 - 7, "seed {seed}: the large slot lost a quarter: {left:?}");
            assert!(left[1] <= 12 - 3, "seed {seed}: the middle slot lost a quarter: {left:?}");
            assert_eq!(
                kouku.api_stage1.api_f_lostcount + kouku.api_stage2.api_f_lostcount,
                45 - left.iter().sum::<i64>(),
                "seed {seed}: the two stages report what the slots lost"
            );
            small_slot_emptied += i64::from(left[2] == 0);

            // The side with fighters alone sends nothing through anti-air fire.
            let (_, kouku) = fly(seed, false);
            assert_eq!(
                (kouku.api_stage2.api_f_count, kouku.api_stage2.api_f_lostcount),
                (0, 0),
                "seed {seed}: fighters are not fired on"
            );
        }
        assert!(small_slot_emptied > 0, "a slot of three is emptied now and then");
    }

    /// A friendly ship's fire always takes one aircraft from each slot it is
    /// aimed at, whether or not either of its shots lands.
    #[test]
    fn friendly_fire_takes_one_from_every_slot_that_strikes() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let fighter = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        let mut carrier = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CV), 50);
        carrier.slot_items = vec![
            slotitem_with_mst_id(bomber),
            slotitem_with_mst_id(fighter),
            slotitem_with_mst_id(bomber),
        ];
        carrier.ship.api_onslot = [30, 12, 3, 0, 0];
        let defender = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        let defenders = vec![BattleRuntimeShip::new(defender, true, true)];

        for seed in 0..50 {
            let mut attackers = vec![BattleRuntimeShip::new(carrier.clone(), false, true)];
            let mut rng = crate::random::SeededRng::new(seed);
            let (flew, shot) = fly_through_anti_air(&codex, &mut rng, &mut attackers, &defenders);
            let left = attackers[0].ship.api_onslot;
            assert_eq!(flew, 33, "the fighters between them do not come to strike");
            assert!(left[0] < 30 && left[2] < 3, "seed {seed}: {left:?}");
            assert_eq!(left[1], 12, "seed {seed}: fighters are not fired on");
            assert_eq!(shot, 45 - left.iter().sum::<i64>());
        }
    }

    #[test]
    fn kouku_flag_arrays_match_fleet_sizes() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // 3 friendly (CVL + 2 DD), 6 enemy (6 DD)
        let mut friendly: Vec<BattleRuntimeShip> = vec![];
        for _ in 0..3 {
            let mut ship = sample_ship(&codex, cvl_mst, 50);
            ship.ship.api_soukou[0] = 200;
            ship.ship.api_nowhp = 200;
            ship.ship.api_maxhp = 200;
            friendly.push(BattleRuntimeShip::from(ship));
        }
        let mut enemies: Vec<BattleRuntimeShip> = vec![];
        for _ in 0..6 {
            let mut ship = sample_ship(&codex, dd_mst, 50);
            ship.ship.api_soukou[0] = 200;
            ship.ship.api_nowhp = 200;
            ship.ship.api_maxhp = 200;
            enemies.push(BattleRuntimeShip::from(ship));
        }
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        let s3 = &kouku.api_stage3;
        assert_eq!(s3.api_frai_flag.len(), 3, "api_frai_flag should be friendly-sized (3)");
        assert_eq!(s3.api_fbak_flag.len(), 3, "api_fbak_flag should be friendly-sized (3)");
        assert_eq!(s3.api_erai_flag.len(), 6, "api_erai_flag should be enemy-sized (6)");
        assert_eq!(s3.api_ebak_flag.len(), 6, "api_ebak_flag should be enemy-sized (6)");
    }

    #[test]
    fn kouku_fdam_uses_display_damage_not_raw_under_protection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        // Friendly DD with very low HP (taiha) — sinking protection should cap damage
        let mut friend = sample_ship(&codex, dd_mst, 50);
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 10;
        friend.ship.api_maxhp = 30;

        // Enemy CVL equipped with bombers to ensure airstrike damage
        let mut enemy = sample_ship(&codex, cvl_mst, 99);
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;
        enemy.slot_items = vec![slotitem_with_mst_id(bomber_id)];
        enemy.ship.api_onslot = [18, 0, 0, 0, 0];

        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];
        let mut rng = crate::random::SeededRng::new(2);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        let fdam = kouku.api_stage3.api_fdam[0].amount();
        assert!(fdam > 0, "enemy CVL with bombers must deal airstrike damage");
        assert!(
            fdam <= 10,
            "api_fdam ({fdam}) should reflect dealt damage, not raw overkill, \
             for friendly ships under sinking protection"
        );
    }

    #[test]
    fn kouku_fdam_equals_actual_hp_loss_at_full_hp() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        // Friendly DD at full HP — no protection triggered, fdam == actual HP lost
        let mut friend = sample_ship(&codex, dd_mst, 50);
        friend.ship.api_soukou[0] = 0;
        let hp_before = friend.ship.api_nowhp;

        // Enemy CVL equipped with bombers to ensure airstrike damage
        let mut enemy = sample_ship(&codex, cvl_mst, 99);
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;
        enemy.slot_items = vec![slotitem_with_mst_id(bomber_id)];
        enemy.ship.api_onslot = [18, 0, 0, 0, 0];

        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];
        let mut rng = crate::random::SeededRng::new(2);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        let fdam = kouku.api_stage3.api_fdam[0].amount();
        let hp_after = friendly[0].hp();
        assert!(fdam > 0, "enemy CVL with bombers must deal airstrike damage");
        assert_eq!(
            fdam,
            hp_before - hp_after,
            "at full HP, api_fdam should equal actual HP lost (no protection)"
        );
    }

    #[test]
    fn kouku_flagship_survives_airstrike() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Flagship (index 0) with low HP — must survive
        let mut friend = sample_ship(&codex, dd_mst, 50);
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 5;
        friend.ship.api_maxhp = 30;

        let mut enemy = sample_ship(&codex, cvl_mst, 99);
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];
        let mut rng = crate::random::SeededRng::new(42);

        simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        assert!(
            friendly[0].hp() > 0,
            "flagship (index 0) must survive airstrike under sinking protection"
        );
    }

    #[test]
    fn kouku_edam_can_exceed_enemy_hp_overkill() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        // Friendly CVL equipped with bombers vs enemy DD with very low HP
        let mut friend = sample_ship(&codex, cvl_mst, 99);
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.slot_items = vec![slotitem_with_mst_id(bomber_id)];
        friend.ship.api_onslot = [18, 0, 0, 0, 0];

        let mut enemy = sample_ship(&codex, dd_mst, 1);
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_nowhp = 5;
        enemy.ship.api_maxhp = 5;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        let edam = kouku.api_stage3.api_edam[0].amount();
        let enemy_hp_after = enemies[0].hp();
        assert!(edam > 0, "friendly CVL with bombers must deal airstrike damage");
        assert!(
            edam >= enemy_hp_after,
            "api_edam ({edam}) should >= remaining HP ({enemy_hp_after}), allowing overkill display"
        );
    }

    #[test]
    fn bb_with_seaplane_not_in_plane_from() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let fighter_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        let seaplane_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);

        // BB equipped with seaplane bomber (has planes in slot)
        let mut bb = sample_ship(&codex, bb_mst, 50);
        bb.slot_items = vec![slotitem_with_mst_id(seaplane_id)];
        bb.ship.api_onslot = [4, 0, 0, 0, 0];

        // CVL with fighter so kouku phase triggers
        let mut cvl = sample_ship(&codex, cvl_mst, 50);
        cvl.slot_items = vec![slotitem_with_mst_id(fighter_id)];
        cvl.ship.api_onslot = [18, 0, 0, 0, 0];

        let ships = vec![BattleRuntimeShip::from(bb), BattleRuntimeShip::from(cvl)];
        let result = attack_plane_from(&codex, &ships);

        assert_eq!(
            result,
            vec![2],
            "BB should not be in plane_from even with seaplane; only CVL at index 2"
        );
    }

    #[test]
    fn bbv_with_seaplane_is_in_plane_from() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);
        let seaplane_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);

        let mut bbv = sample_ship(&codex, bbv_mst, 50);
        bbv.slot_items = vec![slotitem_with_mst_id(seaplane_id)];
        bbv.ship.api_onslot = [4, 0, 0, 0, 0];

        let ships = vec![BattleRuntimeShip::from(bbv)];
        let result = attack_plane_from(&codex, &ships);

        assert_eq!(result, vec![1], "BBV with seaplane should be in plane_from");
    }
}
