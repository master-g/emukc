//! Day attack records: the one place `api_at_type` and `api_si_list` are derived.
//!
//! The client picks an attack animation from `api_at_type` and reads the
//! equipment plate from `api_si_list`, and the two must agree with each other
//! and with the entry's text/integer form. Callers only report who attacked,
//! with what kind of attack, whom, and for how much; the kind decides the rest.
//! The night-side counterpart is `night_si_entry` in `night.rs`.

use emukc_model::codex::Codex;
use emukc_model::kc2::types::KcSlotItemType3;

use crate::simulation::day_cutin::{CarrierCiSubType, DayAttackType, carrier_ci_display_ids};
use crate::targeting::{
    day_attack_display_ids, day_gunnery_display_ids, is_ap_shell_type, is_radar_type,
};
use crate::types::{BattleHougeki, BattleRuntimeShip, DamageCell, SiListId};

/// What kind of day attack was made, and by which ship's equipment.
pub(crate) enum DayAttackKind<'a> {
    /// A shelling attack with its resolved cut-in.
    Shelling {
        codex: &'a Codex,
        ship: &'a BattleRuntimeShip,
        at_type: DayAttackType,
        carrier_sub: Option<CarrierCiSubType>,
    },
    /// An anti-submarine attack: opening ASW, or shelling aimed at a submarine.
    Asw(&'a Codex, &'a BattleRuntimeShip),
    /// A flagship special attack, carrying its client attack-type id.
    Special(&'a Codex, &'a BattleRuntimeShip, i64),
    /// A finishing blow the debug overlay injects; it names no equipment.
    DebugInjected,
}

/// Derive `api_at_type` and the `api_si_list` entry for a day attack.
fn day_si_entry(kind: &DayAttackKind<'_>) -> (i64, Vec<SiListId>) {
    match *kind {
        DayAttackKind::Shelling {
            codex,
            ship,
            at_type,
            carrier_sub,
        } => {
            let si = match at_type {
                DayAttackType::Normal => {
                    SiListId::num_from_i64(&day_attack_display_ids(codex, ship, false))
                }
                DayAttackType::DoubleAttack => {
                    SiListId::text_from_i64(&day_gunnery_display_ids(codex, ship, 2, None))
                }
                DayAttackType::CarrierCI => SiListId::text_from_i64(&carrier_ci_display_ids(
                    codex,
                    ship,
                    carrier_sub.expect("CarrierCI must have sub-type"),
                )),
                // Artillery spotting CI (at_type 3-6). Each is formed from guns
                // plus, for two of them, the piece that qualified it: 主砲/電探
                // needs the radar and the 徹甲弾 cut-ins need the shell, and the
                // client draws all three slots.
                DayAttackType::MainSecCI
                | DayAttackType::MainRadarCI
                | DayAttackType::MainApSecCI
                | DayAttackType::MainApMainCI => {
                    let extra: Option<fn(KcSlotItemType3) -> bool> = match at_type {
                        DayAttackType::MainRadarCI => Some(is_radar_type),
                        DayAttackType::MainApSecCI | DayAttackType::MainApMainCI => {
                            Some(is_ap_shell_type)
                        }
                        _ => None,
                    };
                    SiListId::text_from_i64(&day_gunnery_display_ids(codex, ship, 3, extra))
                }
            };
            (at_type as i64, si)
        }
        // `PhasePreAntiSubmarine` only dispatches 0 (normal) and 2 (double);
        // anything else falls through to `PhaseAttackDanchaku`, which throws
        // outside {3,4,5,6,200,201}. The depth-charge or ASW-plane animation
        // comes from the defender being a submarine, not from the attack type.
        DayAttackKind::Asw(codex, ship) => {
            (0, SiListId::num_from_i64(&day_attack_display_ids(codex, ship, true)))
        }
        DayAttackKind::Special(codex, ship, at_type) => {
            (at_type, SiListId::text_from_i64(&day_attack_display_ids(codex, ship, false)))
        }
        DayAttackKind::DebugInjected => (0, vec![SiListId::Num(-1)]),
    }
}

