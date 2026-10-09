//! Whether an attack lands, and whether it lands as a critical.
//!
//! Transcribed from `KC3Kai/kancolle-replay` `kcsim.js`: `hitRate`,
//! `accuracyAndCrit`, `rollHit`, `formationCountered` and the formation table
//! at the top of that file, with `Ship.moraleMod` / `moraleModEv` from
//! `kcships.js`. The arithmetic is kept in whole percent, as the source does
//! after its `floor(round(x * 1e6) / 1e4)` step.
//!
//! Left out, each a correction the source applies on top of what is here:
//! aircraft proficiency, gun fit, 改修, the combined-fleet accuracy terms, 警戒陣
//! by position, smoke, balloons, PT imps and event bonuses.

use emukc_model::{
    codex::Codex,
    kc2::{
        KcSlotItemType3,
        start2::{ApiMstShip, ApiMstSlotitem},
    },
};

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
        match self {
            Self::Miss => 0.0,
            Self::Hit => capped,
            Self::Critical => (capped * CRITICAL_MODIFIER).floor(),
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
pub(crate) fn roll(rng: &mut impl BattleRng, chance: i64, critical_factor: f64) -> HitOutcome {
    let critical = ((chance as f64).sqrt() * critical_factor).floor() as i64;
    let drawn = rng.roll_range(0, 100);
    // The source compares with `<=` on both steps.
    if critical_factor > 0.0 && drawn <= critical {
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
}

impl Aim {
    pub(crate) fn new(kind: AttackKind, attacker_formation: i64, defender_formation: i64) -> Self {
        Self {
            kind,
            attacker_formation,
            defender_formation,
            modifier: 1.0,
            flat: 0.0,
        }
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
    roll(rng, chance, aim.kind.critical_factor())
}

/// Roll an aircraft's strike on a ship. Aircraft hit at a fixed rate that no
/// formation changes, and score no critical without proficiency.
// ponytail: proficiency is not modelled, so a strike never rolls a critical;
// add its accuracy and critical terms here when 熟練度 reaches the simulation.
pub(crate) fn roll_strike(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defender: &BattleRuntimeShip,
    hit_percent: f64,
    evasion_modifier: f64,
) -> HitOutcome {
    let dodge = evasion_term(
        defender.ship.api_kaihi[0],
        defender.ship.api_lucky[0],
        1.0,
        fuel_shortfall(codex, defender),
    );
    let dodge = (dodge as f64 * evasion_modifier).floor() as i64;
    roll(rng, hit_chance(hit_percent, dodge, target_morale(defender)), 0.0)
}

/// Roll a strike on something that cannot dodge: the air base.
pub(crate) fn roll_strike_on_base(rng: &mut impl BattleRng, hit_percent: f64) -> HitOutcome {
    roll(rng, hit_chance(hit_percent, 0, 1.0), 0.0)
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
        assert_eq!(roll(&mut Fixed(12), 64, 1.5), HitOutcome::Critical);
        assert_eq!(roll(&mut Fixed(13), 64, 1.5), HitOutcome::Hit);
        assert_eq!(roll(&mut Fixed(64), 64, 1.5), HitOutcome::Hit);
        assert_eq!(roll(&mut Fixed(65), 64, 1.5), HitOutcome::Miss);
        // Without a critical factor even a zero is a plain hit.
        assert_eq!(roll(&mut Fixed(0), 64, 0.0), HitOutcome::Hit);
    }

    #[test]
    fn a_roll_draws_exactly_once() {
        let mut one = SeededRng::new(7);
        let mut two = SeededRng::new(7);
        roll(&mut one, 50, 1.3);
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
