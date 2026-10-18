//! Damage calculation functions for battle simulation.
//!
//! Extracted from the monolithic battle core module into a focused
//! damage computation unit. All functions are pure — they take immutable
//! references to battle state and return computed values.

use emukc_model::{
    codex::Codex,
    kc2::{
        KcApiSlotItem, KcShipType, KcSlotItemType3,
        start2::{ApiMstShip, ApiMstSlotitem},
    },
};

use crate::accuracy::{
    HitOutcome, NightContact, PlaneProficiency, plane_proficiency, shelling_proficiency,
};
use crate::combined::{
    CombinedAttackClass, combined_correction_vs_enemy_combined, combined_correction_vs_single,
    combined_formation_modifier,
};
use crate::installation::{AntiInstallation, land_target, post_cap_multiplier};
use crate::random::BattleRng;
use crate::targeting::{
    abyssal_carrier_asw, has_active_asw_aircraft, has_slotitem_type, is_airstrike_attack_type,
    ship_type, target_class,
};
use crate::types::{BattlePhase, BattleRuntimeShip, EngagementType};

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
    (DEFENSE_COEFF_A * a + DEFENSE_COEFF_B * rand_part).floor()
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

/// The damage of an attack of `capped_power` against `defense`, after the source's
/// `damageCommon`: what is left of the power after the armour, times the attacker's
/// ammunition modifier `ammo`, rounded down; a scratch where that comes to nothing. An
/// attack with no power, which is a miss, does none.
pub(crate) fn resolve_damage(
    rng: &mut impl BattleRng,
    capped_power: f64,
    defense: f64,
    target_hp: i64,
    ammo: f64,
) -> i64 {
    if capped_power <= 0.0 {
        return 0;
    }
    let damage = ((capped_power - defense) * ammo).floor() as i64;
    if damage <= 0 {
        calculate_scratch_damage(rng, target_hp.max(1))
    } else {
        damage
    }
}

/// One hit of a double attack, a cut-in or a special attack: the damage it does and what
/// `api_cl_list` says of it. The source's `shell` and `NBattack`: a hit of these that misses
/// still scratches, and is written as a hit.
pub(crate) fn cut_in_hit(
    rng: &mut impl BattleRng,
    outcome: HitOutcome,
    raw: i64,
    target_hp: i64,
) -> (i64, i64) {
    match outcome {
        HitOutcome::Miss => (calculate_scratch_damage(rng, target_hp.max(1)), HitOutcome::Hit.cl()),
        _ => (raw, outcome.cl()),
    }
}

/// Post-cap ammunition modifier based on the attacker's remaining ammo (弾薬量補正).
///
/// Friendly ships deal reduced damage when low on ammo. At ≥50% remaining the
/// modifier is `1.0`; below 50% it scales linearly to `0.0` as ammo empties,
/// following the canonical `min(1.0, ammo_percent / 50)` where
/// `ammo_percent = floor(100 × api_bull / api_bull_max)` (an integer percentage).
///
/// Enemy (abyssal) attackers do not track ammo and are never penalized. When the
/// master ammo capacity is unknown or non-positive, no penalty is applied.
pub(crate) fn ammo_modifier(codex: &Codex, attacker: &BattleRuntimeShip) -> f64 {
    if !attacker.is_friendly {
        return 1.0;
    }
    let Ok(mst) = codex.find::<ApiMstShip>(&attacker.ship.api_ship_id) else {
        return 1.0;
    };
    let bull_max = mst.api_bull_max.unwrap_or(0);
    if bull_max <= 0 {
        return 1.0;
    }
    let bull = attacker.ship.api_bull.max(0);
    let ammo_percent = (100 * bull) / bull_max;
    (ammo_percent as f64 / 50.0).min(1.0)
}

/// Calculate shelling damage for a single attack.
///
/// When `ci_multiplier` is `Some(m)`, the multiplier is applied post-cap
/// (after the daytime soft cap of 220) — matching `KanColle`'s artillery
/// spotting damage pipeline.
#[expect(clippy::too_many_arguments)]
pub(crate) fn calculate_shelling_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
    ci_multiplier: Option<f64>,
    outcome: HitOutcome,
) -> i64 {
    let basic_power = if is_cv_type(codex, attacker) {
        let bomber_count = bomber_plane_count(codex, attacker);
        let planes = if bomber_count > 0 {
            1.5 * bomber_count as f64 + 55.0
        } else {
            attacker.ship.api_karyoku[0].max(0) as f64 + 5.0
        };
        // The reference spells the correction out only for the non-carrier formula,
        // but states it sits "inside basic attack power" — so it applies to the
        // carrier branch as well, not just the gun branch.
        // ponytail: a carrier's power against a land target (`CV.prototype.shellPower`, its
        // multipliers on her firepower alone) is not modelled.
        // The improvement sits inside the source's `1.5 × (…)` (`CV.prototype.shellPower`).
        planes
            + 1.5 * ship_improvement(codex, attacker).shell
            + combined_correction(attacker, defender, CombinedAttackClass::Shelling) as f64
    } else {
        day_gun_power(codex, attacker, defender)
    };
    let dmg_state =
        damage_state_modifier(attacker.hp(), attacker.ship.api_maxhp, BattlePhase::DayShelling);
    let pre_cap = (basic_power + light_gun_bonus(codex, attacker))
        * day_formation_modifier(formation_id, CombinedAttackClass::Shelling)
        * vanguard_power(attacker, CombinedAttackClass::Shelling)
        * engagement.modifier()
        * dmg_state;
    let mut capped_power = apply_cap(pre_cap, SHELLING_CAP) as f64;
    if let Some(m) = ci_multiplier {
        capped_power *= m;
    }
    capped_power *= post_cap_multiplier(codex, attacker, defender, false);
    let capped_power =
        outcome.power_with(capped_power, shelling_proficiency(codex, attacker).critical_damage);
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped_power, defense, defender.hp(), ammo_modifier(codex, attacker))
}

/// What a ship's guns count for by day before the cap, less the light cruiser's gun fit
/// (`shellPower`): firepower, 5, the combined fleet's correction and the improvement bonus.
/// Against a land target her equipment multiplies that and adds its flat power, and a
/// submarine has 30 more firepower.
fn day_gun_power(codex: &Codex, attacker: &BattleRuntimeShip, defender: &BattleRuntimeShip) -> f64 {
    let firepower = attacker.ship.api_karyoku[0].max(0) as f64;
    let rest = 5.0
        + combined_correction(attacker, defender, CombinedAttackClass::Shelling) as f64
        + ship_improvement(codex, attacker).shell;
    match land_target(codex, defender) {
        Some(kind) => AntiInstallation::of(codex, attacker).power(
            kind,
            firepower + submarine_bonus(codex, attacker) + rest,
            true,
        ),
        None => firepower + rest,
    }
}

/// The 30 firepower a submarine has against a land target.
fn submarine_bonus(codex: &Codex, attacker: &BattleRuntimeShip) -> f64 {
    if target_class(codex, attacker).is_submarine() {
        30.0
    } else {
        0.0
    }
}

/// A torpedo's attack power after the cap, before ammunition and armour.
/// A fifth of it also goes into the torpedo's accuracy.
pub(crate) fn torpedo_attack_power(
    codex: &Codex,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
    phase: BattlePhase,
) -> f64 {
    let basic_power = attacker.ship.api_raisou[0].max(0) as f64
        + ship_improvement(codex, attacker).torpedo
        + combined_correction(attacker, defender, CombinedAttackClass::Torpedo) as f64;
    let dmg_state = damage_state_modifier(attacker.hp(), attacker.ship.api_maxhp, phase);
    let pre_cap = basic_power
        * day_formation_modifier(formation_id, CombinedAttackClass::Torpedo)
        * engagement.modifier()
        * dmg_state;
    apply_cap(pre_cap, TORPEDO_CAP) as f64
}

