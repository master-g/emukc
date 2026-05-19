//! Day cut-in (artillery spotting) attack type detection.

use emukc_model::{codex::Codex, kc2::start2::ApiMstSlotitem, kc2::types::KcSlotItemType3};

use crate::damage::is_cv_type;
use crate::random::BattleRng;

use crate::types::AirState;
use crate::types::BattleRuntimeShip;

/// Day attack type (弾着観測射撃 / 連撃 / 戦爆連合CI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DayAttackType {
    Normal = 0,
    DoubleAttack = 2,
    MainSecCI = 3,
    MainRadarCI = 4,
    MainApSecCI = 5,
    MainApMainCI = 6,
    CarrierCI = 7,
}

/// Carrier CI sub-types with different multipliers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarrierCiSubType {
    /// FBA: Fighter + Bomber + Attacker → 1.25x
    Fba,
    /// BBA: 2× Bomber + Attacker → 1.2x
    Bba,
    /// BA: Bomber + Attacker → 1.15x
    Ba,
}

impl CarrierCiSubType {
    pub(crate) fn damage_multiplier(self) -> f64 {
        match self {
            Self::Fba => 1.25,
            Self::Bba => 1.2,
            Self::Ba => 1.15,
        }
    }
}

impl DayAttackType {
    pub(crate) fn api_id(self) -> i64 {
        self as i64
    }
}

/// Post-cap damage multiplier for day CI types.
/// DoubleAttack uses 1.2x per hit (×2 hits total).
pub(crate) fn day_ci_damage_multiplier(at_type: DayAttackType) -> f64 {
    match at_type {
        DayAttackType::Normal => 1.0,
        DayAttackType::DoubleAttack => 1.2,
        DayAttackType::MainSecCI => 1.1,
        DayAttackType::MainRadarCI => 1.2,
        DayAttackType::MainApSecCI => 1.3,
        DayAttackType::MainApMainCI => 1.5,
        DayAttackType::CarrierCI => 1.25, // default; actual multiplier set by sub-type
    }
}

/// Accuracy multiplier for day CI types.
pub(crate) fn day_ci_accuracy_multiplier(at_type: DayAttackType) -> f64 {
    match at_type {
        DayAttackType::Normal => 1.0,
        DayAttackType::DoubleAttack => 1.1,
        DayAttackType::MainSecCI => 1.3,
        DayAttackType::MainRadarCI => 1.5,
        DayAttackType::MainApSecCI => 1.3,
        DayAttackType::MainApMainCI => 1.2,
        DayAttackType::CarrierCI => 1.0,
    }
}

// ---------------------------------------------------------------------------
// Equipment counting helpers
// ---------------------------------------------------------------------------

fn count_type(codex: &Codex, ship: &BattleRuntimeShip, wanted: KcSlotItemType3) -> usize {
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                == Some(wanted)
        })
        .count()
}

fn count_main_guns(codex: &Codex, ship: &BattleRuntimeShip) -> usize {
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(|t| {
                    matches!(
                        t,
                        KcSlotItemType3::SmallCaliberMainGun
                            | KcSlotItemType3::MediumCaliberMainGun
                            | KcSlotItemType3::LargeCaliberMainGun
                            | KcSlotItemType3::LargeCaliberMainGun2
                    )
                })
        })
        .count()
}

fn count_secondary_guns(codex: &Codex, ship: &BattleRuntimeShip) -> usize {
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(|t| {
                    matches!(t, KcSlotItemType3::SecondaryGun | KcSlotItemType3::SecondaryGun2)
                })
        })
        .count()
}

fn has_radar(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().any(|si| {
        codex
            .find::<ApiMstSlotitem>(&si.api_slotitem_id)
            .ok()
            .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
            .is_some_and(|t| {
                matches!(
                    t,
                    KcSlotItemType3::SmallRadar
                        | KcSlotItemType3::LargeRadar
                        | KcSlotItemType3::LargeRadar2
                )
            })
    })
}

