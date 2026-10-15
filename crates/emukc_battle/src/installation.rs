//! What a ship's equipment does against a land target.
//!
//! Ported by hand from `Ship.prototype.loadEquips` in `kcships.js` of
//! `KC3Kai/kancolle-replay` at `69097abc` (lines 429-965), with the source's
//! `MECHANICS.installRevamp` and `panzerIIIBuff` on, as they are by default.
//! The kind of target comes from [`installation_type`]; the power formulas that
//! read these figures are the source's `shellPower` and `NBPower`.

use emukc_model::{codex::Codex, kc2::KcSlotItemType3, kc2::start2::ApiMstSlotitem};

use crate::targeting::{installation_type, target_class};
use crate::types::{BattleRuntimeShip, TargetClass};

/// The landing craft the source counts as its second class (`B_LC2`): 大発動艇(八九式中戦車&陸戦隊),
/// 特大発動艇+一式砲戦車, 特大発動艇+Ⅲ号戦車J型.
const LC2_IDS: [i64; 3] = [166, 449, 514];
/// 特大発動艇+戦車第11連隊.
const TDH11_ID: i64 = 230;
/// 特大発動艇, 特大発動艇+Ⅲ号戦車J型.
const TDH_IDS: [i64; 2] = [193, 514];
const MORTAR_ID: i64 = 346;
const MORTAR_C_ID: i64 = 347;
const ROCKET4_ID: i64 = 348;
const ROCKET4_C_ID: i64 = 349;
const M4A1_ID: i64 = 355;
/// 装甲艇(AB艇).
const SOUKOUTEI_ID: i64 = 408;
const ARMED_DAIHATSU_ID: i64 = 409;
/// 大発動艇(II号戦車/北アフリカ仕様).
const PANZER2_ID: i64 = 436;
/// 特大発動艇+一式砲戦車.
const TOKU_T1_ID: i64 = 449;
/// 特大発動艇+Ⅲ号戦車(北アフリカ仕様), which the source's `panzerIIIBuff` moves to the second class.
const PANZER3_ID: i64 = 482;
const PANZER3J_ID: i64 = 514;
const CHIHA_ID: i64 = 494;
const CHIHA_KAI_ID: i64 = 495;
/// 特四式内火艇 and 改.
const T4_IDS: [i64; 2] = [525, 526];
const T4K_ID: i64 = 526;
/// 大発動艇(R35&フランス兵).
const R35_ID: i64 = 576;
/// WG42 (Wurfgerät 42).
const WG_ID: i64 = 126;
/// 陸軍歩兵部隊, 九七式中戦車(チハ), 九七式中戦車 新砲塔(チハ改), 陸軍歩兵部隊+チハ改.
const ARMY_INFANTRY_ID: i64 = 496;
const ARMY_CHIHA_ID: i64 = 497;
const ARMY_CHIHA_KAI_ID: i64 = 498;
const ARMY_INFANTRY_CHIHA_ID: i64 = 499;
/// The pieces that pair with an AB boat or an armed daihatsu, in the source's two groups.
const AB_PARTNERS_A: [i64; 9] = [68, 166, 193, 436, 449, 514, 525, 526, 576];
const AB_PARTNERS_B: [i64; 6] = [167, 230, 482, 494, 495, 514];

/// The source's `installeqs` and the counts around it, for one ship.
#[derive(Debug, Default)]
struct Gear {
    /// Landing craft of the first, second and third class, and the stars of the first two
    /// together and of the third.
    dh1: i64,
    dh2: i64,
    dh3: i64,
    dh1_stars: i64,
    dh3_stars: i64,
    tdh: i64,
    tdh11: i64,
    ap_shells: i64,
    type3_shells: i64,
    /// Seaplane bombers and seaplane fighters.
    seaplane_bombers: i64,
    /// Dive bombers and jet fighter-bombers.
    dive_bombers: i64,
    mortar: i64,
    mortar_c: i64,
    rocket4: i64,
    rocket4_c: i64,
    m4a1: i64,
    soukoutei: i64,
    armed_daihatsu: i64,
    panzer: i64,
    toku_t1: i64,
    panzer3: i64,
    panzer3j: i64,
    chiha: i64,
    chiha_kai: i64,
    t4: i64,
    t4_stars: i64,
    t4k: i64,
    r35: i64,
    wg: i64,
    /// Army units, and each of the four.
    army: i64,
    army_infantry: i64,
    army_chiha: i64,
    army_chiha_kai: i64,
    army_infantry_chiha: i64,
    landing_tanks: i64,
    /// Whether any of the pieces that pair with an AB boat or an armed daihatsu is on her.
    ab_partners_a: i64,
    ab_partners_b: i64,
}