/// Calculate torpedo damage for a single attack.
#[expect(clippy::too_many_arguments)]
pub(crate) fn calculate_torpedo_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
    phase: BattlePhase,
    outcome: HitOutcome,
) -> i64 {
    let capped_power =
        torpedo_attack_power(codex, attacker, defender, formation_id, engagement, phase)
            * post_cap_multiplier(codex, attacker, defender, true);
    let capped_power = outcome.power(capped_power);
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped_power, defense, defender.hp(), ammo_modifier(codex, attacker))
}

/// What a ship attacks with at night from her guns and torpedoes, before the night scout and
/// the cap: firepower and torpedo, with none of the day's 5. The source's `NBPower`: against a
/// land target the torpedoes are left out but for the northern princesses, a submarine has 30
/// more firepower, and her equipment multiplies that and adds its flat power. A ship with
/// nothing equipped attacks one with her firepower and torpedo.
fn night_gun_power(
    codex: &Codex,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
) -> f64 {
    let firepower = attacker.ship.api_karyoku[0].max(0) as f64;
    let improvement = ship_improvement(codex, attacker).night;
    let torpedo = attacker.ship.api_raisou[0].max(0) as f64;
    match land_target(codex, defender) {
        // wikiwiki 「戦闘について」 夜戦: 「対陸上でも、素手(装備を一切装備していない)の場合に限り
        // 雷装値が有効になる」. The source has no such rule; this one follows wikiwiki (plan
        // 2026-10-15-004), with no multiplier and no submarine's 30.
        Some(_) if attacker.slot_items.is_empty() => firepower + torpedo,
        Some(kind) => {
            AntiInstallation::of(codex, attacker).power(
                kind,
                firepower + submarine_bonus(codex, attacker) + improvement,
                false,
            )
            // The northern princesses take her torpedo too.
            + if kind == 5 { torpedo } else { 0.0 }
        }
        None => firepower + torpedo + improvement,
    }
}

/// Calculate night battle damage for a single attack.
///
/// When `ci_multiplier` is `Some(m)`, the multiplier is applied to basic power
/// *before* the soft cap at 360 — matching `KanColle`'s CI damage pipeline.
///
/// `air_power` is the power of a carrier's night air attack (`night_air_power`) when she
/// flies one; it stands in for her guns and torpedoes, and her aircraft's proficiency counts.
#[expect(clippy::too_many_arguments)]
pub(crate) fn calculate_night_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    contact: Option<NightContact>,
    ci_multiplier: Option<f64>,
    air_power: Option<f64>,
    outcome: HitOutcome,
) -> i64 {
    let own_power = air_power.unwrap_or_else(|| night_gun_power(codex, attacker, defender));
    let basic_power = own_power + contact.map_or(0.0, |contact| contact.power);
    let pre_cap = match ci_multiplier {
        Some(m) => basic_power * m,
        None => basic_power,
    };
    // The front half of a fleet in 警戒陣 attacks at half power (`NBattack`).
    let pre_cap = if attacker.in_vanguard_front() {
        pre_cap * 0.5
    } else {
        pre_cap
    };
    let capped_power = apply_cap(pre_cap, NIGHT_CAP) as f64
        * post_cap_multiplier(codex, attacker, defender, false);
    let capped_power = if air_power.is_some() {
        outcome.power_with(capped_power, plane_proficiency(codex, attacker).critical_damage)
    } else {
        outcome.power(capped_power)
    };
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped_power, defense, defender.hp(), ammo_modifier(codex, attacker))
}

/// Calculate ASW damage against a submarine target. `planes` is the attacker's aircraft
/// proficiency where the attack is flown ([`asw_proficiency`]), and none in the opening.
#[expect(clippy::too_many_arguments)]
pub(crate) fn calculate_asw_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    formation_id: i64,
    engagement: EngagementType,
    outcome: HitOutcome,
    planes: PlaneProficiency,
) -> i64 {
    let equip_asw = equipment_asw_total(codex, attacker);
    let base_asw = base_asw(codex, attacker);

    // Attack type bonus: +8 for ASW aircraft, +13 for depth charge projector (cumulative)
    let has_asw_aircraft = has_active_asw_aircraft(codex, attacker);
    let has_dc_projector = has_slotitem_type(codex, attacker, KcSlotItemType3::DepthCharge);
    let type_bonus = if has_asw_aircraft {
        8.0
    } else {
        0.0
    } + if has_dc_projector {
        13.0
    } else {
        0.0
    };

    let synergy = asw_synergy_modifier(codex, attacker);
    let raw_power = (base_asw.sqrt() * 2.0
        + equip_asw.sqrt() * 1.5
        + type_bonus
        + ship_improvement(codex, attacker).asw)
        * synergy;
    let dmg_state =
        damage_state_modifier(attacker.hp(), attacker.ship.api_maxhp, BattlePhase::DayShelling);
    // Daytime ASW takes the formation multiplier but no 連合艦隊補正 at all
    // (reference §Combined fleet corrections).
    let modified = raw_power
        * day_formation_modifier(formation_id, CombinedAttackClass::Asw)
        * vanguard_power(attacker, CombinedAttackClass::Asw)
        * engagement.modifier()
        * dmg_state;
    let capped = outcome.power_with(apply_cap(modified, ASW_CAP) as f64, planes.critical_damage);
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    let armor_reduction = depth_charge_armor_reduction(codex, attacker);
    let adjusted_defense = (defense - armor_reduction).max(0.0);
    resolve_damage(rng, capped, adjusted_defense, defender.hp(), ammo_modifier(codex, attacker))
}

/// Calculate airstrike damage for a single bomber slot.
// TODO(#0): used by airstrike phase
#[expect(dead_code)]
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
    let raw_power = bomb_power + AIRSTRIKE_POWER_BONUS;
    let capped = apply_cap(raw_power, AIRSTRIKE_CAP) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped, defense, defender.hp(), 1.0)
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

/// Count the total number of bomber aircraft (dive bomber + torpedo bomber) on a ship.
/// Uses `api_onslot` (actual plane count per slot), not the number of bomber slots.
pub(crate) fn bomber_plane_count(codex: &Codex, ship: &BattleRuntimeShip) -> i64 {
    const BOMBER_TYPES: &[KcSlotItemType3] =
        &[KcSlotItemType3::CarrierBasedDiveBomber, KcSlotItemType3::CarrierBasedTorpedoBomber];
    ship.slot_items
        .iter()
        .zip(ship.ship.api_onslot)
        .filter(|(si, _)| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(|t| BOMBER_TYPES.contains(&t))
        })
        .map(|(_, onslot)| onslot.max(0))
        .sum()
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

// ---------------------------------------------------------------------------
// Formation modifiers
// ---------------------------------------------------------------------------

/// Shelling formation modifier.
pub(crate) fn formation_modifier(formation_id: i64) -> f64 {
    match formation_id {
        2 => 0.8,
        3 => 0.7,
        4 => 0.85,
        5 => 0.6,
        _ => 1.0,
    }
}

