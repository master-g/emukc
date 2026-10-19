//! Aerial combat (kouku) phase simulation.
//!
//! Implements the three-stage aerial combat model:
//! Stage 1: Fighter combat (air superiority), Stage 2: Anti-air fire,
//! Stage 3: Bombing damage (dive bombing + torpedo bombing).

use emukc_model::{
    codex::Codex,
    kc2::{KcApiSlotItem, KcShipType, KcSlotItemType3, start2::ApiMstSlotitem},
};

use super::aaci::roll_air_fire;
use super::air_base::{ENEMY_FLAT_SHOT, fleet_anti_air, weighted_anti_air};
use crate::accuracy::{HitOutcome, PROFICIENCY_EXP, plane_proficiency, roll_strike};
use crate::combined::CombinedFleetRole;
use crate::damage::{
    ammo_modifier, apply_cap, balloon_contact, calculate_defense_power, resolve_damage,
};
use crate::installation::post_cap_multiplier;
use crate::random::BattleRng;
use crate::simulation::installation_types::SUPPLY_DEPOT_IDS;
use crate::targeting::{
    choice_with_protect, installation_type, is_air_combat_type, is_airstrike_attack_type,
    ship_type, target_class,
};
use crate::types::{
    AirState, AirstrikeOutput, BattleAirFire, BattleKouku, BattleKoukuStage1, BattleKoukuStage2,
    BattleKoukuStage3, BattleRuntimeShip, DamageCell, TargetClass,
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

/// Aircraft that can make contact (`canContact` in `kcEQDATA.js`).
fn can_contact(type3: i64) -> bool {
    matches!(
        KcSlotItemType3::n(type3),
        Some(
            KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::CarrierBasedRecon
                | KcSlotItemType3::CarrierBasedRecon2
                | KcSlotItemType3::SeaBasedRecon
                | KcSlotItemType3::LargeFlyingBoat
                | KcSlotItemType3::LandBasedRecon
        )
    )
}

/// Contact (触接) made by one side's scouts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Contact {
    /// What the side's strikes take after the cap.
    pub(crate) modifier: f64,
    /// The aircraft in contact, for `api_touch_plane`.
    pub(crate) plane: i64,
}

/// What contact is tried against under `air_state`: the divisor of the
/// fleet's scouting power for the chance, and of each scout's 索敵 for her own
/// roll. `None` under parity and incapability, where no contact is tried.
fn contact_divisors(air_state: AirState) -> Option<(f64, f64)> {
    match air_state {
        AirState::Supremacy => Some((25.0, 14.0)),
        AirState::Superiority => Some((40.0, 16.0)),
        AirState::Denial => Some((55.0, 18.0)),
        AirState::Parity | AirState::Incapability => None,
    }
}

/// The aircraft of `carriers` in ship and slot order, as the contact roll
/// takes them: equipment id and aircraft.
pub(crate) fn carried_planes(carriers: &[&BattleRuntimeShip]) -> Vec<(i64, i64)> {
    carriers
        .iter()
        .flat_map(|ship| {
            ship.slot_items.iter().map(|item| item.api_slotitem_id).zip(ship.ship.api_onslot)
        })
        .collect()
}

/// The chance `planes` (equipment id and aircraft) make contact (触接) with
/// the other side's fleet, seen from their own `air_state`, or `None` when they
/// do not try: only under supremacy, superiority or denial (`getContact` and
/// its caller). Every scout's slot adds ⌊√aircraft × 索敵⌋; torpedo bombers
/// carry no weight here.
fn contact_chance(codex: &Codex, planes: &[(i64, i64)], air_state: AirState) -> Option<f64> {
    let (divisor, _) = contact_divisors(air_state)?;
    let power: f64 = planes
        .iter()
        .filter_map(|&(mst_id, count)| {
            let mst = codex.find::<ApiMstSlotitem>(&mst_id).ok()?;
            let scout = can_contact(mst.api_type[2])
                && KcSlotItemType3::n(mst.api_type[2])
                    != Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
            scout.then(|| ((count.max(0) as f64).sqrt() * mst.api_saku.max(0) as f64).floor())
        })
        .sum();
    Some((1.0 + power) / divisor)
}

