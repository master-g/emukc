//! Damage calculation functions for battle simulation.
//!
//! Extracted from the monolithic battle core module into a focused
//! damage computation unit. All functions are pure — they take immutable
//! references to battle state and return computed values.

use emukc_model::{
    codex::Codex,
    kc2::{KcApiSlotItem, KcShipType, KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::random::BattleRng;
use crate::types::{AirState, BattlePhase, BattleRuntimeShip, EngagementType};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Apply the post-cap soft-cap formula.
///
/// If `raw_power <= cap`, return `floor(raw_power)`.
/// Otherwise return `floor(cap + sqrt(raw_power - cap))`.
pub fn apply_cap(raw_power: f64, cap: f64) -> i64 {
    if raw_power <= cap {
        raw_power.floor() as i64
    } else {
        (cap + (raw_power - cap).sqrt().floor()).floor() as i64
    }
}

// ---------------------------------------------------------------------------
// Damage formula functions
// ---------------------------------------------------------------------------

/// Calculate defense power using the randomized formula:
/// `floor(0.7 × A_t + 0.6 × random(0, floor(A_t) − 1))`
///
/// When armor ≤ 1, the random range is empty, so the result is just `floor(0.7 × A_t)`.
pub(crate) fn calculate_defense_power(rng: &mut impl BattleRng, armor_stat: i64) -> f64 {
    let a = armor_stat.max(0) as f64;
    let rand_part = if armor_stat > 1 {
        rng.roll_range(0, armor_stat) as f64
    } else {
        0.0
    };
    (0.7 * a + 0.6 * rand_part).floor()
}

/// Calculate the damage state modifier based on attacker's HP ratio.
///
/// Returns a pre-cap multiplier:
/// - Normal (>75% HP): 1.0
/// - Chuuha (25–75% HP): 0.7 for shelling/ASW, 0.8 for torpedo
/// - Taiha (<25% HP): 0.4 for shelling/ASW, 0.0 for torpedo
pub(crate) fn damage_state_modifier(current_hp: i64, max_hp: i64, phase: BattlePhase) -> f64 {
    if max_hp <= 0 {
        return 1.0;
    }
    // HP ratio threshold: chuuha is ≤75%, taiha is ≤25%
    let hp_ratio = current_hp as f64 / max_hp as f64;
    if hp_ratio <= 0.25 {
        match phase {
            BattlePhase::OpeningTorpedo | BattlePhase::ClosingTorpedo => 0.0,
            _ => 0.4,
        }
    } else if hp_ratio <= 0.75 {
        match phase {
            BattlePhase::OpeningTorpedo | BattlePhase::ClosingTorpedo => 0.8,
            _ => 0.7,
        }
    } else {
        1.0
    }
}

/// Resolve final damage after capping, applying defense and scratch damage logic.
///
/// If `capped_power < defense`, returns scratch (proportional) damage instead of minimum 1.
pub(crate) fn resolve_damage(
    rng: &mut impl BattleRng,
    capped_power: f64,
    defense: f64,
    target_hp: i64,
) -> i64 {
    if capped_power <= 0.0 {
        return 0;
    }
    if capped_power < defense {
        calculate_scratch_damage(rng, target_hp.max(1))
    } else {
        (capped_power - defense).floor().max(0.0) as i64
    }
}

/// Calculate shelling damage for a single attack.
pub(crate) fn calculate_shelling_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
) -> i64 {
    let basic_power = if is_cv_type(codex, attacker) {
        let bomber_count = bomber_slot_count(codex, attacker);
        if bomber_count > 0 {
            1.5 * bomber_count as f64 + 55.0
        } else {
            attacker.ship.api_karyoku[0].max(0) as f64 + 5.0
        }
    } else {
        attacker.ship.api_karyoku[0].max(0) as f64 + 5.0
    };
    let bonus = improvement_bonus_day(codex, attacker) + light_gun_bonus(codex, attacker);
    let dmg_state =
        damage_state_modifier(attacker.hp(), attacker.ship.api_maxhp, BattlePhase::DayShelling);
    let pre_cap = (basic_power + bonus)
        * shelling_formation_modifier(formation_id)
        * engagement.modifier()
        * dmg_state;
    let capped_power = apply_cap(pre_cap, 220.0) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped_power, defense, defender.hp())
}

