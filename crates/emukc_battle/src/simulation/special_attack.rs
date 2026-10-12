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
use crate::targeting::{has_slotitem_id, select_surface_target, ship_type, target_class};
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

/// One ship of a special attack: her place in the fleet and what her hits are multiplied by.
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

/// The flagship makes it lightly damaged or better: more than half her hull
/// (`canSpecialAttackUnique`: `HP/maxHP <= .5` makes none).
fn is_flagship_healthy(ship: &BattleRuntimeShip) -> bool {
    ship.hp() * 2 > ship.ship.api_maxhp
}

/// A ship named beside her joins moderately damaged or better: more than a quarter.
fn is_companion_healthy(ship: &BattleRuntimeShip) -> bool {
    ship.hp() * 4 > ship.ship.api_maxhp
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

/// The flagship and the second ship, each with the AP shell and radar she carries: the
/// attack of two ships, of which the flagship hits twice.
fn pair(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    attack_type: SpecialAttackType,
    (flagship, second): (f64, f64),
) -> ResolvedSpecialAttack {
    ResolvedSpecialAttack {
        attack_type,
        participants: vec![
            Participant {
                fleet_index: 0,
                multiplier: flagship,
            },
            Participant {
                fleet_index: 1,
                multiplier: second,
            },
        ],
        equip_bonuses: fleet[..2].iter().map(|ship| equip_bonus(codex, ship, false)).collect(),
    }
}

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
    // The third and the fifth ship, neither a carrier nor a submarine, whatever state they
    // are in: 「参加艦が大破していても問題なく発動する」.
    let (third, fifth) = (fleet.get(2)?, fleet.get(4)?);
    if is_submarine_or_carrier(codex, third) || is_submarine_or_carrier(codex, fifth) {
        return None;
    }

    // `getSpecialAttackMod`: with another of the Nelson class in the third or fifth place,
    // 1.15 to the flagship and 1.2 to her.
    let nelson_class =
        |ship: &BattleRuntimeShip| is_ship_id(ship, NELSON_IDS) || is_ship_id(ship, RODNEY_IDS);
    let sisters = nelson_class(third) || nelson_class(fifth);
    let participants = [0, 2, 4]
        .into_iter()
        .map(|fleet_index| Participant {
            fleet_index,
            multiplier: 2.0
                * match (sisters && nelson_class(&fleet[fleet_index]), fleet_index) {
                    (false, _) => 1.0,
                    (true, 0) => 1.15,
                    (true, _) => 1.2,
                },
        })
        .collect::<Vec<_>>();

    // Nelson Touch has NO equipment bonuses
    let equip_bonuses: Vec<f64> = participants.iter().map(|_| 1.0).collect();

    Some(ResolvedSpecialAttack {
        attack_type: SpecialAttackType::NelsonTouch,
        participants,
        equip_bonuses,
    })
}

