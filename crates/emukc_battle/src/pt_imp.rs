//! PT小鬼群 (PT imps): what changes about an attack made on one.
//!
//! Transcribed from `KC3Kai/kancolle-replay` `69097abc` under its default switches
//! (`NERFPTIMPS` and `BREAKPTIMPS` off, `enableAirstrikeSpecialBonus` on): `shell`, `NBattack`,
//! `torpedoPhase`, `airstrike` and `airstrikeLBAS` in `kcsim.js`, the targeting in
//! `shellPhaseTarget`, `doShellC` and `nightPhaseTarget`, and `ptAccMod`, `ptDmgMod` and
//! `isAntiPT` in `kcships.js`.

use emukc_model::{
    codex::Codex,
    kc2::{KcShipType, KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::installation::{ARMED_DAIHATSU_ID, SOUKOUTEI_ID, T4_IDS};
use crate::targeting::{ship_type, target_class};
use crate::types::{BattleRuntimeShip, TargetClass};

/// 天霧改二 and 天霧改二丁, who aim at PT imps first (`isAntiPTShip`).
const ANTI_PT_SHIPS: [i64; 2] = [903, 908];

pub(crate) fn is_pt(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    target_class(codex, ship) == TargetClass::PtBoat
}

/// Give each anti-PT ship of one fleet, or one deck, the 64 accuracy she adds against a PT imp,
/// and each destroyer or escort beside her 32 (`Fleet`, `kcships.js:33-42`). Both go for PT
/// imps first.
pub(crate) fn mark_anti_pt(codex: &Codex, fleet: &mut [BattleRuntimeShip]) {
    for at in 0..fleet.len() {
        if !ANTI_PT_SHIPS.contains(&fleet[at].ship.api_ship_id) {
            continue;
        }
        fleet[at].pt_accuracy_flat = 64;
        for beside in [at.wrapping_sub(1), at + 1] {
            let Some(ship) = fleet.get(beside) else {
                continue;
            };
            if matches!(ship_type(codex, ship), Some(KcShipType::DD | KcShipType::DE)) {
                fleet[beside].pt_accuracy_flat = 32;
            }
        }
    }
}

/// Narrow `candidates` to the PT imps among them when `attacker` goes for them first and
/// there are any.
pub(crate) fn prefer_pt(
    codex: &Codex,
    attacker: &BattleRuntimeShip,
    defenders: &[BattleRuntimeShip],
    candidates: Vec<usize>,
) -> Vec<usize> {
    if attacker.pt_accuracy_flat == 0 {
        return candidates;
    }
    let pt: Vec<usize> =
        candidates.iter().copied().filter(|&at| is_pt(codex, &defenders[at])).collect();
    if pt.is_empty() {
        candidates
    } else {
        pt
    }
}

/// What a ship's equipment and type do to her attacks on a PT imp: `accuracy` multiplies her
/// chance to hit one with her guns (`ptAccMod`), `damage` her power (`ptDmgMod`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AntiPt {
    pub accuracy: f64,
    pub damage: f64,
}

impl AntiPt {
    pub(crate) fn of(codex: &Codex, ship: &BattleRuntimeShip) -> Self {
        let msts: Vec<&ApiMstSlotitem> = ship
            .slot_items
            .iter()
            .filter_map(|item| codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok())
            .collect();
        let count = |kind: KcSlotItemType3| {
            msts.iter().filter(|mst| KcSlotItemType3::n(mst.api_type[2]) == Some(kind)).count()
        };
        let ids = |ids: &[i64]| msts.iter().filter(|mst| ids.contains(&mst.api_id)).count();
        let guns = count(KcSlotItemType3::SmallCaliberMainGun);
        let secondary = count(KcSlotItemType3::SecondaryGun) > 0;
        let aa_guns = count(KcSlotItemType3::AntiAircraftGun);
        let lookouts = count(KcSlotItemType3::SeaplanePersonnel) > 0;
        let dive_bombers = count(KcSlotItemType3::CarrierBasedDiveBomber);
        let bombers = dive_bombers.max(count(KcSlotItemType3::JetFighterBomber));
        let seaplanes =
            count(KcSlotItemType3::SeaBasedBomber) + count(KcSlotItemType3::SeaplaneFighter) > 0;
        let boats = ids(&[SOUKOUTEI_ID, ARMED_DAIHATSU_ID]);
        let two_boats = boats >= 2 || ids(&T4_IDS) >= 2;
        let by = |factors: &[(bool, f64)]| -> f64 {
            factors.iter().filter(|(on, _)| *on).map(|(_, by)| by).product()
        };

        let base = match ship_type(codex, ship) {
            Some(KcShipType::DD | KcShipType::DE) => 1.0,
            Some(KcShipType::CL | KcShipType::CLT | KcShipType::CT) => 0.82,
            _ => 0.7,
        };
        let accuracy = base
            * by(&[
                (guns > 0, 1.3),
                (guns >= 2, 1.15),
                (secondary, 1.55),
                (aa_guns > 0, 1.45),
                (aa_guns >= 2, 1.35),
                (lookouts, 1.75),
                (dive_bombers > 0, 1.38),
                (dive_bombers >= 2, 1.2),
                (seaplanes, 1.5),
                (boats > 0, 1.45),
                (two_boats, 1.3),
            ]);
        let damage = by(&[
            (guns > 0, 1.5),
            (guns >= 2, 1.4),
            (secondary, 1.3),
            (bombers > 0, 1.4),
            (bombers >= 2, 1.3),
            (seaplanes, 1.2),
            (aa_guns > 0, 1.2),
            (aa_guns >= 2, 1.2),
            (lookouts, 1.1),
            (boats > 0, 1.2),
            (two_boats, 1.1),
        ]);
        Self {
            accuracy,
            damage,
        }
    }
}

/// A PT imp is hard to hit: the attacker's accuracy, in percent, becomes
/// `(0.3 x + √x + 15) × 1.2` (the source's `(.3*acc + sqrt(acc*100)/100 + .15)*1.2` on a
/// fraction).
pub(crate) fn pt_accuracy(percent: f64) -> f64 {
    (0.3 * percent + percent.max(0.0).sqrt() + 15.0) * 1.2
}

/// And hard to hurt: the power after the cap becomes `0.3 p + √p + 10`.
pub(crate) fn pt_power(power: f64) -> f64 {
    0.3 * power + power.max(0.0).sqrt() + 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        first_ship_mst_by_type, sample_ship, ship_mst_id_by_name, slotitem_with_mst_id,
    };

    #[test]
    fn a_pt_imp_bends_accuracy_and_power_as_the_source_does() {
        // (30 + 10 + 15) × 1.2.
        assert!((pt_accuracy(100.0) - 66.0).abs() < 1e-9);
        // (15 + √50 + 15) × 1.2: a weaker attacker loses less.
        assert!((pt_accuracy(50.0) - (30.0 + 50.0_f64.sqrt()) * 1.2).abs() < 1e-9);
        // 30 + 10 + 10.
        assert!((pt_power(100.0) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn equipment_and_type_set_what_a_ship_does_to_a_pt_imp() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |ship_type, gear: &[i64]| {
            let mut input = sample_ship(&codex, first_ship_mst_by_type(&codex, ship_type), 1);
            input.slot_items = gear.iter().map(|&id| slotitem_with_mst_id(id)).collect();
            AntiPt::of(&codex, &BattleRuntimeShip::new(input, true, true))
        };
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;

        // Bare: 1 for a destroyer, 0.82 for a light cruiser, 0.7 for anyone else.
        assert_eq!(
            ship(KcShipType::DD, &[]),
            AntiPt {
                accuracy: 1.0,
                damage: 1.0
            }
        );
        assert!(close(ship(KcShipType::CL, &[]).accuracy, 0.82));
        assert!(close(ship(KcShipType::CA, &[]).accuracy, 0.7));
        // Two 12.7cm連装砲 (2) and a 25mm連装機銃 (39).
        let armed = ship(KcShipType::DD, &[2, 2, 39]);
        assert!(close(armed.accuracy, 1.3 * 1.15 * 1.45));
        assert!(close(armed.damage, 1.5 * 1.4 * 1.2));
        // A 熟練見張員 (129) and an AB boat (408).
        let lookout = ship(KcShipType::DD, &[129, SOUKOUTEI_ID]);
        assert!(close(lookout.accuracy, 1.75 * 1.45));
        assert!(close(lookout.damage, 1.1 * 1.2));
    }

    #[test]
    fn amagiri_and_the_destroyers_beside_her_aim_at_pt_imps() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship = |id| BattleRuntimeShip::new(sample_ship(&codex, id, 99), true, true);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ca = first_ship_mst_by_type(&codex, KcShipType::CA);
        let mut fleet = vec![ship(ca), ship(903), ship(dd), ship(dd)];
        mark_anti_pt(&codex, &mut fleet);
        let flats: Vec<i64> = fleet.iter().map(|ship| ship.pt_accuracy_flat).collect();
        assert_eq!(flats, [0, 64, 32, 0]);

        let enemy = |id| {
            let mut input = sample_ship(&codex, dd, 1);
            input.ship.api_ship_id = id;
            BattleRuntimeShip::new(input, false, true)
        };
        let defenders = vec![enemy(dd), enemy(ship_mst_id_by_name(&codex, "PT小鬼群")), enemy(dd)];
        assert_eq!(prefer_pt(&codex, &fleet[2], &defenders, vec![0, 1, 2]), [1]);
        assert_eq!(prefer_pt(&codex, &fleet[3], &defenders, vec![0, 1, 2]), [0, 1, 2]);
        assert_eq!(prefer_pt(&codex, &fleet[1], &defenders, vec![0, 2]), [0, 2]);
    }
}