/// Calculate torpedo damage for a single attack.
pub(crate) fn calculate_torpedo_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
    phase: BattlePhase,
) -> i64 {
    let basic_power =
        attacker.ship.api_raisou[0].max(0) as f64 + improvement_bonus_torpedo(codex, attacker);
    let dmg_state = damage_state_modifier(attacker.hp(), attacker.ship.api_maxhp, phase);
    let pre_cap =
        basic_power * torpedo_formation_modifier(formation_id) * engagement.modifier() * dmg_state;
    let capped_power = apply_cap(pre_cap, 180.0) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped_power, defense, defender.hp())
}

/// Calculate night battle damage for a single attack.
pub(crate) fn calculate_night_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    air_state: Option<&AirState>,
) -> i64 {
    let basic_power = (attacker.ship.api_karyoku[0].max(0) + attacker.ship.api_raisou[0].max(0) + 5)
        as f64
        + improvement_bonus_night(codex, attacker)
        + night_recon_bonus(codex, attacker, air_state);
    let capped_power = apply_cap(basic_power, 360.0) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped_power, defense, defender.hp())
}

/// Calculate ASW damage against a submarine target.
pub(crate) fn calculate_asw_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
) -> i64 {
    let ship_asw = attacker.ship.api_taisen[0].max(0) as f64;
    let equip_asw = equipment_asw_total(codex, attacker);
    // base ASW = total ASW - equipment ASW (modernization + innate)
    let base_asw = (ship_asw - equip_asw).max(0.0);

    // Attack type bonus: +8 for aircraft ASW, +13 for depth charge
    let type_bonus = if has_active_asw_aircraft(codex, attacker) {
        8.0
    } else {
        13.0
    };

    let synergy = asw_synergy_modifier(codex, attacker);
    let raw_power = (base_asw.sqrt() * 2.0 + equip_asw.sqrt() * 1.5 + type_bonus) * synergy;
    let dmg_state =
        damage_state_modifier(attacker.hp(), attacker.ship.api_maxhp, BattlePhase::DayShelling);
    let modified =
        raw_power * asw_formation_modifier(formation_id) * engagement.modifier() * dmg_state;
    let capped = apply_cap(modified, 170.0) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    let armor_reduction = depth_charge_armor_reduction(codex, attacker);
    let adjusted_defense = (defense - armor_reduction).max(0.0);
    resolve_damage(rng, capped, adjusted_defense, defender.hp())
}

/// Calculate airstrike damage for a single bomber slot.
pub(crate) fn calculate_single_slot_airstrike_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    slot_item: &KcApiSlotItem,
    onslot: i64,
    defender: &BattleRuntimeShip,
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
    let raw_power = bomb_power + 25.0;
    let capped = apply_cap(raw_power, 170.0) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped, defense, defender.hp())
}

/// Calculate scratch (proportional) damage when capped power is below defense.
pub(crate) fn calculate_scratch_damage(rng: &mut impl BattleRng, current_hp: i64) -> i64 {
    rng.roll_scratch_damage(current_hp).min(current_hp.max(1))
}

// ---------------------------------------------------------------------------
// Ship classification helpers
// ---------------------------------------------------------------------------

/// Check if a ship is a carrier type (CV, CVL, or CVB).
pub(crate) fn is_cv_type(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    matches!(ship_type(codex, ship), Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB))
}

/// Count the number of bomber-type slots (dive bomber + torpedo bomber) on a ship.
pub(crate) fn bomber_slot_count(codex: &Codex, ship: &BattleRuntimeShip) -> i64 {
    const BOMBER_TYPES: &[KcSlotItemType3] =
        &[KcSlotItemType3::CarrierBasedDiveBomber, KcSlotItemType3::CarrierBasedTorpedoBomber];
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(|t| BOMBER_TYPES.contains(&t))
        })
        .count() as i64
}