/// Ship has a seaplane (水偵 or 水爆) with onslot > 0.
/// 水戦 does NOT qualify for artillery spotting.
fn has_active_seaplane(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(si, onslot)| {
        if onslot <= 0 {
            return false;
        }
        let Some(mst) = codex.find::<ApiMstSlotitem>(&si.api_slotitem_id).ok() else {
            return false;
        };
        let Some(t) = KcSlotItemType3::n(mst.api_type[2]) else {
            return false;
        };
        matches!(t, KcSlotItemType3::SeaBasedRecon | KcSlotItemType3::SeaBasedBomber)
    })
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Detect the best applicable day CI type for a ship.
///
/// Returns `None` if prerequisites (air state + active seaplane) are not met.
/// Priority: MainApMain(6) > MainApSec(5) > MainRadar(4) > MainSec(3).
/// DoubleAttack(2) is NOT returned here — it's a trigger-roll fallback in U5.
pub(crate) fn detect_day_attack_type(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    air_state: Option<&AirState>,
) -> Option<DayAttackType> {
    // Prerequisite: air superiority or supremacy
    let air = air_state?;
    if !matches!(air, AirState::Supremacy | AirState::Superiority) {
        return None;
    }

    // Prerequisite: active seaplane (水偵 or 水爆 with onslot > 0)
    if !has_active_seaplane(codex, ship) {
        return None;
    }

    let main = count_main_guns(codex, ship);
    let sec = count_secondary_guns(codex, ship);
    let ap = count_type(codex, ship, KcSlotItemType3::ArmorPiercingShell);
    let radar = has_radar(codex, ship);

    // Priority order: highest first
    if main >= 2 && ap >= 1 {
        return Some(DayAttackType::MainApMainCI);
    }
    if main >= 1 && sec >= 1 && ap >= 1 {
        return Some(DayAttackType::MainApSecCI);
    }
    if main >= 1 && sec >= 1 && radar {
        return Some(DayAttackType::MainRadarCI);
    }
    if main >= 1 && sec >= 1 {
        return Some(DayAttackType::MainSecCI);
    }

    None
}

/// Detect carrier CI (戦爆連合CI) for CV/CVL/CVB ships.
///
/// Returns the sub-type (FBA > BBA > BA) if conditions are met, `None` otherwise.
/// Requires: air superiority/supremacy + dive bomber + torpedo bomber.
/// Jets do NOT count as dive bombers.
pub(crate) fn detect_carrier_ci(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    air_state: Option<&AirState>,
) -> Option<CarrierCiSubType> {
    if !is_cv_type(codex, ship) {
        return None;
    }

    let air = air_state?;
    if !matches!(air, AirState::Supremacy | AirState::Superiority) {
        return None;
    }

    let fighters = count_type(codex, ship, KcSlotItemType3::CarrierBasedFighter);
    let dive_bombers = count_type(codex, ship, KcSlotItemType3::CarrierBasedDiveBomber);
    let torpedo_bombers = count_type(codex, ship, KcSlotItemType3::CarrierBasedTorpedoBomber);

    // Need at least 1 dive bomber + 1 torpedo bomber
    if dive_bombers == 0 || torpedo_bombers == 0 {
        return None;
    }

    // Priority: FBA > BBA > BA
    if fighters >= 1 {
        return Some(CarrierCiSubType::Fba);
    }
    if dive_bombers >= 2 {
        return Some(CarrierCiSubType::Bba);
    }
    Some(CarrierCiSubType::Ba)
}

/// Whether the ship qualifies for DoubleAttack fallback:
/// has 2+ main guns, plus the air state + seaplane prerequisites.
pub(crate) fn can_double_attack(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    air_state: Option<&AirState>,
) -> bool {
    let air = match air_state {
        Some(a) => a,
        None => return false,
    };
    if !matches!(air, AirState::Supremacy | AirState::Superiority) {
        return false;
    }
    if !has_active_seaplane(codex, ship) {
        return false;
    }
    count_main_guns(codex, ship) >= 2
}

/// Per-type base_attack denominator for trigger rate formula.
pub(crate) fn day_ci_base_attack(at_type: DayAttackType) -> f64 {
    match at_type {
        DayAttackType::MainApMainCI => 150.0,
        DayAttackType::MainApSecCI => 140.0,
        DayAttackType::MainRadarCI => 130.0,
        DayAttackType::MainSecCI => 120.0,
        DayAttackType::DoubleAttack => 130.0,
        DayAttackType::CarrierCI => 140.0,
        DayAttackType::Normal => 0.0,
    }
}

