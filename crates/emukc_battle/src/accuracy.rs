//! Whether an attack lands, and whether it lands as a critical.
//!
//! Transcribed from `KC3Kai/kancolle-replay` `kcsim.js`: `hitRate`,
//! `accuracyAndCrit`, `rollHit`, `formationCountered` and the formation table
//! at the top of that file, with `Ship.moraleMod` / `moraleModEv` from
//! `kcships.js`. The arithmetic is kept in whole percent, as the source does
//! after its `floor(round(x * 1e6) / 1e4)` step ([`floor_percent`]).
//!
//! Aircraft proficiency follows `Ship.updateProficiencyBonus` in `kcships.js`.
//!
//! The gun fit comes from [`gun_fit`](crate::gun_fit).
//!
//! Left out, each a correction the source applies on top of what is here:
//! the combined-fleet accuracy terms, smoke, balloons, PT imps and event
//! bonuses.

use emukc_model::{
    codex::Codex,
    kc2::{
        KcShipType, KcSlotItemType3,
        start2::{ApiMstShip, ApiMstSlotitem},
    },
};

use crate::damage::{is_cv_type, ship_improvement};
use crate::gun_fit::gun_fit;
use crate::random::BattleRng;
use crate::targeting::{
    bonus_gears, has_slotitem_type, is_radar_type, searchlight_rerolls, ship_mst, ship_type,
};
use crate::types::BattleRuntimeShip;

/// What a critical does to the attack power, after the cap and before armour.
const CRITICAL_MODIFIER: f64 = 1.5;
/// No attack is less likely than this to land, in percent.
const HIT_FLOOR: i64 = 10;
/// Nor more likely than this.
const HIT_CEILING: i64 = 96;
/// The condition an abyssal ship fights at: neither tired nor sparkling.
const ENEMY_COND: i64 = 49;

/// How one attack came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HitOutcome {
    Miss,
    Hit,
    Critical,
}

impl HitOutcome {
    /// The value the client reads from `api_cl_list`, `api_fcl` and `api_ecl`.
    pub(crate) fn cl(self) -> i64 {
        match self {
            Self::Miss => 0,
            Self::Hit => 1,
            Self::Critical => 2,
        }
    }

    /// The capped attack power this outcome sends against the armour.
    pub(crate) fn power(self, capped: f64) -> f64 {
        self.power_with(capped, 1.0)
    }

    /// The same for an attack flown by aircraft, whose proficiency makes a
    /// critical worth `critical_damage` times more.
    pub(crate) fn power_with(self, capped: f64, critical_damage: f64) -> f64 {
        match self {
            Self::Miss => 0.0,
            Self::Hit => capped,
            // The source multiplies the two factors first (`rollHit`), then the power.
            Self::Critical => (capped * (CRITICAL_MODIFIER * critical_damage)).floor(),
        }
    }
}

/// What the proficiency of a ship's aircraft adds to the attacks they fly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlaneProficiency {
    /// Added to the chance to hit, above the ceiling.
    pub accuracy: f64,
    /// Added to the chance of a critical, in percent.
    pub critical_rate: f64,
    /// Multiplies what a critical does to the attack power.
    pub critical_damage: f64,
    /// The mean experience of the aircraft counted.
    pub average_exp: f64,
}

impl PlaneProficiency {
    /// No aircraft, or none with any proficiency.
    pub(crate) const NONE: Self = Self {
        accuracy: 0.0,
        critical_rate: 0.0,
        critical_damage: 1.0,
        average_exp: 0.0,
    };

    /// A carrier cut-in swaps the summed critical rate for 13 in a hundred at
    /// full experience (`kcsim.js` 470).
    // ponytail: the source's further terms for the first slot's aircraft type
    // and experience are left out; add them with the cut-in's own critical damage.
    pub(crate) fn for_carrier_cut_in(mut self) -> Self {
        self.critical_rate = 13.0 * self.average_exp / 120.0;
        self
    }
}

/// The experience behind each proficiency level the client shows.
pub(crate) const PROFICIENCY_EXP: [f64; 8] = [0.0, 10.0, 25.0, 40.0, 55.0, 70.0, 85.0, 120.0];
/// What each level is worth towards a critical.
const PROFICIENCY_CRITICAL: [f64; 8] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 10.0];
/// How much of its experience a patrol plane or autogyro counts for.
const PATROL_EXP_SHARE: f64 = 0.825;

/// Sum a ship's aircraft proficiency (`updateProficiencyBonus`). The first
/// piece of equipment counts for more than the others; a patrol plane or
/// autogyro that can bomb counts one level lower.
pub(crate) fn plane_proficiency(codex: &Codex, ship: &BattleRuntimeShip) -> PlaneProficiency {
    let mut out = PlaneProficiency::NONE;
    let (mut total_exp, mut planes) = (0.0, 0);
    for (i, item) in ship.slot_items.iter().enumerate() {
        let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
            continue;
        };
        let patrol = match KcSlotItemType3::n(mst.api_type[2]) {
            Some(
                KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::CarrierBasedDiveBomber
                | KcSlotItemType3::SeaBasedBomber
                | KcSlotItemType3::LargeFlyingBoat
                | KcSlotItemType3::LandBasedAttacker
                | KcSlotItemType3::LargeLandBasedAircraft
                | KcSlotItemType3::JetFighterBomber,
            ) => false,
            Some(KcSlotItemType3::AutoGyro | KcSlotItemType3::AntiSubmarinePatrol)
                if mst.api_baku > 0 =>
            {
                true
            }
            _ => continue,
        };
        planes += 1;
        let mut level = item.api_alv.unwrap_or(0).clamp(0, 7) as usize;
        let mut exp = ship.aircraft_exp.get(i).map_or(PROFICIENCY_EXP[level], |exp| *exp as f64);
        if exp <= 0.0 {
            continue;
        }
        if patrol {
            exp *= PATROL_EXP_SHARE;
            level = level.saturating_sub(1);
        }
        let critical = PROFICIENCY_CRITICAL[level];
        let (rate, divisor) = if i == 0 {
            (0.8, 100.0)
        } else {
            (0.6, 200.0)
        };
        out.critical_rate += critical * rate;
        out.critical_damage += (exp.sqrt() + critical).floor() / divisor;
        total_exp += exp;
    }
    if planes > 0 {
        let average = total_exp / planes as f64;
        out.average_exp = average;
        if average >= 10.0 {
            out.accuracy = (average * 0.1).sqrt();
        }
        out.accuracy += match average {
            a if a >= 100.0 => 9.0,
            a if a >= 80.0 => 6.0,
            a if a >= 70.0 => 4.0,
            a if a >= 55.0 => 3.0,
            a if a >= 40.0 => 2.0,
            a if a >= 25.0 => 1.0,
            _ => 0.0,
        };
    }
    out
}

/// What each level adds to a squadron's chance to hit, on top of the root of
/// its experience.
const SQUADRON_ACCURACY: [f64; 8] = [0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 6.0, 9.0];

