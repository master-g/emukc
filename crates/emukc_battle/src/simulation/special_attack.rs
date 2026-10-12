//! Flagship special attack system.
//!
//! Implements Nelson Touch, Nagato-class broadside, Colorado broadside,
//! Richelieu attack, and Queen Elizabeth attack — multi-ship special
//! attacks triggered by specific flagship ships in specific formations.

use emukc_model::{codex::Codex, kc2::KcShipType};

use crate::accuracy::{Aim, AttackKind, roll_attack};
use crate::damage::calculate_shelling_damage;
use crate::random::BattleRng;
use crate::simulation::day_attack::DayAttackKind;
use crate::targeting::{
    can_shell_day_ship, has_slotitem_id, select_random_target_index, ship_type, target_class,
};
use crate::types::{BattleHougeki, BattleRuntimeShip, DamageCell, EngagementType, ShellingParams};

// ---------------------------------------------------------------------------
// Ship ID constants
// ---------------------------------------------------------------------------

/// Nelson remodels
const NELSON_IDS: &[i64] = &[571, 576];
/// Rodney remodels
const RODNEY_IDS: &[i64] = &[572, 577];
/// 長門改二
const NAGATO_K2_ID: i64 = 541;
/// 陸奥改二
const MUTSU_K2_ID: i64 = 573;
/// Colorado remodels
const COLORADO_IDS: &[i64] = &[601, 1496];
/// Maryland remodels
const MARYLAND_IDS: &[i64] = &[913, 918];
/// Richelieu remodels (Kai + Deux)
const RICHELIEU_IDS: &[i64] = &[392, 969];
/// Jean Bart Kai
const JEAN_BART_KAI_ID: i64 = 724;
/// Warspite Kai
const WARSPIRE_KAI_ID: i64 = 364;
/// Valiant Kai
const VALIANT_KAI_ID: i64 = 733;

/// The Big Seven of the Colorado attack, as wikiwiki and the source's `kcsim.js` list them:
/// not 長門, 陸奥 or Nelson before their remodel, nor Rodney.
const BIG_SEVEN_IDS: &[i64] = &[
    275, 541, // 長門 kai/k2
    276, 573, // 陸奥 kai/k2
    576, // Nelson kai
    601, 1496, // Colorado base/kai
    913, 918, // Maryland base/kai
];

// ---------------------------------------------------------------------------
// Special attack type enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecialAttackType {
    NelsonTouch = 100,
    NagatoClassBroadside = 101,
    NagatoMutsuBroadside = 102,
    ColoradoBroadside = 103,
    RichelieuAttack = 105,
    QueenElizabethAttack = 106,
}

impl SpecialAttackType {
    fn api_value(self) -> i64 {
        self as i64
    }
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Participant info for a special attack.
pub(crate) struct Participant {
    fleet_index: usize,
    multiplier: f64,
}

/// Resolved special attack with participants and target.
pub(crate) struct ResolvedSpecialAttack {
    attack_type: SpecialAttackType,
    participants: Vec<Participant>,
    /// Per-participant equipment bonus multiplier (AP shell, radar, etc.)
    equip_bonuses: Vec<f64>,
}

fn is_bb_class(t: Option<KcShipType>) -> bool {
    matches!(t, Some(KcShipType::BB | KcShipType::FBB | KcShipType::BBV))
}

fn is_submarine_or_carrier(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    let st = ship_type(codex, ship);
    matches!(
        st,
        Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB | KcShipType::SS | KcShipType::SSV)
    )
}

fn is_ship_id(ship: &BattleRuntimeShip, ids: &[i64]) -> bool {
    ids.contains(&ship.ship.api_ship_id)
}

fn hp_ratio(ship: &BattleRuntimeShip) -> f64 {
    ship.hp() as f64 / ship.ship.api_maxhp.max(1) as f64
}

/// Flagship must be shouha or better (HP > 75%).
fn is_flagship_healthy(ship: &BattleRuntimeShip) -> bool {
    hp_ratio(ship) > 0.75
}

/// Companion must be chuuha or better (HP > 50%).
fn is_companion_healthy(ship: &BattleRuntimeShip) -> bool {
    hp_ratio(ship) > 0.5
}

/// Check if a ship has an AP shell equipped.
fn has_ap_shell(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().any(|si| {
        codex
            .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
            .ok()
            .and_then(|mst| emukc_model::kc2::KcSlotItemType3::n(mst.api_type[2]))
            .is_some_and(|t| t == emukc_model::kc2::KcSlotItemType3::ArmorPiercingShell)
    })
}

/// Check if a ship has a surface radar (type3=12/13, LoS>=5).
fn has_surface_radar(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    use emukc_model::kc2::KcSlotItemType3;
    ship.slot_items.iter().any(|si| {
        let Some(mst) =
            codex.find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id).ok()
        else {
            return false;
        };
        let Some(t) = KcSlotItemType3::n(mst.api_type[2]) else {
            return false;
        };
        matches!(t, KcSlotItemType3::SmallRadar | KcSlotItemType3::LargeRadar) && mst.api_saku >= 5
    })
}

fn is_big_seven(ship: &BattleRuntimeShip) -> bool {
    BIG_SEVEN_IDS.contains(&ship.ship.api_ship_id)
}

/// Calculate equipment bonus multiplier for a participating ship.
/// AP shell: x1.35, surface radar: x1.15 (multiplicative).
/// Nelson Touch has NO equipment bonuses.
fn equip_bonus(codex: &Codex, ship: &BattleRuntimeShip, no_equip: bool) -> f64 {
    if no_equip {
        return 1.0;
    }
    let mut bonus = 1.0;
    if has_ap_shell(codex, ship) {
        bonus *= 1.35;
    }
    if has_surface_radar(codex, ship) {
        bonus *= 1.15;
    }
    bonus
}