impl Gear {
    fn of(codex: &Codex, ship: &BattleRuntimeShip) -> Self {
        let mut gear = Self::default();
        for item in &ship.slot_items {
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
                continue;
            };
            let id = mst.api_id;
            let stars = item.api_level.max(0);
            let kind = KcSlotItemType3::n(mst.api_type[2]);
            // The source's chain: the landing craft by class, else the shells and aircraft.
            match kind {
                _ if LC2_IDS.contains(&id) => {
                    gear.dh2 += 1;
                    gear.dh1_stars += stars;
                }
                Some(KcSlotItemType3::LandingCraft) => {
                    gear.dh1 += 1;
                    gear.dh1_stars += stars;
                }
                Some(KcSlotItemType3::SpecialTypeAmphibiousTank) => {
                    gear.dh3 += 1;
                    gear.dh3_stars += stars;
                }
                Some(KcSlotItemType3::ArmorPiercingShell) => gear.ap_shells += 1,
                Some(KcSlotItemType3::AntiAircraftShell) => gear.type3_shells += 1,
                Some(KcSlotItemType3::SeaBasedBomber | KcSlotItemType3::SeaplaneFighter) => {
                    gear.seaplane_bombers += 1;
                }
                Some(
                    KcSlotItemType3::CarrierBasedDiveBomber | KcSlotItemType3::JetFighterBomber,
                ) => gear.dive_bombers += 1,
                _ => {}
            }
            let count = |wanted: bool| i64::from(wanted);
            gear.tdh11 += count(id == TDH11_ID);
            gear.tdh += count(TDH_IDS.contains(&id));
            gear.mortar += count(id == MORTAR_ID);
            gear.mortar_c += count(id == MORTAR_C_ID);
            gear.rocket4 += count(id == ROCKET4_ID);
            gear.rocket4_c += count(id == ROCKET4_C_ID);
            gear.m4a1 += count(id == M4A1_ID);
            gear.soukoutei += count(id == SOUKOUTEI_ID);
            gear.armed_daihatsu += count(id == ARMED_DAIHATSU_ID);
            gear.panzer += count(id == PANZER2_ID);
            gear.toku_t1 += count(id == TOKU_T1_ID);
            gear.panzer3 += count(id == PANZER3_ID || id == PANZER3J_ID);
            gear.panzer3j += count(id == PANZER3J_ID);
            if id == PANZER3_ID {
                gear.dh1 -= 1;
                gear.dh2 += 1;
                gear.tdh += 1;
            }
            gear.chiha += count(id == CHIHA_ID);
            gear.chiha_kai += count(id == CHIHA_KAI_ID);
            if T4_IDS.contains(&id) {
                gear.t4 += 1;
                gear.t4_stars += stars;
            }
            gear.t4k += count(id == T4K_ID);
            gear.r35 += count(id == R35_ID);
            gear.wg += count(id == WG_ID);
            gear.army += count(kind == Some(KcSlotItemType3::LandBattleUnit));
            gear.army_infantry += count(id == ARMY_INFANTRY_ID);
            gear.army_chiha += count(id == ARMY_CHIHA_ID);
            gear.army_chiha_kai += count(id == ARMY_CHIHA_KAI_ID);
            gear.army_infantry_chiha += count(id == ARMY_INFANTRY_CHIHA_ID);
            gear.landing_tanks += count(kind == Some(KcSlotItemType3::SpecialTypeAmphibiousTank));
            gear.ab_partners_a += count(AB_PARTNERS_A.contains(&id));
            gear.ab_partners_b += count(AB_PARTNERS_B.contains(&id));
        }
        gear
    }
}

