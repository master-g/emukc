//! What a ship's equipment does against a land target.
//!
//! Ported by hand from `Ship.prototype.loadEquips` in `kcships.js` of
//! `KC3Kai/kancolle-replay` at `69097abc` (lines 429-965), with the source's
//! `MECHANICS.installRevamp` and `panzerIIIBuff` on, as they are by default.
//! The kind of target comes from [`installation_type`]; the power formulas that
//! read these figures are the source's `shellPower` and `NBPower`.

use emukc_model::{
    codex::Codex,
    kc2::{KcShipType, KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::simulation::installation_types::{
    ANCHORAGE_IDS, DOCK_IDS, SUMMER_BATTLESHIP_IDS, SUMMER_CRUISER_IDS, SUPPLY_DEPOT_IDS,
};
use crate::targeting::{installation_type, ship_mst, ship_type, target_class};
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
const SWORDFISH_IDS: [i64; 3] = [242, 243, 244];
/// The ships the source gives more against the summer battleship and heavy cruiser
/// (`summerBBPostMult`, `summerCAPostMult`).
const SUMMER_SHIP_IDS: [i64; 13] =
    [171, 172, 173, 176, 177, 178, 393, 515, 571, 574, 576, 579, 630];
/// The ships it gives more against the anchorage (`anchoragePostMult`).
const ANCHORAGE_SHIP_IDS: [i64; 13] =
    [80, 81, 131, 136, 143, 148, 275, 276, 541, 546, 573, 911, 916];
/// The ship classes it gives more against the dock (`dockPostMult`).
const DOCK_SHIP_CLASSES: [i64; 7] = [58, 61, 64, 68, 80, 92, 113];

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
    /// Dive bombers and jet fighter-bombers, and each of the two.
    dive_bombers: i64,
    plain_dive_bombers: i64,
    jet_bombers: i64,
    swordfish: i64,
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
            gear.plain_dive_bombers += count(kind == Some(KcSlotItemType3::CarrierBasedDiveBomber));
            gear.jet_bombers += count(kind == Some(KcSlotItemType3::JetFighterBomber));
            gear.swordfish += count(SWORDFISH_IDS.contains(&id));
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
    /// Against a pillbox (`pillboxMult`, `pillboxMultDay`).
    pub pillbox: f64,
    pub pillbox_day: f64,
    /// Against an isolated island (`isoMult`, `isoMultDay`).
    pub isolated: f64,
    pub isolated_day: f64,
    /// Against the northern princesses (`northernMult`), who take her torpedo too at night.
    pub northern: f64,
    /// Against the harbour summer princess (`harbourSummerMult`, `harbourSummerMultDay`).
    pub harbour_summer: f64,
    pub harbour_summer_day: f64,
    /// Added after the multiplier (`installFlat`).
    pub flat: f64,
    /// After the cap, against a supply depot, the anchorage, the dock and the summer battleship
    /// and heavy cruiser (`supplyPostMult`, `anchoragePostMult`, `dockPostMult`,
    /// `summerBBPostMult`, `summerCAPostMult`).
    pub supply_post: f64,
    pub anchorage_post: f64,
    pub dock_post: f64,
    pub summer_battleship_post: f64,
    pub summer_cruiser_post: f64,
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

        let soft_skin = product(&[
            (g.type3_shells > 0, 2.5),
            (g.wg > 0, 1.3),
            (g.wg >= 2, 1.4),
            (mortars > 0, 1.2),
            (mortars >= 2, 1.3),
            (rockets > 0, 1.25),
            (rockets >= 2, 1.5),
            (has_lc, 1.4),
            (g.tdh > 0, 1.15),
            (tanks > 0, 1.1),
            (g.panzer > 0, 1.5),
            (g.r35 > 0, 1.5),
            (g.dh2 > 0, 1.5),
            (paired >= 2, 1.3),
            (bonus1 > 1.0, bonus1),
            (g.dh3 > 0, 1.5 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.2),
            (g.seaplane_bombers > 0, 1.2),
            (true, all),
            (g.army > 0, 1.4),
            (g.army >= 2, 1.2),
            (g.army >= 3, 1.1),
        ]);

        let light = matches!(ship_type(codex, ship), Some(KcShipType::DD | KcShipType::CL));
        let pillbox = product(&[
            (light, 1.4),
            (g.wg == 1, 1.6),
            (g.wg >= 2, 2.72),
            (mortars > 0, 1.3),
            (mortars >= 2, 1.5),
            (rockets > 0, 1.5),
            (rockets >= 2, 1.8),
            (has_lc, 1.8),
            (g.tdh > 0, 1.15),
            (tanks > 0, 2.0),
            (g.panzer > 0, 1.5),
            (g.r35 > 0, 1.5),
            (g.dh2 > 0, 1.5),
            (paired >= 2, 1.4),
            (bonus1 > 1.0, bonus1),
            (g.dh3 > 0, 2.4 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.35),
            (g.dive_bombers >= 2, 2.0),
            (true, all),
            (g.ap_shells > 0, 1.85),
            (g.seaplane_bombers > 0, 1.5),
            (g.dive_bombers > 0, 1.5),
        ]);

        let isolated = product(&[
            (g.wg == 1, 1.4),
            (g.wg >= 2, 2.1),
            (mortars > 0, 1.2),
            (mortars >= 2, 1.4),
            (rockets > 0, 1.3),
            (rockets >= 2, 1.65),
            (has_lc, 1.8),
            (g.tdh > 0, 1.15),
            (tanks > 0, 1.8),
            (g.panzer > 0, 1.2),
            (g.r35 > 0, 1.2),
            (g.dh2 > 0, 1.2),
            (paired >= 2, 1.4),
            (bonus1 > 1.0, bonus1),
            (g.dh3 > 0, 2.4 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.35),
            (g.dive_bombers >= 2, 1.75),
            (true, all),
            (g.type3_shells > 0, 1.75),
            (g.dive_bombers > 0, 1.4),
        ]);

        // The source multiplies by its rockets' 1.4 whatever their number; kept as it is.
        let harbour_summer = product(&[
            (g.wg > 0, 1.4),
            (g.wg >= 2, 1.2),
            (rockets > 0, 1.25),
            (rockets > 0, 1.4),
            (mortars > 0, 1.1),
            (mortars >= 2, 1.15),
            (has_lc, 1.7),
            (g.tdh > 0, 1.2),
            (tanks > 0, 2.0),
            (g.panzer > 0, 1.6),
            (g.r35 > 0, 1.6),
            (g.dh2 > 0, 1.6),
            (paired >= 2, 1.5),
            (bonus1 > 1.0, bonus1),
            (g.dh3 > 0, 2.8 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.5),
            (g.type3_shells > 0, 1.75),
            (g.ap_shells > 0, 1.3),
            (g.dive_bombers > 0, 1.3),
            (g.dive_bombers >= 2, 1.25),
            (g.seaplane_bombers > 0, 1.3),
            (true, all),
        ]);

        // No `installModAll` and no day part here.
        let northern = product(&[
            (g.wg == 1, 1.4),
            (g.wg >= 2, 2.1),
            (g.tdh > 0, 1.8),
            (g.tdh11 > 0, 2.2),
            (g.type3_shells > 0, 1.75),
        ]);

        // Unlike the day parts, two AB boats or two 特四式内火艇 count here without the first.
        let supply_post = product(&[
            (g.wg > 0, 1.25),
            (g.wg >= 2, 1.3),
            (rockets > 0, 1.2),
            (rockets >= 2, 1.4),
            (mortars > 0, 1.15),
            (mortars >= 2, 1.2),
            (has_lc, 1.7 * bonus1),
            (g.tdh > 0, 1.2),
            (g.dh2 > 0, 1.3 * bonus1),
            (paired >= 2, 1.6),
            (tanks > 0, 1.2),
            (g.panzer > 0, 1.3 * bonus1),
            (g.r35 > 0, 1.3 * bonus1),
            (g.dh3 > 0, 1.7 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.5),
            (ab > 0, 1.5),
            (ab >= 2 || g.t4 >= 2, 1.1),
            (g.army > 0, 1.85),
            (g.army >= 2, 1.45),
            (g.army >= 3, 1.2),
        ]);

        let mst_id = ship.ship.api_ship_id;
        let summer_ship = SUMMER_SHIP_IDS.contains(&mst_id);
        let summer_battleship_post = product(&[
            (g.ap_shells > 0, 1.2),
            (g.seaplane_bombers > 0, 1.1),
            (g.swordfish > 0, 1.15),
            (g.swordfish >= 2, 1.05),
            (summer_ship, 1.1),
        ]);
        let summer_cruiser_post = product(&[
            (g.ap_shells > 0, 1.1),
            (g.seaplane_bombers > 0, 1.15),
            (g.swordfish > 0, 1.15),
            (g.swordfish >= 2, 1.05),
            (summer_ship, 1.1),
        ]);

        let anchorage_post = product(&[
            (mortars > 0, 1.1),
            (rockets > 0, 1.15),
            (rockets >= 2, 1.4),
            (g.wg > 0, 1.2),
            (g.wg >= 2, 1.3),
            (has_lc, 1.4 * bonus1),
            (g.tdh > 0, 1.15),
            (g.dh2 > 0, 1.2),
            (paired >= 2, 1.4),
            (g.dh3 > 0, 2.4 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.35),
            (tanks > 0, 1.8),
            (g.panzer > 0, 1.2),
            (g.r35 > 0, 1.2),
            (g.type3_shells > 0, 1.45),
            (g.dive_bombers > 0, 1.4),
            (g.dive_bombers >= 2 || g.jet_bombers > 0, 1.75),
            (ab > 0, 1.2),
            (ab >= 2 || g.t4 >= 2, 1.1),
            (ANCHORAGE_SHIP_IDS.contains(&mst_id), 1.2),
        ]);

        let ship_class = ship_mst(codex, ship).map_or(0, |mst| mst.api_ctype);
        let dock_post = product(&[
            (g.dive_bombers > 0, 1.1),
            (g.dive_bombers >= 2, 1.1),
            (g.seaplane_bombers > 0, 1.1),
            (g.wg > 0, 1.1),
            (g.wg >= 2, 1.2),
            (g.type3_shells > 0, 1.3),
            (g.dh3 > 0, 1.2 * bonus3),
            (g.dh3 >= 2 || g.t4k > 0, 1.2),
            (has_lc, 1.1),
            (g.tdh11 + g.toku_t1 + g.panzer3 > 0, 1.4),
            (g.dh2 > 0, 1.15 * bonus1),
            (paired >= 2, 1.15),
            (tanks > 0, 1.1),
            (g.panzer > 0, 1.15),
            (g.panzer >= 2, 1.15),
            (g.r35 > 0, 1.15),
            (g.r35 >= 2, 1.15),
            (ab > 0, 1.1),
            (DOCK_SHIP_CLASSES.contains(&ship_class), 1.1),
        ]);

        Self {
            soft_skin,
            soft_skin_day: day_part(ab, g.t4, 1.1, 1.1),
            pillbox,
            pillbox_day: day_part(ab, g.t4, 1.3, 1.2),
            isolated,
            isolated_day: day_part(ab, g.t4, 1.3, 1.1),
            northern,
            harbour_summer,
            harbour_summer_day: day_part(ab, g.t4, 1.5, 1.1),
            flat,
            supply_post,
            anchorage_post,
            dock_post,
            summer_battleship_post,
            summer_cruiser_post,
        }
    }

    /// The firepower a gun attack on a land target of kind `kind` counts, before the cap
    /// (`shellPower` by day, `NBPower` at night): `firepower` times the kind's multiplier, and by
    /// day the multiplier's day part, plus the flat power. A supply depot takes the soft-skinned
    /// multiplier, and its own after the cap.
    pub(crate) fn power(&self, kind: u8, firepower: f64, day: bool) -> f64 {
        let (mult, day_part) = match kind {
            2 => (self.pillbox, self.pillbox_day),
            4 => (self.isolated, self.isolated_day),
            5 => (self.northern, 1.0),
            6 => (self.harbour_summer, self.harbour_summer_day),
            _ => (self.soft_skin, self.soft_skin_day),
        };
        firepower
            * mult
            * if day {
                day_part
            } else {
                1.0
            }
            + self.flat
    }

    /// What multiplies her capped power against `target`: by gunfire, a supply depot's, the
    /// anchorage's, the dock's and the summer battleship's and heavy cruiser's; by torpedo the
    /// last three alone (`kcsim.js` `shell`, `NBattack`, `torpedoPhase`).
    pub(crate) fn post_cap(&self, target_id: i64, torpedo: bool) -> f64 {
        product(&[
            (
                !torpedo
                    && (installation_type(target_id) == Some(3)
                        || SUPPLY_DEPOT_IDS.contains(&target_id)),
                self.supply_post,
            ),
            (!torpedo && ANCHORAGE_IDS.contains(&target_id), self.anchorage_post),
            (DOCK_IDS.contains(&target_id), self.dock_post),
            (SUMMER_BATTLESHIP_IDS.contains(&target_id), self.summer_battleship_post),
            (SUMMER_CRUISER_IDS.contains(&target_id), self.summer_cruiser_post),
        ])
    }
}