/// The proficiency one squadron of an air corps strikes with
/// (`airstrikeLBAS`): its own level and experience, the level's representative
/// experience when none is stored, at the first slot's rates. A patrol plane or
/// autogyro counts one level lower.
pub(crate) fn squadron_proficiency(type3: i64, alv: i64, exp: Option<i64>) -> PlaneProficiency {
    let mut level = alv.clamp(0, 7) as usize;
    let mut exp = exp.map_or(PROFICIENCY_EXP[level], |exp| exp.max(0) as f64);
    if matches!(
        KcSlotItemType3::n(type3),
        Some(KcSlotItemType3::AutoGyro | KcSlotItemType3::AntiSubmarinePatrol)
    ) {
        exp *= PATROL_EXP_SHARE;
        level = level.saturating_sub(1);
    }
    let critical = PROFICIENCY_CRITICAL[level];
    PlaneProficiency {
        accuracy: if level == 0 {
            0.0
        } else {
            (exp * 0.1).sqrt() + SQUADRON_ACCURACY[level]
        },
        critical_rate: critical * 0.8,
        critical_damage: 1.0 + (exp.sqrt() + critical).floor() / 100.0,
        average_exp: exp,
    }
}

/// The proficiency a ship's shelling takes: a carrier's, whose shelling is
/// flown by its aircraft, and no one else's.
pub(crate) fn shelling_proficiency(codex: &Codex, ship: &BattleRuntimeShip) -> PlaneProficiency {
    if is_cv_type(codex, ship) {
        plane_proficiency(codex, ship)
    } else {
        PlaneProficiency::NONE
    }
}

/// The proficiency a ship's day attack on a submarine takes (`usePlaneProf`):
/// that of the ships whose aircraft make it — 航巡, 航戦, 水母, 揚陸艦 and 軽空母 —
/// and not in the opening, where depth charges do the work.
pub(crate) fn asw_proficiency(codex: &Codex, ship: &BattleRuntimeShip) -> PlaneProficiency {
    match ship_type(codex, ship) {
        Some(
            KcShipType::CAV | KcShipType::BBV | KcShipType::AV | KcShipType::LHA | KcShipType::CVL,
        ) => plane_proficiency(codex, ship),
        _ => PlaneProficiency::NONE,
    }
}

/// A night scout in contact with the other fleet (夜間触接), and what it gives every attack
/// of its own fleet that night. The source's `NBattack`, by the scout's own accuracy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NightContact {
    /// The scout, for `api_touch_plane`.
    pub plane: i64,
    /// Added to the attack power before the cap.
    pub power: f64,
    /// Multiplies the base accuracy.
    pub accuracy: f64,
    /// Stands in for the night's critical factor of 1.5.
    pub critical: f64,
}

impl NightContact {
    pub(crate) fn of(plane: i64, plane_accuracy: i64) -> Self {
        let (power, accuracy, critical) = match plane_accuracy {
            ..=1 => (5.0, 1.1, 1.57),
            2 => (7.0, 1.15, 1.64),
            _ => (9.0, 1.2, 1.7),
        };
        Self {
            plane,
            power,
            accuracy,
            critical,
        }
    }
}

/// The kind of attack, which picks the base accuracy, the formation columns
/// and the critical factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackKind {
    Shelling,
    Torpedo,
    Night,
    Asw,
}

impl AttackKind {
    fn base(self) -> f64 {
        match self {
            Self::Shelling => 90.0,
            Self::Torpedo => 85.0,
            Self::Asw => 80.0,
            Self::Night => 69.0,
        }
    }

    fn critical_factor(self) -> f64 {
        match self {
            Self::Shelling | Self::Asw => 1.3,
            Self::Torpedo | Self::Night => 1.5,
        }
    }
}

/// One formation's accuracy and evasion multipliers.
struct FormationAim {
    shell_acc: f64,
    torp_acc: f64,
    night_acc: f64,
    asw_acc: f64,
    shell_ev: f64,
    torp_ev: f64,
    night_ev: f64,
    asw_ev: f64,
}

#[expect(clippy::too_many_arguments)]
const fn aim(
    shell_acc: f64,
    torp_acc: f64,
    night_acc: f64,
    asw_acc: f64,
    shell_ev: f64,
    torp_ev: f64,
    night_ev: f64,
    asw_ev: f64,
) -> FormationAim {
    FormationAim {
        shell_acc,
        torp_acc,
        night_acc,
        asw_acc,
        shell_ev,
        torp_ev,
        night_ev,
        asw_ev,
    }
}