/// Roll contact for `planes` (`getContact`): the multiplier their strike's
/// power takes after the cap, and the aircraft that made it. Once the chance
/// comes up, each scout in order rolls 索敵 / 14, 16 or 18, and one no more
/// accurate than the scout already holding contact does not roll.
pub(crate) fn roll_contact(
    codex: &Codex,
    rng: &mut impl BattleRng,
    planes: &[(i64, i64)],
    air_state: AirState,
) -> Option<Contact> {
    let chance = contact_chance(codex, planes, air_state)?;
    if rng.random_f64_range(0.0, 1.0) >= chance {
        return None;
    }
    let (_, divisor) = contact_divisors(air_state)?;
    let mut contact: Option<&ApiMstSlotitem> = None;
    // The source counts no aircraft here: an emptied slot can still hold contact.
    for (mst_id, _) in planes {
        let Ok(mst) = codex.find::<ApiMstSlotitem>(mst_id) else {
            continue;
        };
        if !can_contact(mst.api_type[2]) || mst.api_saku <= 0 {
            continue;
        }
        if contact.is_some_and(|held| held.api_houm >= mst.api_houm) {
            continue;
        }
        if rng.random_f64_range(0.0, 1.0) < mst.api_saku as f64 / divisor {
            contact = Some(mst);
        }
    }
    contact.map(|mst| Contact {
        modifier: match mst.api_houm {
            3.. => 1.2,
            2 => 1.17,
            _ => 1.12,
        },
        plane: mst.api_id,
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
/// A friendly fleet rolls its 対空カットイン first, once any aircraft come.
/// Returns how many aircraft flew in, how many were shot down, and the cut-in.
// ponytail: no formation modifier or 改修, and the enemy fires no cut-in; add
// them to the fixed shot when they are modelled.
fn fly_through_anti_air(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &[BattleRuntimeShip],
) -> (i64, i64, Option<BattleAirFire>) {
    let fleet_aa = fleet_anti_air(codex, defenders) as f64;
    let attackers_combined = attackers.iter().any(|ship| ship.enemy_deck.is_some());
    let strikes = attackers.iter().filter(|ship| ship.is_alive()).any(|ship| {
        ship.slot_items.iter().zip(ship.ship.api_onslot).take(5).any(|(item, count)| {
            count > 0
                && codex
                    .find::<ApiMstSlotitem>(&item.api_slotitem_id)
                    .is_ok_and(|mst| is_airstrike_attack_type(mst.api_type[2]))
        })
    });
    let air_fire = (strikes && defenders.iter().any(|ship| ship.is_friendly))
        .then(|| roll_air_fire(codex, rng, defenders))
        .flatten();
    let (cut_in_fixed, cut_in_modifier) =
        air_fire.as_ref().map_or((0, 1.0), |fire| (fire.fixed, fire.modifier));
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
                return (flew, shot, air_fire.map(|fire| fire.packet));
            };
            let defender = afloat[pick];
            // A fleet in its smoke screen fires no anti-air but the fixed shot
            // (`getAAShotProp`, `getAAShotFlat`).
            let kept = if defender.smoke > 0 {
                0.0
            } else {
                combined_fire_share(defender, attackers_combined)
            };
            let ship_aa = weighted_anti_air(codex, defender) as f64 * kept;
            // A friendly fleet's fire counts for 1/1.3 as much at a rate of 0.2,
            // and always takes one aircraft more (`getAAShotFlat`, `shotFix`).
            let (fleet_fire, rate, at_least) = if defender.is_friendly {
                ((fleet_aa / 1.3).floor() * kept, 0.2, 1)
            } else {
                (fleet_aa * kept, ENEMY_FLAT_SHOT, 0)
            };
            // A cut-in takes its own number from every slot and makes the
            // fixed shot heavier (`shotFix`, `AACImod`).
            let mut lost = at_least + cut_in_fixed;
            if rng.roll_range(0, 2) == 0 {
                lost += (count as f64 * ship_aa / 200.0).floor() as i64;
            }
            if rng.roll_range(0, 2) == 0 {
                lost += ((ship_aa + fleet_fire) * rate * cut_in_modifier).floor() as i64;
            }
            let lost = lost.min(count);
            ship.ship.api_onslot[slot] = count - lost;
            flew += count;
            shot += lost;
        }
    }
    (flew, shot, air_fire.map(|fire| fire.packet))
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

/// Calculate airstrike damage for a single bomber slot (`kcsim.js` `airstrike`).
///
/// Bomb/torpedo stat × √(onslot) + 25; a torpedo bomber that hits takes ×0.8 or
/// ×1.5 of it, then the cap at 170, then `contact` (触接), the target's own roll
/// ([`special_bonus`]) and the attacker's dock and summer multipliers. `ammo` is the carrier's
/// ammunition modifier. The slot's stars add to a stat above zero
/// ([`crate::damage::with_airstrike_improvement`]). `fit_bonus` is what the slot's 装備ボーナス torpedo
/// adds to its stat, against land too (`getEquipBonusCVTorp(slot)`, `kcsim.js:2022`). A miss
/// rolls nothing.
#[expect(clippy::too_many_arguments)]
fn calculate_single_slot_airstrike_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    slot_item: &KcApiSlotItem,
    onslot: i64,
    defender: &BattleRuntimeShip,
    outcome: HitOutcome,
    critical_damage: f64,
    ammo: f64,
    contact: f64,
    fit_bonus: f64,
) -> i64 {
    if onslot <= 0 || outcome == HitOutcome::Miss {
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
    // A torpedo does nothing to a land target, but the base 25 still strikes.
    let stat = if !is_torpedo_bomber {
        mst.api_baku.max(0) as f64
    } else if target_class(codex, defender) == TargetClass::Installation {
        0.0
    } else {
        mst.api_raig.max(0) as f64
    };
    // A submarine is never her target here.
    let stat = crate::damage::with_airstrike_improvement(stat, mst, slot_item.api_level);
    let bomb_power = (stat + fit_bonus) * (onslot as f64).sqrt();
    // ponytail: a jet's strike outside the jet phase (×0.7 + 0.5 in the source)
    // is not modelled; add it with the jet phase.
    let pre = if is_torpedo_bomber {
        if rng.random_f64_range(0.0, 1.0) < 0.5 {
            0.8
        } else {
            1.5
        }
    } else {
        1.0
    };
    let on_land = target_class(codex, defender) == TargetClass::Installation;
    let pre = pre * skip_bombing_pre_cap(mst.api_id, on_land, ship_type(codex, defender));
    // The target's own roll comes after the torpedo's.
    let post = special_bonus(codex, rng, defender, !is_torpedo_bomber)
        * post_cap_multiplier(codex, attacker, defender, true);
    let raw_power = (bomb_power + 25.0 + aerial_correction(defender)) * pre;
    let capped =
        outcome.power_with(apply_cap(raw_power, 170.0) as f64 * contact * post, critical_damage);
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped, defense, defender.hp(), ammo)
}