/// Formation ID: 複縦陣=2, 梯形陣=4
const FORMATION_DOUBLE_LINE: i64 = 2;
const FORMATION_ECHELON: i64 = 4;
/// 第二警戒航行序列, which a combined fleet takes in place of either (`kcsim.js`
/// `canSpecialAttackUnique`: `formation.id != 12 && formation.id != 4`, or `!= 2`).
const FORMATION_CRUISING_2: i64 = 12;

/// Check Nelson Touch eligibility.
/// Flagship: Nelson/Rodney, positions 1/3/5, formation 複縦陣.
fn check_nelson_touch(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_DOUBLE_LINE | FORMATION_CRUISING_2) {
        return None;
    }
    let flagship = fleet.first()?;
    if !is_ship_id(flagship, NELSON_IDS) && !is_ship_id(flagship, RODNEY_IDS) {
        return None;
    }
    if !is_flagship_healthy(flagship) {
        return None;
    }

    let mut participants = vec![Participant {
        fleet_index: 0,
        multiplier: 2.0,
    }];

    // Positions 3 (index 2) and 5 (index 4)
    let companion_indices: &[usize] = &[2, 4];
    for &idx in companion_indices {
        let Some(companion) = fleet.get(idx) else {
            continue;
        };
        if !companion.is_alive() || is_submarine_or_carrier(codex, companion) {
            continue;
        }
        if !is_companion_healthy(companion) {
            continue;
        }
        participants.push(Participant {
            fleet_index: idx,
            multiplier: 2.0,
        });
    }

    // Need at least 1 companion to trigger
    if participants.len() < 2 {
        return None;
    }

    // Nelson+Rodney bonus: if both present, flagship x1.15, Rodney companion x1.20
    let flag_is_nelson = is_ship_id(flagship, NELSON_IDS);
    let flag_is_rodney = is_ship_id(flagship, RODNEY_IDS);
    if flag_is_nelson || flag_is_rodney {
        let other_ids = if flag_is_nelson {
            RODNEY_IDS
        } else {
            NELSON_IDS
        };
        for p in participants.iter_mut().skip(1) {
            if is_ship_id(&fleet[p.fleet_index], other_ids) {
                p.multiplier *= 1.20;
                participants[0].multiplier *= 1.15;
                break;
            }
        }
    }

    // Nelson Touch has NO equipment bonuses
    let equip_bonuses: Vec<f64> = participants.iter().map(|_| 1.0).collect();

    Some(ResolvedSpecialAttack {
        attack_type: SpecialAttackType::NelsonTouch,
        participants,
        equip_bonuses,
    })
}

/// Check 長門一斉射 (101) / 長門陸奥 (102) eligibility.
/// Flagship: 長門改二, 2nd ship: BB type (or 陸奥改二 for 102), 梯形陣.
fn check_nagato_broadside(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_ECHELON | FORMATION_CRUISING_2) {
        return None;
    }
    let flagship = fleet.first()?;
    if flagship.ship.api_ship_id != NAGATO_K2_ID {
        return None;
    }
    if !is_flagship_healthy(flagship) {
        return None;
    }

    let second = fleet.get(1)?;
    if !second.is_alive() || !is_bb_class(ship_type(codex, second)) {
        return None;
    }
    if !is_companion_healthy(second) {
        return None;
    }

    // Check if 2nd ship is 陸奥改二 for type 102
    let is_mutsu_k2 = second.ship.api_ship_id == MUTSU_K2_ID;
    let (attack_type, flag_mult, companion_mult) = if is_mutsu_k2 {
        (SpecialAttackType::NagatoMutsuBroadside, 1.68, 1.68)
    } else {
        (SpecialAttackType::NagatoClassBroadside, 1.4, 1.1)
    };

    let participants = vec![
        Participant {
            fleet_index: 0,
            multiplier: flag_mult,
        },
        Participant {
            fleet_index: 1,
            multiplier: companion_mult,
        },
    ];

    let equip_bonuses: Vec<f64> =
        participants.iter().map(|p| equip_bonus(codex, &fleet[p.fleet_index], false)).collect();

    Some(ResolvedSpecialAttack {
        attack_type,
        participants,
        equip_bonuses,
    })
}

/// Check Colorado broadside (103) eligibility.
/// Flagship: Colorado/Maryland, positions 1/2/3 all BB-type, 梯形陣.
fn check_colorado_broadside(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_ECHELON | FORMATION_CRUISING_2) {
        return None;
    }
    let flagship = fleet.first()?;
    if !is_ship_id(flagship, COLORADO_IDS) && !is_ship_id(flagship, MARYLAND_IDS) {
        return None;
    }
    if !is_flagship_healthy(flagship) {
        return None;
    }

    let second = fleet.get(1)?;
    let third = fleet.get(2)?;
    if !second.is_alive() || !is_bb_class(ship_type(codex, second)) || !is_companion_healthy(second)
    {
        return None;
    }
    if !third.is_alive() || !is_bb_class(ship_type(codex, third)) || !is_companion_healthy(third) {
        return None;
    }

    let mut participants = vec![Participant {
        fleet_index: 0,
        multiplier: 1.5,
    }];

    // 2nd ship
    let mut second_mult = 1.3;
    if is_big_seven(second) {
        second_mult *= 1.15;
    }
    participants.push(Participant {
        fleet_index: 1,
        multiplier: second_mult,
    });

    // 3rd ship
    let mut third_mult = 1.3;
    if is_big_seven(third) {
        third_mult *= 1.17;
    }
    participants.push(Participant {
        fleet_index: 2,
        multiplier: third_mult,
    });

    // wikiwiki: an SG レーダー(後期型) is worth 1.15 again on top of a surface radar's.
    let equip_bonuses: Vec<f64> = participants
        .iter()
        .map(|p| {
            let ship = &fleet[p.fleet_index];
            let sg = if has_slotitem_id(ship, SG_RADAR_LATE_ID) {
                1.15
            } else {
                1.0
            };
            equip_bonus(codex, ship, false) * sg
        })
        .collect();

    Some(ResolvedSpecialAttack {
        attack_type: SpecialAttackType::ColoradoBroadside,
        participants,
        equip_bonuses,
    })
}