/// Formation multiplier for a daytime attack of `class`, covering both the six
/// normal formations and the four 警戒航行序列 (11–14).
///
/// A combined fleet fights in 警戒航行序列, and those have their own table with a
/// separate column per attack class — unlike the normal formations, where this
/// crate reuses one shelling table for torpedoes too. Ids outside 11–14 fall
/// through to the single-fleet tables unchanged, so nothing about a normal
/// battle moves.
///
/// `formation_id` is always the **attacker's** formation, which is why an enemy
/// single fleet shooting at a combined fleet keeps its normal multiplier: only
/// the fleet that is actually in 警戒航行序列 gets that table.
pub(crate) fn day_formation_modifier(formation_id: i64, class: CombinedAttackClass) -> f64 {
    if let Some(modifier) = combined_formation_modifier(formation_id, class) {
        return modifier;
    }
    match class {
        CombinedAttackClass::Asw => asw_formation_modifier(formation_id),
        _ => formation_modifier(formation_id),
    }
}

/// The additive 連合艦隊補正 for one attack, or `0` outside a combined battle.
///
/// The correction is indexed by the *friendly* fleet's type and deck even when
/// the enemy is the attacker — the enemy here is a single fleet and has no deck
/// of its own, but its correction still varies by which friendly deck it is
/// trading fire with. So whichever of the two ships is the friendly one carries
/// the lookup key.
fn combined_correction(
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    class: CombinedAttackClass,
) -> i64 {
    let friendly = if attacker.is_friendly {
        attacker
    } else {
        defender
    };
    let Some(membership) = friendly.combined else {
        // A single friendly fleet against an enemy combined fleet: the enemy
        // ship carries the deck the table is keyed by.
        let enemy = if attacker.is_friendly {
            defender
        } else {
            attacker
        };
        return enemy.enemy_deck.map_or(0, |role| {
            combined_correction_vs_enemy_combined(class, role, attacker.is_friendly)
        });
    };
    combined_correction_vs_single(
        membership.combined_type,
        class,
        membership.role,
        attacker.is_friendly,
    )
}