/// Resolved day attack: the final attack type determined after trigger rolls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedDayAttack {
    pub(crate) at_type: DayAttackType,
    /// Number of hits (1 for CI/normal, 2 for DoubleAttack).
    pub(crate) hit_count: usize,
    /// Post-cap damage multiplier per hit.
    pub(crate) damage_multiplier: f64,
}

/// Calculate the trigger rate (0.0–1.0) for a day CI type.
///
/// Formula:
/// - Under AS: `Base = floor(sqrt(Luck) + 0.6 * (1.2 * LoS_equip + floor(sqrt(LoS_fleet) + LoS_fleet/10)))`
/// - Under AS+: `Base = floor(sqrt(Luck) + 0.7 * (1.6 * LoS_equip + floor(sqrt(LoS_fleet) + LoS_fleet/10)) + 10)`
/// - `Rate = (10 + Base + FlagshipBonus) / Base_attack`
/// - FlagshipBonus = 15 if index 0 in fleet
fn day_ci_trigger_rate(
    ship: &BattleRuntimeShip,
    air_state: &AirState,
    fleet_los: i64,
    is_flagship: bool,
    at_type: DayAttackType,
) -> f64 {
    let luck = ship.ship.api_lucky[0].max(0) as f64;
    let los_equip = ship_los_from_equipment(ship);

    let base = match air_state {
        AirState::Supremacy => (luck.sqrt().floor()
            + 0.7 * (1.6 * los_equip + (los_fleet_term(fleet_los)).floor())
            + 10.0)
            .floor(),
        AirState::Superiority => (luck.sqrt().floor()
            + 0.6 * (1.2 * los_equip + (los_fleet_term(fleet_los)).floor()))
        .floor(),
        _ => return 0.0,
    };

    let flagship_bonus = if is_flagship {
        15.0
    } else {
        0.0
    };
    let base_attack = day_ci_base_attack(at_type);
    if base_attack <= 0.0 {
        return 0.0;
    }

    (10.0 + base + flagship_bonus) / base_attack
}

/// `floor(sqrt(LoS_fleet) + LoS_fleet / 10)`
fn los_fleet_term(fleet_los: i64) -> f64 {
    let f = fleet_los as f64;
    f.sqrt().floor() + f / 10.0
}

/// Sum of equipment LoS (`api_saku`) from all equipped items.
fn ship_los_from_equipment(ship: &BattleRuntimeShip) -> f64 {
    // This is approximate — the real game uses a weighted formula per equipment.
    // For now, use the ship's total sakuteki stat minus base.
    // A more precise implementation would sum individual equipment api_saku values.
    ship.ship.api_sakuteki[0].max(0) as f64
}

/// Resolve the day attack type for a ship: detect CI, roll trigger, fallback.
///
/// Returns a [`ResolvedDayAttack`] with the final attack type, hit count, and
/// damage multiplier.
pub(crate) fn resolve_day_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    ship: &BattleRuntimeShip,
    air_state: Option<&AirState>,
    fleet_los: i64,
    ship_index: usize,
) -> ResolvedDayAttack {
    let air = match air_state {
        Some(a) => a,
        None => return normal_attack(),
    };
    if !matches!(air, AirState::Supremacy | AirState::Superiority) {
        return normal_attack();
    }

    let is_flagship = ship_index == 0;

    // Carrier CI (mutually exclusive with artillery spotting)
    if is_cv_type(codex, ship) {
        if let Some(sub) = detect_carrier_ci(codex, ship, Some(air)) {
            return ResolvedDayAttack {
                at_type: DayAttackType::CarrierCI,
                hit_count: 1,
                damage_multiplier: sub.damage_multiplier(),
            };
        }
        return normal_attack();
    }

    // Artillery spotting CI detection and trigger roll
    if let Some(ci_type) = detect_day_attack_type(codex, ship, Some(air)) {
        let rate = day_ci_trigger_rate(ship, air, fleet_los, is_flagship, ci_type);
        if rng.random_f64_range(0.0, 1.0) < rate.min(1.0) {
            return ResolvedDayAttack {
                at_type: ci_type,
                hit_count: 1,
                damage_multiplier: day_ci_damage_multiplier(ci_type),
            };
        }
    }

    // Fallback: DoubleAttack
    if can_double_attack(codex, ship, Some(air)) {
        let rate =
            day_ci_trigger_rate(ship, air, fleet_los, is_flagship, DayAttackType::DoubleAttack);
        if rng.random_f64_range(0.0, 1.0) < rate.min(1.0) {
            return ResolvedDayAttack {
                at_type: DayAttackType::DoubleAttack,
                hit_count: 2,
                damage_multiplier: day_ci_damage_multiplier(DayAttackType::DoubleAttack),
            };
        }
    }

    normal_attack()
}