/// Check Richelieu attack (105) eligibility.
/// Flagship: Richelieu Kai/Deux, 2nd: Jean Bart Kai, 3rd: any BB, 複縦陣.
fn check_richelieu_attack(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_DOUBLE_LINE | FORMATION_CRUISING_2) {
        return None;
    }
    let flagship = fleet.first()?;
    if !is_ship_id(flagship, RICHELIEU_IDS) {
        return None;
    }
    if !is_flagship_healthy(flagship) {
        return None;
    }

    let second = fleet.get(1)?;
    if second.ship.api_ship_id != JEAN_BART_KAI_ID {
        return None;
    }
    if !second.is_alive() || !is_companion_healthy(second) {
        return None;
    }

    let third = fleet.get(2)?;
    if !third.is_alive() || !is_bb_class(ship_type(codex, third)) || !is_companion_healthy(third) {
        return None;
    }

    let participants = vec![
        Participant {
            fleet_index: 0,
            multiplier: 1.3,
        },
        Participant {
            fleet_index: 1,
            multiplier: 1.3,
        },
        Participant {
            fleet_index: 2,
            multiplier: 1.24,
        },
    ];

    let equip_bonuses: Vec<f64> =
        participants.iter().map(|p| equip_bonus(codex, &fleet[p.fleet_index], false)).collect();

    Some(ResolvedSpecialAttack {
        attack_type: SpecialAttackType::RichelieuAttack,
        participants,
        equip_bonuses,
    })
}

/// Check Queen Elizabeth attack (106) eligibility.
/// Flagship: Warspite Kai or Valiant Kai, 2nd: the other sister, 3rd: any BB, 梯形陣.
/// Requires slow fleet (min soku = 5).
fn check_qe_attack(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_ECHELON | FORMATION_CRUISING_2) {
        return None;
    }
    let flagship = fleet.first()?;

    let (_, sister_id) = if flagship.ship.api_ship_id == WARSPIRE_KAI_ID {
        (WARSPIRE_KAI_ID, VALIANT_KAI_ID)
    } else if flagship.ship.api_ship_id == VALIANT_KAI_ID {
        (VALIANT_KAI_ID, WARSPIRE_KAI_ID)
    } else {
        return None;
    };

    if !is_flagship_healthy(flagship) {
        return None;
    }

    // Slow fleet check
    let min_soku =
        fleet.iter().filter(|s| s.is_alive()).map(|s| s.ship.api_soku).min().unwrap_or(0);
    if min_soku > 5 {
        return None;
    }

    let second = fleet.get(1)?;
    if second.ship.api_ship_id != sister_id {
        return None;
    }
    if !second.is_alive() || !is_companion_healthy(second) {
        return None;
    }

    let third = fleet.get(2)?;
    if !third.is_alive() || !is_bb_class(ship_type(codex, third)) || !is_companion_healthy(third) {
        return None;
    }

    let participants = vec![
        Participant {
            fleet_index: 0,
            multiplier: 1.24,
        },
        Participant {
            fleet_index: 1,
            multiplier: 1.24,
        },
        Participant {
            fleet_index: 2,
            multiplier: 1.24,
        },
    ];

    let equip_bonuses: Vec<f64> =
        participants.iter().map(|p| equip_bonus(codex, &fleet[p.fleet_index], false)).collect();

    Some(ResolvedSpecialAttack {
        attack_type: SpecialAttackType::QueenElizabethAttack,
        participants,
        equip_bonuses,
    })
}

// ---------------------------------------------------------------------------
// Trigger rate
// ---------------------------------------------------------------------------

/// wikiwiki's estimate for Nelson Touch, which is a percentage as it stands.
fn nelson_touch_rate(flagship: &BattleRuntimeShip, fleet: &[BattleRuntimeShip]) -> f64 {
    let flag_lv = flagship.ship.api_lv.max(1) as f64;
    let flag_luck = flagship.ship.api_lucky[0].max(0) as f64;
    let third_lv = fleet.get(2).map(|s| s.ship.api_lv.max(1) as f64).unwrap_or(0.0);
    let fifth_lv = fleet.get(4).map(|s| s.ship.api_lv.max(1) as f64).unwrap_or(0.0);
    let total =
        1.1 * flag_lv.sqrt() + third_lv.sqrt() + fifth_lv.sqrt() + 1.4 * flag_luck.sqrt() + 25.0;
    (total / 100.0).clamp(0.0, 1.0)
}

/// wikiwiki's estimate for the Nagato class, a percentage as it stands. The Colorado, Richelieu
/// and Queen Elizabeth attacks have no formula of their own on record and take this one, as
/// wikiwiki supposes of the last.
fn nagato_broadside_rate(flagship: &BattleRuntimeShip, companion: &BattleRuntimeShip) -> f64 {
    let flag_lv = flagship.ship.api_lv.max(1) as f64;
    let comp_lv = companion.ship.api_lv.max(1) as f64;
    let flag_luck = flagship.ship.api_lucky[0].max(0) as f64;
    let comp_luck = companion.ship.api_lucky[0].max(0) as f64;
    let total =
        30.0 + flag_lv.sqrt() + comp_lv.sqrt() + 1.2 * (flag_luck.sqrt() + comp_luck.sqrt());
    (total / 100.0).clamp(0.0, 1.0)
}

/// SG レーダー(後期型)
const SG_RADAR_LATE_ID: i64 = 456;