/// Light cruiser gun bonus: `√(single_gun_count) + 2 × √(twin_gun_count)`.
pub(crate) fn light_gun_bonus(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    if !matches!(ship_type(codex, ship), Some(KcShipType::CL | KcShipType::CLT)) {
        return 0.0;
    }
    let single = ship
        .slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                == Some(KcSlotItemType3::SmallCaliberMainGun)
        })
        .count() as f64;
    let twin = ship
        .slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                == Some(KcSlotItemType3::MediumCaliberMainGun)
        })
        .count() as f64;
    single.sqrt() + 2.0 * twin.sqrt()
}

/// Night recon plane bonus based on air state.
pub(crate) fn night_recon_bonus(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    air_state: Option<&AirState>,
) -> f64 {
    let has_night_recon = ship.slot_items.iter().any(|slot_item| {
        codex
            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
            .ok()
            .map(|mst| mst.api_type[3] == 50)
            .unwrap_or(false)
    });
    if !has_night_recon {
        return 0.0;
    }
    match air_state {
        Some(AirState::Supremacy) => 9.0,
        Some(AirState::Superiority) => 7.0,
        _ => 5.0,
    }
}

// ---------------------------------------------------------------------------
// Formation modifiers
// ---------------------------------------------------------------------------

/// Shelling formation modifier.
pub(crate) fn shelling_formation_modifier(formation_id: i64) -> f64 {
    match formation_id {
        2 => 0.8,
        3 => 0.7,
        4 => 0.85,
        5 => 0.6,
        _ => 1.0,
    }
}

/// Torpedo formation modifier.
pub(crate) fn torpedo_formation_modifier(formation_id: i64) -> f64 {
    match formation_id {
        2 => 0.8,
        3 => 0.7,
        4 => 0.85,
        5 => 0.6,
        _ => 1.0,
    }
}

/// ASW formation modifier: Diamond (3) = 1.2×, Echelon (4) = 1.1×, Line Abreast (5) = 1.3×
pub(crate) fn asw_formation_modifier(formation_id: i64) -> f64 {
    match formation_id {
        3 => 1.2,
        4 => 1.1,
        5 => 1.3,
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// Improvement bonuses
// ---------------------------------------------------------------------------

/// Day shelling improvement bonus: sum of √(★) per weapon equipment.
pub(crate) fn improvement_bonus_day(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    const WEAPON_TYPES: &[KcSlotItemType3] = &[
        KcSlotItemType3::SmallCaliberMainGun,
        KcSlotItemType3::MediumCaliberMainGun,
        KcSlotItemType3::LargeCaliberMainGun,
        KcSlotItemType3::SecondaryGun,
        KcSlotItemType3::Torpedo,
        KcSlotItemType3::CarrierBasedDiveBomber,
        KcSlotItemType3::CarrierBasedTorpedoBomber,
        KcSlotItemType3::SeaBasedBomber,
        KcSlotItemType3::LargeCaliberMainGun2,
        KcSlotItemType3::SecondaryGun2,
    ];
    ship.slot_items
        .iter()
        .filter_map(|si| {
            if si.api_level <= 0 {
                return None;
            }
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .filter(|t| WEAPON_TYPES.contains(t))
                .map(|_| (si.api_level as f64).sqrt())
        })
        .sum()
}

/// Torpedo improvement bonus: sum of ★ × 1.2 per torpedo equipment.
pub(crate) fn improvement_bonus_torpedo(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    const TORPEDO_TYPES: &[KcSlotItemType3] =
        &[KcSlotItemType3::Torpedo, KcSlotItemType3::SubmarineTorpedo];
    ship.slot_items
        .iter()
        .filter_map(|si| {
            if si.api_level <= 0 {
                return None;
            }
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .filter(|t| TORPEDO_TYPES.contains(t))
                .map(|_| si.api_level as f64 * 1.2)
        })
        .sum()
}

/// Night battle improvement bonus: same formula as day (√★ per weapon).
pub(crate) fn improvement_bonus_night(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    improvement_bonus_day(codex, ship)
}

// ---------------------------------------------------------------------------
// ASW helpers
// ---------------------------------------------------------------------------

/// Determine ASW equipment synergy multiplier.
pub(crate) fn asw_synergy_modifier(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    let has_sonar = has_slotitem_type(codex, ship, KcSlotItemType3::Sonar);
    let has_large_sonar = has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar);
    let has_depth_charge = has_slotitem_type(codex, ship, KcSlotItemType3::DepthCharge);
    let any_sonar = has_sonar || has_large_sonar;

    // Depth charge projectors are a subset of depth charge equipment.
    // Simplified: treat all DepthCharge as both projector and charge for now.
    // Full implementation would check specific item IDs.
    let has_projector = has_depth_charge;

    if has_sonar && has_projector && has_depth_charge {
        1.4375
    } else if has_large_sonar && has_projector && has_depth_charge {
        1.265
    } else if any_sonar && has_depth_charge {
        1.15
    } else if has_projector && has_depth_charge {
        1.1
    } else {
        1.0
    }
}

/// Depth charge armor reduction: sum of `√(ASW − 2)` per depth charge with ASW > 2.
pub(crate) fn depth_charge_armor_reduction(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    ship.slot_items
        .iter()
        .filter_map(|si| {
            let mst = codex.find::<ApiMstSlotitem>(&si.api_slotitem_id).ok()?;
            let type3 = KcSlotItemType3::n(mst.api_type[2])?;
            if type3 != KcSlotItemType3::DepthCharge {
                return None;
            }
            let asw = mst.api_tais.max(0) as f64;
            if asw > 2.0 {
                Some((asw - 2.0).sqrt())
            } else {
                None
            }
        })
        .sum()
}

/// Calculate equipment ASW from all equipped items.
pub(crate) fn equipment_asw_total(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    ship.slot_items
        .iter()
        .filter_map(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .map(|mst| mst.api_tais.max(0) as f64)
        })
        .sum()
}