/// What a carrier's strike does to particular targets after the cap, each rolled for as the
/// source's chain has them, first match only (`kcsim.js:2053-2083` at `69097abc`, `airstrike`,
/// `enableAirstrikeSpecialBonus`, on by default): 40 times in a hundred the higher figure (50 against a supply depot), else the lower.
/// A dive bomber on any other target takes her `divebombWeak`. Rolls nothing for a target in none.
fn special_bonus(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defender: &BattleRuntimeShip,
    dive_bomber: bool,
) -> f64 {
    let id = defender.ship.api_ship_id;
    let (chance, high, low) = match id {
        1557 => (0.4, 2.0, 1.4),
        1586 | 1620 | 1781 | 1782 | 2105..=2108 => (0.4, 2.2, 1.7),
        _ if crate::pt_imp::is_pt(codex, defender) => (0.4, 0.8, 0.5),
        _ if installation_type(id) == Some(3) || SUPPLY_DEPOT_IDS.contains(&id) => (0.5, 2.4, 1.5),
        1665..=1667 => (0.4, 1.7, 1.3),
        1696..=1698 => (0.4, 1.8, 1.3),
        1699..=1704 | 2023..=2028 | 2243..=2246 => (0.4, 1.4, 1.2),
        1708..=1710 => (0.4, 1.3, 1.0),
        1751 => (0.4, 1.6, 1.3),
        1755..=1760 => (0.4, 1.2, 0.9),
        2178 | 2179 | 2196 | 2197 => (0.4, 1.6, 1.4),
        2188..=2191 => (0.4, 1.9, 1.5),
        _ if dive_bomber => return dive_bomb_weak(id),
        _ => return 1.0,
    };
    if rng.random_f64_range(0.0, 1.0) < chance {
        high
    } else {
        low
    }
}

/// Whether this aircraft skip-bombs: the B-25 and five abyssal planes (`isSkipBomber`,
/// `kcEQDATA.js` at `69097abc`).
pub(super) fn is_skip_bomber(id: i64) -> bool {
    matches!(id, 459 | 1625 | 1626 | 1634..=1636)
}

/// What skip bombing multiplies aircraft `id`'s strike by before the cap, carrier's or air
/// corps' alike (`kcsim.js:2042-2051` and `3286-3295` at `69097abc`, `enableSkipTorpBonus`
/// on): ×0.9 on land, at sea by the target's type, a PT imp counted as the destroyer she is
/// typed; 1 for any other aircraft.
pub(super) fn skip_bombing_pre_cap(id: i64, on_land: bool, ship_type: Option<KcShipType>) -> f64 {
    use KcShipType::*;
    if !is_skip_bomber(id) {
        return 1.0;
    }
    if on_land {
        return 0.9;
    }
    match ship_type {
        Some(DD) => 1.9,
        Some(CL | CLT | AV) => 1.75,
        Some(CA | CAV) => 1.6,
        // The source's `AT` is the 輸送ワ級, stype 15.
        Some(CVL | FBB | BB | BBV | CV | CVB | AP) => 1.3,
        _ => 1.0,
    }
}

/// What skip bombing adds to aircraft `id`'s chance to hit at sea, in percent, by the target's
/// type: `destroyer_points` for a destroyer that is no PT imp, 14 from a carrier and 13 from an
/// air corps (`kcsim.js:1983-1988` and `3200-3206` at `69097abc`); 0 for any other aircraft.
pub(super) fn skip_bombing_accuracy(
    id: i64,
    ship_type: Option<KcShipType>,
    pt: bool,
    destroyer_points: f64,
) -> f64 {
    use KcShipType::*;
    if !is_skip_bomber(id) {
        return 0.0;
    }
    match ship_type {
        Some(FBB | BB | BBV | CVL | CV | CVB | AP) => 31.0,
        Some(CA | CAV) => 22.0,
        Some(CL | CLT | AV) => 18.0,
        Some(DD) if !pt => destroyer_points,
        _ => 0.0,
    }
}