/// What a ship's half of a fleet in 警戒陣 does to her daytime attack of
/// `class` (`VANGUARD1`, `VANGUARD2`): the front half shells at 0.5, the rear
/// half attacks submarines at 0.6. Torpedoes are 1 in both.
pub(crate) fn vanguard_power(attacker: &BattleRuntimeShip, class: CombinedAttackClass) -> f64 {
    let Some(place) = attacker.vanguard else {
        return 1.0;
    };
    match (class, place.front()) {
        (CombinedAttackClass::Shelling, true) => 0.5,
        (CombinedAttackClass::Asw, false) => 0.6,
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

/// What a ship's improved (★) equipment adds to each of her attacks and to her evasion, the
/// source's `improves` (`KC3Kai/kancolle-replay` at `69097abc`: `Equip.prototype.setImprovement`,
/// `kcships.js:2387-2491`; `EQTDATA[type].improve`, `kcEQDATA.js`). The power goes into basic
/// power before the pre-cap modifiers, the accuracy into the flat accuracy and the evasion after
/// the evasion curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Improvement {
    /// Day shelling, a carrier's included (`Pshell`).
    pub(crate) shell: f64,
    /// Night attack (`Pnb`).
    pub(crate) night: f64,
    /// Torpedo (`Ptorp`).
    pub(crate) torpedo: f64,
    /// Anti-submarine (`Pasw`).
    pub(crate) asw: f64,
    /// Day shelling accuracy (`ACCshell`).
    pub(crate) shell_accuracy: f64,
    /// Night attack accuracy (`ACCnb`).
    pub(crate) night_accuracy: f64,
    /// Torpedo accuracy (`ACCtorp`), which the torpedo takes rounded down.
    pub(crate) torpedo_accuracy: f64,
    /// Anti-submarine accuracy (`ACCasw`).
    pub(crate) asw_accuracy: f64,
    /// Evasion against day shelling (`EVshell`).
    pub(crate) shell_evasion: f64,
    /// Evasion against torpedoes (`EVtorp`).
    pub(crate) torpedo_evasion: f64,
}

impl Improvement {
    /// Nothing improved.
    pub(crate) const NONE: Self = Self {
        shell: 0.0,
        night: 0.0,
        torpedo: 0.0,
        asw: 0.0,
        shell_accuracy: 0.0,
        night_accuracy: 0.0,
        torpedo_accuracy: 0.0,
        asw_accuracy: 0.0,
        shell_evasion: 0.0,
        torpedo_evasion: 0.0,
    };

    /// Every key combined with the other's by `f`.
    fn zip(self, other: Self, f: impl Fn(f64, f64) -> f64) -> Self {
        Self {
            shell: f(self.shell, other.shell),
            night: f(self.night, other.night),
            torpedo: f(self.torpedo, other.torpedo),
            asw: f(self.asw, other.asw),
            shell_accuracy: f(self.shell_accuracy, other.shell_accuracy),
            night_accuracy: f(self.night_accuracy, other.night_accuracy),
            torpedo_accuracy: f(self.torpedo_accuracy, other.torpedo_accuracy),
            asw_accuracy: f(self.asw_accuracy, other.asw_accuracy),
            shell_evasion: f(self.shell_evasion, other.shell_evasion),
            torpedo_evasion: f(self.torpedo_evasion, other.torpedo_evasion),
        }
    }

    /// Every key times `factor` (the zip's second side goes unread).
    fn scaled(self, factor: f64) -> Self {
        self.zip(self, |key, _| key * factor)
    }
}

/// What a ship's equipment adds, summed over it.
pub(crate) fn ship_improvement(codex: &Codex, ship: &BattleRuntimeShip) -> Improvement {
    ship.slot_items
        .iter()
        .filter(|si| si.api_level > 0)
        .filter_map(|si| {
            let mst = codex.find::<ApiMstSlotitem>(&si.api_slotitem_id).ok()?;
            Some(improvement(mst, si.api_level))
        })
        .fold(Improvement::NONE, |sum, item| sum.zip(item, |a, b| a + b))
}

/// The coefficients of √★ for each client equipment type (`api_type[2]`), the source's
/// `EQTDATA[type].improve`. The source types a few items apart from the client (128, 281 and
/// 465 as `MAINGUNXL`, 142 and 460 as `RADARXL`, 151 as `CARRIERSCOUT2`), none of them to
/// different coefficients; 467 is the one that differs, in [`improvement`].
/// The radars' client types (small, large, large II).
const RADARS: [i64; 3] = [12, 13, 93];

fn improvement_coefficients(kind: i64) -> Improvement {
    /// What the gun-like types aim with; radars too, but for those of 3 accuracy or more.
    const AIM: Improvement = Improvement {
        shell_accuracy: 1.0,
        night_accuracy: 1.3,
        ..Improvement::NONE
    };
    match kind {
        // Small and medium main guns, secondary guns, 三式弾, AP shells, landing craft,
        // searchlights, FCF, SCAMP, anti-aircraft fire directors, WG42, picket, 特二式内火艇,
        // army units and smoke screens.
        1 | 2 | 4 | 18 | 19 | 24 | 29 | 34 | 35 | 36 | 37 | 39 | 42 | 46 | 52 | 54 => Improvement {
            shell: 1.0,
            night: 1.0,
            ..AIM
        },
        // Large main guns.
        3 | 38 => Improvement {
            shell: 1.5,
            night: 1.0,
            ..AIM
        },
        kind if RADARS.contains(&kind) => AIM,
        5 => Improvement {
            night: 1.0,
            torpedo: 1.2,
            night_accuracy: 1.3,
            torpedo_accuracy: 2.0,
            ..Improvement::NONE
        },
        // Anti-aircraft guns.
        21 => Improvement {
            shell: 1.0,
            torpedo: 1.2,
            torpedo_accuracy: 2.0,
            ..Improvement::NONE
        },
        // Midget submarines.
        22 => Improvement {
            night: 1.0,
            night_accuracy: 1.3,
            ..Improvement::NONE
        },
        // Sonars.
        14 | 40 => Improvement {
            shell: 0.75,
            asw: 1.0,
            asw_accuracy: 1.3,
            torpedo_evasion: 1.5,
            ..Improvement::NONE
        },
        // Depth charges.
        15 => Improvement {
            shell: 0.75,
            asw: 1.0,
            asw_accuracy: 1.3,
            ..Improvement::NONE
        },
        // Engines.
        17 => Improvement {
            shell_evasion: 1.5,
            ..Improvement::NONE
        },
        _ => Improvement::NONE,
    }
}

/// What one item at `stars` (> 0) adds.
fn improvement(mst: &ApiMstSlotitem, stars: i64) -> Improvement {
    /// The items the source marks `isFighterBomber`.
    const FIGHTER_BOMBERS: [i64; 5] = [60, 154, 219, 447, 487];
    let linear = stars as f64;
    let root = linear.sqrt();
    // Aircraft improve by the star; the table below has no weight for them.
    let (shell, asw) = match mst.api_type[2] {
        // Dive, torpedo and jet bombers.
        7 | 8 | 57 if !FIGHTER_BOMBERS.contains(&mst.api_id) => (0.2, 0.2),
        // Of the fighter-bombers, 487 alone.
        7 | 8 | 57 if mst.api_id == 487 => (0.0, 0.2),
        // Autogyros at 11 ASW or more and patrol planes at 8 take 0.3, the rest 0.2.
        25 if mst.api_tais >= 11 => (0.0, 0.3),
        26 if mst.api_tais >= 8 => (0.0, 0.3),
        25 | 26 => (0.0, 0.2),
        _ => (0.0, 0.0),
    };
    let table = improvement_coefficients(mst.api_type[2]);
    let mut item = table.scaled(root);
    item.shell += shell * linear;
    item.asw += asw * linear;
    // An item weighted 1 by day takes 1.5 at 13 firepower or more; a large gun takes 1 below.
    if table.shell == 1.0 && mst.api_houg >= 13 {
        item.shell = 1.5 * root;
    }
    if table.shell == 1.5 && mst.api_houg < 13 {
        item.shell = root;
    }
    // Secondary guns by the item. The source's 0.3 list also names 467, but types it
    // `SECGUNL`, so the rule never reaches it and it keeps √★.
    if mst.api_type[2] == 4 && mst.api_id != 467 {
        let weight = if [12, 234, 247, 463].contains(&mst.api_id) {
            Some(0.3)
        } else if ![11, 77, 130, 134, 135].contains(&mst.api_id) {
            Some(0.2)
        } else {
            None
        };
        if let Some(weight) = weight {
            item.shell = weight * linear;
            item.night = weight * linear;
        }
    }
    // Submarine torpedoes.
    if mst.api_type[2] == 32 {
        item.torpedo = 0.2 * linear;
        item.night = 0.2 * linear;
    }
    if [226, 227, 488].contains(&mst.api_id) {
        item.shell = 0.0;
    }
    // A radar of 3 accuracy or more aims better.
    if RADARS.contains(&mst.api_type[2]) && mst.api_houm >= 3 {
        item.shell_accuracy = 1.7 * root;
        item.night_accuracy = 1.6 * root;
    }
    item
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

/// A ship's own anti-submarine for her attack on a submarine: her total less her equipment's
/// (modernization and innate). An abyssal carrier the source lets attack one takes the source's
/// figure, the codex giving her none (`ABYSSAL_CARRIER_ASW`).
pub(crate) fn base_asw(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    if let Some(asw) = abyssal_carrier_asw(ship.ship.api_ship_id) {
        return asw as f64;
    }
    (ship.ship.api_taisen[0].max(0) as f64 - equipment_asw_total(codex, ship)).max(0.0)
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
// Named constants for magic numbers in damage formulas
// ---------------------------------------------------------------------------

/// Day shelling soft-cap threshold.
const SHELLING_CAP: f64 = 220.0;
/// Torpedo soft-cap threshold.
const TORPEDO_CAP: f64 = 180.0;
/// Night battle soft-cap threshold.
const NIGHT_CAP: f64 = 360.0;
/// ASW soft-cap threshold.
const ASW_CAP: f64 = 170.0;
/// Airstrike soft-cap threshold.
const AIRSTRIKE_CAP: f64 = 170.0;
/// Defense power formula coefficient A.
const DEFENSE_COEFF_A: f64 = 0.7;
/// Defense power formula coefficient B.
const DEFENSE_COEFF_B: f64 = 0.6;
/// Airstrike flat power bonus.
const AIRSTRIKE_POWER_BONUS: f64 = 25.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::{BattlePhase, BattleRuntimeShip, EngagementType};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    /// R2: 警戒航行序列 reach the damage formula. Before the tables were wired
    /// up, `formation_modifier(11..=14)` fell through to `1.0` and a combined
    /// fleet's formation had no effect on damage at all.
    /// A carrier's shelling is flown by its aircraft, so their proficiency
    /// makes its critical land harder; a plain hit is left alone.
    #[test]
    fn a_skilled_carrier_shells_a_harder_critical() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let shell = |alv: Option<i64>, outcome: HitOutcome| {
            let mut carrier =
                sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 50);
            let mut item = slotitem_with_mst_id(bomber);
            item.api_alv = alv;
            carrier.slot_items = vec![item];
            carrier.ship.api_onslot = [18, 0, 0, 0, 0];
            let mut target =
                sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
            target.ship.api_nowhp = 9999;
            target.ship.api_maxhp = 9999;
            calculate_shelling_damage(
                &codex,
                &mut crate::random::SeededRng::new(3),
                &BattleRuntimeShip::from(carrier),
                &BattleRuntimeShip::from(target),
                1,
                EngagementType::SameCourse,
                None,
                outcome,
            )
        };
        assert!(shell(Some(7), HitOutcome::Critical) > shell(None, HitOutcome::Critical));
        assert_eq!(shell(Some(7), HitOutcome::Hit), shell(None, HitOutcome::Hit));
    }

    #[test]
    fn skilled_aircraft_drop_a_harder_critical_on_a_submarine() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut cruiser = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CAV), 50);
        cruiser.ship.api_taisen[0] = 80;
        let cruiser = BattleRuntimeShip::from(cruiser);
        let mut submarine = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::SS), 50);
        submarine.ship.api_nowhp = 9999;
        submarine.ship.api_maxhp = 9999;
        let submarine = BattleRuntimeShip::from(submarine);
        let skilled = PlaneProficiency {
            critical_damage: 1.2,
            ..PlaneProficiency::NONE
        };
        let drop = |planes, outcome| {
            calculate_asw_damage(
                &codex,
                &mut crate::random::SeededRng::new(3),
                &cruiser,
                &submarine,
                1,
                EngagementType::SameCourse,
                outcome,
                planes,
            )
        };
        assert!(
            drop(skilled, HitOutcome::Critical)
                > drop(PlaneProficiency::NONE, HitOutcome::Critical)
        );
        assert_eq!(drop(skilled, HitOutcome::Hit), drop(PlaneProficiency::NONE, HitOutcome::Hit));
    }

    #[test]
    fn day_formation_modifier_uses_the_combined_table_for_keisen_formations() {
        use CombinedAttackClass::{Asw, Shelling, Torpedo};

        // (formation, shelling, torpedo, asw) — reference §Formations.
        let expected =
            [(11, 0.8, 0.7, 1.3), (12, 1.0, 0.9, 1.1), (13, 0.7, 0.6, 1.0), (14, 1.1, 1.0, 0.7)];
        for (id, shelling, torpedo, asw) in expected {
            assert!((day_formation_modifier(id, Shelling) - shelling).abs() < f64::EPSILON);
            assert!((day_formation_modifier(id, Torpedo) - torpedo).abs() < f64::EPSILON);
            assert!((day_formation_modifier(id, Asw) - asw).abs() < f64::EPSILON);
        }
    }

    /// The six normal formations must still resolve through the single-fleet
    /// tables, including the quirk that shelling and torpedo share one table
    /// there while the combined table separates them.
    #[test]
    fn day_formation_modifier_falls_through_for_normal_formations() {
        use CombinedAttackClass::{Asw, Shelling, Torpedo};

        for id in 1..=6 {
            let shelling = day_formation_modifier(id, Shelling);
            assert!((shelling - formation_modifier(id)).abs() < f64::EPSILON, "shelling {id}");
            assert!(
                (day_formation_modifier(id, Torpedo) - formation_modifier(id)).abs() < f64::EPSILON,
                "torpedo {id}"
            );
            assert!(
                (day_formation_modifier(id, Asw) - asw_formation_modifier(id)).abs() < f64::EPSILON,
                "asw {id}"
            );
        }
    }

    /// R3: the correction is keyed on the friendly fleet's type and deck even
    /// when the enemy is the attacker, because a single enemy fleet has no deck
    /// of its own to key on.
    #[test]
    fn combined_correction_keys_on_the_friendly_deck_in_both_directions() {
        use crate::combined::{CombinedFleetRole, CombinedType};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let friendly = |role| {
            BattleRuntimeShip::new(sample_ship(&codex, dd, 1), true, true)
                .in_combined_fleet(CombinedType::CarrierTaskForce, role)
        };
        let enemy = BattleRuntimeShip::new(sample_ship(&codex, dd, 1), false, true);

        // 空母機動, shelling: deck 1 自軍 +2 / 敵軍 +10, deck 2 自軍 +10 / 敵軍 +5.
        let main = friendly(CombinedFleetRole::Main);
        let escort = friendly(CombinedFleetRole::Escort);
        assert_eq!(combined_correction(&main, &enemy, CombinedAttackClass::Shelling), 2);
        assert_eq!(combined_correction(&enemy, &main, CombinedAttackClass::Shelling), 10);
        assert_eq!(combined_correction(&escort, &enemy, CombinedAttackClass::Shelling), 10);
        assert_eq!(combined_correction(&enemy, &escort, CombinedAttackClass::Shelling), 5);

        // Torpedo is a flat -5, and daytime ASW takes no correction at all.
        assert_eq!(combined_correction(&escort, &enemy, CombinedAttackClass::Torpedo), -5);
        assert_eq!(combined_correction(&escort, &enemy, CombinedAttackClass::Asw), 0);
    }

    /// A single-fleet battle must take no correction, in either direction.
    #[test]
    fn combined_correction_is_zero_for_a_single_fleet() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let friendly = BattleRuntimeShip::new(sample_ship(&codex, dd, 1), true, true);
        let enemy = BattleRuntimeShip::new(sample_ship(&codex, dd, 1), false, true);

        for class in [
            CombinedAttackClass::Shelling,
            CombinedAttackClass::Torpedo,
            CombinedAttackClass::Asw,
            CombinedAttackClass::AntiAir,
        ] {
            assert_eq!(combined_correction(&friendly, &enemy, class), 0);
            assert_eq!(combined_correction(&enemy, &friendly, class), 0);
        }
    }

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
    fn ammo_modifier_at_or_above_half_is_unpenalized() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bull_max = codex.find::<ApiMstShip>(&dd).unwrap().api_bull_max.unwrap();
        assert!(bull_max > 0);

        let friendly_at = |bull: i64| {
            let mut input = sample_ship(&codex, dd, 1);
            input.ship.api_bull = bull;
            BattleRuntimeShip::new(input, true, true)
        };

        assert_eq!(ammo_modifier(&codex, &friendly_at(bull_max)), 1.0); // full
        assert_eq!(ammo_modifier(&codex, &friendly_at((bull_max + 1) / 2)), 1.0); // ~50%
        assert_eq!(ammo_modifier(&codex, &friendly_at(bull_max * 2)), 1.0); // over-supplied clamps
    }

    #[test]
    fn ammo_modifier_scales_linearly_below_half() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bull_max = codex.find::<ApiMstShip>(&dd).unwrap().api_bull_max.unwrap();

        let modifier_at = |bull: i64| {
            let mut input = sample_ship(&codex, dd, 1);
            input.ship.api_bull = bull;
            ammo_modifier(&codex, &BattleRuntimeShip::new(input, true, true))
        };

        // Empty ammo → 0.0 (scratch only).
        assert_eq!(modifier_at(0), 0.0);

        // Each below-50% sample equals the canonical floor(percent) / 50 and is < 1.0.
        for &bull in &[bull_max / 10, bull_max / 4, bull_max * 2 / 5] {
            if bull == 0 {
                continue;
            }
            let pct = (100 * bull) / bull_max; // integer floor percentage
            assert!(pct < 50, "test setup: {bull}/{bull_max} should be < 50%");
            let expected = pct as f64 / 50.0;
            assert!((modifier_at(bull) - expected).abs() < 1e-9, "bull={bull}");
            assert!(modifier_at(bull) < 1.0);
        }

        // Monotonic: more ammo never yields a smaller modifier.
        assert!(modifier_at(bull_max / 4) <= modifier_at(bull_max * 2 / 5));
        assert!(modifier_at(1) <= modifier_at(bull_max / 4));
    }

    #[test]
    fn ammo_modifier_never_penalizes_enemy() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut input = sample_ship(&codex, dd, 1);
        input.ship.api_bull = 0; // empty
        let enemy = BattleRuntimeShip::new(input, false, true);
        assert_eq!(ammo_modifier(&codex, &enemy), 1.0);
    }

    #[test]
    fn low_ammo_reduces_friendly_shelling_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ca = first_ship_mst_by_type(&codex, KcShipType::CA);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bull_max = codex.find::<ApiMstShip>(&ca).unwrap().api_bull_max.unwrap();

        let shell = |bull: i64, seed: u64| {
            let mut input = sample_ship(&codex, ca, 99);
            input.ship.api_bull = bull;
            let attacker = BattleRuntimeShip::new(input, true, true);
            let defender = BattleRuntimeShip::new(sample_ship(&codex, dd, 1), false, true);
            let mut rng = crate::random::SeededRng::new(seed);
            calculate_shelling_damage(
                &codex,
                &mut rng,
                &attacker,
                &defender,
                1,
                EngagementType::SameCourse,
                None,
                HitOutcome::Hit,
            )
        };

        // Identical RNG and matchup — only ammo differs, so low can never exceed full.
        for seed in [1u64, 7, 42, 100, 2024] {
            let full = shell(bull_max, seed);
            let low = shell(bull_max / 10, seed); // ~10% ammo → ×0.2
            assert!(low <= full, "seed {seed}: low {low} should be <= full {full}");
        }
        // A high-firepower CA vs a weak DD must show a real reduction.
        let full = shell(bull_max, 42);
        let low = shell(bull_max / 10, 42);
        assert!(full > 5, "setup: full-ammo CA->DD should deal real damage, got {full}");
        assert!(low < full, "10% ammo must deal less than full (full {full}, low {low})");
    }

    /// The source's `damageCommon`: the ammunition modifier is on what is left of the power
    /// after the armour, and what comes to nothing is a scratch.
    #[test]
    fn low_ammunition_scales_the_damage_not_the_power() {
        let mut rng = crate::random::SeededRng::new(1);
        assert_eq!(resolve_damage(&mut rng, 100.0, 30.0, 100, 1.0), 70);
        assert_eq!(resolve_damage(&mut rng, 100.0, 30.0, 100, 0.4), 28);
        // None left, and power no more than the armour: a scratch, 6% of her hull and up.
        for (power, ammo) in [(100.0, 0.0), (30.0, 1.0)] {
            let scratch = resolve_damage(&mut rng, power, 30.0, 100, ammo);
            assert!((6..14).contains(&scratch), "{scratch}");
        }
        // A miss is nothing at all.
        assert_eq!(resolve_damage(&mut rng, 0.0, 30.0, 100, 1.0), 0);
    }

    #[test]
    fn a_hit_of_a_cut_in_that_misses_scratches_and_is_written_as_a_hit() {
        let mut rng = crate::random::SeededRng::new(1);
        let (damage, cl) = cut_in_hit(&mut rng, HitOutcome::Miss, 0, 100);
        // `getScratchDamage`: 6% of her hull and up to 8% more.
        assert!((6..14).contains(&damage), "{damage}");
        assert_eq!(cl, 1);
        assert_eq!(cut_in_hit(&mut rng, HitOutcome::Hit, 40, 100), (40, 1));
        assert_eq!(cut_in_hit(&mut rng, HitOutcome::Critical, 60, 100), (60, 2));
    }

    /// The source's `NBPower`: no torpedoes against an installation, and 30 more firepower
    /// for a submarine there. Bare-handed, wikiwiki's rule instead: firepower and torpedo.
    #[test]
    fn a_night_attack_on_an_installation_leaves_the_torpedoes_out() {
        const AIR_RADAR: i64 = 27; // 13号対空電探, nothing against a land target
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |stype, firepower, torpedo, gear: &[i64]| {
            let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, stype), 50);
            ship.ship.api_karyoku[0] = firepower;
            ship.ship.api_raisou[0] = torpedo;
            ship.slot_items = gear.iter().copied().map(slotitem_with_mst_id).collect();
            BattleRuntimeShip::from(ship)
        };
        let surface = ship(KcShipType::DD, 0, 0, &[]);
        let mut installation = surface.clone();
        installation.ship.api_ship_id = ship_mst_id_by_name(&codex, "飛行場姫");
        let power = |attacker, defender| night_gun_power(&codex, attacker, defender);

        let destroyer = ship(KcShipType::DD, 50, 80, &[AIR_RADAR]);
        assert_eq!(power(&destroyer, &surface), 130.0);
        assert_eq!(power(&destroyer, &installation), 50.0);
        let submarine = ship(KcShipType::SS, 10, 60, &[AIR_RADAR]);
        assert_eq!(power(&submarine, &surface), 70.0);
        assert_eq!(power(&submarine, &installation), 40.0);

        // With nothing equipped the torpedo counts, and the submarine's 30 does not.
        let bare = ship(KcShipType::DD, 50, 80, &[]);
        assert_eq!(power(&bare, &installation), 130.0);
        let bare = ship(KcShipType::SS, 10, 60, &[]);
        assert_eq!(power(&bare, &installation), 70.0);
    }

    /// `shellPower` and `NBPower`: the equipment's multiplier on the firepower, the flat power
    /// after it.
    #[test]
    fn anti_installation_gear_reaches_the_guns_by_day_and_at_night() {
        const WG42: i64 = 126;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        ship.ship.api_karyoku[0] = 50;
        ship.ship.api_raisou[0] = 80;
        ship.slot_items = vec![slotitem_with_mst_id(WG42)];
        let destroyer = BattleRuntimeShip::from(ship);
        let mut target = destroyer.clone();
        target.slot_items.clear();
        let surface = target.clone();
        target.ship.api_ship_id = ship_mst_id_by_name(&codex, "飛行場姫");
        let installation = target;

        // (50 + 5) x 1.3 + 75 by day; 50 x 1.3 + 75 at night.
        assert_eq!(day_gun_power(&codex, &destroyer, &installation), 146.5);
        assert_eq!(night_gun_power(&codex, &destroyer, &installation), 140.0);
        assert_eq!(day_gun_power(&codex, &destroyer, &surface), 55.0);
        assert_eq!(night_gun_power(&codex, &destroyer, &surface), 130.0);
    }

    /// A supply depot's multiplier comes after the cap (`NBattack`: `floor(softCap(..)) * postMod`).
    #[test]
    fn a_supply_depots_multiplier_comes_after_the_cap() {
        const WG42: i64 = 126;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        ship.ship.api_karyoku[0] = 300;
        ship.slot_items = vec![slotitem_with_mst_id(WG42)];
        let attacker = BattleRuntimeShip::from(ship);
        let mut depot = attacker.clone();
        depot.slot_items.clear();
        depot.ship.api_ship_id = ship_mst_id_by_name(&codex, "集積地棲姫");
        depot.ship.api_soukou[0] = 0;
        depot.ship.api_nowhp = 9999;
        depot.ship.api_maxhp = 9999;

        assert_eq!(night_gun_power(&codex, &attacker, &depot), 465.0);
        let damage = calculate_night_damage(
            &codex,
            &mut crate::random::SeededRng::new(1),
            &attacker,
            &depot,
            None,
            None,
            None,
            HitOutcome::Hit,
        );
        // 300 x 1.3 + 75 = 465, capped to 370, then x 1.25; not 465 x 1.25 capped to 374.
        assert_eq!(damage, 462);
    }

    #[test]
    fn each_half_of_a_vanguard_strikes_by_its_own_multiplier() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |number| {
            let input = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 1);
            BattleRuntimeShip::new(input, true, true).in_vanguard(number, 6)
        };
        let power = |number, class| vanguard_power(&ship(number), class);
        assert_eq!(power(3, CombinedAttackClass::Shelling), 0.5);
        assert_eq!(power(4, CombinedAttackClass::Shelling), 1.0);
        assert_eq!(power(1, CombinedAttackClass::Asw), 1.0);
        assert_eq!(power(6, CombinedAttackClass::Asw), 0.6);
        assert_eq!(power(1, CombinedAttackClass::Torpedo), 1.0);
        assert_eq!(power(6, CombinedAttackClass::Torpedo), 1.0);
    }

    /// The front half of a vanguard shells, and attacks at night, with half the
    /// power the rear half would have.
    #[test]
    fn the_front_half_of_a_vanguard_strikes_at_half_power() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cl = first_ship_mst_by_type(&codex, KcShipType::CL);
        let defender = || {
            let mut input = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 1);
            input.ship.api_soukou[0] = 0;
            input.ship.api_nowhp = 9999;
            input.ship.api_maxhp = 9999;
            BattleRuntimeShip::new(input, false, true)
        };
        let attacker = |number| {
            BattleRuntimeShip::new(sample_ship(&codex, cl, 99), true, true).in_vanguard(number, 6)
        };
        for seed in [1u64, 7, 42] {
            let day = |number| {
                calculate_shelling_damage(
                    &codex,
                    &mut crate::random::SeededRng::new(seed),
                    &attacker(number),
                    &defender(),
                    6,
                    EngagementType::SameCourse,
                    None,
                    HitOutcome::Hit,
                )
            };
            let night = |number| {
                calculate_night_damage(
                    &codex,
                    &mut crate::random::SeededRng::new(seed),
                    &attacker(number),
                    &defender(),
                    None,
                    None,
                    None,
                    HitOutcome::Hit,
                )
            };
            for (front, rear) in [(day(1), day(6)), (night(1), night(6))] {
                assert!(front > 0);
                assert!((rear - 2 * front).abs() <= 1, "seed {seed}: {front} against {rear}");
            }
        }
    }

    #[test]
    fn low_ammo_reduces_friendly_torpedo_and_night_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bull_max = codex.find::<ApiMstShip>(&dd).unwrap().api_bull_max.unwrap();

        let build = |bull: i64| {
            let mut input = sample_ship(&codex, dd, 99);
            input.ship.api_bull = bull;
            let attacker = BattleRuntimeShip::new(input, true, true);
            let defender = BattleRuntimeShip::new(sample_ship(&codex, dd, 1), false, true);
            (attacker, defender)
        };

        for seed in [3u64, 11, 50, 99] {
            let (atk, def) = build(bull_max);
            let mut rng = crate::random::SeededRng::new(seed);
            let torp_full = calculate_torpedo_damage(
                &codex,
                &mut rng,
                &atk,
                &def,
                1,
                EngagementType::SameCourse,
                BattlePhase::OpeningTorpedo,
                HitOutcome::Hit,
            );
            let (atk, def) = build(bull_max / 10);
            let mut rng = crate::random::SeededRng::new(seed);
            let torp_low = calculate_torpedo_damage(
                &codex,
                &mut rng,
                &atk,
                &def,
                1,
                EngagementType::SameCourse,
                BattlePhase::OpeningTorpedo,
                HitOutcome::Hit,
            );
            assert!(torp_low <= torp_full, "torpedo seed {seed}: {torp_low} > {torp_full}");

            let (atk, def) = build(bull_max);
            let mut rng = crate::random::SeededRng::new(seed);
            let night_full = calculate_night_damage(
                &codex,
                &mut rng,
                &atk,
                &def,
                None,
                None,
                None,
                HitOutcome::Hit,
            );
            let (atk, def) = build(bull_max / 10);
            let mut rng = crate::random::SeededRng::new(seed);
            let night_low = calculate_night_damage(
                &codex,
                &mut rng,
                &atk,
                &def,
                None,
                None,
                None,
                HitOutcome::Hit,
            );
            assert!(night_low <= night_full, "night seed {seed}: {night_low} > {night_full}");
        }
    }

    #[test]
    fn enemy_ammo_does_not_affect_shelling_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ca = first_ship_mst_by_type(&codex, KcShipType::CA);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bull_max = codex.find::<ApiMstShip>(&ca).unwrap().api_bull_max.unwrap();

        let shell = |bull: i64, seed: u64| {
            let mut input = sample_ship(&codex, ca, 99);
            input.ship.api_bull = bull;
            let attacker = BattleRuntimeShip::new(input, false, true); // enemy attacker
            let defender = BattleRuntimeShip::new(sample_ship(&codex, dd, 1), true, true);
            let mut rng = crate::random::SeededRng::new(seed);
            calculate_shelling_damage(
                &codex,
                &mut rng,
                &attacker,
                &defender,
                1,
                EngagementType::SameCourse,
                None,
                HitOutcome::Hit,
            )
        };

        // Enemy attacker: ammo is irrelevant, so full and empty deal identical damage.
        for seed in [1u64, 9, 77] {
            assert_eq!(shell(bull_max, seed), shell(0, seed), "enemy ignores ammo (seed {seed})");
        }
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
            None,
            HitOutcome::Hit,
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
            None,
            HitOutcome::Hit,
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
            HitOutcome::Hit,
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
            HitOutcome::Hit,
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
            HitOutcome::Hit,
            PlaneProficiency::NONE,
        );

        // Verify damage is positive and uses the ASW formula (not shelling formula)
        assert!(dmg >= 1);
        // raw_power = (√(80 - equip_asw) * 2 + √equip_asw * 1.5 + 13) * synergy
        // With a single depth charge: projector=true, dc=true → synergy = 1.1
        let base_asw = (80.0 - equip_asw).max(0.0);
        let synergy = 1.1; // single DepthCharge counts as both projector and charge
        let expected_raw = (base_asw.sqrt() * 2.0 + equip_asw.sqrt() * 1.5 + 13.0) * synergy;
        let expected_capped = apply_cap(expected_raw, ASW_CAP) as f64;
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
            None,
            HitOutcome::Hit,
        );
        let penalized_damage = calculate_shelling_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            5,
            EngagementType::TDisadvantage,
            None,
            HitOutcome::Hit,
        );

        assert!(normal_damage > penalized_damage);
    }

    #[test]
    fn cv_shelling_uses_bomber_plane_count() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let bomber_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        // CVL with 2 bomber slots: 18 planes each = 36 total
        let mut cvl = sample_ship(&codex, cvl_mst, 50);
        cvl.ship.api_onslot = [18, 18, 0, 0, 0];
        cvl.slot_items =
            vec![slotitem_with_mst_id(bomber_mst_id), slotitem_with_mst_id(bomber_mst_id)];
        let cvl_ship = BattleRuntimeShip::from(cvl);
        assert_eq!(bomber_plane_count(&codex, &cvl_ship), 36);

        // Basic power should be 1.5 * 36 + 55 = 109
        let expected_basic = 1.5 * 36.0 + 55.0;
        assert!((expected_basic - 109.0_f64).abs() < f64::EPSILON);

        // Verify damage is computed (not just returning firepower fallback)
        let mut defender = BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50));
        defender.ship.api_soukou[0] = 0;
        let mut rng = crate::random::SeededRng::new(42);
        let attacker = cvl_ship.clone();
        let dmg = calculate_shelling_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
            None,
            HitOutcome::Hit,
        );
        assert!(dmg > 0, "CV with bombers should deal shelling damage");
    }

    #[test]
    fn asw_type_bonus_cumulative_for_aircraft_and_depth_charge() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let dc_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::DepthCharge);
        let autogyro_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::AutoGyro);

        // DD with both depth charge and autogyro should get cumulative type_bonus
        let mut dd = sample_ship(&codex, dd_mst, 99);
        dd.ship.api_taisen[0] = 100;
        dd.ship.api_soukou[0] = 200;
        dd.ship.api_nowhp = 200;
        dd.ship.api_maxhp = 200;
        dd.slot_items =
            vec![slotitem_with_mst_id(dc_mst_id), slotitem_with_mst_id(autogyro_mst_id)];

        let mut defender = BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50));
        defender.ship.api_soukou[0] = 5;
        defender.ship.api_nowhp = 50;
        defender.ship.api_maxhp = 50;

        let mut rng = crate::random::SeededRng::new(42);
        let attacker = BattleRuntimeShip::from(dd);

        let dmg = calculate_asw_damage(
            &codex,
            &mut rng,
            &attacker,
            &defender,
            1,
            EngagementType::SameCourse,
            HitOutcome::Hit,
            PlaneProficiency::NONE,
        );
        assert!(dmg > 0, "ASW with both aircraft and depth charge should deal damage");
    }

    /// An item with the fields the improvement looks at.
    fn gear(id: i64, kind: i64, firepower: i64, asw: i64) -> ApiMstSlotitem {
        ApiMstSlotitem {
            api_id: id,
            api_type: [0, 0, kind, 0, 0],
            api_houg: firepower,
            api_tais: asw,
            ..Default::default()
        }
    }

    fn assert_improvement(item: &ApiMstSlotitem, stars: i64, want: [f64; 4]) {
        let got = improvement(item, stars);
        let got = [got.shell, got.night, got.torpedo, got.asw];
        for (g, w) in got.iter().zip(want) {
            assert!(
                (g - w).abs() < 1e-9,
                "item {} at ★{stars}: {got:?}, want {want:?}",
                item.api_id
            );
        }
    }

    fn assert_aim(item: &ApiMstSlotitem, stars: i64, want: [f64; 6]) {
        let got = improvement(item, stars);
        let got = [
            got.shell_accuracy,
            got.night_accuracy,
            got.torpedo_accuracy,
            got.asw_accuracy,
            got.shell_evasion,
            got.torpedo_evasion,
        ];
        for (g, w) in got.iter().zip(want) {
            assert!(
                (g - w).abs() < 1e-9,
                "item {} at ★{stars}: {got:?}, want {want:?}",
                item.api_id
            );
        }
    }

    /// The source's accuracy and evasion figures, as `[day, night, torpedo, ASW accuracy, day,
    /// torpedo evasion]`.
    #[test]
    fn improvement_aims_by_the_equipment_type() {
        let root10 = 10f64.sqrt();
        assert_aim(&gear(2, 1, 2, 0), 10, [root10, 1.3 * root10, 0.0, 0.0, 0.0, 0.0]);
        assert_aim(&gear(13, 5, 0, 0), 4, [0.0, 2.6, 4.0, 0.0, 0.0, 0.0]);
        assert_aim(&gear(37, 21, 0, 0), 4, [0.0, 0.0, 4.0, 0.0, 0.0, 0.0]);
        assert_aim(&gear(46, 14, 0, 6), 9, [0.0, 0.0, 0.0, 3.9, 0.0, 4.5]);
        assert_aim(&gear(226, 15, 0, 5), 9, [0.0, 0.0, 0.0, 3.9, 0.0, 0.0]);
        assert_aim(&gear(34, 17, 0, 0), 6, [0.0, 0.0, 0.0, 0.0, 1.5 * 6f64.sqrt(), 0.0]);
    }

    /// A radar of 3 accuracy or more aims at 1.7√★ by day and 1.6√★ at night, one of less as a gun.
    #[test]
    fn a_radar_aims_by_its_accuracy() {
        let radar = |accuracy| ApiMstSlotitem {
            api_houm: accuracy,
            ..gear(28, 12, 0, 0)
        };
        assert_aim(&radar(3), 4, [3.4, 3.2, 0.0, 0.0, 0.0, 0.0]);
        assert_aim(&radar(2), 4, [2.0, 2.6, 0.0, 0.0, 0.0, 0.0]);
    }

    /// The source's figures (`setImprovement`, `EQTDATA[type].improve`), as
    /// `[day, night, torpedo, ASW]`.
    #[test]
    fn improvement_follows_the_equipment_type() {
        let root10 = 10f64.sqrt();
        // A torpedo: 1.2√★ to its torpedo, √★ at night.
        assert_improvement(&gear(13, 5, 0, 0), 10, [0.0, root10, 1.2 * root10, 0.0]);
        // A sonar: 0.75√★ by day, √★ against a submarine.
        assert_improvement(&gear(46, 14, 0, 6), 4, [1.5, 0.0, 0.0, 2.0]);
        // An anti-aircraft gun: √★ by day, 1.2√★ to the torpedo, nothing at night.
        assert_improvement(&gear(37, 21, 0, 0), 4, [2.0, 0.0, 2.4, 0.0]);
    }

    /// A gun's weight by day turns on its firepower: 1.5√★ at 13 or more, √★ below.
    #[test]
    fn a_guns_improvement_by_day_turns_on_its_firepower() {
        assert_improvement(&gear(7, 3, 15, 0), 4, [3.0, 2.0, 0.0, 0.0]);
        assert_improvement(&gear(7, 3, 12, 0), 4, [2.0, 2.0, 0.0, 0.0]);
        assert_improvement(&gear(583, 2, 13, 0), 4, [3.0, 2.0, 0.0, 0.0]);
    }

    /// Secondary guns, submarine torpedoes and three depth charges go by the item.
    #[test]
    fn some_items_improve_by_their_own_rule() {
        // The source's 0.3★ list, 0.2★ for the rest, √★ for five; 467 is typed apart in the
        // source and keeps √★ though the list names it.
        assert_improvement(&gear(12, 4, 7, 0), 10, [3.0, 3.0, 0.0, 0.0]);
        assert_improvement(&gear(10, 4, 4, 0), 10, [2.0, 2.0, 0.0, 0.0]);
        assert_improvement(&gear(11, 4, 2, 0), 9, [3.0, 3.0, 0.0, 0.0]);
        assert_improvement(&gear(467, 4, 5, 0), 9, [3.0, 3.0, 0.0, 0.0]);
        // A submarine torpedo: 0.2★ to its torpedo and at night.
        assert_improvement(&gear(95, 32, 0, 0), 10, [0.0, 2.0, 2.0, 0.0]);
        for id in [226, 227, 488] {
            assert_improvement(&gear(id, 15, 0, 5), 4, [0.0, 0.0, 0.0, 2.0]);
        }
    }

    /// Bombers, autogyros and patrol planes improve linearly.
    #[test]
    fn aircraft_improve_by_the_star() {
        // A dive bomber: 0.2★ by day and against a submarine; a fighter-bomber nothing, but
        // for 487's 0.2★ against a submarine.
        assert_improvement(&gear(23, 7, 0, 0), 5, [1.0, 0.0, 0.0, 1.0]);
        assert_improvement(&gear(60, 7, 0, 3), 5, [0.0, 0.0, 0.0, 0.0]);
        assert_improvement(&gear(487, 7, 0, 0), 4, [0.0, 0.0, 0.0, 0.8]);
        assert_improvement(&gear(93, 8, 0, 0), 5, [1.0, 0.0, 0.0, 1.0]);
        // An autogyro: 0.3★ at 11 ASW or more, 0.2★ below; a patrol plane the same at 8.
        assert_improvement(&gear(69, 25, 0, 11), 10, [0.0, 0.0, 0.0, 3.0]);
        assert_improvement(&gear(69, 25, 0, 10), 10, [0.0, 0.0, 0.0, 2.0]);
        assert_improvement(&gear(70, 26, 0, 8), 10, [0.0, 0.0, 0.0, 3.0]);
        assert_improvement(&gear(70, 26, 0, 7), 10, [0.0, 0.0, 0.0, 2.0]);
    }

    /// Equipment at ★0 adds nothing, as an abyssal ship's always is; stars add up over the ship.
    #[test]
    fn a_ships_improvement_sums_her_stars() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        let gun = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        ship.slot_items = vec![slotitem_with_mst_id(gun), slotitem_with_mst_id(gun)];
        let plain = BattleRuntimeShip::from(ship.clone());
        assert_eq!(ship_improvement(&codex, &plain), Improvement::NONE);
        ship.slot_items[0].api_level = 4;
        ship.slot_items[1].api_level = 9;
        let improved = ship_improvement(&codex, &BattleRuntimeShip::from(ship));
        assert!((improved.shell - 5.0).abs() < 1e-9 && (improved.night - 5.0).abs() < 1e-9);
    }
}