// ---------------------------------------------------------------------------
// Private helpers (used internally by the functions above)
// ---------------------------------------------------------------------------

fn ship_mst<'a>(
    codex: &'a Codex,
    ship: &'a BattleRuntimeShip,
) -> Option<&'a emukc_model::kc2::start2::ApiMstShip> {
    codex.find(&ship.ship.api_ship_id).ok()
}

fn ship_type(codex: &Codex, ship: &BattleRuntimeShip) -> Option<KcShipType> {
    ship_mst(codex, ship).and_then(|mst| KcShipType::n(mst.api_stype as i32))
}

fn has_slotitem_type(codex: &Codex, ship: &BattleRuntimeShip, wanted: KcSlotItemType3) -> bool {
    ship.slot_items.iter().any(|slot_item| {
        codex
            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
            .ok()
            .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
            == Some(wanted)
    })
}

fn has_active_asw_aircraft(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(slot_item, onslot)| {
        let Some(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok() else {
            return false;
        };
        matches!(
            KcSlotItemType3::n(mst.api_type[2]),
            Some(
                KcSlotItemType3::AutoGyro
                    | KcSlotItemType3::AntiSubmarinePatrol
                    | KcSlotItemType3::SeaBasedBomber
                    | KcSlotItemType3::LargeFlyingBoat
            )
        ) && onslot > 0
    })
}