impl BattleHougeki {
    /// Record one day attack by `attacker_idx` of the side `attacker_is_enemy`
    /// names: each hit lands on the matching entry of `targets` for the matching
    /// entry of `damage`.
    pub(crate) fn record_day_attack(
        &mut self,
        kind: DayAttackKind<'_>,
        attacker_is_enemy: bool,
        attacker_idx: usize,
        targets: Vec<i64>,
        damage: Vec<DamageCell>,
    ) {
        let (at_type, si) = day_si_entry(&kind);
        self.api_at_eflag.push(i64::from(attacker_is_enemy));
        self.api_at_list.push(attacker_idx as i64);
        self.api_at_type.push(at_type);
        self.api_df_list.push(targets);
        self.api_si_list.push(si);
        self.api_cl_list.push(vec![1; damage.len()]);
        self.api_damage.push(damage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::day_cutin::CarrierCiSubType;
    use crate::test_utils::*;
    use emukc_model::kc2::types::KcShipType;

    fn codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex").unwrap()
    }

    fn shelling<'a>(
        codex: &'a Codex,
        ship: &'a BattleRuntimeShip,
        at_type: DayAttackType,
    ) -> DayAttackKind<'a> {
        DayAttackKind::Shelling {
            codex,
            ship,
            at_type,
            carrier_sub: None,
        }
    }

    fn ship_with(codex: &Codex, stype: KcShipType, slots: &[KcSlotItemType3]) -> BattleRuntimeShip {
        let mst = first_ship_mst_by_type(codex, stype);
        let mut ship = sample_ship(codex, mst, 99);
        ship.slot_items = slots
            .iter()
            .map(|&t| slotitem_with_mst_id(first_slotitem_mst_by_type(codex, t)))
            .collect();
        BattleRuntimeShip::from(ship)
    }

    fn is_text(si: &[SiListId]) -> bool {
        si.iter().any(|id| matches!(id, SiListId::Text(_)))
    }

    #[test]
    fn normal_shelling_is_type_0_with_integer_plate() {
        let codex = codex();
        let ship = ship_with(&codex, KcShipType::BB, &[KcSlotItemType3::LargeCaliberMainGun]);
        let (at_type, si) = day_si_entry(&shelling(&codex, &ship, DayAttackType::Normal));
        assert_eq!(at_type, 0);
        assert!(!is_text(&si), "normal attack plate must be integers: {si:?}");
    }

    #[test]
    fn double_attack_is_text_and_names_only_guns() {
        let codex = codex();
        let radar = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);
        let ship = ship_with(
            &codex,
            KcShipType::BB,
            &[
                KcSlotItemType3::LargeCaliberMainGun,
                KcSlotItemType3::LargeCaliberMainGun,
                KcSlotItemType3::SmallRadar,
            ],
        );
        let (at_type, si) = day_si_entry(&shelling(&codex, &ship, DayAttackType::DoubleAttack));
        assert_eq!(at_type, 2);
        assert!(is_text(&si));
        assert!(!si.contains(&SiListId::Text(radar.to_string())), "{si:?}");
    }

    #[test]
    fn spotting_cut_ins_carry_the_piece_that_qualified_them() {
        let codex = codex();
        let radar = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);
        let shell = first_slotitem_mst_by_type(&codex, KcSlotItemType3::ArmorPiercingShell);
        let ship = ship_with(
            &codex,
            KcShipType::BB,
            &[
                KcSlotItemType3::LargeCaliberMainGun,
                KcSlotItemType3::SecondaryGun,
                KcSlotItemType3::SmallRadar,
                KcSlotItemType3::ArmorPiercingShell,
            ],
        );
        for (at_type, piece) in [
            (DayAttackType::MainRadarCI, radar),
            (DayAttackType::MainApSecCI, shell),
            (DayAttackType::MainApMainCI, shell),
        ] {
            let (value, si) = day_si_entry(&shelling(&codex, &ship, at_type));
            assert_eq!(value, at_type as i64);
            assert!(is_text(&si));
            assert!(si.contains(&SiListId::Text(piece.to_string())), "{at_type:?}: {si:?}");
        }
    }

    #[test]
    fn carrier_cut_in_is_type_7_with_text_plate() {
        let codex = codex();
        let ship = ship_with(
            &codex,
            KcShipType::CV,
            &[
                KcSlotItemType3::CarrierBasedFighter,
                KcSlotItemType3::CarrierBasedDiveBomber,
                KcSlotItemType3::CarrierBasedTorpedoBomber,
            ],
        );
        let kind = DayAttackKind::Shelling {
            codex: &codex,
            ship: &ship,
            at_type: DayAttackType::CarrierCI,
            carrier_sub: Some(CarrierCiSubType::Fba),
        };
        let (at_type, si) = day_si_entry(&kind);
        assert_eq!(at_type, 7);
        assert!(is_text(&si));
        assert_eq!(si.len(), 3, "FBA names all three planes: {si:?}");
    }

    /// 12b0fb1d: an ASW attack is type 0 with an integer plate, whether it opens
    /// the battle or lands during shelling.
    #[test]
    fn asw_is_type_0_with_integer_plate() {
        let codex = codex();
        let ship = ship_with(&codex, KcShipType::DD, &[KcSlotItemType3::Sonar]);
        let (at_type, si) = day_si_entry(&DayAttackKind::Asw(&codex, &ship));
        assert_eq!(at_type, 0);
        assert_eq!(si, vec![SiListId::Num(-1)]);
    }

    #[test]
    fn special_attack_keeps_its_id_with_text_plate() {
        let codex = codex();
        let ship = ship_with(&codex, KcShipType::BB, &[KcSlotItemType3::LargeCaliberMainGun]);
        let (at_type, si) = day_si_entry(&DayAttackKind::Special(&codex, &ship, 101));
        assert_eq!(at_type, 101);
        assert!(is_text(&si));
    }

    #[test]
    fn injected_attack_is_type_0_naming_nothing() {
        let (at_type, si) = day_si_entry(&DayAttackKind::DebugInjected);
        assert_eq!(at_type, 0);
        assert_eq!(si, vec![SiListId::Num(-1)]);
    }

    #[test]
    fn record_keeps_the_seven_arrays_aligned() {
        let mut hougeki = BattleHougeki::default();
        hougeki.record_day_attack(
            DayAttackKind::DebugInjected,
            false,
            2,
            vec![4, 4],
            vec![DamageCell::Plain(10), DamageCell::Shielded(3)],
        );
        assert_eq!(hougeki.api_at_eflag, vec![0]);
        assert_eq!(hougeki.api_at_list, vec![2]);
        assert_eq!(hougeki.api_df_list, vec![vec![4, 4]]);
        assert_eq!(hougeki.api_cl_list, vec![vec![1, 1]]);
        assert_eq!(hougeki.api_damage.len(), 1);
        assert_eq!(hougeki.api_si_list.len(), 1);
    }
}
