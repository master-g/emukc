//! How well a ship's guns fit her: a correction to her accuracy in day shelling and
//! night attacks, in percent points.
//!
//! Ported by hand from `Ship.prototype.getFit` in `kcships.js` of
//! `KC3Kai/kancolle-replay` at `69097abc` (lines 1041-1251), with the source's
//! `MECHANICS.fitGunUpdate1` and `fitGunUpdate2` on and `SIMCONSTS.enableASFit` off,
//! as they are by default. Under `fitGunUpdate1` the source takes this one figure for
//! both `ACCfit` and `ACCfitN`, for every ship on either side (`kcships.js:570-573`).
//! A gun the source does not list has no fit until the pin moves.

use emukc_model::{
    codex::Codex,
    kc2::{KcShipType, KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::targeting::ship_mst;
use crate::types::BattleRuntimeShip;

/// 46cm三連装砲.
const GUN_46CM: [i64; 1] = [9];
/// 試製46cm連装砲.
const PROTO_46CM: [i64; 1] = [117];
/// 46cm三連装砲改.
const GUN_46CM_KAI: [i64; 1] = [276];
/// The 41cm and 16inch guns other than the Mk.7.
const GUN_41CM: [i64; 14] = [8, 105, 236, 290, 298, 299, 300, 318, 330, 331, 332, 381, 385, 390];
/// The 35.6cm and 38cm guns.
const GUN_356CM: [i64; 13] = [7, 76, 103, 104, 114, 190, 192, 289, 328, 329, 502, 503, 530];
/// 381mm/50 三連装砲 and its 改.
const GUN_381MM: [i64; 2] = [133, 137];
/// 16inch三連装砲 Mk.7 and its +GFCS.
const GUN_16INCH_MK7: [i64; 2] = [161, 183];
/// The 51cm guns.
const GUN_51CM: [i64; 3] = [128, 281, 465];
/// 30.5cm三連装砲 and its 改.
const GUN_305CM: [i64; 2] = [231, 232];
/// The 305mm/46 and 320mm/44 guns.
const GUN_320MM: [i64; 4] = [426, 427, 428, 429];
/// The 38cm四連装砲.
const GUN_38CM_QUAD: [i64; 3] = [245, 246, 468];
/// 38.1cm Mk.I連装砲 and its /N改.
const GUN_381CM_MK1: [i64; 2] = [190, 192];
/// The 15.5cm三連装副砲.
const SECONDARY_155: [i64; 3] = [12, 234, 463];
/// The 14cm and 15.2cm guns (`IDS_14_152`).
const GUN_14_152: [i64; 10] = [4, 119, 310, 65, 139, 407, 303, 359, 360, 361];
/// The 15.2cm guns among them (`IDS_152`).
const GUN_152: [i64; 7] = [65, 139, 407, 303, 359, 360, 361];
/// 8inch三連装砲 Mk.9 and its mod.2.
const GUN_8INCH: [i64; 2] = [356, 357];
/// 15.5cm三連装砲 and its 改.
const GUN_155: [i64; 2] = [5, 235];
/// 5inch連装両用砲(集中配備) and its GFCS Mk.37+.
const GUN_ATLANTA: [i64; 2] = [362, 363];

const VERNIY: i64 = 147;
const YURA_KAI_NI: i64 = 488;
const NAGATO_KAI_NI: i64 = 541;

/// 金剛型.
const KONGOU_CLASS: i64 = 6;
/// Bismarck級, V.Veneto級.
const BISMARCK_LITTORIO_CLASSES: [i64; 2] = [47, 58];
const IOWA_CLASS: i64 = 65;
const GANGUT_CLASS: i64 = 73;
const RICHELIEU_CLASS: i64 = 79;
const CLASS_113: i64 = 113;
const YAMATO_CLASS: i64 = 37;
const NAGATO_CLASS: i64 = 19;
const QUEEN_ELIZABETH_CLASS: i64 = 67;
const AGANO_CLASS: i64 = 41;
const OOYODO_CLASS: i64 = 52;
const ATLANTA_CLASS: i64 = 99;
const MOGAMI_CLASS: i64 = 9;
const TASHKENT_CLASS: i64 = 81;
const J_CLASS: i64 = 82;
const JOHN_C_BUTLER_CLASS: i64 = 87;
const FLETCHER_CLASS: i64 = 91;
const MUTSUKI_CLASS: i64 = 28;

/// The ship's gun fit in percent points of accuracy (`getFit`).
pub(crate) fn gun_fit(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    let Some(mst) = ship_mst(codex, ship) else {
        return 0.0;
    };
    let ids: Vec<i64> = ship.slot_items.iter().map(|item| item.api_slotitem_id).collect();
    // `eq.type == MAINGUNM`, which is the manifest's 中口径主砲 for every item.
    let medium_guns = ids
        .iter()
        .filter(|&&id| {
            codex.find::<ApiMstSlotitem>(&id).is_ok_and(|item| {
                KcSlotItemType3::n(item.api_type[2]) == Some(KcSlotItemType3::MediumCaliberMainGun)
            })
        })
        .count() as f64;
    let count = |wanted: &[i64]| ids.iter().filter(|&id| wanted.contains(id)).count() as f64;
    let lone = |id: i64| count(&[id]);
    let class = mst.api_ctype;
    let level = ship.ship.api_lv;
    // `modMarried`, which the source keys on the level alone.
    let married = if level >= 100 {
        0.6
    } else {
        1.0
    };

    let n46 = count(&GUN_46CM);
    let p46 = count(&PROTO_46CM);
    let n46k = count(&GUN_46CM_KAI);
    let n41 = count(&GUN_41CM);
    let n356 = count(&GUN_356CM);
    let n381 = count(&GUN_381MM);
    let n51 = count(&GUN_51CM);

    match KcShipType::n(mst.api_stype) {
        Some(KcShipType::FBB) => {
            let mk7 = count(&GUN_16INCH_MK7);
            let mut fit = -10.0 * n46.sqrt() * married
                - 7.0 * p46.sqrt() * married
                - 5.0 * n41.sqrt() * married
                + 4.0 * n356.sqrt()
                - 2.0 * n381.sqrt() * married
                - 5.0 * mk7.sqrt() * married
                - 10.0 * n46k.sqrt() * married
                - 10.0 * n51.sqrt() * married
                + 12.0 * count(&GUN_305CM).sqrt()
                + 14.0 * count(&GUN_320MM).sqrt();
            if BISMARCK_LITTORIO_CLASSES.contains(&class) {
                fit += 3.0 * n381.sqrt() * married;
            }
            if class == KONGOU_CLASS {
                fit += 3.0 * n356.sqrt();
            }
            if class == IOWA_CLASS {
                fit += 7.0 * lone(161).sqrt() * married;
                fit += 14.0 * lone(183).sqrt() * married;
            }
            if class == GANGUT_CLASS {
                fit += -4.0 * n41.sqrt() * married;
                fit += 3.0 * n356.sqrt();
            }
            if class == RICHELIEU_CLASS {
                fit += 2.0 * count(&GUN_38CM_QUAD).sqrt();
                fit += -2.0 * (n41 + mk7).sqrt() * married;
            }
            if class == CLASS_113 {
                fit += -4.0 * n41.sqrt() * married;
            }
            if class == YAMATO_CLASS {
                fit += 3.0 * n46.sqrt() + 3.0 * p46.sqrt() + 7.0 * n46k.sqrt();
            }
            fit
        }
        Some(KcShipType::BB) => {
            let mut fit = 0.0;
            // `SHIPDATA[mid].HP`, the unmarried maximum.
            if mst.api_taik.is_some_and(|taik| taik[0] <= 92) {
                fit += -7.0 * n46.sqrt() * married;
                fit += -3.0 * p46.sqrt() * married;
                fit += 2.0 * n41.sqrt();
                fit += 2.0 * n356.sqrt();
                fit += 2.0 * n381.sqrt() * married;
            }
            if class != YAMATO_CLASS {
                fit += -3.0 * n46k.sqrt() * married;
                fit += -5.0 * n51.sqrt() * married;
            }
            if class == NAGATO_CLASS {
                if mst.api_id == NAGATO_KAI_NI {
                    fit += 4.0 * lone(8).sqrt();
                    fit += 3.0 * count(&GUN_41CM[1..]).sqrt();
                } else {
                    fit += 2.0 * n41.sqrt();
                }
            }
            if class == QUEEN_ELIZABETH_CLASS {
                fit += 2.0 * count(&GUN_381CM_MK1).sqrt();
            }
            if class == YAMATO_CLASS {
                fit += 3.0 * n46.sqrt() + 3.0 * p46.sqrt() + 7.0 * n46k.sqrt();
                if count(&SECONDARY_155) > 0.0 {
                    fit += 2.0;
                }
            }
            fit
        }
        Some(KcShipType::BBV) => {
            if class == YAMATO_CLASS {
                let mut fit = -8.0 * (n46 + n46k + n51).sqrt() * married
                    - 5.0 * p46.sqrt() * married
                    + 2.0 * n41.sqrt()
                    + 4.0 * n356.sqrt()
                    + 2.0 * n381.sqrt() * married
                    + 3.0 * n46.sqrt()
                    + 3.0 * p46.sqrt()
                    + 7.0 * n46k.sqrt();
                if count(&SECONDARY_155) > 0.0 {
                    fit += 2.0;
                }
                fit
            } else {
                -7.0 * n46.sqrt() * married - 3.0 * p46.sqrt() * married
                    + 2.0 * n41.sqrt()
                    + 4.0 * n356.sqrt()
                    + 2.0 * n381.sqrt() * married
                    - 7.0 * n46k.sqrt() * married
            }
        }
        Some(KcShipType::CL | KcShipType::CLT | KcShipType::CT) => {
            let n8inch = count(&GUN_8INCH);
            let mut fit = 4.0 * count(&GUN_14_152).sqrt() - 6.0 * n8inch;
            if class == AGANO_CLASS {
                let n152 = count(&GUN_152);
                fit -= medium_guns;
                fit += 3.0 * n152.sqrt();
                // `fitGunUpdate2`, marked "guess" upstream.
                if n152 > 0.0 {
                    fit += 3.0;
                }
            }
            if class == OOYODO_CLASS {
                fit -= 2.0 * medium_guns;
                fit += 4.0 * count(&GUN_155);
            }
            if class == ATLANTA_CLASS {
                fit += 3.0 * count(&GUN_ATLANTA);
            }
            if mst.api_id == YURA_KAI_NI {
                fit += 10.0 * lone(229).sqrt();
            }
            let penalty = 3.0 * medium_guns + 2.0 * n8inch;
            fit - if level >= 100 {
                penalty * 0.6
            } else {
                penalty
            }
        }
        Some(KcShipType::AV | KcShipType::AO) => {
            if medium_guns == 0.0 {
                return 0.0;
            }
            let n14_152 = count(&GUN_14_152);
            let fit = if n14_152 > 0.0 {
                -6.0 * n14_152
            } else {
                -8.0
            };
            fit - 10.0 * (medium_guns - n14_152)
        }
        _ if class == MOGAMI_CLASS => 4.0 * count(&GUN_155).sqrt(),
        _ if class == TASHKENT_CLASS || mst.api_id == VERNIY => 5.0 * lone(282).sqrt(),
        _ if class == J_CLASS => 3.0 * lone(280).sqrt() + 3.0 * count(&[260, 261, 262]).sqrt(),
        _ if class == JOHN_C_BUTLER_CLASS => 4.0 * count(&[308, 313]).sqrt(),
        _ if class == FLETCHER_CLASS => {
            let fit = 4.0 * count(&[284, 308, 313]).sqrt();
            if lone(284) >= 2.0 {
                fit + 4.0
            } else {
                fit
            }
        }
        _ if class == MUTSUKI_CLASS => 5.0 * count(&[229, 293]).sqrt(),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{make_test_ship, slotitem_with_mst_id};

    const KONGOU: i64 = 78;
    const KUMA: i64 = 99;
    const AGANO: i64 = 137;
    const JERVIS: i64 = 519;
    const MYOUKOU: i64 = 62;
    /// 軽巡ホ級.
    const ABYSSAL_LIGHT_CRUISER: i64 = 1505;

    /// 35.6cm連装砲.
    const GUN_356CM_TWIN: i64 = 7;
    /// 46cm三連装砲.
    const GUN_46CM_TRIPLE: i64 = 9;
    /// 15.5cm三連装砲, a medium gun.
    const GUN_155CM_TRIPLE: i64 = 5;
    /// 15.2cm連装砲, a medium gun on the 14cm/15.2cm list.
    const GUN_152CM_TWIN: i64 = 65;
    /// 20.3cm連装砲.
    const GUN_203CM_TWIN: i64 = 6;
    /// QF 4.7inch砲 Mk.XII改.
    const QF_47INCH: i64 = 280;
    /// Type124 ASDIC.
    const TYPE124_ASDIC: i64 = 260;

    fn fit(codex: &Codex, ship_id: i64, level: i64, items: &[i64]) -> f64 {
        let mut ship = make_test_ship(10, 10, 10, 10);
        ship.ship.api_ship_id = ship_id;
        ship.ship.api_lv = level;
        ship.slot_items = items.iter().map(|&id| slotitem_with_mst_id(id)).collect();
        gun_fit(codex, &ship)
    }

    /// Figures from the source's `getFit`.
    #[test]
    fn a_ships_guns_fit_her_as_the_source_reckons() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // 金剛型: 4 for a 35.6cm gun and 3 more for her class.
        assert_eq!(fit(&codex, KONGOU, 99, &[GUN_356CM_TWIN]), 7.0);
        // An overweight gun costs 10, three fifths of that from level 100.
        assert_eq!(fit(&codex, KONGOU, 99, &[GUN_46CM_TRIPLE]), -10.0);
        assert_eq!(fit(&codex, KONGOU, 100, &[GUN_46CM_TRIPLE]), -6.0);
        // A light cruiser pays 3 for each medium gun.
        assert_eq!(fit(&codex, KUMA, 99, &[GUN_155CM_TRIPLE]), -3.0);
        // 阿賀野型 with a 15.2cm twin: 4 for the list, -1 and +3 for her class, +3 for having
        // one, and -3 for it being a medium gun.
        assert_eq!(fit(&codex, AGANO, 99, &[GUN_152CM_TWIN]), 6.0);
        // J級: 3 for her gun and 3 for an ASDIC.
        assert_eq!(fit(&codex, JERVIS, 99, &[QF_47INCH, TYPE124_ASDIC]), 6.0);
        // A heavy cruiser has no rule.
        assert_eq!(fit(&codex, MYOUKOU, 99, &[GUN_203CM_TWIN]), 0.0);
        // Nor does the enemy get off: an abyssal light cruiser pays for a medium gun too.
        assert_eq!(fit(&codex, ABYSSAL_LIGHT_CRUISER, 1, &[GUN_155CM_TRIPLE]), -3.0);
    }
}