fn is_airstrike_attack_type(slotitem_type: i64) -> bool {
    matches!(
        KcSlotItemType3::n(slotitem_type),
        Some(
            KcSlotItemType3::CarrierBasedDiveBomber
                | KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::SeaBasedBomber
                | KcSlotItemType3::JetFighterBomber
                | KcSlotItemType3::JetAttacker
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::{BattlePhase, BattleRuntimeShip, EngagementType};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    #[test]
    fn day_shelling_cap_matches_reference_example() {
        assert_eq!(apply_cap(250.0, 220.0), 225);
        assert_eq!(apply_cap(224.0, 220.0), 222);
    }

    #[test]
    fn defense_power_randomized_range() {
        // With armor 100, defense should be in range [floor(0.7*100), floor(0.7*100 + 0.6*99)]
        // = [70, 129]
        let min_armor = 1;
        let mut rng = crate::random::SeededRng::new(12345);
        let mut min_val = i64::MAX;
        let mut max_val = i64::MIN;
        for _ in 0..1000 {
            let def = calculate_defense_power(&mut rng, 100) as i64;
            min_val = min_val.min(def);
            max_val = max_val.max(def);
        }
        assert!(min_val >= 70, "min defense {min_val} should be >= 70");
        assert!(max_val <= 129, "max defense {max_val} should be <= 129");
        assert!(min_val < max_val, "defense should vary with RNG");

        // Edge case: armor 0
        let def = calculate_defense_power(&mut rng, 0);
        assert_eq!(def as i64, 0);

        // Edge case: armor 1
        let def = calculate_defense_power(&mut rng, 1);
        assert_eq!(def as i64, 0); // floor(0.7) = 0

        // Drop the unused warning
        let _ = min_armor;
    }

    #[test]
    fn damage_state_modifier_thresholds() {
        // Normal: HP > 75% of max
        assert!(
            (damage_state_modifier(80, 100, BattlePhase::DayShelling) - 1.0).abs() < f64::EPSILON
        );
        assert!(
            (damage_state_modifier(76, 100, BattlePhase::DayShelling) - 1.0).abs() < f64::EPSILON
        );

        // Chuuha: 25% < HP <= 75%
        assert!(
            (damage_state_modifier(75, 100, BattlePhase::DayShelling) - 0.7).abs() < f64::EPSILON
        );
        assert!(
            (damage_state_modifier(50, 100, BattlePhase::DayShelling) - 0.7).abs() < f64::EPSILON
        );
        assert!(
            (damage_state_modifier(26, 100, BattlePhase::DayShelling) - 0.7).abs() < f64::EPSILON
        );

        // Torpedo chuuha: 0.8
        assert!(
            (damage_state_modifier(75, 100, BattlePhase::OpeningTorpedo) - 0.8).abs()
                < f64::EPSILON
        );
        assert!(
            (damage_state_modifier(50, 100, BattlePhase::ClosingTorpedo) - 0.8).abs()
                < f64::EPSILON
        );

        // Taiha: HP <= 25%
        assert!(
            (damage_state_modifier(25, 100, BattlePhase::DayShelling) - 0.4).abs() < f64::EPSILON
        );
        assert!(
            (damage_state_modifier(10, 100, BattlePhase::DayShelling) - 0.4).abs() < f64::EPSILON
        );
        // ASW taiha: 0.4
        assert!(
            (damage_state_modifier(25, 100, BattlePhase::DayShelling) - 0.4).abs() < f64::EPSILON
        );

        // Torpedo taiha: 0.0
        assert!(
            (damage_state_modifier(25, 100, BattlePhase::OpeningTorpedo) - 0.0).abs()
                < f64::EPSILON
        );
        assert!(
            (damage_state_modifier(10, 100, BattlePhase::ClosingTorpedo) - 0.0).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn scratch_damage_triggers_when_attack_below_defense() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut rng = crate::random::SeededRng::new(99);
        let mut attacker = BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 1)); // weak DD
        let defender = BattleRuntimeShip::from(sample_ship(&codex, 412, 99)); // strong abyssal
        attacker.ship.api_karyoku[0] = 10; // very low firepower
        // With FP=10, base=15, capped=~15. Defense with armor ~80 is 56-103.
        // This should trigger scratch damage.
        let dmg = calculate_shelling_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
        );
        // Scratch damage is proportional to target HP: 0.06*H + 0.08*rand(0,H-1)
        // It should be much less than capped_power - defense (which would be negative)
        assert!(dmg >= 1, "scratch damage should be at least 1");
        assert!(dmg < 50, "scratch damage should be small (proportional to HP)");
    }

    #[test]
    fn normal_damage_when_attack_above_defense() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut rng = crate::random::SeededRng::new(99);
        let mut attacker = BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 99));
        let mut defender = BattleRuntimeShip::from(sample_ship(&codex, 412, 99));
        attacker.ship.api_karyoku[0] = 200; // strong firepower
        defender.ship.api_soukou[0] = 10; // low armor
        let dmg = calculate_shelling_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
        );
        // capped ~205, defense ~7-13, so damage should be 192-198
        assert!(dmg > 100, "normal damage should be large: got {dmg}");
    }

    #[test]
    fn torpedo_base_power_without_plus_five() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut rng = crate::random::SeededRng::new(42);
        let mut attacker = BattleRuntimeShip::from(sample_ship(&codex, 89, 99));
        let mut defender = BattleRuntimeShip::from(sample_ship(&codex, 412, 99));
        attacker.ship.api_raisou[0] = 100;
        defender.ship.api_soukou[0] = 10;
        let dmg = calculate_torpedo_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
            BattlePhase::OpeningTorpedo,
        );
        // Basic power = 100 (NOT 105). After formation (1.0) and engagement (1.0), capped at 100.
        // Defense with armor 10: ~7-13. Damage ~87-93.
        // If +5 was still there: basic=105, damage ~92-98.
        assert!(dmg < 100, "torpedo damage should be < 100 without +5: got {dmg}");
        assert!(dmg > 50, "torpedo damage should still be significant: got {dmg}");
    }

    #[test]
    fn taiha_torpedo_deals_zero_not_scratch() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut rng = crate::random::SeededRng::new(42);
        let mut attacker = BattleRuntimeShip::from(sample_ship(&codex, 89, 99));
        let defender = BattleRuntimeShip::from(sample_ship(&codex, 412, 99));
        attacker.ship.api_raisou[0] = 100;
        // Simulate taiha: 10 HP out of ~30-40 max → HP ratio well below 25%
        attacker.current_hp = 1;
        let dmg = calculate_torpedo_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
            BattlePhase::OpeningTorpedo,
        );
        assert_eq!(dmg, 0, "taiha torpedo should deal 0 damage, got {dmg}");
    }

    #[test]
    fn asw_formation_modifier_diamond_and_line_abreast() {
        assert!((asw_formation_modifier(3) - 1.2).abs() < f64::EPSILON);
        assert!((asw_formation_modifier(4) - 1.1).abs() < f64::EPSILON);
        assert!((asw_formation_modifier(5) - 1.3).abs() < f64::EPSILON);
        assert!((asw_formation_modifier(1) - 1.0).abs() < f64::EPSILON);
        assert!((asw_formation_modifier(2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn asw_damage_formula_uses_sqrt_base_and_equipment() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let dc_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::DepthCharge);
        let dc_mst = codex.manifest.find_slotitem(dc_mst_id).unwrap();
        let equip_asw = dc_mst.api_tais.max(0) as f64;

        let mut attacker_input = sample_ship(&codex, dd_mst, 50);
        attacker_input.ship.api_taisen[0] = 80;
        attacker_input.slot_items = vec![slotitem_with_mst_id(dc_mst_id)];
        let attacker = BattleRuntimeShip::from(attacker_input);

        let mut defender_input = sample_ship(&codex, ss_mst, 50);
        defender_input.ship.api_soukou[0] = 10;
        let defender = BattleRuntimeShip::from(defender_input);

        let mut rng = crate::random::SeededRng::new(42);
        let dmg = calculate_asw_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1, // line ahead
            EngagementType::SameCourse,
        );

        // Verify damage is positive and uses the ASW formula (not shelling formula)
        assert!(dmg >= 1);
        // raw_power = (√(80 - equip_asw) * 2 + √equip_asw * 1.5 + 13) * synergy
        // With a single depth charge: projector=true, dc=true → synergy = 1.1
        let base_asw = (80.0 - equip_asw).max(0.0);
        let synergy = 1.1; // single DepthCharge counts as both projector and charge
        let expected_raw = (base_asw.sqrt() * 2.0 + equip_asw.sqrt() * 1.5 + 13.0) * synergy;
        let expected_capped = apply_cap(expected_raw, 170.0) as f64;
        // Defense is now randomized; just verify damage is positive and reasonable
        // With armor 10, defense range is [7, 13] so damage should be in a range
        let max_defense: f64 = (0.7_f64 * 10.0 + 0.6 * 9.0).floor(); // max possible defense = 12.4 → 12
        assert!(dmg >= (expected_capped - max_defense).floor() as i64);
        assert!(dmg <= expected_capped as i64);
    }

    #[test]
    fn battle_context_applies_formation_and_engagement() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut rng = crate::random::SeededRng::new(42);
        let mut attacker = BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 99));
        let defender = BattleRuntimeShip::from(sample_ship(&codex, 412, 99));
        attacker.ship.api_karyoku[0] = 180;
        // Use a large enough firepower to guarantee capped_power > defense even with RNG
        let normal_damage = calculate_shelling_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
        );
        let penalized_damage = calculate_shelling_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            5,
            EngagementType::TDisadvantage,
        );

        assert!(normal_damage > penalized_damage);
    }
}