fn normal_attack() -> ResolvedDayAttack {
    ResolvedDayAttack {
        at_type: DayAttackType::Normal,
        hit_count: 1,
        damage_multiplier: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::{KcShipType, KcSlotItemType3};

    fn with_seaplane(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let seaplane_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::SeaBasedRecon);
        let idx = ship.slot_items.len();
        ship.slot_items.push(slotitem_with_mst_id(seaplane_id));
        if idx < ship.ship.api_onslot.len() {
            ship.ship.api_onslot[idx] = 1;
        }
    }

    fn with_ap_shell(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let ap_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::ArmorPiercingShell);
        ship.slot_items.push(slotitem_with_mst_id(ap_id));
    }

    fn with_secondary(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let sec_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::SecondaryGun);
        ship.slot_items.push(slotitem_with_mst_id(sec_id));
    }

    fn with_radar(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let radar_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::SmallRadar);
        ship.slot_items.push(slotitem_with_mst_id(radar_id));
    }

    fn with_main_gun(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let main_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::LargeCaliberMainGun);
        ship.slot_items.push(slotitem_with_mst_id(main_id));
    }

    /// Create a BB with no equipment and zeroed onslot.
    fn bare_bb(codex: &Codex) -> BattleRuntimeShip {
        let bb_mst = first_ship_mst_by_type(codex, KcShipType::BB);
        let mut input = sample_ship(codex, bb_mst, 99);
        input.slot_items.clear();
        input.ship.api_onslot = [0; 5];
        BattleRuntimeShip::from(input)
    }

    #[test]
    fn main_ap_main_ci_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship); // main 1
        with_main_gun(&codex, &mut ship); // main 2
        with_ap_shell(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, Some(DayAttackType::MainApMainCI));
        assert_eq!(result.unwrap().api_id(), 6);
    }

    #[test]
    fn main_ap_sec_ci_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_secondary(&codex, &mut ship);
        with_ap_shell(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Superiority));
        assert_eq!(result, Some(DayAttackType::MainApSecCI));
    }

    #[test]
    fn main_radar_ci_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_secondary(&codex, &mut ship);
        with_radar(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, Some(DayAttackType::MainRadarCI));
    }

    #[test]
    fn main_sec_ci_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_secondary(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Superiority));
        assert_eq!(result, Some(DayAttackType::MainSecCI));
    }

    #[test]
    fn no_ci_without_ap_shell_when_2_main_guns() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        // 2 main guns but no AP shell → no CI type detected (DoubleAttack is U5 fallback)
        assert_eq!(result, None);
    }

    #[test]
    fn no_ci_at_air_parity() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_ap_shell(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Parity));
        assert_eq!(result, None);
    }

    #[test]
    fn no_ci_without_seaplane() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_ap_shell(&codex, &mut ship);
        // no seaplane

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, None);
    }

    #[test]
    fn no_ci_with_seaplane_onslot_zero() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_ap_shell(&codex, &mut ship);

        let seaplane_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedRecon);
        ship.slot_items.push(slotitem_with_mst_id(seaplane_id));
        // onslot left at 0 — seaplane shot down

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, None);
    }

    #[test]
    fn no_ci_with_water_fighter_instead_of_recon() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_ap_shell(&codex, &mut ship);

        let wf_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaplaneFighter);
        ship.slot_items.push(slotitem_with_mst_id(wf_id));
        ship.ship.api_onslot[ship.slot_items.len() - 1] = 1;

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, None, "水戦 should not qualify for artillery spotting");
    }

    #[test]
    fn priority_main_ap_main_wins_over_main_ap_sec() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship); // main 1
        with_main_gun(&codex, &mut ship); // main 2
        with_secondary(&codex, &mut ship); // also has secondary
        with_ap_shell(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, Some(DayAttackType::MainApMainCI));
    }

    #[test]
    fn no_air_state_means_no_ci() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_ap_shell(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        let result = detect_day_attack_type(&codex, &ship, None);
        assert_eq!(result, None);
    }

    #[test]
    fn double_attack_eligible_with_2_main_guns() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_main_gun(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        assert!(can_double_attack(&codex, &ship, Some(&AirState::Supremacy)));
    }

    #[test]
    fn double_attack_not_eligible_with_1_main_gun() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_main_gun(&codex, &mut ship);
        with_seaplane(&codex, &mut ship);

        assert!(!can_double_attack(&codex, &ship, Some(&AirState::Supremacy)));
    }

    // -- Carrier CI tests --

    fn cvl_ship(codex: &Codex) -> BattleRuntimeShip {
        let cvl_mst = first_ship_mst_by_type(codex, KcShipType::CVL);
        let mut input = sample_ship(codex, cvl_mst, 99);
        input.slot_items.clear();
        input.ship.api_onslot = [0; 5];
        BattleRuntimeShip::from(input)
    }

    fn with_fighter(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let id = first_slotitem_mst_by_type(codex, KcSlotItemType3::CarrierBasedFighter);
        ship.slot_items.push(slotitem_with_mst_id(id));
    }

    fn with_dive_bomber(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let id = first_slotitem_mst_by_type(codex, KcSlotItemType3::CarrierBasedDiveBomber);
        ship.slot_items.push(slotitem_with_mst_id(id));
    }

    fn with_torpedo_bomber(codex: &Codex, ship: &mut BattleRuntimeShip) {
        let id = first_slotitem_mst_by_type(codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        ship.slot_items.push(slotitem_with_mst_id(id));
    }

    #[test]
    fn carrier_ci_fba_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = cvl_ship(&codex);
        with_fighter(&codex, &mut ship);
        with_dive_bomber(&codex, &mut ship);
        with_torpedo_bomber(&codex, &mut ship);

        let result = detect_carrier_ci(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, Some(CarrierCiSubType::Fba));
    }

    #[test]
    fn carrier_ci_bba_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = cvl_ship(&codex);
        with_dive_bomber(&codex, &mut ship);
        with_dive_bomber(&codex, &mut ship);
        with_torpedo_bomber(&codex, &mut ship);

        let result = detect_carrier_ci(&codex, &ship, Some(&AirState::Superiority));
        assert_eq!(result, Some(CarrierCiSubType::Bba));
    }

    #[test]
    fn carrier_ci_ba_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = cvl_ship(&codex);
        with_dive_bomber(&codex, &mut ship);
        with_torpedo_bomber(&codex, &mut ship);

        let result = detect_carrier_ci(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, Some(CarrierCiSubType::Ba));
    }

    #[test]
    fn carrier_ci_no_torpedo_bomber() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = cvl_ship(&codex);
        with_fighter(&codex, &mut ship);
        with_dive_bomber(&codex, &mut ship);
        // no torpedo bomber

        let result = detect_carrier_ci(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, None);
    }

    #[test]
    fn carrier_ci_air_parity() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = cvl_ship(&codex);
        with_fighter(&codex, &mut ship);
        with_dive_bomber(&codex, &mut ship);
        with_torpedo_bomber(&codex, &mut ship);

        let result = detect_carrier_ci(&codex, &ship, Some(&AirState::Parity));
        assert_eq!(result, None);
    }

    #[test]
    fn carrier_ci_not_carrier() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = bare_bb(&codex);
        with_fighter(&codex, &mut ship);
        with_dive_bomber(&codex, &mut ship);
        with_torpedo_bomber(&codex, &mut ship);

        let result = detect_carrier_ci(&codex, &ship, Some(&AirState::Supremacy));
        assert_eq!(result, None);
    }
}
