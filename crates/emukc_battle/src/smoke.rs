//! 煙幕 (smoke screens): whether a fleet's smoke generators lay one when the player asks, how
//! thick, and what it does to the attacks made in it.
//!
//! The source (`KC3Kai/kancolle-replay` `69097abc`) rolls it where this does (`getSmokeType`,
//! after the engagement, before any phase) but ships with no chance and no effect
//! (`smokeChance` 0, every `smokeMod*` 1). The rates here are wikiwiki's 「発煙装置(煙幕)」
//! 発動率推定式, its unsettled n ≥ 3 coefficient taken at 0.04; the accuracy multipliers are the
//! source's own estimates, `SIMCONSTS.smokeEst` (plan 2026-10-19-009).

use emukc_model::{
    codex::Codex,
    kc2::{KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::accuracy::AttackKind;
use crate::random::BattleRng;
use crate::targeting::is_radar_type;
use crate::types::BattleRuntimeShip;

/// 発煙装置(煙幕).
const SMOKE: i64 = 500;
/// 発煙装置改(煙幕), which counts for two.
const SMOKE_KAI: i64 = 501;

/// How thick a smoke screen the fleet lays, 1 to 3, or 0 for none. The source's gate: some ship
/// afloat carries a smoke generator, and a single fleet has four ships afloat. Draws once when
/// both hold and nothing otherwise.
pub(crate) fn roll(rng: &mut impl BattleRng, fleet: &[BattleRuntimeShip], combined: bool) -> i64 {
    let afloat: Vec<&BattleRuntimeShip> = fleet.iter().filter(|ship| ship.is_alive()).collect();
    let (mut n, mut stars) = (0, 0.0);
    for item in afloat.iter().flat_map(|ship| &ship.slot_items) {
        let level = item.api_level.max(0) as f64;
        match item.api_slotitem_id {
            SMOKE => (n, stars) = (n + 1, stars + 0.3 * level),
            SMOKE_KAI => (n, stars) = (n + 2, stars + 0.5 * level),
            _ => {}
        }
    }
    if n == 0 || (!combined && afloat.len() < 4) {
        return 0;
    }
    let luck = fleet.first().map_or(0, |flagship| flagship.ship.api_lucky[0]);
    let [single, double, triple] = rates(luck, n, stars);
    let r = rng.random_f64_range(0.0, 1.0);
    if r < triple {
        3
    } else if r < triple + double {
        2
    } else if r < triple + double + single {
        1
    } else {
        0
    }
}

/// The chances of a single, double and triple screen (wikiwiki's Pg1–Pg3), from the flagship's
/// luck, `n` (one for each smoke generator, two for each improved one) and the stars' share
/// (0.3 for each star of the one, 0.5 of the other).
fn rates(luck: i64, n: i64, stars: f64) -> [f64; 3] {
    // ⌈√luck + stars⌉, rounded first so that 0.3 × 10 does not come to 3.0000000000000004.
    let k = (((luck.max(0) as f64).sqrt() + stars) * 1e6).round() / 1e6;
    let k = k.ceil();
    let fail = if luck >= 1 && n >= 3 {
        0.0
    } else {
        (3.2 - 0.2 * k - n as f64).clamp(0.0, 1.0)
    };
    match n {
        ..=0 => [0.0; 3],
        1 => [1.0 - fail, 0.0, 0.0],
        2 => {
            let double = ((1.0 - fail) * 0.05 * (k + 2.0)).min(1.0 - fail);
            [1.0 - fail - double, double, 0.0]
        }
        _ => {
            let triple = (0.04 * k + 0.15 * (n - 3) as f64).min(1.0);
            let double = (1.0 - triple).min(0.3);
            [(1.0 - double - triple - fail).max(0.0), double, triple]
        }
    }
}

/// What a smoke screen of `level` leaves of an attack's accuracy (`smokeEst`): the player's
/// shells keep a hundredth, a third or a quarter with a radar; the enemy's half, or most of it
/// with a radar; the player's depth charges a quarter, her torpedoes under half. Aircraft and
/// night attacks are not smoked.
pub(crate) fn accuracy(
    codex: &Codex,
    level: i64,
    kind: AttackKind,
    attacker: &BattleRuntimeShip,
) -> f64 {
    let Some(at) = usize::try_from(level - 1).ok().filter(|at| *at < 3) else {
        return 1.0;
    };
    let radar = || {
        attacker.slot_items.iter().any(|item| {
            codex
                .find::<ApiMstSlotitem>(&item.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(is_radar_type)
        })
    };
    let row: [f64; 3] = match (kind, attacker.is_friendly) {
        (AttackKind::Shelling, true) if radar() => [0.35, 0.25, 0.25],
        (AttackKind::Shelling, true) => [0.01, 0.01, 0.01],
        (AttackKind::Shelling, false) if radar() => [0.9, 0.83, 0.75],
        (AttackKind::Shelling, false) => [0.5, 0.5, 0.5],
        (AttackKind::Asw, true) => [0.25, 0.25, 0.25],
        (AttackKind::Torpedo, true) => [0.45, 0.42, 0.42],
        (AttackKind::Torpedo, false) => [0.7, 0.6, 0.5],
        (AttackKind::Asw, false) | (AttackKind::Night, _) => [1.0, 1.0, 1.0],
    };
    row[at]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{first_ship_mst_by_type, sample_ship, slotitem_with_mst_id};
    use emukc_model::kc2::KcShipType;

    /// Draws `self.0` of the range and counts the draws.
    struct At(f64, usize);

    impl BattleRng for At {
        fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
            self.1 += 1;
            min + (max - min) * self.0
        }

        fn roll_range_impl(&mut self, min: i64, _max: i64) -> i64 {
            self.1 += 1;
            min
        }
    }

    fn fleet(
        codex: &Codex,
        ships: usize,
        luck: i64,
        gear: &[(i64, i64)],
    ) -> Vec<BattleRuntimeShip> {
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);
        (0..ships)
            .map(|at| {
                let mut input = sample_ship(codex, dd, 50);
                input.ship.api_lucky[0] = luck;
                if at == 0 {
                    input.slot_items = gear
                        .iter()
                        .map(|&(id, level)| {
                            let mut item = slotitem_with_mst_id(id);
                            item.api_level = level;
                            item
                        })
                        .collect();
                }
                BattleRuntimeShip::new(input, true, true)
            })
            .collect()
    }

    #[test]
    fn the_rates_follow_the_wikis_estimate() {
        let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-9);
        // One generator, luck 100: k = 10, so 1 - (3.2 - 2 - 1) = 0.8 lays one.
        assert!(close(rates(100, 1, 0.0), [0.8, 0.0, 0.0]));
        // Two, luck 26: k = 6 and it never fails; 0.05 × 8 of it is double.
        assert!(close(rates(26, 2, 0.0), [0.6, 0.4, 0.0]));
        // Luck 25: k = 5, failing 0.2 of the time.
        assert!(close(rates(25, 2, 0.0), [0.8 - 0.8 * 0.35, 0.8 * 0.35, 0.0]));
        // Ten stars on one generator add 3 to k: ⌈√1 + 3⌉ = 4, failing 0.4; a tenth more is 5.
        assert!(close(rates(1, 2, 3.0), [0.6 - 0.6 * 0.3, 0.6 * 0.3, 0.0]));
        assert!(close(rates(1, 2, 3.1), [0.8 - 0.8 * 0.35, 0.8 * 0.35, 0.0]));
        // Three or more never fail: k = 4 gives 0.16 triple, 0.3 double.
        assert!(close(rates(16, 3, 0.0), [0.54, 0.3, 0.16]));
    }

    #[test]
    fn a_fleet_lays_smoke_only_with_a_generator_and_four_ships() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let laid = |ships, luck, gear: &[(i64, i64)], combined, at| {
            let mut rng = At(at, 0);
            let level = roll(&mut rng, &fleet(&codex, ships, luck, gear), combined);
            (level, rng.1)
        };
        // Nothing to lay it with, or too few ships: no draw at all.
        assert_eq!(laid(6, 50, &[], false, 0.0), (0, 0));
        assert_eq!(laid(3, 50, &[(SMOKE_KAI, 0), (SMOKE, 0)], false, 0.0), (0, 0));
        // A combined fleet has no such gate.
        assert_eq!(laid(3, 50, &[(SMOKE_KAI, 0), (SMOKE, 0)], true, 0.0), (3, 1));
        // n = 3, luck 50 (k = 8): 0.32 triple, 0.3 double, the rest single, never none.
        assert_eq!(laid(6, 50, &[(SMOKE_KAI, 0), (SMOKE, 0)], false, 0.31), (3, 1));
        assert_eq!(laid(6, 50, &[(SMOKE_KAI, 0), (SMOKE, 0)], false, 0.5), (2, 1));
        assert_eq!(laid(6, 50, &[(SMOKE_KAI, 0), (SMOKE, 0)], false, 0.999), (1, 1));
        // One generator at luck 9 (k = 3) always fails, but still draws.
        assert_eq!(laid(6, 9, &[(SMOKE, 0)], false, 0.0), (0, 1));
    }

    #[test]
    fn smoke_keeps_the_sources_estimated_share_of_each_attack() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ship = |friendly, gear: &[i64]| {
            let mut input = sample_ship(&codex, dd, 50);
            input.slot_items = gear.iter().map(|&id| slotitem_with_mst_id(id)).collect();
            BattleRuntimeShip::new(input, friendly, true)
        };
        // 28 is a 22号対水上電探.
        let (bare, radar) = (ship(true, &[]), ship(true, &[28]));
        let enemy = ship(false, &[]);
        let at = |level, kind, ship| accuracy(&codex, level, kind, ship);
        assert_eq!(at(0, AttackKind::Shelling, &bare), 1.0);
        assert_eq!(at(1, AttackKind::Shelling, &bare), 0.01);
        assert_eq!(at(2, AttackKind::Shelling, &radar), 0.25);
        assert_eq!(at(3, AttackKind::Shelling, &enemy), 0.5);
        assert_eq!(at(1, AttackKind::Asw, &bare), 0.25);
        assert_eq!(at(1, AttackKind::Asw, &enemy), 1.0);
        assert_eq!(at(3, AttackKind::Torpedo, &enemy), 0.5);
        assert_eq!(at(3, AttackKind::Night, &bare), 1.0);
    }
}
