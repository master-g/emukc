//! Whether an attack lands, and whether it lands as a critical.
//!
//! Transcribed from `KC3Kai/kancolle-replay` `kcsim.js`: `hitRate`,
//! `accuracyAndCrit`, `rollHit`, `formationCountered` and the formation table
//! at the top of that file, with `Ship.moraleMod` / `moraleModEv` from
//! `kcships.js`. The arithmetic is kept in whole percent, as the source does
//! after its `floor(round(x * 1e6) / 1e4)` step.
//!
//! Aircraft proficiency follows `Ship.updateProficiencyBonus` in `kcships.js`.
//!
//! Left out, each a correction the source applies on top of what is here:
//! gun fit, 改修, the combined-fleet accuracy terms, 警戒陣 by position, smoke,
//! balloons, PT imps and event bonuses.

use emukc_model::{
    codex::Codex,
    kc2::{
        KcSlotItemType3,
        start2::{ApiMstShip, ApiMstSlotitem},
    },
};

use crate::damage::is_cv_type;
use crate::random::BattleRng;
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
        if level == 0 {
            continue;
        }
        let mut exp = PROFICIENCY_EXP[level];
        if patrol {
            exp *= PATROL_EXP_SHARE;
            level -= 1;
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

/// The proficiency a ship's shelling takes: a carrier's, whose shelling is
/// flown by its aircraft, and no one else's.
pub(crate) fn shelling_proficiency(codex: &Codex, ship: &BattleRuntimeShip) -> PlaneProficiency {
    if is_cv_type(codex, ship) {
        plane_proficiency(codex, ship)
    } else {
        PlaneProficiency::NONE
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
            // ponytail: star shells (+5) and night contact (x1.1 to x1.2) are
            // not counted; add them when the night battle rolls them.
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
/// column stands in, as `ASW()` does there.
fn formation_aim(formation_id: i64) -> FormationAim {
    match formation_id {
        2 => aim(1.2, 0.8, 0.9, 1.2, 1.0, 1.0, 1.0, 1.0),
        3 => aim(1.0, 0.4, 0.7, 1.0, 1.1, 1.1, 1.0, 1.0),
        4 => aim(1.2, 0.75, 0.9, 1.2, 1.4, 1.3, 1.3, 1.3),
        5 => aim(1.2, 0.3, 0.8, 1.2, 1.3, 1.4, 1.2, 1.1),
        // ponytail: 警戒陣 has one row for the front half of the fleet and one
        // for the rear; this is the rear row. Split by position when a ship
        // knows its place in the line.
        6 => aim(1.2, 0.9, 1.2, 1.1, 1.0, 1.0, 1.0, 1.0),
        11 => aim(0.9, 0.75, 0.8, 1.25, 1.1, 1.0, 1.0, 1.0),
        12 => aim(1.0, 1.0, 0.9, 1.0, 1.2, 1.0, 1.0, 1.0),
        13 => aim(0.8, 0.35, 0.7, 1.1, 1.1, 1.0, 1.1, 1.0),
        14 => aim(1.1, 1.2, 1.0, 0.7, 1.0, 1.0, 1.0, 1.0),
        _ => aim(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
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

/// The chance to hit in percent, between the floor and the ceiling.
pub(crate) fn hit_chance(hit: f64, dodge: i64, target_morale: f64) -> i64 {
    let chance = (hit.floor() as i64 - dodge).max(HIT_FLOOR) as f64 * target_morale;
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
    /// A cut-in's own accuracy multiplier, 1 for a plain attack.
    pub modifier: f64,
    /// Accuracy the phase adds on its own: a fifth of the torpedo's power.
    pub flat: f64,
    /// Whether this is a carrier's cut-in, which has its own critical rate.
    pub carrier_cut_in: bool,
}

impl Aim {
    pub(crate) fn new(kind: AttackKind, attacker_formation: i64, defender_formation: i64) -> Self {
        Self {
            kind,
            attacker_formation,
            defender_formation,
            modifier: 1.0,
            flat: 0.0,
            carrier_cut_in: false,
        }
    }

    pub(crate) fn as_carrier_cut_in(mut self, carrier_cut_in: bool) -> Self {
        self.carrier_cut_in = carrier_cut_in;
        self
    }

    pub(crate) fn with_modifier(mut self, modifier: f64) -> Self {
        self.modifier = modifier;
        self
    }

    pub(crate) fn with_flat(mut self, flat: f64) -> Self {
        self.flat = flat;
        self
    }
}

/// Roll a ship's attack on another ship.
pub(crate) fn roll_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    aim: Aim,
) -> HitOutcome {
    let own = formation_aim(aim.attacker_formation);
    let theirs = formation_aim(aim.defender_formation);
    let countered = formation_countered(aim.attacker_formation, aim.defender_formation);
    let (formation_acc, formation_ev) = match aim.kind {
        AttackKind::Shelling if countered => (1.0, theirs.shell_ev),
        AttackKind::Shelling => (own.shell_acc, theirs.shell_ev),
        AttackKind::Asw if countered => (1.0, theirs.asw_ev),
        AttackKind::Asw => (own.asw_acc, theirs.asw_ev),
        AttackKind::Torpedo => (own.torp_acc, theirs.torp_ev),
        AttackKind::Night => (own.night_acc, theirs.night_ev),
    };

    let level = attacker.ship.api_lv.max(0) as f64;
    let luck = attacker.ship.api_lucky[0].max(0) as f64;
    let hit = (aim.kind.base()
        + 2.0 * level.sqrt()
        + 1.5 * luck.sqrt()
        + equipment_accuracy(codex, attacker, aim.kind)
        + aim.flat)
        * attacker_morale(attacker, aim.kind)
        * formation_acc
        * aim.modifier;
    let dodge = evasion_term(
        defender.ship.api_kaihi[0],
        defender.ship.api_lucky[0],
        formation_ev,
        fuel_shortfall(codex, defender),
    );
    let chance = hit_chance(hit, dodge, target_morale(defender));
    let planes = match aim.kind {
        AttackKind::Shelling if aim.carrier_cut_in => {
            shelling_proficiency(codex, attacker).for_carrier_cut_in()
        }
        AttackKind::Shelling => shelling_proficiency(codex, attacker),
        _ => PlaneProficiency::NONE,
    };
    roll(rng, chance, aim.kind.critical_factor(), planes)
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
    let dodge = (dodge as f64 * evasion_modifier).floor() as i64;
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
    fn a_roll_draws_exactly_once() {
        let mut one = SeededRng::new(7);
        let mut two = SeededRng::new(7);
        roll(&mut one, 50, 1.3, PlaneProficiency::NONE);
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
}