/// The source's formation table. Where it gives no `ASWacc` the shelling
/// column stands in, as `ASW()` does there. 警戒陣 has a row for each half
/// of the fleet (`VANGUARD1`, `VANGUARD2`).
fn formation_aim(formation_id: i64, front_half: bool) -> FormationAim {
    match formation_id {
        2 => aim(1.2, 0.8, 0.9, 1.2, 1.0, 1.0, 1.0, 1.0),
        3 => aim(1.0, 0.4, 0.7, 1.0, 1.1, 1.1, 1.0, 1.0),
        4 => aim(1.2, 0.75, 0.9, 1.2, 1.4, 1.3, 1.3, 1.3),
        5 => aim(1.2, 0.3, 0.8, 1.2, 1.3, 1.4, 1.2, 1.1),
        6 if front_half => aim(0.8, 0.7, 0.8, 1.0, 1.0, 1.0, 1.0, 1.0),
        6 => aim(1.2, 0.9, 1.2, 1.1, 1.0, 1.0, 1.0, 1.0),
        11 => aim(0.9, 0.75, 0.8, 1.25, 1.1, 1.0, 1.0, 1.0),
        12 => aim(1.0, 1.0, 0.9, 1.0, 1.2, 1.0, 1.0, 1.0),
        13 => aim(0.8, 0.35, 0.7, 1.1, 1.1, 1.0, 1.1, 1.0),
        14 => aim(1.1, 1.2, 1.0, 0.7, 1.0, 1.0, 1.0, 1.0),
        _ => aim(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
    }
}

/// What a target's place in a fleet in 警戒陣 leaves of the attacker's
/// accuracy, by whether she is a destroyer (`vanguardUseType` 2, the `*Mod`
/// rows); 1 for a ship in any other formation. Torpedoes have rows of their
/// own; shelling, night and anti-submarine attacks share one.
fn vanguard_dodge(codex: &Codex, defender: &BattleRuntimeShip, kind: AttackKind) -> f64 {
    let Some(place) = defender.vanguard else {
        return 1.0;
    };
    let destroyer = ship_type(codex, defender) == Some(KcShipType::DD);
    let row: [f64; 7] = match (kind == AttackKind::Torpedo, destroyer) {
        (false, true) => [0.95, 0.95, 0.8, 0.8, 0.69, 0.64, 0.64],
        (false, false) => [0.95, 0.95, 0.95, 0.95, 0.86, 0.79, 0.7],
        (true, true) => [0.9, 0.9, 0.65, 0.58, 0.5, 0.42, 0.42],
        (true, false) => [0.9, 0.9, 0.77, 0.67, 0.63, 0.55, 0.51],
    };
    row.get(place.number - 1).copied().unwrap_or(1.0)
}

/// A fleet in 警戒陣 aims its torpedoes at a destroyer at 1.2 times and its
/// shelling and night attacks at 1.1 (`torpedoPhase`, `shell`, `NBattack`).
fn vanguard_against_destroyers(
    codex: &Codex,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    kind: AttackKind,
) -> f64 {
    if attacker.vanguard.is_none() || ship_type(codex, defender) != Some(KcShipType::DD) {
        return 1.0;
    }
    match kind {
        AttackKind::Torpedo => 1.2,
        AttackKind::Shelling | AttackKind::Night => 1.1,
        AttackKind::Asw => 1.0,
    }
}

/// 複縦 against 単横, 梯形 against 単縦, 単横 against 梯形: the attacker's
/// formation then gives its shelling and ASW no accuracy.
fn formation_countered(attacker: i64, defender: i64) -> bool {
    matches!((attacker, defender), (2, 5) | (4, 1) | (5, 4))
}

fn cond(ship: &BattleRuntimeShip) -> i64 {
    if ship.is_friendly {
        ship.ship.api_cond
    } else {
        ENEMY_COND
    }
}

/// What the attacker's condition does to its accuracy.
fn attacker_morale(ship: &BattleRuntimeShip, kind: AttackKind) -> f64 {
    let cond = cond(ship);
    let table = if kind == AttackKind::Torpedo {
        [1.3, 1.0, 0.7, 0.35]
    } else {
        [1.2, 1.0, 0.8, 0.5]
    };
    match cond {
        50.. => table[0],
        30..=49 => table[1],
        20..=29 => table[2],
        _ => table[3],
    }
}

/// What the target's condition does to the chance of hitting it.
fn target_morale(ship: &BattleRuntimeShip) -> f64 {
    match cond(ship) {
        50.. => 0.7,
        30..=49 => 1.0,
        20..=29 => 1.2,
        _ => 1.4,
    }
}

/// The accuracy the attacker's equipment adds.
fn equipment_accuracy(codex: &Codex, ship: &BattleRuntimeShip, kind: AttackKind) -> f64 {
    ship.slot_items
        .iter()
        .filter_map(|item| codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok())
        .map(|mst| {
            if kind == AttackKind::Asw {
                // Against submarines only sonar counts, at twice its 対潜.
                match KcSlotItemType3::n(mst.api_type[2]) {
                    Some(KcSlotItemType3::Sonar | KcSlotItemType3::LargeSonar) => 2 * mst.api_tais,
                    _ => 0,
                }
            } else {
                mst.api_houm
            }
        })
        .sum::<i64>() as f64
}

/// The accuracy the attacker's 装備ボーナス adds. The source keeps it off the ship's stats and
/// adds it to day shelling alone (`kcsim.js` `shell`, `statsEqBonus.ACC`).
fn bonus_accuracy(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    ship_mst(codex, ship)
        .and_then(|hull| codex.gear_bonus(hull, &bonus_gears(codex, ship)).ok())
        .map_or(0.0, |bonus| bonus.houm as f64)
}

/// How far below three quarters the target's fuel is, in percent. A ship short
/// of fuel dodges that much worse. Abyssal ships are always full.
fn fuel_shortfall(codex: &Codex, ship: &BattleRuntimeShip) -> i64 {
    if !ship.is_friendly {
        return 0;
    }
    let Ok(mst) = codex.find::<ApiMstShip>(&ship.ship.api_ship_id) else {
        return 0;
    };
    let fuel_max = mst.api_fuel_max.unwrap_or(0);
    if fuel_max <= 0 {
        return 0;
    }
    (75 - 100 * ship.ship.api_fuel.max(0) / fuel_max).max(0)
}

/// The percentage the target's evasion takes off an attack: its 回避 and 運
/// through the source's three-step curve, less any fuel shortfall.
pub(crate) fn evasion_term(evasion: i64, luck: i64, formation: f64, fuel_shortfall: i64) -> i64 {
    let evade = ((evasion.max(0) as f64 + (2.0 * luck.max(0) as f64).sqrt()) * formation).floor();
    let dodge = if evade > 65.0 {
        (55.0 + 2.0 * (evade - 65.0).sqrt()).floor()
    } else if evade > 40.0 {
        (40.0 + 3.0 * (evade - 40.0).sqrt()).floor()
    } else {
        evade
    };
    dodge as i64 - fuel_shortfall
}

/// A target's night evasion (`NBattack`): her formation's multiplier, and the flat evasion added
/// after the curve, 5 for a heavy or aviation cruiser and 10 for a destroyer with a radar and a
/// lookout. Her own searchlight leaves her a fifth of each.
fn night_evasion(codex: &Codex, ship: &BattleRuntimeShip, formation: f64) -> (f64, i64) {
    let flat = match ship_type(codex, ship) {
        Some(KcShipType::CA | KcShipType::CAV) => 5,
        Some(KcShipType::DD)
            if has_slotitem_type(codex, ship, KcSlotItemType3::SeaplanePersonnel)
                && ship.slot_items.iter().any(|item| {
                    codex
                        .find::<ApiMstSlotitem>(&item.api_slotitem_id)
                        .ok()
                        .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                        .is_some_and(is_radar_type)
                }) =>
        {
            10
        }
        _ => 0,
    };
    if searchlight_rerolls(codex, ship) > 0 {
        (formation * 0.2, flat / 5)
    } else {
        (formation, flat)
    }
}

/// Round a percentage down to a whole one as `accuracyAndCrit` does: first to four places, so
/// floating-point noise just under a whole number (`100 x 1.15`) does not cost a point.
fn floor_percent(x: f64) -> i64 {
    // `Math.round` is `floor(x + 0.5)`.
    ((x * 1e4 + 0.5).floor() / 1e4).floor() as i64
}

/// The chance to hit in percent, between the floor and the ceiling.
pub(crate) fn hit_chance(hit: f64, dodge: i64, target_morale: f64) -> i64 {
    let chance = (floor_percent(hit) - dodge).max(HIT_FLOOR) as f64 * target_morale;
    (chance.floor() as i64).min(HIT_CEILING)
}

/// Roll one attack against its chance to hit. A critical is rolled on the same
/// draw: it happens `floor(sqrt(chance) * critical_factor)` times in a hundred.
/// Aircraft proficiency adds to the chance after its ceiling, and to the
/// critical threshold after the square root.
pub(crate) fn roll(
    rng: &mut impl BattleRng,
    chance: i64,
    critical_factor: f64,
    planes: PlaneProficiency,
) -> HitOutcome {
    let chance = chance as f64 + planes.accuracy;
    let critical = (chance.sqrt() * critical_factor + planes.critical_rate).floor() as i64;
    let chance = chance.floor() as i64;
    let drawn = rng.roll_range(0, 100);
    // The source compares with `<=` on both steps.
    if (critical_factor > 0.0 || planes.critical_rate > 0.0) && drawn <= critical {
        HitOutcome::Critical
    } else if drawn <= chance {
        HitOutcome::Hit
    } else {
        HitOutcome::Miss
    }
}

/// Everything about one gun, torpedo or depth-charge attack that the roll
/// needs besides the two ships.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Aim {
    pub kind: AttackKind,
    pub attacker_formation: i64,
    pub defender_formation: i64,
    /// The attack's own accuracy multiplier inside `hitRate` (a special attack's, a night
    /// cut-in's), 1 for a plain attack.
    pub modifier: f64,
    /// A day shelling cut-in's or double attack's multiplier, which the source keeps apart
    /// (`accMod2`) and applies after the gun fit; 1 otherwise.
    pub day_cut_in: f64,
    /// Accuracy the phase adds on its own: a fifth of the torpedo's power.
    pub flat: f64,
    /// Whether this is a carrier's cut-in, which has its own critical rate.
    pub carrier_cut_in: bool,
    /// Whether this is the opening ASW, which no aircraft proficiency reaches.
    pub opening: bool,
    /// Whether this is a carrier's night air attack, which her aircraft's proficiency does.
    pub night_air_attack: bool,
    /// The attacker's fleet's night scout in contact, if one is.
    pub contact: Option<NightContact>,
    /// Whether the attacker's fleet fired a star shell as the night began.
    pub star_shell: bool,
    /// Whether the attacker's fleet has a searchlight lit for the night.
    pub searchlight: bool,
}