/// What a ship's equipment does against land targets: the multipliers on her firepower by kind
/// of target, the parts of them that count by day only, and the flat power added after.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AntiInstallation {
    /// Against a soft-skinned target or a supply depot (`softSkinMult`, `softSkinMultDay`).
    pub soft_skin: f64,
    pub soft_skin_day: f64,
    /// Added after the multiplier (`installFlat`).
    pub flat: f64,
}

impl AntiInstallation {
    pub(crate) fn of(codex: &Codex, ship: &BattleRuntimeShip) -> Self {
        let mut g = Gear::of(codex, ship);

        let mut bonus1 = 1.0;
        if g.dh1 + g.dh2 > 0 {
            bonus1 += (g.dh1_stars as f64 / (g.dh1 + g.dh2) as f64) / 50.0;
        }
        if g.t4 > 0 {
            bonus1 += (g.t4_stars as f64 / g.t4 as f64) / 50.0;
        }
        let mut bonus3 = 1.0;
        if g.dh3 > 0 {
            bonus3 += (g.dh3_stars as f64 / g.dh3 as f64) / 30.0;
        }

        // `installModAll` and `installFlat`: each step multiplies both, then adds to the flat.
        let mut acc = (1.0, 0.0);
        let step = |acc: &mut (f64, f64), on: bool, mult: f64, add: f64| {
            if on {
                *acc = (acc.0 * mult, acc.1 * mult + add);
            }
        };
        step(&mut acc, g.tdh11 + g.toku_t1 + g.panzer3 > 0, 1.8, 25.0);
        step(&mut acc, g.m4a1 > 0, 1.4, 35.0);
        step(&mut acc, g.toku_t1 > 0, 1.3, 42.0);
        step(&mut acc, g.chiha > 0, 1.4, 28.0);
        step(&mut acc, g.chiha_kai > 0, 1.5, 33.0);
        step(&mut acc, g.army_infantry + g.army_infantry_chiha > 0, 1.2, 60.0);
        step(&mut acc, g.army_chiha + g.army_chiha_kai > 0, 1.5, 70.0);
        step(&mut acc, g.army_chiha_kai > 0, 1.5, 50.0);
        step(&mut acc, g.army_infantry_chiha > 0, 1.6, 70.0);
        if g.army >= 2 {
            step(&mut acc, true, 2.0, 100.0);
            step(&mut acc, g.army_infantry_chiha > 0 || g.army + g.landing_tanks >= 3, 3.0, 150.0);
            if g.t4 > 0 {
                acc.1 += 100.0;
            }
            if g.t4k > 0 {
                acc.1 += 72.0;
            }
        }
        step(&mut acc, g.t4 > 0, 1.2, 42.0);
        step(&mut acc, g.t4k > 0, 1.1, 28.0);
        step(&mut acc, g.panzer > 0, 1.15, 15.0);
        step(&mut acc, g.r35 > 0, 1.2, 20.0);

        let (mut all, mut flat) = acc;
        // An AB boat or an armed daihatsu, one of either at most, with the pieces they pair with.
        let ab = g.soukoutei + g.armed_daihatsu;
        let (mut ab_mult, mut ab_flat) = (1.0, 0.0);
        if (g.soukoutei == 1 || g.armed_daihatsu == 1) && g.soukoutei < 2 && g.armed_daihatsu < 2 {
            let partners = g.ab_partners_a + g.ab_partners_b;
            (ab_mult, ab_flat) = if g.soukoutei == 1 && g.armed_daihatsu == 1 {
                if partners >= 2 {
                    (1.5, 25.0)
                } else if g.ab_partners_b > 0 {
                    (1.4, 20.0)
                } else if g.ab_partners_a > 0 {
                    (1.3, 15.0)
                } else {
                    (1.0, 0.0)
                }
            } else if partners > 0 {
                (1.2, 10.0)
            } else {
                (1.0, 0.0)
            };
        }
        all *= ab_mult;
        flat = flat * ab_mult + ab_flat;

        flat += match g.wg {
            0 => 0.0,
            1 => 75.0,
            2 => 110.0,
            3 => 140.0,
            _ => 160.0,
        };
        flat += tiered(g.mortar_c, [60.0, 110.0, 150.0, 180.0]);
        flat += tiered(g.mortar, [30.0, 55.0, 75.0, 90.0]);
        flat += tiered(g.rocket4, [55.0, 115.0, 160.0, 190.0]);
        flat += tiered(g.rocket4_c, [80.0, 170.0, 230.0, 260.0]);

        let has_lc = g.dh1 + g.dh2 + g.t4 + g.army > 0;
        let mortars = g.mortar + g.mortar_c;
        let rockets = g.rocket4 + g.rocket4_c;
        // The army units count as the pieces they carry from here on.
        let infantry = g.army_infantry + g.army_infantry_chiha;
        g.tdh += infantry;
        g.dh2 += infantry;
        g.panzer3 += infantry;
        g.panzer3j += g.army_infantry_chiha;
        g.chiha += g.army_chiha;
        g.chiha_kai += g.army_chiha_kai;
        let tanks = g.m4a1 + g.chiha_kai + g.panzer3j;
        let paired = g.dh2 + g.chiha + g.chiha_kai;

        let mut soft_skin = 1.0;
        let mut mult = |on: bool, by: f64| {
            if on {
                soft_skin *= by;
            }
        };
        mult(g.type3_shells > 0, 2.5);
        mult(g.wg > 0, 1.3);
        mult(g.wg >= 2, 1.4);
        mult(mortars > 0, 1.2);
        mult(mortars >= 2, 1.3);
        mult(rockets > 0, 1.25);
        mult(rockets >= 2, 1.5);
        mult(has_lc, 1.4);
        mult(g.tdh > 0, 1.15);
        mult(tanks > 0, 1.1);
        mult(g.panzer > 0, 1.5);
        mult(g.r35 > 0, 1.5);
        mult(g.dh2 > 0, 1.5);
        mult(paired >= 2, 1.3);
        mult(bonus1 > 1.0, bonus1);
        mult(g.dh3 > 0, 1.5 * bonus3);
        mult(g.dh3 >= 2 || g.t4k > 0, 1.2);
        mult(g.seaplane_bombers > 0, 1.2);
        mult(true, all);
        mult(g.army > 0, 1.4);
        mult(g.army >= 2, 1.2);
        mult(g.army >= 3, 1.1);

        Self {
            soft_skin,
            soft_skin_day: day_part(ab, g.t4, 1.1, 1.1),
            flat,
        }
    }