/// Check the Nagato class's broadside: 長門改二 (101) or 陸奥改二 (102) as flagship, a
/// battleship second, 梯形陣.
fn check_nagato_broadside(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_ECHELON | FORMATION_CRUISING_2) {
        return None;
    }
    let (flagship, second) = (fleet.first()?, fleet.get(1)?);
    let attack_type = match flagship.ship.api_ship_id {
        NAGATO_K2_ID => SpecialAttackType::NagatoClassBroadside,
        MUTSU_K2_ID => SpecialAttackType::NagatoMutsuBroadside,
        _ => return None,
    };
    if !is_flagship_healthy(flagship)
        || !is_bb_class(ship_type(codex, second))
        || !is_companion_healthy(second)
    {
        return None;
    }
    // `getSpecialAttackMod`: 1.4 and 1.2, and more by who the second ship is.
    let with = match (attack_type, second.ship.api_ship_id) {
        (SpecialAttackType::NagatoClassBroadside, MUTSU_K2_ID)
        | (SpecialAttackType::NagatoMutsuBroadside, NAGATO_K2_ID) => (1.2, 1.4),
        // 陸奥改 beside 長門改二, 長門改 beside 陸奥改二
        (SpecialAttackType::NagatoClassBroadside, 276)
        | (SpecialAttackType::NagatoMutsuBroadside, 275) => (1.15, 1.35),
        // Nelson改
        (SpecialAttackType::NagatoClassBroadside, 576) => (1.1, 1.25),
        _ => (1.0, 1.0),
    };
    Some(pair(codex, fleet, attack_type, (1.4 * with.0, 1.2 * with.1)))
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
    if !is_bb_class(ship_type(codex, second)) || !is_companion_healthy(second) {
        return None;
    }
    if !is_bb_class(ship_type(codex, third)) || !is_companion_healthy(third) {
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

/// Check the Richelieu class's attack (105): Richelieu改, Richelieu Deux or Jean Bart改 as
/// flagship and one of them second, 複縦陣.
fn check_richelieu_attack(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_DOUBLE_LINE | FORMATION_CRUISING_2) {
        return None;
    }
    let (flagship, second) = (fleet.first()?, fleet.get(1)?);
    let of_the_class = |ship: &BattleRuntimeShip| {
        is_ship_id(ship, RICHELIEU_IDS) || is_ship_id(ship, &[JEAN_BART_KAI_ID])
    };
    if !of_the_class(flagship)
        || !of_the_class(second)
        || !is_flagship_healthy(flagship)
        || !is_companion_healthy(second)
    {
        return None;
    }
    // `getSpecialAttackMod`: 1.24, and 1.3 for Richelieu as flagship.
    let flagship_mult = if is_ship_id(flagship, RICHELIEU_IDS) {
        1.3
    } else {
        1.24
    };
    Some(pair(codex, fleet, SpecialAttackType::RichelieuAttack, (flagship_mult, 1.24)))
}