impl Aim {
    pub(crate) fn new(kind: AttackKind, attacker_formation: i64, defender_formation: i64) -> Self {
        Self {
            kind,
            attacker_formation,
            defender_formation,
            modifier: 1.0,
            day_cut_in: 1.0,
            flat: 0.0,
            carrier_cut_in: false,
            opening: false,
            night_air_attack: false,
            contact: None,
            star_shell: false,
            searchlight: false,
        }
    }

    /// The base accuracy (`NBattack`): at night a fleet's own star shell adds 5, before its
    /// scout multiplies the sum.
    fn base(&self) -> f64 {
        let flare = if self.star_shell && self.kind == AttackKind::Night {
            5.0
        } else {
            0.0
        };
        (self.kind.base() + flare) * self.contact.map_or(1.0, |contact| contact.accuracy)
    }

    pub(crate) fn lit_by_star_shell(mut self, star_shell: bool) -> Self {
        self.star_shell = star_shell;
        self
    }

    pub(crate) fn lit_by_searchlight(mut self, searchlight: bool) -> Self {
        self.searchlight = searchlight;
        self
    }

    pub(crate) fn under_contact(mut self, contact: Option<NightContact>) -> Self {
        self.contact = contact;
        self
    }

    pub(crate) fn as_night_air_attack(mut self, night_air_attack: bool) -> Self {
        self.night_air_attack = night_air_attack;
        self
    }

    pub(crate) fn in_the_opening(mut self) -> Self {
        self.opening = true;
        self
    }

    pub(crate) fn as_carrier_cut_in(mut self, carrier_cut_in: bool) -> Self {
        self.carrier_cut_in = carrier_cut_in;
        self
    }

    pub(crate) fn with_modifier(mut self, modifier: f64) -> Self {
        self.modifier = modifier;
        self
    }

    pub(crate) fn with_day_cut_in(mut self, modifier: f64) -> Self {
        self.day_cut_in = modifier;
        self
    }

    pub(crate) fn with_flat(mut self, flat: f64) -> Self {
        self.flat = flat;
        self
    }
}

/// The accuracy the attacker's improved equipment adds to an attack of `kind` (`ACCshell`,
/// `ACCnb`, `floor(ACCtorp)`, `ACCasw`).
fn improvement_accuracy(codex: &Codex, ship: &BattleRuntimeShip, kind: AttackKind) -> f64 {
    let improvement = ship_improvement(codex, ship);
    match kind {
        AttackKind::Shelling => improvement.shell_accuracy,
        AttackKind::Night => improvement.night_accuracy,
        AttackKind::Torpedo => improvement.torpedo_accuracy.floor(),
        AttackKind::Asw => improvement.asw_accuracy,
    }
}

/// The percentage the target dodges an attack of `kind` by: her evasion through the curve, the
/// flat evasion after it and, against shelling and torpedoes, what her improved equipment adds
/// (`EVshell`, `EVtorp`), rounded down together (`accuracyAndCrit`).
fn dodge(
    codex: &Codex,
    defender: &BattleRuntimeShip,
    kind: AttackKind,
    formation_ev: f64,
    evasion_flat: i64,
) -> i64 {
    let improvement = ship_improvement(codex, defender);
    let improved = match kind {
        AttackKind::Shelling => improvement.shell_evasion,
        AttackKind::Torpedo => improvement.torpedo_evasion,
        AttackKind::Night | AttackKind::Asw => 0.0,
    };
    let curve = evasion_term(
        defender.ship.api_kaihi[0],
        defender.ship.api_lucky[0],
        formation_ev,
        fuel_shortfall(codex, defender),
    );
    floor_percent(curve as f64 + evasion_flat as f64 + improved)
}

/// The attacker's accuracy before the target's evasion (`hitRate`), in percent.
fn hit_value(codex: &Codex, attacker: &BattleRuntimeShip, aim: Aim, formation_acc: f64) -> f64 {
    let level = attacker.ship.api_lv.max(0) as f64;
    let luck = attacker.ship.api_lucky[0].max(0) as f64;
    let fit = match aim.kind {
        AttackKind::Shelling | AttackKind::Night => gun_fit(codex, attacker),
        AttackKind::Torpedo | AttackKind::Asw => 0.0,
    };
    ((aim.base()
        + 2.0 * level.sqrt()
        + 1.5 * luck.sqrt()
        + equipment_accuracy(codex, attacker, aim.kind)
        + if aim.kind == AttackKind::Shelling { bonus_accuracy(codex, attacker) } else { 0.0 }
        + improvement_accuracy(codex, attacker, aim.kind)
        + aim.flat)
        * attacker_morale(attacker, aim.kind)
        * formation_acc
        * aim.modifier
        // `shell`, `NBattack`: the gun fit comes after `hitRate`, before `accMod2`.
        + fit)
        * aim.day_cut_in
        // `NBattack`: the attacker's fleet's searchlight adds 7 after every multiplier.
        + if aim.searchlight && aim.kind == AttackKind::Night { 7.0 } else { 0.0 }
}

/// Roll a ship's attack on another ship.
pub(crate) fn roll_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    aim: Aim,
) -> HitOutcome {
    let own = formation_aim(aim.attacker_formation, attacker.in_vanguard_front());
    let theirs = formation_aim(aim.defender_formation, defender.in_vanguard_front());
    let countered = formation_countered(aim.attacker_formation, aim.defender_formation);
    let (formation_acc, formation_ev) = match aim.kind {
        AttackKind::Shelling if countered => (1.0, theirs.shell_ev),
        AttackKind::Shelling => (own.shell_acc, theirs.shell_ev),
        AttackKind::Asw if countered => (1.0, theirs.asw_ev),
        AttackKind::Asw => (own.asw_acc, theirs.asw_ev),
        AttackKind::Torpedo => (own.torp_acc, theirs.torp_ev),
        AttackKind::Night => (own.night_acc, theirs.night_ev),
    };
    let (formation_ev, evasion_flat) = if aim.kind == AttackKind::Night {
        night_evasion(codex, defender, formation_ev)
    } else {
        (formation_ev, 0)
    };

    // The target's place counts whatever counters the attacker's formation.
    let formation_acc = formation_acc
        * vanguard_dodge(codex, defender, aim.kind)
        * vanguard_against_destroyers(codex, attacker, defender, aim.kind);
    let hit = hit_value(codex, attacker, aim, formation_acc);
    let dodged = dodge(codex, defender, aim.kind, formation_ev, evasion_flat);
    let chance = hit_chance(hit, dodged, target_morale(defender));
    let planes = match aim.kind {
        AttackKind::Shelling if aim.carrier_cut_in => {
            shelling_proficiency(codex, attacker).for_carrier_cut_in()
        }
        AttackKind::Shelling => shelling_proficiency(codex, attacker),
        AttackKind::Night if aim.night_air_attack => plane_proficiency(codex, attacker),
        AttackKind::Asw if !aim.opening => asw_proficiency(codex, attacker),
        _ => PlaneProficiency::NONE,
    };
    let critical = aim.contact.map_or(aim.kind.critical_factor(), |contact| contact.critical);
    roll(rng, chance, critical, planes)
}