/// The kind of land target a ship is ([`installation_type`]), if she is one. One the source's
/// table does not know is soft-skinned, as the source's `default` branch has it.
pub(crate) fn land_target(codex: &Codex, ship: &BattleRuntimeShip) -> Option<u8> {
    (target_class(codex, ship) == TargetClass::Installation)
        .then(|| installation_type(ship.ship.api_ship_id).unwrap_or(1))
}

/// What multiplies `attacker`'s capped power against `defender` ([`AntiInstallation::post_cap`]),
/// 1 against a target that takes none.
pub(crate) fn post_cap_multiplier(
    codex: &Codex,
    attacker: &BattleRuntimeShip,
    defender: &BattleRuntimeShip,
    torpedo: bool,
) -> f64 {
    let id = defender.ship.api_ship_id;
    let any = installation_type(id) == Some(3)
        || [SUPPLY_DEPOT_IDS, ANCHORAGE_IDS, DOCK_IDS, SUMMER_BATTLESHIP_IDS, SUMMER_CRUISER_IDS]
            .iter()
            .any(|ids| ids.contains(&id));
    if any {
        AntiInstallation::of(codex, attacker).post_cap(id, torpedo)
    } else {
        1.0
    }
}

/// The product of the factors that are on.
fn product(factors: &[(bool, f64)]) -> f64 {
    factors.iter().filter(|(on, _)| *on).map(|(_, by)| by).product()
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
    fn each_kind_of_land_target_has_its_own_multiplier() {
        const AP_SHELL: i64 = 36; // 九一式徹甲弾
        const TOKU_DAIHATSU: i64 = 193;
        let codex = codex();
        // A light cruiser starts at 1.4 against a pillbox; WG42 1.6, an AP shell 1.85.
        assert_eq!(with(&codex, &[]).pillbox, 1.4);
        assert_near(with(&codex, &[(WG_ID, 0)]).pillbox, 1.4 * 1.6);
        assert_near(with(&codex, &[(AP_SHELL, 0)]).pillbox, 1.4 * 1.85);
        // An isolated island: two WG42 2.1, a type 3 shell 1.75.
        assert_eq!(with(&codex, &[(WG_ID, 0), (WG_ID, 0)]).isolated, 2.1);
        assert_eq!(with(&codex, &[(TYPE3_SHELL, 0)]).isolated, 1.75);
        // The northern princesses: no `installModAll`, so the 11th regiment is 2.2 alone.
        assert_eq!(with(&codex, &[(TDH11_ID, 0)]).northern, 2.2);
        assert_eq!(with(&codex, &[(TOKU_DAIHATSU, 0)]).northern, 1.8);
        // The harbour summer princess: one rocket launcher takes both of its factors.
        assert_near(with(&codex, &[(ROCKET4_ID, 0)]).harbour_summer, 1.25 * 1.4);
        assert_eq!(with(&codex, &[(WG_ID, 0)]).harbour_summer, 1.4);

        let shell = with(&codex, &[(TYPE3_SHELL, 0)]);
        assert_eq!(shell.power(2, 100.0, true), 140.0);
        assert_eq!(shell.power(4, 100.0, true), 175.0);
        assert_eq!(shell.power(5, 100.0, false), 175.0);
        assert_eq!(shell.power(3, 100.0, false), 250.0);
    }

    #[test]
    fn the_multipliers_after_the_cap_follow_their_targets() {
        const AP_SHELL: i64 = 36;
        const SUPPLY_DEPOT: i64 = 1653; // 集積地棲姫
        const VACATION_DEPOT: i64 = 2138; // 集積地棲姫III バカンスmode
        const ANCHORAGE: i64 = 1815;
        const DOCK: i64 = 1827;
        const SUMMER_BB: i64 = 1696;
        const SUMMER_CA: i64 = 1705;
        let codex = codex();
        assert_eq!(installation_type(SUPPLY_DEPOT), Some(3));
        let wg = with(&codex, &[(WG_ID, 0)]);
        assert_eq!(wg.supply_post, 1.25);
        assert_eq!(wg.post_cap(SUPPLY_DEPOT, false), 1.25);
        assert_eq!(wg.post_cap(VACATION_DEPOT, false), 1.25);
        assert_eq!(wg.post_cap(ANCHORAGE, false), 1.2);
        assert_eq!(wg.post_cap(DOCK, false), 1.1);
        // A torpedo takes the dock's and the summer ships' alone.
        assert_eq!(wg.post_cap(SUPPLY_DEPOT, true), 1.0);
        assert_eq!(wg.post_cap(ANCHORAGE, true), 1.0);
        assert_eq!(wg.post_cap(DOCK, true), 1.1);
        assert_eq!(wg.post_cap(1501, false), 1.0);

        assert_near(with(&codex, &[(DAIHATSU, 10)]).supply_post, 1.7 * 1.2);
        let ap = with(&codex, &[(AP_SHELL, 0)]);
        assert_eq!(ap.post_cap(SUMMER_BB, true), 1.2);
        assert_eq!(ap.post_cap(SUMMER_CA, false), 1.1);
        let shell = with(&codex, &[(TYPE3_SHELL, 0)]);
        assert_eq!((shell.anchorage_post, shell.dock_post), (1.45, 1.3));
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
        // A pillbox's day part is 1.3, an isolated island's 1.3, the harbour's 1.5.
        assert_eq!((pair.pillbox_day, pair.isolated_day, pair.harbour_summer_day), (1.3, 1.3, 1.5));
        // Without an AB boat two 特四式内火艇 do nothing by day; with one they add the second step.
        assert_eq!(with(&codex, &[(525, 0), (525, 0)]).soft_skin_day, 1.0);
        let two_t4 = with(&codex, &[(SOUKOUTEI_ID, 0), (525, 0), (525, 0)]);
        assert_near(two_t4.soft_skin_day, 1.1 * 1.1);
    }
}