/// `divebombWeak` (`kcSHIPDATA.js`) of the targets the chain above does not catch first; an air
/// corps' chain catches more of them.
pub(super) fn dive_bomb_weak(id: i64) -> f64 {
    match id {
        1668..=1672 => 1.7,
        1684..=1686 => 5.0,
        1687..=1689 => 2.5,
        1690..=1692 => 2.0,
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// Airstrike phase execution
// ---------------------------------------------------------------------------

/// One side's strike (`kcsim.js` `AADefenceBombersAndAirstrike`): every bomber
/// slot still flying draws its target through `choiceWProtect`, and only then
/// do the strikes land, so several slots may pick one ship and a strike on a
/// ship an earlier one sank is spent on her. `flying` marks the attackers
/// afloat when the air battle began; the targets are the defenders' surface
/// ships afloat now. `contact` is the side's 触接 multiplier.
#[expect(clippy::too_many_arguments)]
fn execute_airstrike_phase(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    flying: &[bool],
    defenders: &mut [BattleRuntimeShip],
    defender_formation_id: i64,
    contact: f64,
    is_enemy_side: bool,
    output: &mut AirstrikeOutput,
) {
    // The fleets' balloons bend the contact multiplier (`AADefenceBombersAndAirstrike`).
    let balloons = |fleet: &[BattleRuntimeShip]| fleet.first().map_or(0, |ship| ship.balloons);
    let contact = contact * balloon_contact(balloons(attackers), balloons(defenders));
    let candidates: Vec<usize> = defenders
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.is_alive() && !target_class(codex, ship).is_submarine())
        .map(|(idx, _)| idx)
        .collect();
    let mut strikes = Vec::new();
    for (ship_idx, ship) in attackers.iter().enumerate().filter(|(idx, _)| flying[*idx]) {
        for (slot_idx, (item, count)) in
            ship.slot_items.iter().zip(ship.ship.api_onslot).enumerate()
        {
            let bomber = codex
                .find::<ApiMstSlotitem>(&item.api_slotitem_id)
                .is_ok_and(|mst| is_airstrike_attack_type(mst.api_type[2]));
            if count <= 0 || !bomber {
                continue;
            }
            let Some((target_idx, shown)) = choice_with_protect(
                codex,
                rng,
                defenders,
                &candidates,
                0,
                true,
                defender_formation_id,
            ) else {
                break;
            };
            strikes.push((ship_idx, slot_idx, target_idx, shown));
        }
    }

    for (ship_idx, slot_idx, target_idx, shown) in strikes {
        let ship = &attackers[ship_idx];
        let planes = plane_proficiency(codex, ship);
        let slot_item = &ship.slot_items[slot_idx];
        let mst = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok();
        let torpedo = mst.is_some_and(|mst| {
            KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::CarrierBasedTorpedoBomber)
        });
        let defender = &defenders[target_idx];
        // The source asks nothing about land here.
        let skip = mst.map_or(0.0, |mst| {
            let pt = crate::pt_imp::is_pt(codex, defender);
            skip_bombing_accuracy(mst.api_id, ship_type(codex, defender), pt, 14.0)
        });
        let outcome = roll_strike(codex, rng, defender, AIRSTRIKE_HIT_PERCENT + skip, 1.0, planes);
        let damage = calculate_single_slot_airstrike_damage(
            codex,
            rng,
            ship,
            slot_item,
            ship.ship.api_onslot[slot_idx],
            &defenders[target_idx],
            outcome,
            planes.critical_damage,
            ammo_modifier(codex, ship),
            contact,
            super::night::carrier_slot_torpedo_bonus(codex, ship, slot_idx),
        );
        if outcome == HitOutcome::Critical {
            output.cl_flags[target_idx] = 1;
        }
        output.damage[target_idx]
            .accumulate(defenders[target_idx].take_hit(rng, damage, shown).cell);
        // A strike that misses is still flown at its target: the client
        // draws the run from the flag and the miss from the zero.
        if damage > 0 || outcome == HitOutcome::Miss {
            let (targets, flags) = if torpedo {
                (&mut *output.rai_targets, &mut *output.rai_flags)
            } else {
                (&mut *output.bak_targets, &mut *output.bak_flags)
            };
            targets[ship_idx] = target_idx as i64;
            flags[target_idx] = 1;
        }
    }

    // Attribute total damage to best bomber ship (for statistics)
    if !is_enemy_side && let Some(best_idx) = best_bomber_index(codex, attackers) {
        let total: i64 = output.damage.iter().map(|cell| cell.amount()).sum();
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
    formations: [i64; 2],
    rng: &mut impl BattleRng,
) -> BattleKouku {
    // The ships afloat when the air battle begins fly it to the end: one the
    // friendly strike sinks still sends her own.
    let friendly_flying: Vec<bool> = friendly.iter().map(BattleRuntimeShip::is_alive).collect();
    let enemy_flying: Vec<bool> = enemy.iter().map(BattleRuntimeShip::is_alive).collect();
    let friend_planes = total_plane_count(codex, friendly);
    let enemy_planes = total_plane_count(codex, enemy);

    let friend_fighter_power = calculate_fighter_power(codex, friendly);
    let enemy_fighter_power = calculate_fighter_power(codex, enemy);
    let air_state = AirState::from_power(friend_fighter_power, enemy_fighter_power);

    // Stage 1: the fight for the air costs every slot that flies in it.
    let stage1_f_lost = fight_for_the_air(codex, rng, friendly, air_state, true);
    let stage1_e_lost = fight_for_the_air(codex, rng, enemy, air_state, false);

    // Stage 2: each slot that comes to strike is fired on by one ship.
    let (stage2_f_count, stage2_f_lost, _) = fly_through_anti_air(codex, rng, friendly, enemy);
    let (stage2_e_count, stage2_e_lost, api_air_fire) =
        fly_through_anti_air(codex, rng, enemy, friendly);

    // Stage 3: bombing damage
    let mut api_edam = vec![DamageCell::Plain(0); enemy.len()];
    let mut api_fdam = vec![DamageCell::Plain(0); friendly.len()];
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

    // Contact is tried only by a side whose bombers came to strike.
    let touch = |rng: &mut _, ships: &[BattleRuntimeShip], flying: &[bool], state, struck: i64| {
        let carriers: Vec<&BattleRuntimeShip> =
            ships.iter().zip(flying).filter(|(_, flying)| **flying).map(|(ship, _)| ship).collect();
        (struck > 0).then(|| roll_contact(codex, rng, &carried_planes(&carriers), state)).flatten()
    };
    let friendly_contact = touch(rng, friendly, &friendly_flying, air_state, stage2_f_count);
    let enemy_contact = touch(rng, enemy, &enemy_flying, air_state.reversed(), stage2_e_count);

    // Stage 3: each side draws every target, then strikes.
    execute_airstrike_phase(
        codex,
        rng,
        friendly,
        &friendly_flying,
        enemy,
        formations[1],
        friendly_contact.map_or(1.0, |contact| contact.modifier),
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
        &enemy_flying,
        friendly,
        formations[0],
        enemy_contact.map_or(1.0, |contact| contact.modifier),
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
                friendly_contact.map_or(-1, |contact| contact.plane),
                enemy_contact.map_or(-1, |contact| contact.plane),
            ],
        },
        api_stage2: BattleKoukuStage2 {
            api_f_count: stage2_f_count,
            api_f_lostcount: stage2_f_lost,
            api_e_count: stage2_e_count,
            api_e_lostcount: stage2_e_lost,
            api_air_fire,
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
            api_fdam,
            api_edam,
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
    fn a_strike_rolls_the_targets_own_multiplier_after_the_torpedos() {
        /// Every `f64` draw at `self.0` of the range, counted.
        struct At(f64, usize);
        impl crate::random::BattleRng for At {
            fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
                self.1 += 1;
                min + (max - min) * self.0
            }
            fn roll_range_impl(&mut self, min: i64, _max: i64) -> i64 {
                min
            }
        }

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let target = |id| {
            let mut ship = bare_target(&codex, first_ship_mst_by_type(&codex, KcShipType::DD));
            ship.ship.api_ship_id = id;
            ship
        };
        let bonus = |id, at, dive| {
            let mut rng = At(at, 0);
            (special_bonus(&codex, &mut rng, &target(id), dive), rng.1)
        };
        // 戦艦棲姫: 2 under 0.4, else 1.4, one draw.
        assert_eq!(bonus(1557, 0.39, false), (2.0, 1));
        assert_eq!(bonus(1557, 0.4, false), (1.4, 1));
        // A supply depot rolls at a half.
        assert_eq!(bonus(1653, 0.45, false), (2.4, 1));
        // 離島棲姫 is in no arm: a dive bomber takes her divebombWeak, a torpedo bomber nothing,
        // and neither draws.
        assert_eq!(bonus(1668, 0.0, true), (1.7, 0));
        assert_eq!(bonus(1668, 0.0, false), (1.0, 0));
        // An ordinary ship draws nothing.
        assert_eq!(bonus(1501, 0.0, true), (1.0, 0));

        // A torpedo bomber on 戦艦棲姫 draws its own ×0.8 / ×1.5 first: at 0.45 that is 0.8, and
        // then 1.4 for her.
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        let raig = codex.manifest.find_slotitem(bomber).unwrap().api_raig as f64;
        let mut rng = At(0.45, 0);
        let hime = target(1557);
        let item = slotitem_with_mst_id(bomber);
        let damage = calculate_single_slot_airstrike_damage(
            &codex,
            &mut rng,
            &hime,
            &item,
            16,
            &hime,
            HitOutcome::Hit,
            1.0,
            1.0,
            1.0,
            0.0,
        );
        let capped = crate::damage::apply_cap((raig * 4.0 + 25.0) * 0.8, 170.0) as f64;
        assert_eq!(damage, (capped * 1.4).floor() as i64);
        assert_eq!(rng.1, 2);
    }

    #[test]
    fn a_strike_on_a_pt_imp_rolls_its_own_multiplier() {
        /// A die that always draws `0`, and `f64` draws at `self.0` of the range.
        struct At(f64);
        impl crate::random::BattleRng for At {
            fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
                min + (max - min) * self.0
            }
            fn roll_range_impl(&mut self, min: i64, _max: i64) -> i64 {
                min
            }
        }

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let baku = codex.manifest.find_slotitem(bomber).unwrap().api_baku as f64;
        let mut input = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 1);
        input.ship.api_soukou[0] = 0;
        input.ship.api_maxhp = 999;
        input.ship.api_nowhp = 999;
        let dd = BattleRuntimeShip::from(input);
        let mut pt = dd.clone();
        pt.ship.api_ship_id = ship_mst_id_by_name(&codex, "PT小鬼群");
        let strike = |at, target: &BattleRuntimeShip| {
            let item = slotitem_with_mst_id(bomber);
            let hit = HitOutcome::Hit;
            calculate_single_slot_airstrike_damage(
                &codex,
                &mut At(at),
                target,
                &item,
                16,
                target,
                hit,
                1.0,
                1.0,
                1.0,
                0.0,
            )
        };
        let base = crate::damage::apply_cap(baku * 4.0 + 25.0, 170.0) as f64;
        assert_eq!(strike(0.0, &dd), base as i64);
        // Under 0.4 it lands at 0.8, else at 0.5.
        assert_eq!(strike(0.39, &pt), (base * 0.8).floor() as i64);
        assert_eq!(strike(0.4, &pt), (base * 0.5).floor() as i64);
    }

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

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

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

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

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

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);
        assert_eq!(kouku.api_stage1.api_disp_seiku, 1); // supremacy
    }

    /// Skilled bombers score criticals and flag their target; green ones never
    /// do. (Proficiency turns some misses into hits, and only a hit rolls its
    /// damage, so the two streams part; `accuracy`'s `a_roll_draws_exactly_once`
    /// pins the single draw.)
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
            let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);
            kouku.api_stage3.api_ecl_flag[0]
        };

        let mut criticals = 0;
        for seed in 0..200 {
            assert_eq!(fly(None, seed), 0, "seed {seed}: a green squadron scored a critical");
            criticals += fly(Some(7), seed);
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
            let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);
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
            let (flew, shot, _) =
                fly_through_anti_air(&codex, &mut rng, &mut attackers, &defenders);
            let left = attackers[0].ship.api_onslot;
            assert_eq!(flew, 33, "the fighters between them do not come to strike");
            assert!(left[0] < 30 && left[2] < 3, "seed {seed}: {left:?}");
            assert_eq!(left[1], 12, "seed {seed}: fighters are not fired on");
            assert_eq!(shot, 45 - left.iter().sum::<i64>());
        }
    }

    #[test]
    fn a_fleet_in_its_smoke_screen_fires_only_the_fixed_shot() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let mut carrier = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CV), 50);
        carrier.slot_items = vec![slotitem_with_mst_id(bomber), slotitem_with_mst_id(bomber)];
        carrier.ship.api_onslot = [30, 3, 0, 0, 0];
        let mut defender = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        defender.ship.api_taiku[0] = 200;
        let mut defenders = vec![BattleRuntimeShip::new(defender, true, true)];
        defenders[0].smoke = 1;

        for seed in 0..20 {
            let mut attackers = vec![BattleRuntimeShip::new(carrier.clone(), false, true)];
            let mut rng = crate::random::SeededRng::new(seed);
            let (_, shot, _) = fly_through_anti_air(&codex, &mut rng, &mut attackers, &defenders);
            assert_eq!(attackers[0].ship.api_onslot[..2], [29, 2], "seed {seed}");
            assert_eq!(shot, 2);
        }
    }

    /// 秋月 with two 10cm連装高角砲+高射装置 and 13号対空電探改 fires kind 1, 2
    /// or 3, which take seven, six or four aircraft more from every slot.
    #[test]
    fn a_cut_in_takes_its_number_from_every_slot_and_is_reported() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let mut carrier = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CV), 50);
        carrier.slot_items = vec![slotitem_with_mst_id(bomber), slotitem_with_mst_id(bomber)];
        carrier.ship.api_onslot = [60, 60, 0, 0, 0];
        let mut akizuki = sample_ship(&codex, 421, 50);
        let plain = vec![BattleRuntimeShip::new(akizuki.clone(), true, true)];
        akizuki.slot_items = [122, 122, 106].map(slotitem_with_mst_id).to_vec();
        let armed = vec![BattleRuntimeShip::new(akizuki.clone(), true, true)];

        let (mut fired, mut held) = (0, 0);
        for seed in 0..100 {
            let mut attackers = vec![BattleRuntimeShip::new(carrier.clone(), false, true)];
            let mut rng = crate::random::SeededRng::new(seed);
            let (_, shot, fire) = fly_through_anti_air(&codex, &mut rng, &mut attackers, &armed);
            let Some(fire) = fire else {
                held += 1;
                continue;
            };
            fired += 1;
            let (fixed, shown) = match fire.api_kind {
                1 => (7, vec![122, 122, 106]),
                2 => (6, vec![122, 106]),
                3 => (4, vec![122, 122]),
                other => panic!("seed {seed}: kind {other}"),
            };
            assert_eq!(fire.api_idx, 0);
            assert_eq!(fire.api_use_items, shown);
            assert!(shot >= 2 * (1 + fixed), "seed {seed}: kind {}, {shot} shot", fire.api_kind);
        }
        assert!(fired > held && held > 0, "{fired} fired, {held} held");

        // Without the equipment there is none.
        let mut attackers = vec![BattleRuntimeShip::new(carrier.clone(), false, true)];
        let mut rng = crate::random::SeededRng::new(1);
        let (_, shot, fire) = fly_through_anti_air(&codex, &mut rng, &mut attackers, &plain);
        assert!(fire.is_none());
        assert!(shot >= 2);

        // An enemy fleet fires none.
        let enemy = vec![BattleRuntimeShip::new(akizuki, false, true)];
        let mut attackers = vec![BattleRuntimeShip::new(carrier.clone(), true, true)];
        let (.., fire) = fly_through_anti_air(&codex, &mut rng, &mut attackers, &enemy);
        assert!(fire.is_none());

        // Fighters alone bring none on, and nothing is drawn for it.
        let fighter = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        carrier.slot_items = vec![slotitem_with_mst_id(fighter), slotitem_with_mst_id(fighter)];
        let mut attackers = vec![BattleRuntimeShip::new(carrier, false, true)];
        let mut rng = crate::random::SeededRng::new(1);
        let mut untouched = crate::random::SeededRng::new(1);
        let (flew, _, fire) = fly_through_anti_air(&codex, &mut rng, &mut attackers, &armed);
        assert!(fire.is_none() && flew == 0);
        assert_eq!(rng.roll_range(0, 1000), untouched.roll_range(0, 1000));
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

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

        let s3 = &kouku.api_stage3;
        assert_eq!(s3.api_frai_flag.len(), 3, "api_frai_flag should be friendly-sized (3)");
        assert_eq!(s3.api_fbak_flag.len(), 3, "api_fbak_flag should be friendly-sized (3)");
        assert_eq!(s3.api_erai_flag.len(), 6, "api_erai_flag should be enemy-sized (6)");
        assert_eq!(s3.api_ebak_flag.len(), 6, "api_ebak_flag should be enemy-sized (6)");
    }

    #[test]
    fn kouku_fdam_shows_what_a_protected_ship_lost() {
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
        let mut rng = crate::random::SeededRng::new(4);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

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
        let mut rng = crate::random::SeededRng::new(4);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

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

        // Flagship with low HP — must survive
        let mut friend = sample_ship(&codex, dd_mst, 50);
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 5;
        friend.ship.api_maxhp = 30;

        let mut enemy = sample_ship(&codex, cvl_mst, 99);
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];
        friendly[0].is_flagship = true;
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];
        let mut rng = crate::random::SeededRng::new(42);

        simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

        assert!(friendly[0].hp() > 0, "flagship must survive airstrike under sinking protection");
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
        let mut rng = crate::random::SeededRng::new(4);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);

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

    /// A defender that cannot armour herself: the damage is the power itself.
    fn bare_target(codex: &Codex, mst_id: i64) -> BattleRuntimeShip {
        let mut target = sample_ship(codex, mst_id, 50);
        target.ship.api_soukou[0] = 0;
        target.ship.api_nowhp = 9999;
        target.ship.api_maxhp = 9999;
        BattleRuntimeShip::new(target, false, true)
    }

    /// A torpedo bomber that hits takes its power ×0.8 or ×1.5 before the cap
    /// (`airstrike`, `preMod`); a dive bomber always ×1.
    #[test]
    fn a_torpedo_bomber_hits_for_0_8_or_1_5_of_its_power() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let target = bare_target(&codex, first_ship_mst_by_type(&codex, KcShipType::DD));
        let damage = |kind: KcSlotItemType3| {
            let item = slotitem_with_mst_id(first_slotitem_mst_by_type(&codex, kind));
            let raw = codex.manifest.find_slotitem(item.api_slotitem_id).unwrap();
            let stat = if kind == KcSlotItemType3::CarrierBasedTorpedoBomber {
                raw.api_raig
            } else {
                raw.api_baku
            };
            let seen: std::collections::BTreeSet<i64> = (0..40)
                .map(|seed| {
                    let mut rng = crate::random::SeededRng::new(seed);
                    calculate_single_slot_airstrike_damage(
                        &codex,
                        &mut rng,
                        &target,
                        &item,
                        16,
                        &target,
                        HitOutcome::Hit,
                        1.0,
                        1.0,
                        1.0,
                        0.0,
                    )
                })
                .collect();
            (25.0 + 4.0 * stat as f64, seen)
        };

        let (power, seen) = damage(KcSlotItemType3::CarrierBasedTorpedoBomber);
        let expected = [(power * 0.8).floor() as i64, (power * 1.5).floor() as i64];
        assert_eq!(seen.into_iter().collect::<Vec<_>>(), expected, "power {power}");
        let (power, seen) = damage(KcSlotItemType3::CarrierBasedDiveBomber);
        assert_eq!(seen.into_iter().collect::<Vec<_>>(), [power as i64]);
    }

    /// An abyssal skip bomber strikes a destroyer ×1.9 before the cap, and lands more often on
    /// the bigger ships, a destroyer only when she is no PT imp (`airstrike`,
    /// `kcsim.js:1983-1988` and `2042-2051`).
    #[test]
    fn an_abyssal_skip_bomber_strikes_by_ship_type() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let target = bare_target(&codex, first_ship_mst_by_type(&codex, KcShipType::DD));
        // 深海双発陸爆, 爆装 12, a dive bomber: (25 + 12 × 4) × 1.9 = 138.7 → 138.
        let item = slotitem_with_mst_id(1625);
        assert_eq!(codex.manifest.find_slotitem(1625).unwrap().api_baku, 12);
        let mut rng = crate::random::SeededRng::new(1);
        let damage = calculate_single_slot_airstrike_damage(
            &codex,
            &mut rng,
            &target,
            &item,
            16,
            &target,
            HitOutcome::Hit,
            1.0,
            1.0,
            1.0,
            0.0,
        );
        assert_eq!(damage, 138);

        use KcShipType::*;
        let accuracy = |ship_type, pt| skip_bombing_accuracy(1625, Some(ship_type), pt, 14.0);
        assert_eq!(
            [accuracy(BB, false), accuracy(CAV, false), accuracy(AV, false), accuracy(DD, false)],
            [31.0, 22.0, 18.0, 14.0]
        );
        assert_eq!([accuracy(DD, true), accuracy(DE, false)], [0.0, 0.0]);
    }

    /// Against a land target a torpedo bomber's torpedo stat does not count, but
    /// the base 25 does (`airstrike`: `planebase = isInstall ? 0 : TP`).
    #[test]
    fn a_torpedo_bomber_brings_only_the_base_power_against_land() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut land = bare_target(&codex, first_ship_mst_by_type(&codex, KcShipType::DD));
        land.ship.api_ship_id = ship_mst_id_by_name(&codex, "砲台小鬼");
        let item = slotitem_with_mst_id(first_slotitem_mst_by_type(
            &codex,
            KcSlotItemType3::CarrierBasedTorpedoBomber,
        ));
        let seen: std::collections::BTreeSet<i64> = (0..40)
            .map(|seed| {
                let mut rng = crate::random::SeededRng::new(seed);
                calculate_single_slot_airstrike_damage(
                    &codex,
                    &mut rng,
                    &land,
                    &item,
                    16,
                    &land,
                    HitOutcome::Hit,
                    1.0,
                    1.0,
                    1.0,
                    0.0,
                )
            })
            .collect();
        // 25 × 0.8 and 25 × 1.5, each × 1.3 or × 1.7 for 砲台小鬼 (`special_bonus`).
        assert_eq!(seen.into_iter().collect::<Vec<_>>(), [26, 34, 48, 62]);
    }

    /// A bomber's stars add 0.2★ to her figure, but not a fighter-bomber's, and not to a
    /// figure of nothing (`airstrikePowerImprove`, `kcsim.js:2019`).
    #[test]
    fn stars_add_to_a_bombers_figure_but_not_a_fighter_bombers_or_a_zero() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = bare_target(&codex, first_ship_mst_by_type(&codex, KcShipType::DD));
        let mut land = bare_target(&codex, first_ship_mst_by_type(&codex, KcShipType::DD));
        land.ship.api_ship_id = ship_mst_id_by_name(&codex, "砲台小鬼");
        let strike = |mst_id: i64, stars: i64, target: &BattleRuntimeShip| {
            let mut item = slotitem_with_mst_id(mst_id);
            item.api_level = stars;
            (0..40)
                .map(|seed| {
                    calculate_single_slot_airstrike_damage(
                        &codex,
                        &mut crate::random::SeededRng::new(seed),
                        target,
                        &item,
                        16,
                        target,
                        HitOutcome::Hit,
                        1.0,
                        1.0,
                        1.0,
                        0.0,
                    )
                })
                .collect::<std::collections::BTreeSet<i64>>()
        };
        // 彗星 (爆装 8) at ★10: (8 + 2) × √16 + 25 = 65.
        assert_eq!(codex.manifest.find_slotitem(24).unwrap().api_baku, 8);
        assert_eq!(strike(24, 10, &ship).into_iter().collect::<Vec<_>>(), [65]);
        // 瑞雲 (爆装 4), a seaplane bomber, at ★5: (4 + 1) × √16 + 25 = 45.
        assert_eq!(codex.manifest.find_slotitem(26).unwrap().api_baku, 4);
        assert_eq!(strike(26, 5, &ship).into_iter().collect::<Vec<_>>(), [45]);
        // 零式艦戦62型(爆戦) is a fighter-bomber: her stars add nothing.
        assert_eq!(strike(60, 10, &ship), strike(60, 0, &ship));
        // A torpedo bomber has nothing to bring against land, so her stars do not either.
        let torpedo =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        assert_eq!(strike(torpedo, 10, &land).into_iter().collect::<Vec<_>>(), [26, 34, 48, 62]);
    }

    /// A carrier with 16 scouts, 16 torpedo bombers and fighters to hold the
    /// air: the scouts count toward the contact chance, the bombers do not
    /// (`getContact`).
    fn scouting_carrier(codex: &Codex) -> (BattleRuntimeShip, i64) {
        let scout = first_slotitem_mst_by_type(codex, KcSlotItemType3::CarrierBasedRecon);
        let bomber = first_slotitem_mst_by_type(codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        let fighter = first_slotitem_mst_by_type(codex, KcSlotItemType3::CarrierBasedFighter);
        let mut carrier = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::CV), 50);
        carrier.slot_items = vec![
            slotitem_with_mst_id(bomber),
            slotitem_with_mst_id(scout),
            slotitem_with_mst_id(fighter),
        ];
        carrier.ship.api_onslot = [16, 16, 16, 0, 0];
        let los = codex.manifest.find_slotitem(scout).unwrap().api_saku;
        (BattleRuntimeShip::new(carrier, true, true), los)
    }

    #[test]
    fn contact_chance_follows_the_air_state() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let (carrier, los) = scouting_carrier(&codex);
        let power = 1.0 + (4.0 * los as f64).floor();
        for (state, divisor) in [
            (AirState::Supremacy, Some(25.0)),
            (AirState::Superiority, Some(40.0)),
            (AirState::Denial, Some(55.0)),
            (AirState::Parity, None),
            (AirState::Incapability, None),
        ] {
            assert_eq!(
                contact_chance(&codex, &carried_planes(&[&carrier]), state),
                divisor.map(|divisor| power / divisor),
                "{state:?}"
            );
        }
    }

    /// Every target is drawn before any damage: two bombers both aim at a
    /// lone destroyer, and the second strikes her after the first has sunk her.
    /// With one ship afloat no seed is needed to make them pick the same one.
    #[test]
    fn every_target_is_drawn_before_the_first_strike() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cv_mst = first_ship_mst_by_type(&codex, KcShipType::CV);
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let carrier = || {
            let mut carrier = sample_ship(&codex, cv_mst, 50);
            carrier.slot_items = vec![slotitem_with_mst_id(bomber)];
            carrier.ship.api_onslot = [18, 0, 0, 0, 0];
            BattleRuntimeShip::new(carrier, true, true)
        };
        let mut target = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        target.ship.api_nowhp = 1;
        target.ship.api_taiku[0] = 0;

        let mut sunk = 0;
        for seed in 0..20 {
            let mut friendly = vec![carrier(), carrier()];
            let mut enemies = vec![BattleRuntimeShip::new(target.clone(), false, true)];
            let mut rng = crate::random::SeededRng::new(seed);
            let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);
            assert_eq!(kouku.api_stage3.api_fbak, [0, 0], "seed {seed}: both flew at her");
            sunk += i64::from(enemies[0].is_sunk());
        }
        // Where she sank, the other strike still flew at her.
        assert!(sunk > 0, "no strike ever sank her");
    }

    /// `api_touch_plane` names the aircraft that made contact, or -1; a side
    /// with no bomber to strike never tries.
    #[test]
    fn touch_plane_reports_the_rolled_contact() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let scout = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedRecon);
        let mut enemy = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        enemy.ship.api_nowhp = 9999;
        enemy.ship.api_maxhp = 9999;

        let fly = |carrier: &BattleRuntimeShip, seed: u64| {
            let mut friendly = vec![carrier.clone()];
            let mut enemies = vec![BattleRuntimeShip::new(enemy.clone(), false, true)];
            let mut rng = crate::random::SeededRng::new(seed);
            simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng)
                .api_stage1
                .api_touch_plane
        };

        let (striking, _) = scouting_carrier(&codex);
        let seen: std::collections::BTreeSet<i64> = (0..40)
            .map(|seed| fly(&striking, seed))
            .inspect(|touch| assert_eq!(touch[1], -1))
            .map(|touch| touch[0])
            .collect();
        assert!(seen.contains(&scout) && seen.contains(&-1), "{seen:?}");

        let mut scouting = striking.clone();
        scouting.slot_items.remove(0);
        scouting.ship.api_onslot = [16, 16, 0, 0, 0];
        for seed in 0..40 {
            assert_eq!(fly(&scouting, seed), [-1, -1], "seed {seed}: no bomber, no contact");
        }
    }

    /// A strike aimed at the flagship can be taken by a healthy ship beside
    /// her (`choiceWProtect`), and the damage is marked as covered.
    #[test]
    fn a_flagship_can_be_covered_from_a_strike() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let mut carrier = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CV), 50);
        carrier.slot_items = vec![slotitem_with_mst_id(bomber); 4];
        carrier.ship.api_onslot = [18, 18, 18, 18, 0];
        let mut target = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        target.ship.api_nowhp = 9999;
        target.ship.api_maxhp = 9999;
        target.ship.api_taiku[0] = 0;

        let covered = (0..40).any(|seed| {
            let mut friendly = vec![BattleRuntimeShip::new(carrier.clone(), true, true)];
            let mut enemies = vec![
                BattleRuntimeShip::new(target.clone(), false, true),
                BattleRuntimeShip::new(target.clone(), false, true),
            ];
            let mut rng = crate::random::SeededRng::new(seed);
            let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, [1, 1], &mut rng);
            matches!(kouku.api_stage3.api_edam[1], DamageCell::Shielded(_))
        });
        assert!(covered, "no strike on the flagship was ever covered");
    }

    /// Under supremacy 彩雲 (索敵 9, 命中 2) and 二式艦上偵察機 (索敵 7, 命中 3)
    /// make contact for certain, (1 + 36 + 28) / 25; then 彩雲 rolls 9/14 for
    /// ×1.17, and the more accurate 二式 rolls 7/14 for ×1.2 after her.
    #[test]
    fn contact_goes_to_the_most_accurate_scout_that_rolls_it() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut carrier = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CV), 50);
        carrier.slot_items = vec![slotitem_with_mst_id(54), slotitem_with_mst_id(61)];
        carrier.ship.api_onslot = [16, 16, 0, 0, 0];
        let carrier = BattleRuntimeShip::new(carrier, true, true);
        assert!(
            contact_chance(&codex, &carried_planes(&[&carrier]), AirState::Supremacy).unwrap()
                > 1.0
        );

        let seen: Vec<Option<(f64, i64)>> = (0..60)
            .map(|seed| {
                let mut rng = crate::random::SeededRng::new(seed);
                roll_contact(&codex, &mut rng, &carried_planes(&[&carrier]), AirState::Supremacy)
                    .map(|contact| (contact.modifier, contact.plane))
            })
            .collect();
        for outcome in [None, Some((1.17, 54)), Some((1.2, 61))] {
            assert!(seen.contains(&outcome), "{outcome:?} never came up");
        }
        assert!(
            seen.iter().all(|outcome| [None, Some((1.17, 54)), Some((1.2, 61))].contains(outcome))
        );
    }
}