/// Roll an aircraft's strike on a ship. Aircraft hit at a fixed rate that no
/// formation changes, and score no critical without proficiency.
pub(crate) fn roll_strike(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defender: &BattleRuntimeShip,
    hit_percent: f64,
    evasion_modifier: f64,
    planes: PlaneProficiency,
) -> HitOutcome {
    let dodge = evasion_term(
        defender.ship.api_kaihi[0],
        defender.ship.api_lucky[0],
        1.0,
        fuel_shortfall(codex, defender),
    );
    let dodge = floor_percent(dodge as f64 * evasion_modifier);
    roll(rng, hit_chance(hit_percent, dodge, target_morale(defender)), 0.0, planes)
}

/// Roll a strike on something that cannot dodge: the air base.
pub(crate) fn roll_strike_on_base(rng: &mut impl BattleRng, hit_percent: f64) -> HitOutcome {
    roll(rng, hit_chance(hit_percent, 0, 1.0), 0.0, PlaneProficiency::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::SeededRng;

    /// A die that always shows the same face.
    struct Fixed(i64);

    impl BattleRng for Fixed {
        fn random_f64_range(&mut self, min: f64, _max: f64) -> f64 {
            min
        }

        fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
            self.0
        }
    }

    #[test]
    fn a_squadron_strikes_with_its_own_level_at_the_first_slots_rates() {
        let attacker = KcSlotItemType3::LandBasedAttacker as i64;
        assert_eq!(squadron_proficiency(attacker, 0, None), PlaneProficiency::NONE);
        let full = squadron_proficiency(attacker, 7, None);
        // sqrt(12) + 9 to hit; 10 x 0.8 to the critical rate; floor(sqrt(120) + 10) = 20
        // hundredths to its damage.
        assert_eq!(full.accuracy, 12.0_f64.sqrt() + 9.0);
        assert_eq!(full.critical_rate, 8.0);
        assert_eq!(full.critical_damage, 1.2);
        // Level 1 adds the root alone.
        assert_eq!(squadron_proficiency(attacker, 1, None).accuracy, 1.0);
        // A patrol plane counts 0.825 of its experience and one level lower:
        // sqrt(9.9) + 6, 7 x 0.8, floor(sqrt(99) + 7) = 16.
        let patrol = squadron_proficiency(KcSlotItemType3::AntiSubmarinePatrol as i64, 7, None);
        assert_eq!(patrol.accuracy, 9.9_f64.sqrt() + 6.0);
        assert!((patrol.critical_rate - 5.6).abs() < 1e-9);
        assert_eq!(patrol.critical_damage, 1.16);
    }

    #[test]
    fn a_squadron_reads_its_stored_experience() {
        let attacker = KcSlotItemType3::LandBasedAttacker as i64;
        let critical = |exp| squadron_proficiency(attacker, 5, exp).critical_damage;
        // floor(sqrt(70) + 5) = 13 and floor(sqrt(81) + 5) = 14 hundredths.
        assert!((critical(Some(70)) - 1.13).abs() < 1e-9);
        assert!((critical(Some(81)) - 1.14).abs() < 1e-9);
        assert_eq!(squadron_proficiency(attacker, 5, Some(81)).accuracy, 8.1_f64.sqrt() + 4.0);
        // Without one it reads its level's 70.
        assert!((critical(None) - 1.13).abs() < 1e-9);
    }

    #[test]
    fn each_half_of_a_vanguard_aims_by_its_own_row() {
        let front = formation_aim(6, true);
        let rear = formation_aim(6, false);
        let row = |aim: &FormationAim| [aim.shell_acc, aim.torp_acc, aim.night_acc, aim.asw_acc];
        assert_eq!(row(&front), [0.8, 0.7, 0.8, 1.0]);
        assert_eq!(row(&rear), [1.2, 0.9, 1.2, 1.1]);
        // Neither half dodges by its formation; its place does that.
        assert_eq!([front.shell_ev, front.torp_ev, front.night_ev, front.asw_ev], [1.0; 4]);
    }

    #[test]
    fn a_ship_in_vanguard_dodges_by_her_place() {
        use crate::test_utils::{first_ship_mst_by_type, sample_ship};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |ship_type, number| {
            let input = sample_ship(&codex, first_ship_mst_by_type(&codex, ship_type), 1);
            BattleRuntimeShip::new(input, false, true).in_vanguard(number, 6)
        };
        let dodge =
            |ship_type, number, kind| vanguard_dodge(&codex, &ship(ship_type, number), kind);

        assert_eq!(dodge(KcShipType::DD, 5, AttackKind::Shelling), 0.69);
        assert_eq!(dodge(KcShipType::CA, 5, AttackKind::Shelling), 0.86);
        assert_eq!(dodge(KcShipType::DD, 5, AttackKind::Night), 0.69);
        assert_eq!(dodge(KcShipType::DD, 5, AttackKind::Asw), 0.69);
        assert_eq!(dodge(KcShipType::DD, 4, AttackKind::Torpedo), 0.58);
        assert_eq!(dodge(KcShipType::CA, 4, AttackKind::Torpedo), 0.67);
        assert_eq!(dodge(KcShipType::DD, 1, AttackKind::Shelling), 0.95);

        let in_line = BattleRuntimeShip::new(
            sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 1),
            false,
            true,
        );
        assert_eq!(vanguard_dodge(&codex, &in_line, AttackKind::Shelling), 1.0);
    }

    /// A fleet in 警戒陣 aims better at destroyers with its guns, its torpedoes
    /// and at night, wherever the attacker stands.
    #[test]
    fn a_fleet_in_vanguard_aims_better_at_destroyers() {
        use crate::test_utils::{first_ship_mst_by_type, sample_ship};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |ship_type| {
            let input = sample_ship(&codex, first_ship_mst_by_type(&codex, ship_type), 1);
            BattleRuntimeShip::new(input, true, true)
        };
        let attacker = ship(KcShipType::CL).in_vanguard(1, 6);
        let (dd, ca) = (ship(KcShipType::DD), ship(KcShipType::CA));
        let bonus = |attacker, defender, kind| {
            vanguard_against_destroyers(&codex, attacker, defender, kind)
        };

        assert_eq!(bonus(&attacker, &dd, AttackKind::Torpedo), 1.2);
        assert_eq!(bonus(&attacker, &dd, AttackKind::Night), 1.1);
        assert_eq!(bonus(&attacker, &dd, AttackKind::Shelling), 1.1);
        assert_eq!(bonus(&attacker, &dd, AttackKind::Asw), 1.0);
        assert_eq!(bonus(&attacker, &ca, AttackKind::Torpedo), 1.0);
        assert_eq!(bonus(&ship(KcShipType::CL), &dd, AttackKind::Torpedo), 1.0);
    }

    #[test]
    fn evasion_bends_at_forty_and_sixty_five() {
        assert_eq!(evasion_term(40, 0, 1.0, 0), 40);
        assert_eq!(evasion_term(41, 0, 1.0, 0), 43);
        assert_eq!(evasion_term(65, 0, 1.0, 0), 55);
        assert_eq!(evasion_term(66, 0, 1.0, 0), 57);
        // 運 50 adds sqrt(100) = 10 before the curve.
        assert_eq!(evasion_term(30, 50, 1.0, 0), 40);
        // A ship at half fuel dodges 25 points worse.
        assert_eq!(evasion_term(40, 0, 1.0, 25), 15);
    }

    #[test]
    fn the_chance_stays_between_ten_and_ninety_six() {
        assert_eq!(hit_chance(30.0, 80, 1.0), 10);
        assert_eq!(hit_chance(150.0, 0, 1.0), 96);
        assert_eq!(hit_chance(90.9, 20, 1.0), 70);
        // 100 x 1.15 is 114.99999999999999 in floating point; the source still counts 115.
        assert_eq!(hit_chance(100.0 * 1.15, 20, 1.0), 95);
        // A sparkling target is hit 0.7 times as often, a tired one 1.4 times.
        assert_eq!(hit_chance(90.0, 20, 0.7), 49);
        assert_eq!(hit_chance(90.0, 80, 1.4), 14);
    }

    #[test]
    fn one_draw_decides_miss_hit_and_critical() {
        // chance 64 with factor 1.5: critical up to 12, hit up to 64.
        assert_eq!(roll(&mut Fixed(12), 64, 1.5, PlaneProficiency::NONE), HitOutcome::Critical);
        assert_eq!(roll(&mut Fixed(13), 64, 1.5, PlaneProficiency::NONE), HitOutcome::Hit);
        assert_eq!(roll(&mut Fixed(64), 64, 1.5, PlaneProficiency::NONE), HitOutcome::Hit);
        assert_eq!(roll(&mut Fixed(65), 64, 1.5, PlaneProficiency::NONE), HitOutcome::Miss);
        // Without a critical factor even a zero is a plain hit.
        assert_eq!(roll(&mut Fixed(0), 64, 0.0, PlaneProficiency::NONE), HitOutcome::Hit);
    }

    #[test]
    fn proficiency_lifts_the_chance_and_the_critical_on_the_same_draw() {
        let planes = PlaneProficiency {
            accuracy: 12.5,
            critical_rate: 8.0,
            critical_damage: 1.2,
            average_exp: 120.0,
        };
        // An aircraft's strike has no factor: the critical is the bonus alone.
        assert_eq!(roll(&mut Fixed(8), 95, 0.0, planes), HitOutcome::Critical);
        assert_eq!(roll(&mut Fixed(9), 95, 0.0, planes), HitOutcome::Hit);
        // 96 + 12.5 passes the ceiling, so nothing the die shows is a miss.
        assert_eq!(roll(&mut Fixed(100), 96, 0.0, planes), HitOutcome::Hit);
        // A carrier's shelling: floor(sqrt(76.5) x 1.3 + 8) = 19.
        assert_eq!(roll(&mut Fixed(19), 64, 1.3, planes), HitOutcome::Critical);
        assert_eq!(roll(&mut Fixed(20), 64, 1.3, planes), HitOutcome::Hit);
        assert_eq!(HitOutcome::Critical.power_with(101.0, 1.2), 181.0);
        // 1.5 x 1.2 falls a hair short of 1.8, as it does in the source.
        assert_eq!(HitOutcome::Critical.power_with(100.0, 1.2), 179.0);
        assert_eq!(planes.for_carrier_cut_in().critical_rate, 13.0);
        assert_eq!(HitOutcome::Hit.power_with(101.0, 1.2), 101.0);
    }

    #[test]
    fn a_fully_skilled_first_slot_is_worth_the_sources_figures() {
        use crate::test_utils::{
            first_ship_mst_by_type, first_slotitem_mst_by_type, sample_ship, slotitem_with_mst_id,
        };
        use emukc_model::kc2::KcShipType;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        let carrier = |stype, levels: &[Option<i64>]| {
            let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, stype), 50);
            ship.slot_items = levels
                .iter()
                .map(|alv| {
                    let mut item = slotitem_with_mst_id(bomber);
                    item.api_alv = *alv;
                    item
                })
                .collect();
            BattleRuntimeShip::from(ship)
        };

        let skilled = plane_proficiency(&codex, &carrier(KcShipType::CVL, &[Some(7)]));
        assert_eq!(skilled.accuracy, 12.0_f64.sqrt() + 9.0);
        assert_eq!(skilled.critical_rate, 8.0);
        assert_eq!(skilled.critical_damage, 1.2);

        // A second slot counts for less, and a green one only drags the average down.
        let mixed = plane_proficiency(&codex, &carrier(KcShipType::CVL, &[None, Some(7)]));
        assert_eq!(mixed.accuracy, 6.0_f64.sqrt() + 3.0);
        assert_eq!(mixed.critical_rate, 6.0);
        assert_eq!(mixed.critical_damage, 1.1);

        let green = carrier(KcShipType::CVL, &[None]);
        assert_eq!(plane_proficiency(&codex, &green), PlaneProficiency::NONE);

        // Only a carrier's shelling is flown by its aircraft.
        assert_eq!(shelling_proficiency(&codex, &carrier(KcShipType::CVL, &[Some(7)])), skilled);
        assert_eq!(
            shelling_proficiency(&codex, &carrier(KcShipType::BBV, &[Some(7)])),
            PlaneProficiency::NONE
        );
    }

    #[test]
    fn stored_experience_moves_proficiency_within_a_level() {
        use crate::test_utils::{
            first_ship_mst_by_type, first_slotitem_mst_by_type, sample_ship, slotitem_with_mst_id,
        };
        use emukc_model::kc2::KcShipType;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        let carrier = |alv: i64, exp: i64| {
            let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 50);
            let mut item = slotitem_with_mst_id(bomber);
            item.api_alv = Some(alv);
            ship.slot_items = vec![item];
            ship.aircraft_exp = vec![exp];
            plane_proficiency(&codex, &BattleRuntimeShip::from(ship))
        };

        // floor(√70 + 5) = 13, floor(√81 + 5) = 14.
        assert_eq!(carrier(5, 70).critical_damage, 1.0 + 13.0 / 100.0);
        assert_eq!(carrier(5, 81).critical_damage, 1.0 + 14.0 / 100.0);
        assert_eq!(carrier(5, 70).accuracy, 7.0_f64.sqrt() + 4.0);
        assert_eq!(carrier(5, 81).accuracy, 8.1_f64.sqrt() + 6.0);

        // At the top level √100 and √120 floor alike; only the accuracy tells them apart.
        assert_eq!(carrier(7, 100).critical_damage, carrier(7, 120).critical_damage);
        assert_eq!(carrier(7, 100).accuracy, 10.0_f64.sqrt() + 9.0);
        assert_eq!(carrier(7, 120).accuracy, 12.0_f64.sqrt() + 9.0);

        // The source counts any experience, even below the first level.
        let novice = carrier(0, 5);
        assert_eq!(novice.critical_damage, 1.0 + 2.0 / 100.0);
        assert_eq!(novice.critical_rate, 0.0);
        assert_eq!(novice.average_exp, 5.0);
    }

    #[test]
    fn aircraft_proficiency_reaches_the_day_attack_on_a_submarine_but_not_the_opening() {
        use crate::test_utils::{
            first_ship_mst_by_type, first_slotitem_mst_by_type, sample_ship, slotitem_with_mst_id,
        };

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);
        let ship = |stype, alv: Option<i64>| {
            let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, stype), 50);
            let mut item = slotitem_with_mst_id(bomber);
            item.api_alv = alv;
            ship.slot_items = vec![item];
            BattleRuntimeShip::from(ship)
        };
        for stype in
            [KcShipType::CAV, KcShipType::BBV, KcShipType::AV, KcShipType::LHA, KcShipType::CVL]
        {
            assert_eq!(asw_proficiency(&codex, &ship(stype, Some(7))).critical_damage, 1.2);
        }
        for stype in [KcShipType::CV, KcShipType::DD, KcShipType::CL] {
            assert_eq!(asw_proficiency(&codex, &ship(stype, Some(7))), PlaneProficiency::NONE);
        }

        let submarine = BattleRuntimeShip::from(sample_ship(
            &codex,
            first_ship_mst_by_type(&codex, KcShipType::SS),
            50,
        ));
        let roll = |alv, aim: Aim, seed| {
            let attacker = ship(KcShipType::CAV, alv);
            roll_attack(&codex, &mut SeededRng::new(seed), &attacker, &submarine, aim)
        };
        let aim = Aim::new(AttackKind::Asw, 1, 1);
        // In the opening the skilled ship rolls as the green one does, draw for draw.
        assert!(
            (0..200).all(|seed| roll(Some(7), aim.in_the_opening(), seed) == roll(None, aim, seed))
        );
        assert!((0..200).any(|seed| roll(Some(7), aim, seed) != roll(None, aim, seed)));
    }

    #[test]
    fn the_equipment_bonus_accuracy_reaches_day_shelling_alone() {
        use crate::test_utils::{sample_ship, slotitem_with_mst_id};
        const SUZUYA_CARRIER_KAI_NI: i64 = 508;
        // 天山一二型甲改(空六号電探改装備機): 命中 1 of its own, +1 a copy on her.
        const TENZAN_RADAR: i64 = 373;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |gear: &[i64]| {
            let mut ship = sample_ship(&codex, SUZUYA_CARRIER_KAI_NI, 99);
            ship.ship.api_cond = 50;
            ship.slot_items = gear.iter().copied().map(slotitem_with_mst_id).collect();
            BattleRuntimeShip::new(ship, true, true)
        };
        let (bare, armed) = (ship(&[]), ship(&[TENZAN_RADAR, TENZAN_RADAR]));
        // Sparkling (1.2, torpedo 1.3), a formation of 1.1 and a cut-in of 1.5 multiply it all.
        let gain = |kind| {
            let aim = Aim::new(kind, 1, 1).with_modifier(1.5);
            hit_value(&codex, &armed, aim, 1.1) - hit_value(&codex, &bare, aim, 1.1)
        };
        // By day the two copies add 2 of their own and 2 of bonus.
        assert!((gain(AttackKind::Shelling) - 4.0 * 1.2 * 1.1 * 1.5).abs() < 1e-9);
        // At night and with torpedoes only their own 2; at a submarine nothing.
        assert!((gain(AttackKind::Night) - 2.0 * 1.2 * 1.1 * 1.5).abs() < 1e-9);
        assert!((gain(AttackKind::Torpedo) - 2.0 * 1.3 * 1.1 * 1.5).abs() < 1e-9);
        assert_eq!(gain(AttackKind::Asw), 0.0);
        // Night has the same morale, so the difference is the bonus alone.
        let bonus = gain(AttackKind::Shelling) - gain(AttackKind::Night);
        assert!((bonus - 2.0 * 1.2 * 1.1 * 1.5).abs() < 1e-9);
    }

    #[test]
    fn a_star_shell_adds_five_before_the_scout_multiplies() {
        let night = Aim::new(AttackKind::Night, 1, 1);
        assert_eq!(night.base(), 69.0);
        assert_eq!(night.lit_by_star_shell(true).base(), 74.0);
        // (69 + 5) x 1.1 under a scout of accuracy 1, not 69 x 1.1 + 5.
        let scout = Some(NightContact::of(102, 1));
        assert_eq!(night.under_contact(scout).base(), 69.0 * 1.1);
        assert_eq!(night.under_contact(scout).lit_by_star_shell(true).base(), 74.0 * 1.1);
        // An attack on a submarine rolls as by day, with no star shell in it.
        assert_eq!(Aim::new(AttackKind::Asw, 1, 1).lit_by_star_shell(true).base(), 80.0);
    }

    #[test]
    fn a_searchlight_on_her_own_side_adds_seven_to_hit_after_the_multipliers() {
        use crate::test_utils::{first_ship_mst_by_type, sample_ship};
        use emukc_model::kc2::KcShipType;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |stype, level| {
            BattleRuntimeShip::from(sample_ship(
                &codex,
                first_ship_mst_by_type(&codex, stype),
                level,
            ))
        };
        let (attacker, target) = (ship(KcShipType::DD, 1), ship(KcShipType::DD, 99));
        // The highest face of the die that still hits is the chance to hit.
        let chance = |aim: Aim| {
            (0..100)
                .filter(|face| {
                    roll_attack(&codex, &mut Fixed(*face), &attacker, &target, aim)
                        != HitOutcome::Miss
                })
                .max()
                .unwrap()
        };
        // 単横陣 at night multiplies by 0.8 and a cut-in by 1.5; the 7 comes after both.
        let aim = Aim::new(AttackKind::Night, 5, 1).with_modifier(1.5);
        let dark = chance(aim);
        assert!(dark > 10 && dark + 7 < 96, "the base chance {dark} must sit between the bounds");
        assert_eq!(chance(aim.lit_by_searchlight(true)), dark + 7);
        // By day, or at a submarine, a searchlight adds nothing.
        let day = Aim::new(AttackKind::Shelling, 1, 1);
        assert_eq!(chance(day.lit_by_searchlight(true)), chance(day));
        let asw = Aim::new(AttackKind::Asw, 1, 1);
        assert_eq!(chance(asw.lit_by_searchlight(true)), chance(asw));
    }

    #[test]
    fn a_searchlight_strips_most_of_its_carriers_night_evasion() {
        use crate::test_utils::{first_ship_mst_by_type, sample_ship, slotitem_with_mst_id};
        use emukc_model::kc2::KcShipType;
        const SURFACE_RADAR: i64 = 28;
        const LOOKOUT: i64 = 129;
        const SEARCHLIGHT: i64 = 74;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |stype, gear: &[i64]| {
            let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, stype), 50);
            ship.slot_items = gear.iter().copied().map(slotitem_with_mst_id).collect();
            BattleRuntimeShip::from(ship)
        };
        // A heavy cruiser dodges 5 more at night; a destroyer with a radar and a lookout 10.
        assert_eq!(night_evasion(&codex, &ship(KcShipType::CA, &[]), 1.2), (1.2, 5));
        assert_eq!(night_evasion(&codex, &ship(KcShipType::CAV, &[]), 1.0), (1.0, 5));
        let picket = ship(KcShipType::DD, &[SURFACE_RADAR, LOOKOUT]);
        assert_eq!(night_evasion(&codex, &picket, 1.0), (1.0, 10));
        assert_eq!(night_evasion(&codex, &ship(KcShipType::DD, &[SURFACE_RADAR]), 1.0), (1.0, 0));
        // Her own searchlight leaves her a fifth of both.
        let lit = ship(KcShipType::CA, &[SEARCHLIGHT]);
        assert_eq!(night_evasion(&codex, &lit, 1.2), (1.2 * 0.2, 1));
        let lit = ship(KcShipType::DD, &[SURFACE_RADAR, LOOKOUT, SEARCHLIGHT]);
        assert_eq!(night_evasion(&codex, &lit, 1.0), (0.2, 2));
    }

    #[test]
    fn a_roll_draws_exactly_once() {
        let mut one = SeededRng::new(7);
        let mut two = SeededRng::new(7);
        roll(&mut one, 50, 1.3, PlaneProficiency::NONE);
        two.roll_range(0, 100);
        assert_eq!(one.roll_range(0, 1000), two.roll_range(0, 1000));
        // Proficiency moves the thresholds, not the number of draws.
        let skilled = PlaneProficiency {
            accuracy: 12.5,
            critical_rate: 8.0,
            critical_damage: 1.2,
            average_exp: 120.0,
        };
        roll(&mut one, 50, 1.3, skilled);
        two.roll_range(0, 100);
        assert_eq!(one.roll_range(0, 1000), two.roll_range(0, 1000));
    }

    #[test]
    fn a_critical_is_half_again_after_the_cap() {
        assert_eq!(HitOutcome::Critical.power(101.0), 151.0);
        assert_eq!(HitOutcome::Hit.power(101.0), 101.0);
        assert_eq!(HitOutcome::Miss.power(101.0), 0.0);
        assert_eq!(
            [HitOutcome::Miss.cl(), HitOutcome::Hit.cl(), HitOutcome::Critical.cl()],
            [0, 1, 2]
        );
    }

    #[test]
    fn a_countered_formation_loses_its_accuracy() {
        assert!(formation_countered(2, 5));
        assert!(formation_countered(4, 1));
        assert!(formation_countered(5, 4));
        assert!(!formation_countered(1, 4));
    }

    /// A destroyer with a gun, a torpedo and a sonar at `stars`.
    fn improved_destroyer(codex: &Codex, stars: [i64; 3]) -> BattleRuntimeShip {
        use crate::test_utils::{
            first_ship_mst_by_type, first_slotitem_mst_by_type, sample_ship, slotitem_with_mst_id,
        };
        let mut ship = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::DD), 50);
        ship.ship.api_cond = 49;
        let gear = [
            KcSlotItemType3::SmallCaliberMainGun,
            KcSlotItemType3::Torpedo,
            KcSlotItemType3::Sonar,
        ];
        ship.slot_items = gear
            .iter()
            .zip(stars)
            .map(|(&kind, stars)| {
                let mut item = slotitem_with_mst_id(first_slotitem_mst_by_type(codex, kind));
                item.api_level = stars;
                item
            })
            .collect();
        BattleRuntimeShip::new(ship, true, true)
    }

    /// Each attack takes its own improvement accuracy into the flat accuracy, before the
    /// multipliers; the torpedo takes it rounded down.
    #[test]
    fn improved_equipment_aims_better() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let (plain, improved) =
            (improved_destroyer(&codex, [0, 0, 0]), improved_destroyer(&codex, [10, 3, 9]));
        let gain = |kind| {
            let aim = Aim::new(kind, 1, 1).with_modifier(1.5);
            hit_value(&codex, &improved, aim, 1.1) - hit_value(&codex, &plain, aim, 1.1)
        };
        let root10 = 10f64.sqrt();
        assert!((gain(AttackKind::Shelling) - root10 * 1.1 * 1.5).abs() < 1e-9);
        let night = 1.3 * root10 + 1.3 * 3f64.sqrt();
        assert!((gain(AttackKind::Night) - night * 1.1 * 1.5).abs() < 1e-9);
        // 2√3 ≈ 3.46 counts 3.
        assert!((gain(AttackKind::Torpedo) - 3.0 * 1.1 * 1.5).abs() < 1e-9);
        assert!((gain(AttackKind::Asw) - 3.9 * 1.1 * 1.5).abs() < 1e-9);
    }

    /// An improved engine dodges shelling better, an improved sonar torpedoes, after the curve.
    #[test]
    fn improved_equipment_dodges_better() {
        use crate::test_utils::{first_slotitem_mst_by_type, slotitem_with_mst_id};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // A sonar and an engine at the stars given.
        let ship = |sonar, engine_stars| {
            let mut ship = improved_destroyer(&codex, [0, 0, sonar]);
            let mut engine = slotitem_with_mst_id(first_slotitem_mst_by_type(
                &codex,
                KcSlotItemType3::EngineBoost,
            ));
            engine.api_level = engine_stars;
            ship.slot_items.push(engine);
            ship
        };
        let (plain, improved) = (ship(0, 0), ship(9, 6));
        let gain =
            |kind| dodge(&codex, &improved, kind, 1.0, 0) - dodge(&codex, &plain, kind, 1.0, 0);
        // 1.5√6 ≈ 3.67 and 1.5·3 = 4.5, on an evasion that is a whole number.
        assert_eq!(gain(AttackKind::Shelling), 3);
        assert_eq!(gain(AttackKind::Torpedo), 4);
        assert_eq!(gain(AttackKind::Night), 0);
        assert_eq!(gain(AttackKind::Asw), 0);
    }

    /// The gun fit joins day shelling and night attacks after `hitRate`'s multipliers: by day a
    /// cut-in's multiplier (`accMod2`) scales it, a special attack's does not, and at night
    /// nothing does. Torpedoes and depth charges leave it out.
    #[test]
    fn the_gun_fit_comes_after_the_hit_rate_multipliers() {
        use crate::test_utils::{sample_ship, slotitem_with_mst_id};
        const KONGOU: i64 = 78;
        const MYOUKOU: i64 = 62;
        const GUN_356CM_TWIN: i64 = 7;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // One ship with one 35.6cm gun, and only the master differs: it fits 金剛 by 7 and
        // 妙高 not at all.
        let ship = |mst_id| {
            let mut ship = BattleRuntimeShip::from(sample_ship(&codex, KONGOU, 50));
            ship.ship.api_ship_id = mst_id;
            ship.slot_items = vec![slotitem_with_mst_id(GUN_356CM_TWIN)];
            ship
        };
        let (fitting, plain) = (ship(KONGOU), ship(MYOUKOU));
        assert_eq!(bonus_accuracy(&codex, &fitting), bonus_accuracy(&codex, &plain));
        let gain =
            |aim: Aim| hit_value(&codex, &fitting, aim, 1.1) - hit_value(&codex, &plain, aim, 1.1);
        let close = |gain: f64, want: f64| (gain - want).abs() < 1e-9;
        let day = Aim::new(AttackKind::Shelling, 1, 1);
        assert!(close(gain(day), 7.0));
        assert!(close(gain(day.with_day_cut_in(1.5)), 7.0 * 1.5));
        assert!(close(gain(day.with_modifier(1.5)), 7.0));
        assert!(close(gain(Aim::new(AttackKind::Night, 1, 1).with_modifier(1.5)), 7.0));
        assert_eq!(gain(Aim::new(AttackKind::Torpedo, 1, 1)), 0.0);
        assert_eq!(gain(Aim::new(AttackKind::Asw, 1, 1)), 0.0);
    }
}