/// The chance that an attack the fleet can make comes up.
fn trigger_rate(attack_type: SpecialAttackType, fleet: &[BattleRuntimeShip]) -> f64 {
    let flagship = &fleet[0];
    match attack_type {
        SpecialAttackType::NelsonTouch => nelson_touch_rate(flagship, fleet),
        // wikiwiki: 5% more with an SG レーダー(後期型) on one of the three. It does not say
        // the radars add up, so one is counted.
        SpecialAttackType::ColoradoBroadside
            if fleet.iter().take(3).any(|ship| has_slotitem_id(ship, SG_RADAR_LATE_ID)) =>
        {
            (nagato_broadside_rate(flagship, &fleet[1]) + 0.05).min(1.0)
        }
        // Every attack but Nelson Touch names the second ship.
        _ => nagato_broadside_rate(flagship, &fleet[1]),
    }
}

// ---------------------------------------------------------------------------
// Detection entry point
// ---------------------------------------------------------------------------

/// Check if a special attack is eligible and triggered for the attacking fleet.
/// Returns the resolved attack if triggered, None otherwise.
pub(crate) fn try_special_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    let flagship = attackers.first()?;
    if !can_shell_day_ship(codex, flagship) {
        return None;
    }

    // Try each special attack type in priority order
    let candidates: Vec<Option<ResolvedSpecialAttack>> = vec![
        check_nagato_broadside(codex, attackers, formation_id),
        check_colorado_broadside(codex, attackers, formation_id),
        check_nelson_touch(codex, attackers, formation_id),
        check_richelieu_attack(codex, attackers, formation_id),
        check_qe_attack(codex, attackers, formation_id),
    ];

    candidates.into_iter().flatten().find(|candidate| {
        rng.random_f64_range(0.0, 1.0) < trigger_rate(candidate.attack_type, attackers)
    })
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Execute a resolved special attack, producing hougeki entries.
pub(crate) fn execute_special_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    resolved: ResolvedSpecialAttack,
    params: &ShellingParams,
) -> BattleHougeki {
    let attack_type_val = resolved.attack_type.api_value();
    let is_enemy = params.attacker_is_enemy;

    let mut hougeki = BattleHougeki::default();

    // T-disadvantage multiplier for Nelson Touch
    let t_disadv = matches!(params.engagement, EngagementType::TDisadvantage);
    let base_multiplier = if resolved.attack_type == SpecialAttackType::NelsonTouch && t_disadv {
        2.5
    } else {
        1.0
    };

    for (i, participant) in resolved.participants.iter().enumerate() {
        let fleet_idx = participant.fleet_index;

        let attacker = &mut attackers[fleet_idx];
        if !attacker.is_alive() {
            continue;
        }

        let Some(mut target_idx) =
            select_random_target_index(codex, rng, attacker, defenders, params.phase)
        else {
            continue;
        };
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let shield = match crate::targeting::select_escort_shield(
            codex,
            rng,
            defenders,
            target_idx,
            params.defender_formation_id,
        ) {
            Some(escort) => {
                target_idx = escort;
                true
            }
            None => false,
        };

        let is_sub = target_class(codex, &defenders[target_idx]).is_submarine();
        if is_sub {
            continue;
        }

        let total_mult = if base_multiplier != 1.0 {
            participant.multiplier * base_multiplier
        } else {
            participant.multiplier
        };
        let equip_mult = resolved.equip_bonuses.get(i).copied().unwrap_or(1.0);

        let num_hits = if matches!(
            resolved.attack_type,
            SpecialAttackType::NagatoClassBroadside | SpecialAttackType::NagatoMutsuBroadside
        ) && i == 0
        {
            2
        } else {
            1
        };

        let mut hit_damages = Vec::with_capacity(num_hits);
        let mut cls = Vec::with_capacity(num_hits);

        for _ in 0..num_hits {
            let ci_mult = Some(total_mult * equip_mult);
            // ponytail: the source gives each special attack its own accuracy
            // multiplier (`getSpecialAttackMod`, 1.05 to 1.4 and more with AP
            // shells or radar); a plain shelling roll stands in for all of them.
            let outcome = roll_attack(
                codex,
                rng,
                attacker,
                &defenders[target_idx],
                Aim::new(AttackKind::Shelling, params.formation_id, params.defender_formation_id),
            );
            cls.push(outcome.cl());
            let raw = calculate_shelling_damage(
                codex,
                rng,
                attacker,
                &defenders[target_idx],
                params.formation_id,
                params.engagement,
                ci_mult,
                outcome,
            );
            let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
            if !is_enemy {
                attacker.damage_dealt += dealt;
            }
            let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
            hit_damages.push(if shield {
                DamageCell::Shielded(display)
            } else {
                DamageCell::Plain(display)
            });
        }

        hougeki.record_day_attack(
            DayAttackKind::Special(codex, attacker, attack_type_val),
            is_enemy,
            fleet_idx,
            vec![target_idx as i64; num_hits],
            hit_damages,
            cls,
        );
    }

    hougeki
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::BattleRuntimeShip;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;

    fn sample_bb_at(codex: &Codex, mst_id: i64, level: i64) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, level);
        ship.ship.api_nowhp = ship.ship.api_maxhp;
        ship.slot_items.clear();
        ship.ship.api_onslot = [0; 5];
        BattleRuntimeShip::from(ship)
    }

    fn sample_damaged_bb_at(
        codex: &Codex,
        mst_id: i64,
        level: i64,
        hp_ratio: f64,
    ) -> BattleRuntimeShip {
        let mut rt = sample_bb_at(codex, mst_id, level);
        let maxhp = rt.ship.api_maxhp;
        rt.current_hp = (maxhp as f64 * hp_ratio) as i64;
        rt.ship.api_nowhp = rt.current_hp;
        rt
    }

    #[test]
    fn nelson_touch_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nelson = sample_bb_at(&codex, 571, 99);
        let dd = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let dd2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);

        // Fleet: Nelson, DD, BB, DD, BB → positions 0,2,4 are valid
        let fleet = vec![nelson, dd, bb1, dd2, bb2];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_DOUBLE_LINE);
        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.attack_type, SpecialAttackType::NelsonTouch);
        assert_eq!(r.participants.len(), 3);
        assert_eq!(r.participants[0].fleet_index, 0);
        assert_eq!(r.participants[1].fleet_index, 2);
        assert_eq!(r.participants[2].fleet_index, 4);
    }

    #[test]
    fn nelson_touch_wrong_formation() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nelson = sample_bb_at(&codex, 571, 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![nelson, bb1, bb2];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_none(), "wrong formation should fail");
    }

    #[test]
    fn nelson_touch_not_flagship_position() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let nelson = sample_bb_at(&codex, 571, 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        // Nelson at position 1 (not flagship)
        let fleet = vec![bb, nelson, bb2];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_DOUBLE_LINE);
        assert!(resolved.is_none(), "Nelson not at flagship position should fail");
    }

    #[test]
    fn nelson_touch_chuuha_flagship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nelson = sample_damaged_bb_at(&codex, 571, 99, 0.5); // chuuha
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![nelson, bb1, bb2];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_DOUBLE_LINE);
        assert!(resolved.is_none(), "chuuha flagship should not trigger");
    }

    #[test]
    fn nelson_touch_no_equipment_bonuses() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nelson = sample_bb_at(&codex, 571, 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![nelson, bb1, bb2];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_DOUBLE_LINE).unwrap();
        // All equipment bonuses should be 1.0
        assert!(resolved.equip_bonuses.iter().all(|&b| b == 1.0));
    }

    #[test]
    fn nelson_rodney_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nelson = sample_bb_at(&codex, 571, 99);
        let dd = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 99);
        let rodney = sample_bb_at(&codex, 577, 99);
        let fleet = vec![nelson, dd, rodney];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_DOUBLE_LINE).unwrap();
        // Flagship gets 1.15x bonus, Rodney companion gets 1.20x
        assert!((resolved.participants[0].multiplier - 2.0 * 1.15).abs() < f64::EPSILON);
        assert!((resolved.participants[1].multiplier - 2.0 * 1.20).abs() < f64::EPSILON);
    }

    #[test]
    fn special_attack_si_list_entries_are_strings() {
        use crate::types::{AirState, BattlePhase, EngagementType, ShellingParams, SiListId};
        use emukc_model::kc2::types::KcSlotItemType3;

        // Nelson Touch participants push their si_list via text_from_i64. Drive
        // execute_special_attack end-to-end and assert the entries serialize as
        // strings — a swap to num_from_i64 at the push site passes every other
        // test. Ships are armed so display ids are real master ids (positive),
        // not the -1 sentinel which would legitimately stay Num.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let arm = |mst_id: i64| {
            let mut ship = sample_ship(&codex, mst_id, 99);
            ship.ship.api_nowhp = ship.ship.api_maxhp;
            ship.slot_items =
                vec![slotitem_with_mst_id(main_gun_id), slotitem_with_mst_id(main_gun_id)];
            BattleRuntimeShip::from(ship)
        };
        // Nelson flagship + valid surface companions at fleet positions 0,2,4.
        let attackers_template =
            || vec![arm(571), arm(dd_mst), arm(bb_mst), arm(dd_mst), arm(bb_mst)];

        let resolved = check_nelson_touch(&codex, &attackers_template(), FORMATION_DOUBLE_LINE)
            .expect("armed Nelson fleet in double line must resolve Nelson Touch");

        let mut attackers = attackers_template();
        let mut defender = sample_ship(&codex, dd_mst, 30);
        defender.ship.api_soukou[0] = 1;
        defender.ship.api_nowhp = 2000;
        defender.ship.api_maxhp = 2000;
        let mut defenders = vec![BattleRuntimeShip::from(defender)];

        let air_state = AirState::Supremacy;
        let result = execute_special_attack(
            &codex,
            &mut crate::random::SeededRng::new(7),
            &mut attackers,
            &mut defenders,
            resolved,
            &ShellingParams {
                attacker_is_enemy: false,
                formation_id: FORMATION_DOUBLE_LINE,
                defender_formation_id: 0, // unused here: no かばう in this test
                engagement: EngagementType::SameCourse,
                phase: BattlePhase::DayShelling,
                air_state: Some(&air_state),
            },
        );

        assert!(
            !result.api_si_list.is_empty(),
            "special attack must push at least one si_list entry"
        );
        assert!(
            result.api_si_list.iter().flatten().any(|id| matches!(id, SiListId::Text(_))),
            "special attack si_list must contain string entries: {:?}",
            result.api_si_list
        );
    }

    #[test]
    fn nagato_broadside_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nagato_k2 = sample_bb_at(&codex, NAGATO_K2_ID, 99);
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![nagato_k2, bb];
        let resolved = check_nagato_broadside(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.attack_type, SpecialAttackType::NagatoClassBroadside);
        assert_eq!(r.participants.len(), 2);
        // Flagship: 2 hits at 1.4x, companion: 1 hit at 1.1x
        assert!((r.participants[0].multiplier - 1.4).abs() < f64::EPSILON);
        assert!((r.participants[1].multiplier - 1.1).abs() < f64::EPSILON);
    }

    #[test]
    fn nagato_mutsu_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nagato_k2 = sample_bb_at(&codex, NAGATO_K2_ID, 99);
        let mutsu_k2 = sample_bb_at(&codex, MUTSU_K2_ID, 99);
        let fleet = vec![nagato_k2, mutsu_k2];
        let resolved = check_nagato_broadside(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.attack_type, SpecialAttackType::NagatoMutsuBroadside);
        assert!((r.participants[0].multiplier - 1.68).abs() < f64::EPSILON);
        assert!((r.participants[1].multiplier - 1.68).abs() < f64::EPSILON);
    }

    #[test]
    fn nagato_broadside_requires_bb_companion() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nagato_k2 = sample_bb_at(&codex, NAGATO_K2_ID, 99);
        let dd = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 99);
        let fleet = vec![nagato_k2, dd];
        let resolved = check_nagato_broadside(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_none(), "DD companion should fail");
    }

    #[test]
    fn colorado_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let colorado = sample_bb_at(&codex, 601, 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![colorado, bb1, bb2];
        let resolved = check_colorado_broadside(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.attack_type, SpecialAttackType::ColoradoBroadside);
        assert_eq!(r.participants.len(), 3);
    }

    #[test]
    fn colorado_big_seven_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // The second ship's and the third's, with `mst_id` in both places.
        let multipliers = |mst_id| {
            let fleet = vec![
                sample_bb_at(&codex, 601, 99),
                sample_bb_at(&codex, mst_id, 99),
                sample_bb_at(&codex, mst_id, 99),
            ];
            let resolved = check_colorado_broadside(&codex, &fleet, FORMATION_ECHELON).unwrap();
            (resolved.participants[1].multiplier, resolved.participants[2].multiplier)
        };
        // 長門改/改二, 陸奥改/改二, Nelson改, Colorado and Maryland: 1.495 and 1.521.
        for mst_id in [275, 541, 276, 573, 576, 601, 1496, 913, 918] {
            let (second, third) = multipliers(mst_id);
            assert!((second - 1.495).abs() < 1e-9 && (third - 1.521).abs() < 1e-9, "{mst_id}");
        }
        // Not 長門, 陸奥 or Nelson before their remodel, nor Rodney.
        for mst_id in [80, 81, 571, 572, 577] {
            assert_eq!(multipliers(mst_id), (1.3, 1.3), "{mst_id}");
        }
    }

    #[test]
    fn richelieu_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let richelieu = sample_bb_at(&codex, 392, 99); // Richelieu Kai
        let jb = sample_bb_at(&codex, JEAN_BART_KAI_ID, 99);
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![richelieu, jb, bb];
        let resolved = check_richelieu_attack(&codex, &fleet, FORMATION_DOUBLE_LINE);
        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.attack_type, SpecialAttackType::RichelieuAttack);
        assert!((r.participants[0].multiplier - 1.3).abs() < f64::EPSILON);
        assert!((r.participants[1].multiplier - 1.3).abs() < f64::EPSILON);
        assert!((r.participants[2].multiplier - 1.24).abs() < f64::EPSILON);
    }

    #[test]
    fn richelieu_requires_jean_bart() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let richelieu = sample_bb_at(&codex, 392, 99);
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![richelieu, bb, bb2];
        let resolved = check_richelieu_attack(&codex, &fleet, FORMATION_DOUBLE_LINE);
        assert!(resolved.is_none(), "no Jean Bart Kai should fail");
    }

    #[test]
    fn qe_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let warspite = sample_bb_at(&codex, WARSPIRE_KAI_ID, 99);
        let valiant = sample_bb_at(&codex, VALIANT_KAI_ID, 99);
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![warspite, valiant, bb];
        // Make fleet slow
        let mut fleet = fleet;
        fleet[0].ship.api_soku = 5;
        fleet[1].ship.api_soku = 5;
        fleet[2].ship.api_soku = 5;
        let resolved = check_qe_attack(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some());
        let r = resolved.unwrap();
        assert_eq!(r.attack_type, SpecialAttackType::QueenElizabethAttack);
        assert!(r.participants.iter().all(|p| (p.multiplier - 1.24).abs() < f64::EPSILON));
    }

    #[test]
    fn qe_fast_fleet_rejected() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let warspite = sample_bb_at(&codex, WARSPIRE_KAI_ID, 99);
        let valiant = sample_bb_at(&codex, VALIANT_KAI_ID, 99);
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let mut fleet = vec![warspite, valiant, bb];
        fleet[0].ship.api_soku = 10; // fast
        fleet[1].ship.api_soku = 10;
        fleet[2].ship.api_soku = 10;
        let resolved = check_qe_attack(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_none(), "fast fleet should be rejected");
    }

    #[test]
    fn rodney_flagship_also_triggers_nelson_touch() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let rodney = sample_bb_at(&codex, 577, 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![rodney, bb1, bb2];
        let resolved = check_nelson_touch(&codex, &fleet, FORMATION_DOUBLE_LINE);
        assert!(resolved.is_some(), "Rodney as flagship should also trigger");
    }

    #[test]
    fn maryland_flagship_triggers_colorado() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let maryland = sample_bb_at(&codex, 913, 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![maryland, bb1, bb2];
        let resolved = check_colorado_broadside(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some(), "Maryland as flagship should trigger Colorado type");
    }

    #[test]
    fn valiant_flagship_triggers_qe() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let valiant = sample_bb_at(&codex, VALIANT_KAI_ID, 99);
        let warspite = sample_bb_at(&codex, WARSPIRE_KAI_ID, 99);
        let bb = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let mut fleet = vec![valiant, warspite, bb];
        fleet[0].ship.api_soku = 5;
        fleet[1].ship.api_soku = 5;
        fleet[2].ship.api_soku = 5;
        let resolved = check_qe_attack(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some(), "Valiant flagship + Warspite companion should trigger QE");
    }

    #[test]
    fn nagato_mutsu_flagship_produces_two_hits() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let nagato_k2 = sample_bb_at(&codex, NAGATO_K2_ID, 99);
        let mutsu_k2 = sample_bb_at(&codex, MUTSU_K2_ID, 99);
        let mut attackers = vec![nagato_k2, mutsu_k2];

        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut defenders = vec![sample_ship(&codex, dd_mst, 50).into()];

        let resolved = try_special_attack(
            &codex,
            &mut crate::random::SeededRng::new(2),
            &attackers,
            FORMATION_ECHELON,
        )
        .expect("NagatoMutsu should trigger");

        let result = execute_special_attack(
            &codex,
            &mut crate::random::SeededRng::new(2),
            &mut attackers,
            &mut defenders,
            resolved,
            &crate::types::ShellingParams {
                formation_id: FORMATION_ECHELON,
                defender_formation_id: 0, // unused here: no かばう in this test
                engagement: crate::types::EngagementType::SameCourse,
                air_state: None,
                phase: crate::types::BattlePhase::DayShelling,
                attacker_is_enemy: false,
            },
        );

        // Flagship (index 0) entry should have 2 damage values (2 hits)
        let flagship_idx = result.api_at_list.iter().position(|&i| i == 0).unwrap();
        let flagship_damage = &result.api_damage[flagship_idx];
        assert_eq!(
            flagship_damage.len(),
            2,
            "NagatoMutsu flagship should have 2 hits, got {}",
            flagship_damage.len()
        );
        assert_eq!(result.api_at_type[flagship_idx], 102);
    }

    /// The special attack takes the flagship's turn and nobody else's, and a
    /// fleet that has made it does not make it again in the second round.
    #[test]
    fn a_special_attack_is_made_once_and_costs_only_the_flagship_her_turn() {
        use crate::simulation::shelling::{ShellingRound, simulate_shelling_round};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let special = |round: &BattleHougeki| round.api_at_type.iter().any(|&t| t >= 100);

        let mut made = 0;
        for seed in 0..40 {
            let mut friendly =
                vec![sample_bb_at(&codex, NAGATO_K2_ID, 99), sample_bb_at(&codex, MUTSU_K2_ID, 99)];
            let mut enemy =
                vec![sample_bb_at(&codex, bb_mst, 99), sample_bb_at(&codex, bb_mst, 99)];
            for ship in &mut enemy {
                ship.ship.api_soukou[0] = 1000;
            }
            let mut rng = crate::random::SeededRng::new(seed);
            let mut used = [false; 2];
            let mut round = |by_range| {
                let round = ShellingRound {
                    friendly_deck: 0..2,
                    enemy_deck: 0..2,
                    by_range,
                    friendly_formation: FORMATION_ECHELON,
                    enemy_formation: 1,
                    engagement: crate::types::EngagementType::SameCourse,
                    air_state: None,
                };
                simulate_shelling_round(
                    &codex,
                    &mut rng,
                    &mut friendly,
                    &mut enemy,
                    &round,
                    &mut used,
                )
                .unwrap()
            };
            let first = round(true);
            let second = round(false);
            if !special(&first) {
                continue;
            }
            made += 1;
            let companion_attacks = first
                .api_at_eflag
                .iter()
                .zip(&first.api_at_list)
                .filter(|(eflag, attacker)| **eflag == 0 && **attacker == 1)
                .count();
            assert_eq!(companion_attacks, 2, "seed {seed}: once in the special attack, once alone");
            assert!(!special(&second), "seed {seed}: made again in the second round");
        }
        assert!(made > 0, "no seed made the special attack, so nothing was tested");
    }

    /// `kcsim.js` `canSpecialAttackUnique`: 4 or 12 for 101, 102, 103 and 106, 2 or 12 for
    /// 100 and 105. 3 is 輪形陣.
    #[test]
    fn the_formations_are_the_echelon_the_double_line_and_the_second_cruising_formation() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let fleet = |ids: [i64; 5]| ids.map(|id| sample_bb_at(&codex, id, 99)).to_vec();
        let nagato = fleet([NAGATO_K2_ID, MUTSU_K2_ID, bb, bb, bb]);
        let colorado = fleet([601, bb, bb, bb, bb]);
        let mut warspite = fleet([WARSPIRE_KAI_ID, VALIANT_KAI_ID, bb, bb, bb]);
        for ship in &mut warspite {
            ship.ship.api_soku = 5;
        }
        let nelson = fleet([571, bb, bb, bb, bb]);
        let richelieu = fleet([392, JEAN_BART_KAI_ID, bb, bb, bb]);
        for formation in [1, 2, 3, 4, 5, 6, 11, 12, 13, 14] {
            let echelon = [4, 12].contains(&formation);
            let double_line = [2, 12].contains(&formation);
            assert_eq!(check_nagato_broadside(&codex, &nagato, formation).is_some(), echelon);
            assert_eq!(check_colorado_broadside(&codex, &colorado, formation).is_some(), echelon);
            assert_eq!(check_qe_attack(&codex, &warspite, formation).is_some(), echelon);
            assert_eq!(check_nelson_touch(&codex, &nelson, formation).is_some(), double_line);
            assert_eq!(
                check_richelieu_attack(&codex, &richelieu, formation).is_some(),
                double_line
            );
        }
    }

    /// `canSpecialAttackUnique`: `ship.isescort` makes none of them. 第二警戒航行序列 is the
    /// formation of both decks, so the formation does not keep 第2艦隊's flagship from it.
    #[test]
    fn the_flagship_of_the_escort_fleet_makes_no_special_attack() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        use crate::simulation::shelling::{ShellingRound, simulate_shelling_round};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        for seed in 0..40 {
            let deck = |ids: [i64; 2], role| {
                ids.map(|id| {
                    sample_bb_at(&codex, id, 99)
                        .in_combined_fleet(CombinedType::SurfaceTaskForce, role)
                })
            };
            let mut friendly = [
                deck([bb_mst, bb_mst], CombinedFleetRole::Main),
                deck([NAGATO_K2_ID, MUTSU_K2_ID], CombinedFleetRole::Escort),
            ]
            .concat();
            let mut enemy =
                vec![sample_bb_at(&codex, bb_mst, 99), sample_bb_at(&codex, bb_mst, 99)];
            let round = ShellingRound {
                friendly_deck: 2..4,
                enemy_deck: 0..2,
                by_range: true,
                friendly_formation: 12,
                enemy_formation: 1,
                engagement: crate::types::EngagementType::SameCourse,
                air_state: None,
            };
            let hougeki = simulate_shelling_round(
                &codex,
                &mut crate::random::SeededRng::new(seed),
                &mut friendly,
                &mut enemy,
                &round,
                &mut [false; 2],
            )
            .unwrap();
            assert!(hougeki.api_at_type.iter().all(|&t| t < 100), "seed {seed}");
        }
    }

    #[test]
    fn the_trigger_rates_read_the_current_luck_not_what_it_can_be_modernised_to() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let fleet = |luck: [i64; 2]| {
            let mut ship = sample_bb_at(&codex, bb, 99);
            ship.ship.api_lucky = luck;
            vec![ship; 6]
        };
        let rates = |luck| {
            let fleet = fleet(luck);
            [nelson_touch_rate(&fleet[0], &fleet), nagato_broadside_rate(&fleet[0], &fleet[1])]
        };
        assert_eq!(rates([10, 100]), rates([10, 10]));
        for (current, cap) in rates([10, 10]).into_iter().zip(rates([100, 100])) {
            assert!(current < cap);
        }
    }

    #[test]
    fn the_trigger_rates_are_the_percentages_the_wiki_gives() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |mst_id, luck| {
            let mut ship = sample_bb_at(&codex, mst_id, 99);
            ship.ship.api_lucky = [luck, luck];
            ship
        };
        // 長門 Lv99 with 40 luck and 陸奥 Lv99 with 16: 62.2% on the page that works it out.
        let rate = nagato_broadside_rate(&ship(NAGATO_K2_ID, 40), &ship(MUTSU_K2_ID, 16));
        assert!((rate - 0.622).abs() < 0.002, "{rate}");
        // Warspite改 with 70 and Valiant改 with 40, neither modernised: wikiwiki's 67.5%.
        let rate = nagato_broadside_rate(&ship(WARSPIRE_KAI_ID, 70), &ship(VALIANT_KAI_ID, 40));
        assert!((rate - 0.675).abs() < 0.001, "{rate}");
        // Nelson at Lv99 with her own luck beside Lv99 ships: about six in ten.
        let fleet = vec![ship(NELSON_IDS[0], 24); 6];
        let rate = nelson_touch_rate(&fleet[0], &fleet);
        assert!((0.60..0.65).contains(&rate), "{rate}");
    }

    #[test]
    fn an_sg_radar_late_model_on_one_of_the_three_adds_five_to_the_colorado_rate() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fleet = |radars: &[usize]| {
            let mut fleet = vec![sample_bb_at(&codex, COLORADO_IDS[1], 99); 6];
            for &at in radars {
                fleet[at].slot_items = vec![slotitem_with_mst_id(SG_RADAR_LATE_ID)];
            }
            fleet
        };
        let rate = |kind, radars: &[usize]| trigger_rate(kind, &fleet(radars));
        let colorado = SpecialAttackType::ColoradoBroadside;
        let plain = rate(colorado, &[]);
        assert!((rate(colorado, &[0]) - plain - 0.05).abs() < 1e-9);
        assert!((rate(colorado, &[2]) - plain - 0.05).abs() < 1e-9);
        assert!((rate(colorado, &[0, 1, 2]) - plain - 0.05).abs() < 1e-9, "one is counted");
        assert_eq!(rate(colorado, &[3]), plain, "the fourth ship is not one of the three");
        // The other attacks have no such piece.
        let nagato = SpecialAttackType::NagatoClassBroadside;
        assert_eq!(rate(nagato, &[0, 1]), rate(nagato, &[]));
    }

    #[test]
    fn an_sg_radar_late_model_multiplies_the_colorado_attack_once_more() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ap_shell = first_slotitem_mst_by_type(
            &codex,
            emukc_model::kc2::KcSlotItemType3::ArmorPiercingShell,
        );
        let with = |mst_id, equipment: &[i64]| {
            let mut ship = sample_bb_at(&codex, mst_id, 99);
            ship.slot_items = equipment.iter().copied().map(slotitem_with_mst_id).collect();
            ship
        };
        let other_bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        // What each of the three does, as wikiwiki's table of the attack has it.
        let colorado = vec![
            with(COLORADO_IDS[1], &[SG_RADAR_LATE_ID, ap_shell]),
            with(other_bb, &[SG_RADAR_LATE_ID]),
            with(other_bb, &[]),
        ];
        let resolved = check_colorado_broadside(&codex, &colorado, FORMATION_ECHELON).unwrap();
        let totals: Vec<f64> = resolved
            .participants
            .iter()
            .zip(&resolved.equip_bonuses)
            .map(|(participant, bonus)| participant.multiplier * bonus)
            .collect();
        for (total, table) in totals.iter().zip([2.67, 1.71, 1.3]) {
            assert!((total - table).abs() < 0.01, "{totals:?}");
        }
        // It is a surface radar and no more to the Nagato class.
        let nagato =
            vec![with(NAGATO_K2_ID, &[SG_RADAR_LATE_ID]), with(MUTSU_K2_ID, &[SG_RADAR_LATE_ID])];
        let resolved = check_nagato_broadside(&codex, &nagato, FORMATION_ECHELON).unwrap();
        assert!(resolved.equip_bonuses.iter().all(|bonus| (bonus - 1.15).abs() < 1e-9));
    }
}