/// Check the Queen Elizabeth class's attack (106): Warspite改 or Valiant改 as flagship and
/// one of them second, 梯形陣.
fn check_qe_attack(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<ResolvedSpecialAttack> {
    if !matches!(formation_id, FORMATION_ECHELON | FORMATION_CRUISING_2) {
        return None;
    }
    let (flagship, second) = (fleet.first()?, fleet.get(1)?);
    let of_the_class =
        |ship: &BattleRuntimeShip| is_ship_id(ship, &[WARSPIRE_KAI_ID, VALIANT_KAI_ID]);
    if !of_the_class(flagship)
        || !of_the_class(second)
        || !is_flagship_healthy(flagship)
        || !is_companion_healthy(second)
    {
        return None;
    }
    // `getSpecialAttackMod`: 1.24, and 1.2 for Valiant as flagship.
    let flagship_mult = if flagship.ship.api_ship_id == VALIANT_KAI_ID {
        1.2
    } else {
        1.24
    };
    Some(pair(codex, fleet, SpecialAttackType::QueenElizabethAttack, (flagship_mult, 1.24)))
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

/// Check if a special attack is eligible and triggered for the attacking fleet, in a turn
/// of its flagship's in which she `can_attack`: by day or at night.
/// Returns the resolved attack if triggered, None otherwise.
pub(crate) fn try_special_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &[BattleRuntimeShip],
    formation_id: i64,
    can_attack: bool,
) -> Option<ResolvedSpecialAttack> {
    let flagship = attackers.first()?;
    // None is made in an exercise, none by 第2艦隊's flagship, and none with fewer than six
    // ships afloat that are not submarines.
    let surface = attackers
        .iter()
        .filter(|ship| ship.is_alive() && !target_class(codex, ship).is_submarine())
        .count();
    if !can_attack || !flagship.is_sortie || flagship.is_escort_deck() || surface < 6 {
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

/// One hit of a special attack: who makes it, by her place in the fleet, what her attack is
/// multiplied by, and what her accuracy is.
pub(crate) struct SpecialHit {
    pub(crate) fleet_index: usize,
    pub(crate) multiplier: f64,
    pub(crate) accuracy: f64,
}

impl ResolvedSpecialAttack {
    /// `api_at_type` by day, `api_sp_list` at night.
    pub(crate) fn api_value(&self) -> i64 {
        self.attack_type.api_value()
    }

    /// The hits in order (`getSpecialAttackShips`): the flagship hits twice where two ships
    /// make the attack.
    pub(crate) fn hits(
        &self,
        codex: &Codex,
        fleet: &[BattleRuntimeShip],
        engagement: EngagementType,
    ) -> Vec<SpecialHit> {
        let mut hits = Vec::new();
        for (i, participant) in self.participants.iter().enumerate() {
            let ship = &fleet[participant.fleet_index];
            // `getSpecialAttackMod`: 1.05 to the accuracy of a Nelson Touch, 1.4 to the
            // others', and 1.15 more for an AP shell and for a surface radar.
            let accuracy = if self.attack_type == SpecialAttackType::NelsonTouch {
                1.05
            } else {
                let piece = |carried: bool| {
                    if carried {
                        1.15
                    } else {
                        1.0
                    }
                };
                1.4 * piece(has_ap_shell(codex, ship)) * piece(has_surface_radar(codex, ship))
            };
            let multiplier = participant.multiplier
                * engagement_multiplier(self.attack_type, engagement)
                * self.equip_bonuses.get(i).copied().unwrap_or(1.0);
            let count = if self.participants.len() == 2 && i == 0 {
                2
            } else {
                1
            };
            hits.extend((0..count).map(|_| SpecialHit {
                fleet_index: participant.fleet_index,
                multiplier,
                accuracy,
            }));
        }
        hits
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// `getSpecialAttackMod`: a Nelson Touch at T disadvantage is worth 1.25 again, 2.5 in all.
fn engagement_multiplier(attack_type: SpecialAttackType, engagement: EngagementType) -> f64 {
    if attack_type == SpecialAttackType::NelsonTouch && engagement == EngagementType::TDisadvantage
    {
        1.25
    } else {
        1.0
    }
}

/// Execute a resolved special attack, producing hougeki entries.
pub(crate) fn execute_special_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    resolved: ResolvedSpecialAttack,
    params: &ShellingParams,
) -> BattleHougeki {
    let is_enemy = params.attacker_is_enemy;

    let mut hougeki = BattleHougeki::default();
    // The client reads every hit of the attack from one row (`PhaseHougeki._nelson_touch`
    // and the others: `d_indexes[0..3]`, `getDamage(0..2)`), as the source's
    // `apiAdjustHougekiSpecial` folds them into one.
    let (mut targets, mut damages, mut hits) = (Vec::new(), Vec::new(), Vec::new());

    for hit in resolved.hits(codex, attackers, params.engagement) {
        let attacker = &mut attackers[hit.fleet_index];
        if !attacker.is_alive() {
            continue;
        }

        let Some(mut target_idx) = select_surface_target(codex, rng, defenders) else {
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

        let outcome = roll_attack(
            codex,
            rng,
            attacker,
            &defenders[target_idx],
            Aim::new(AttackKind::Shelling, params.formation_id, params.defender_formation_id)
                .with_modifier(hit.accuracy),
        );
        hits.push(outcome.cl());
        // By day the multiplier is applied after the cap.
        let raw = calculate_shelling_damage(
            codex,
            rng,
            attacker,
            &defenders[target_idx],
            params.formation_id,
            params.engagement,
            Some(hit.multiplier),
            outcome,
        );
        let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
        if !is_enemy {
            attacker.damage_dealt += dealt;
        }
        let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
        targets.push(target_idx as i64);
        damages.push(if shield {
            DamageCell::Shielded(display)
        } else {
            DamageCell::Plain(display)
        });
    }

    if !targets.is_empty() {
        hougeki.record_day_attack(
            DayAttackKind::Special(codex, &attackers[0], resolved.api_value()),
            is_enemy,
            0,
            targets,
            damages,
            hits,
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

    fn fleet_of<const N: usize>(codex: &Codex, ids: [i64; N]) -> Vec<BattleRuntimeShip> {
        ids.map(|id| sample_bb_at(codex, id, 99)).to_vec()
    }

    /// What each attack is, who makes it and at what multiplier: `canSpecialAttackUnique`,
    /// `getSpecialAttackShips` and `getSpecialAttackMod`, and wikiwiki's tables.
    #[test]
    fn who_makes_each_attack_and_what_her_hits_are_multiplied_by() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let cv = first_ship_mst_by_type(&codex, KcShipType::CV);
        let made = |resolved: Option<ResolvedSpecialAttack>| {
            resolved.map(|r| {
                let ships = r.participants.iter();
                let of = ships.map(|p| (p.fleet_index, (p.multiplier * 1000.0).round() as i64));
                (r.attack_type.api_value(), of.collect::<Vec<_>>())
            })
        };
        let pair = |kind, flagship, second| Some((kind, vec![(0, flagship), (1, second)]));

        // 長門改二 or 陸奥改二 and a battleship: 1.4 and 1.2, and more by who she is.
        let nagato = |ids| made(check_nagato_broadside(&codex, &fleet_of(&codex, ids), 4));
        assert_eq!(nagato([NAGATO_K2_ID, bb]), pair(101, 1400, 1200));
        assert_eq!(nagato([NAGATO_K2_ID, MUTSU_K2_ID]), pair(101, 1680, 1680));
        assert_eq!(nagato([NAGATO_K2_ID, 276]), pair(101, 1610, 1620));
        assert_eq!(nagato([NAGATO_K2_ID, 576]), pair(101, 1540, 1500));
        assert_eq!(nagato([MUTSU_K2_ID, bb]), pair(102, 1400, 1200));
        assert_eq!(nagato([MUTSU_K2_ID, NAGATO_K2_ID]), pair(102, 1680, 1680));
        assert_eq!(nagato([MUTSU_K2_ID, 275]), pair(102, 1610, 1620));
        assert_eq!(nagato([NAGATO_K2_ID, dd]), None);
        assert_eq!(nagato([275, MUTSU_K2_ID]), None);

        // Two of the Richelieu class, either way round.
        let richelieu = |ids| made(check_richelieu_attack(&codex, &fleet_of(&codex, ids), 2));
        assert_eq!(richelieu([392, JEAN_BART_KAI_ID]), pair(105, 1300, 1240));
        assert_eq!(richelieu([969, JEAN_BART_KAI_ID]), pair(105, 1300, 1240));
        assert_eq!(richelieu([JEAN_BART_KAI_ID, 392]), pair(105, 1240, 1240));
        assert_eq!(richelieu([392, bb]), None);

        // Warspite改 and Valiant改, either way round, whatever the fleet's speed.
        let qe = |ids| made(check_qe_attack(&codex, &fleet_of(&codex, ids), 4));
        assert_eq!(qe([WARSPIRE_KAI_ID, VALIANT_KAI_ID]), pair(106, 1240, 1240));
        assert_eq!(qe([VALIANT_KAI_ID, WARSPIRE_KAI_ID]), pair(106, 1200, 1240));
        assert_eq!(qe([WARSPIRE_KAI_ID, bb]), None);

        // Nelson or Rodney with the third and the fifth ship, neither a carrier.
        let nelson = |ids: [i64; 5]| made(check_nelson_touch(&codex, &fleet_of(&codex, ids), 2));
        let three = |a, b, c| Some((100, vec![(0, a), (2, b), (4, c)]));
        assert_eq!(nelson([571, dd, bb, dd, dd]), three(2000, 2000, 2000));
        assert_eq!(nelson([572, dd, bb, dd, dd]), three(2000, 2000, 2000));
        assert_eq!(nelson([571, dd, 572, dd, dd]), three(2300, 2400, 2000));
        assert_eq!(nelson([576, dd, bb, dd, 577]), three(2300, 2000, 2400));
        assert_eq!(nelson([571, dd, cv, dd, dd]), None);
        assert_eq!(made(check_nelson_touch(&codex, &fleet_of(&codex, [571, dd, bb, dd]), 2)), None);
        // It takes no AP shell and no radar; the others do.
        let touch = check_nelson_touch(&codex, &fleet_of(&codex, [571, dd, bb, dd, dd]), 2);
        assert_eq!(touch.unwrap().equip_bonuses, [1.0, 1.0, 1.0]);
    }

    /// The flagship lightly damaged or better and the ships named beside her moderately
    /// damaged or better; the third and fifth ship of a Nelson Touch in any state.
    #[test]
    fn how_damaged_the_ships_of_an_attack_may_be() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        // The fleet with one ship left `hp` of 100.
        let with = |ids: &[i64], at: usize, hp: i64| {
            let mut fleet: Vec<_> = ids.iter().map(|&id| sample_bb_at(&codex, id, 99)).collect();
            fleet[at].ship.api_maxhp = 100;
            fleet[at].ship.api_nowhp = hp;
            fleet[at].current_hp = hp;
            fleet
        };
        let nagato = |at, hp| {
            check_nagato_broadside(&codex, &with(&[NAGATO_K2_ID, MUTSU_K2_ID], at, hp), 4).is_some()
        };
        assert_eq!([nagato(0, 51), nagato(0, 50)], [true, false]);
        assert_eq!([nagato(1, 26), nagato(1, 25)], [true, false]);
        let colorado =
            |at, hp| check_colorado_broadside(&codex, &with(&[601, bb, bb], at, hp), 4).is_some();
        assert_eq!([colorado(0, 51), colorado(0, 50)], [true, false]);
        assert_eq!([colorado(1, 26), colorado(1, 25)], [true, false]);
        assert_eq!([colorado(2, 26), colorado(2, 25)], [true, false]);
        let richelieu = |at, hp| {
            check_richelieu_attack(&codex, &with(&[392, JEAN_BART_KAI_ID], at, hp), 2).is_some()
        };
        assert_eq!([richelieu(0, 51), richelieu(0, 50)], [true, false]);
        assert_eq!([richelieu(1, 26), richelieu(1, 25)], [true, false]);
        let qe = |at, hp| {
            check_qe_attack(&codex, &with(&[WARSPIRE_KAI_ID, VALIANT_KAI_ID], at, hp), 4).is_some()
        };
        assert_eq!([qe(0, 51), qe(0, 50)], [true, false]);
        assert_eq!([qe(1, 26), qe(1, 25)], [true, false]);
        let nelson =
            |at, hp| check_nelson_touch(&codex, &with(&[571, dd, bb, dd, dd], at, hp), 2).is_some();
        assert_eq!([nelson(0, 51), nelson(0, 50)], [true, false]);
        assert_eq!([nelson(2, 1), nelson(4, 1)], [true, true]);
    }

    /// 「水上艦6隻」: six ships afloat that are not submarines.
    #[test]
    fn a_fleet_of_fewer_than_six_surface_ships_makes_no_special_attack() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let comes_up = |fleet: &[BattleRuntimeShip]| {
            (0..40).any(|seed| {
                let rng = &mut crate::random::SeededRng::new(seed);
                try_special_attack(&codex, rng, fleet, 4, true).is_some()
            })
        };
        let six = fleet_of(&codex, [NAGATO_K2_ID, MUTSU_K2_ID, bb, bb, bb, bb]);
        assert!(comes_up(&six));
        assert!(!comes_up(&six[..5]));
        assert!(!comes_up(&fleet_of(&codex, [NAGATO_K2_ID, MUTSU_K2_ID, bb, bb, bb, ss])));
        assert!(comes_up(&fleet_of(&codex, [NAGATO_K2_ID, MUTSU_K2_ID, bb, bb, bb, bb, ss])));
        let mut sunk = six.clone();
        sunk[5].current_hp = 0;
        assert!(!comes_up(&sunk));
    }

    /// One row: the flagship's two hits and her partner's for a pair, a hit each for three
    /// ships, and never on a submarine while a surface ship is afloat.
    #[test]
    fn the_hits_of_an_attack_are_one_row_on_surface_ships() {
        use crate::types::{BattlePhase, ShellingParams};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let row = |mut fleet: Vec<BattleRuntimeShip>, formation, seed| {
            let resolved = [
                check_richelieu_attack(&codex, &fleet, formation),
                check_nelson_touch(&codex, &fleet, formation),
            ]
            .into_iter()
            .flatten()
            .next()
            .unwrap();
            // A submarine first, then a ship nobody sinks.
            let mut target = sample_bb_at(&codex, bb, 99);
            target.ship.api_soukou[0] = 9999;
            let mut enemy = vec![sample_bb_at(&codex, ss, 99), target];
            execute_special_attack(
                &codex,
                &mut crate::random::SeededRng::new(seed),
                &mut fleet,
                &mut enemy,
                resolved,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: formation,
                    defender_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    phase: BattlePhase::DayShelling,
                    air_state: None,
                },
            )
        };
        for seed in 0..20 {
            let pair = row(fleet_of(&codex, [392, JEAN_BART_KAI_ID]), 2, seed);
            assert_eq!((&pair.api_at_list, &pair.api_at_type), (&vec![0], &vec![105]));
            assert_eq!(pair.api_df_list, [[1, 1, 1]]);
            assert_eq!((pair.api_damage[0].len(), pair.api_cl_list[0].len()), (3, 3));
            // Destroyers would rather fire at the submarine.
            let touch = row(fleet_of(&codex, [571, dd, dd, dd, dd]), 2, seed);
            assert_eq!((&touch.api_at_list, &touch.api_at_type), (&vec![0], &vec![100]));
            assert_eq!(touch.api_df_list, [[1, 1, 1]]);
        }
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
    fn maryland_flagship_triggers_colorado() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let maryland = sample_bb_at(&codex, 913, 99);
        let bb1 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let bb2 = sample_bb_at(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 99);
        let fleet = vec![maryland, bb1, bb2];
        let resolved = check_colorado_broadside(&codex, &fleet, FORMATION_ECHELON);
        assert!(resolved.is_some(), "Maryland as flagship should trigger Colorado type");
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
                fleet_of(&codex, [NAGATO_K2_ID, MUTSU_K2_ID, bb_mst, bb_mst, bb_mst, bb_mst]);
            let mut enemy =
                vec![sample_bb_at(&codex, bb_mst, 99), sample_bb_at(&codex, bb_mst, 99)];
            for ship in &mut enemy {
                ship.ship.api_soukou[0] = 1000;
            }
            let mut rng = crate::random::SeededRng::new(seed);
            let mut used = [false; 2];
            let mut round = |by_range| {
                let round = ShellingRound {
                    friendly_deck: 0..6,
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
            // One row for the whole attack, by the flagship: her two hits and her partner's.
            let rows = |attacker: i64| {
                (0..first.api_at_list.len())
                    .filter(|&i| (first.api_at_eflag[i], first.api_at_list[i]) == (0, attacker))
                    .map(|i| (first.api_at_type[i], first.api_damage[i].len()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(rows(0), [(101, 3)], "seed {seed}");
            assert_eq!(rows(1).len(), 1, "seed {seed}: her partner keeps her own turn");
            let row = first.api_at_type.iter().position(|&t| t == 101).unwrap();
            assert_eq!(first.api_df_list[row].len(), 3);
            assert_eq!(first.api_cl_list[row].len(), 3);
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
            let deck = |ids: [i64; 6], role| {
                ids.map(|id| {
                    sample_bb_at(&codex, id, 99)
                        .in_combined_fleet(CombinedType::SurfaceTaskForce, role)
                })
            };
            let mut friendly = [
                deck([bb_mst; 6], CombinedFleetRole::Main),
                deck(
                    [NAGATO_K2_ID, MUTSU_K2_ID, bb_mst, bb_mst, bb_mst, bb_mst],
                    CombinedFleetRole::Escort,
                ),
            ]
            .concat();
            let mut enemy =
                vec![sample_bb_at(&codex, bb_mst, 99), sample_bb_at(&codex, bb_mst, 99)];
            let round = ShellingRound {
                friendly_deck: 6..12,
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

    /// wikiwiki: 「発動するのは出撃1回につき1度のみ」「演習では発動しない」. The source keeps
    /// `fleet.didSpecial` set from one battle of a sortie to the next.
    #[test]
    fn a_special_attack_is_made_once_a_sortie_and_never_in_an_exercise() {
        use crate::types::{BattleContext, BattleType, SpecialAttacksMade};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let battles = |is_sortie, made: SpecialAttacksMade| {
            let mut after = Vec::new();
            for seed in 0..40 {
                let mut enemy_ships = vec![sample_ship(&codex, bb_mst, 99); 2];
                for ship in &mut enemy_ships {
                    ship.ship.api_soukou[0] = 1000;
                }
                let context = BattleContext {
                    battle_type: BattleType::Normal,
                    is_sortie,
                    friendly_formation_id: 4,
                    enemy_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    friend_ships: [NAGATO_K2_ID, MUTSU_K2_ID, bb_mst, bb_mst, bb_mst, bb_mst]
                        .map(|id| sample_ship(&codex, id, 99))
                        .to_vec(),
                    enemy_ships,
                    enemy_escort_ships: Vec::new(),
                    combined: None,
                    air_corps: Vec::new(),
                    special_attacks_made: made,
                };
                let mut rng = crate::random::SeededRng::new(seed);
                let packet = crate::execute_day(&codex, context, &mut rng).packet;
                let rounds = [&packet.hougeki1, &packet.hougeki2, &packet.hougeki3];
                let special = rounds
                    .into_iter()
                    .flatten()
                    .flat_map(|round| &round.api_at_type)
                    .any(|&attack_type| attack_type >= 100);
                let mut made = made;
                made.record_day(&packet);
                after.push((special, made));
            }
            after
        };
        let fresh = SpecialAttacksMade::default();
        let spent = SpecialAttacksMade {
            gunnery: true,
            kongou_assaults: 0,
        };
        // A fleet that has not made it makes it in some battles, and that is recorded.
        let first = battles(true, fresh);
        assert!(first.contains(&(true, spent)));
        assert!(first.iter().all(|battle| [(false, fresh), (true, spent)].contains(battle)));
        // One that has does not make it again.
        assert!(battles(true, spent).iter().all(|battle| *battle == (false, spent)));
        // The night assaults are counted apart from it.
        let assaulted = SpecialAttacksMade {
            gunnery: false,
            kongou_assaults: 3,
        };
        assert!(battles(true, assaulted).iter().any(|battle| battle.0));
        // And none is made in an exercise.
        assert!(battles(false, fresh).iter().all(|battle| *battle == (false, fresh)));
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

    /// The source: `mod = 2`, and `mod *= 1.25` at T disadvantage. wikiwiki: 「T不利時に発動
    /// すると、倍率が2.5倍に上昇」. It was the 2.0 of each ship times 2.5.
    #[test]
    fn a_nelson_touch_at_t_disadvantage_is_two_and_a_half_times() {
        use SpecialAttackType::{ColoradoBroadside, NelsonTouch};

        assert_eq!(2.0 * engagement_multiplier(NelsonTouch, EngagementType::TDisadvantage), 2.5);
        assert_eq!(engagement_multiplier(NelsonTouch, EngagementType::TAdvantage), 1.0);
        assert_eq!(engagement_multiplier(NelsonTouch, EngagementType::SameCourse), 1.0);
        assert_eq!(engagement_multiplier(ColoradoBroadside, EngagementType::TDisadvantage), 1.0);
    }
}