    /// The firepower a gun attack on a land target of kind `kind` counts, before the cap
    /// (`shellPower` by day, `NBPower` at night): `firepower` times the kind's multiplier, and by
    /// day the multiplier's day part, plus the flat power.
    pub(crate) fn power(&self, _kind: u8, firepower: f64, day: bool) -> f64 {
        let day_part = if day {
            self.soft_skin_day
        } else {
            1.0
        };
        firepower * self.soft_skin * day_part + self.flat
    }
}

/// The kind of land target a ship is ([`installation_type`]), if she is one. One the source's
/// table does not know is soft-skinned, as the source's `default` branch has it.
pub(crate) fn land_target(codex: &Codex, ship: &BattleRuntimeShip) -> Option<u8> {
    (target_class(codex, ship) == TargetClass::Installation)
        .then(|| installation_type(ship.ship.api_ship_id).unwrap_or(1))
}

/// The flat power of one to four or more pieces of a kind.
fn tiered(count: i64, values: [f64; 4]) -> f64 {
    match count {
        ..=0 => 0.0,
        1..=3 => values[count as usize - 1],
        _ => values[3],
    }
}

/// A multiplier's day part: `first` with an AB boat or an armed daihatsu, `second` more with two
/// of them or two 特四式内火艇. The source leaves the part unset without the first, and so the
/// second does nothing alone.
fn day_part(ab: i64, t4: i64, first: f64, second: f64) -> f64 {
    if ab == 0 {
        1.0
    } else if ab >= 2 || t4 >= 2 {
        first * second
    } else {
        first
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{first_ship_mst_by_type, sample_ship, slotitem_with_mst_id};
    use emukc_model::kc2::KcShipType;

    const TYPE3_SHELL: i64 = 35;
    const DAIHATSU: i64 = 68;
    const DAIHATSU_TYPE89: i64 = 166;

    fn codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex").unwrap()
    }

    /// A light cruiser with this equipment, each piece at the stars given.
    fn with(codex: &Codex, gear: &[(i64, i64)]) -> AntiInstallation {
        let mut ship = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::CL), 50);
        ship.slot_items = gear
            .iter()
            .map(|&(id, stars)| {
                let mut item = slotitem_with_mst_id(id);
                item.api_level = stars;
                item
            })
            .collect();
        AntiInstallation::of(codex, &BattleRuntimeShip::from(ship))
    }

    fn assert_near(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn bare_hands_change_nothing() {
        let codex = codex();
        let none = with(&codex, &[]);
        assert_eq!((none.soft_skin, none.soft_skin_day, none.flat), (1.0, 1.0, 0.0));
        assert_eq!(none.power(1, 100.0, true), 100.0);
    }

    #[test]
    fn a_type3_shell_is_two_and_a_half_against_soft_skin() {
        let codex = codex();
        let shell = with(&codex, &[(TYPE3_SHELL, 0)]);
        assert_eq!(shell.soft_skin, 2.5);
        assert_eq!(shell.power(1, 100.0, true), 250.0);
        assert_eq!(shell.power(3, 100.0, false), 250.0);
    }

    #[test]
    fn wg42_multiplies_and_adds_by_how_many() {
        let codex = codex();
        let one = with(&codex, &[(WG_ID, 0)]);
        assert_eq!((one.soft_skin, one.flat), (1.3, 75.0));
        let two = with(&codex, &[(WG_ID, 0), (WG_ID, 0)]);
        assert_near(two.soft_skin, 1.3 * 1.4);
        assert_eq!(two.flat, 110.0);
        // 100 x 1.82 + 110.
        assert_near(two.power(1, 100.0, false), 292.0);
    }

    #[test]
    fn a_daihatsu_with_tanks_counts_as_landing_craft_and_its_stars() {
        let codex = codex();
        // A landing craft 1.4, of the second class 1.5.
        assert_near(with(&codex, &[(DAIHATSU_TYPE89, 0)]).soft_skin, 2.1);
        // Ten stars add a fifth.
        assert_near(with(&codex, &[(DAIHATSU_TYPE89, 10)]).soft_skin, 2.1 * 1.2);
        // 特大発動艇+戦車第11連隊: a plain landing craft, and 1.8 and 25 to every kind.
        let tdh11 = with(&codex, &[(TDH11_ID, 0)]);
        assert_near(tdh11.soft_skin, 1.4 * 1.8);
        assert_eq!(tdh11.flat, 25.0);
    }

    #[test]
    fn mortars_and_rockets_add_by_how_many() {
        let codex = codex();
        let one = with(&codex, &[(MORTAR_ID, 0)]);
        assert_eq!((one.soft_skin, one.flat), (1.2, 30.0));
        let two = with(&codex, &[(MORTAR_C_ID, 0), (MORTAR_C_ID, 0)]);
        assert_near(two.soft_skin, 1.2 * 1.3);
        assert_eq!(two.flat, 110.0);
        let rocket = with(&codex, &[(ROCKET4_ID, 0)]);
        assert_eq!((rocket.soft_skin, rocket.flat), (1.25, 55.0));
    }

    #[test]
    fn an_ab_boat_with_a_daihatsu_pairs_and_counts_more_by_day() {
        let codex = codex();
        let pair = with(&codex, &[(SOUKOUTEI_ID, 0), (DAIHATSU, 0)]);
        // Landing craft 1.4, the pairing 1.2; 10 flat; 1.1 by day.
        assert_near(pair.soft_skin, 1.4 * 1.2);
        assert_eq!((pair.flat, pair.soft_skin_day), (10.0, 1.1));
        assert_near(pair.power(1, 100.0, true), 100.0 * 1.68 * 1.1 + 10.0);
        assert_near(pair.power(1, 100.0, false), 178.0);
    }
}
