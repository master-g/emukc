//! Target selection, eligibility checks, and display helpers for battle phases.

use emukc_model::{
    codex::Codex,
    kc2::{
        KcApiSlotItem, KcShipType, KcSlotItemType3,
        start2::{ApiMstShip, ApiMstSlotitem},
    },
};

use crate::random::BattleRng;
use crate::types::{AttackCapability, BattlePhase, BattleRuntimeShip, TargetClass};

// ---------------------------------------------------------------------------
// Constants — type-category lists used by display / eligibility helpers
// ---------------------------------------------------------------------------

const DAY_SURFACE_DISPLAY_TYPES: &[KcSlotItemType3] = &[
    KcSlotItemType3::SmallCaliberMainGun,
    KcSlotItemType3::MediumCaliberMainGun,
    KcSlotItemType3::LargeCaliberMainGun,
    KcSlotItemType3::SecondaryGun,
    KcSlotItemType3::LargeCaliberMainGun2,
    KcSlotItemType3::SecondaryGun2,
    KcSlotItemType3::CarrierBasedDiveBomber,
    KcSlotItemType3::CarrierBasedTorpedoBomber,
    KcSlotItemType3::SeaBasedBomber,
    KcSlotItemType3::JetFighterBomber,
    KcSlotItemType3::JetAttacker,
];

const ASW_DISPLAY_TYPES: &[KcSlotItemType3] = &[
    KcSlotItemType3::Sonar,
    KcSlotItemType3::LargeSonar,
    KcSlotItemType3::DepthCharge,
    KcSlotItemType3::AutoGyro,
    KcSlotItemType3::AntiSubmarinePatrol,
    KcSlotItemType3::SeaBasedBomber,
    KcSlotItemType3::LargeFlyingBoat,
];

// TODO(#0): used by night battle display helpers
const NIGHT_MAIN_GUN_TYPES: &[KcSlotItemType3] = &[
    KcSlotItemType3::SmallCaliberMainGun,
    KcSlotItemType3::MediumCaliberMainGun,
    KcSlotItemType3::LargeCaliberMainGun,
    KcSlotItemType3::LargeCaliberMainGun2,
];

// TODO(#0): used by night battle display helpers
const NIGHT_SECONDARY_GUN_TYPES: &[KcSlotItemType3] =
    &[KcSlotItemType3::SecondaryGun, KcSlotItemType3::SecondaryGun2];

// TODO(#0): used by night battle display helpers
const NIGHT_TORPEDO_TYPES: &[KcSlotItemType3] =
    &[KcSlotItemType3::Torpedo, KcSlotItemType3::SubmarineTorpedo];

// TODO(#0): used by night battle display helpers
const RADAR_DISPLAY_TYPES: &[KcSlotItemType3] =
    &[KcSlotItemType3::SmallRadar, KcSlotItemType3::LargeRadar, KcSlotItemType3::LargeRadar2];

const PT_TARGET_NAME_MARKERS: &[&str] = &["PT小鬼群", "Schnellboot小鬼群"];
const INSTALLATION_TARGET_NAME_MARKERS: &[&str] =
    &["砲台", "飛行場", "港湾", "離島", "集積地", "泊地", "要塞", "トーチカ"];

// ---------------------------------------------------------------------------
// Core targeting — random target selection
// ---------------------------------------------------------------------------

/// Select a random target index from `defenders` based on the attacker's
/// [`AttackCapability`] in the given `phase`.
pub(crate) fn select_random_target_index(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defenders: &[BattleRuntimeShip],
    phase: BattlePhase,
) -> Option<usize> {
    let alive_targets = defenders
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.is_alive())
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    if alive_targets.is_empty() {
        return None;
    }

    let surface_like_targets = alive_targets
        .iter()
        .copied()
        .filter(|idx| target_class(codex, &defenders[*idx]).is_surface_like())
        .collect::<Vec<_>>();
    let submarine_targets = alive_targets
        .iter()
        .copied()
        .filter(|idx| target_class(codex, &defenders[*idx]).is_submarine())
        .collect::<Vec<_>>();

    let candidates = match attack_capability_for_phase(codex, attacker, phase) {
        AttackCapability::CannotAttack => return None,
        AttackCapability::SurfaceOnly => surface_like_targets,
        AttackCapability::BothPreferSubmarine => {
            if submarine_targets.is_empty() {
                surface_like_targets
            } else {
                submarine_targets
            }
        }
    };
    if candidates.is_empty() {
        return None;
    }

    Some(
        candidates
            [rng.choose_index(candidates.len()).expect("candidates non-empty by construction")],
    )
}

/// Select a random alive submarine target.
pub(crate) fn select_submarine_target(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defenders: &[BattleRuntimeShip],
) -> Option<usize> {
    let subs: Vec<usize> = defenders
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.is_alive() && target_class(codex, ship).is_submarine())
        .map(|(idx, _)| idx)
        .collect();

    if subs.is_empty() {
        return None;
    }
    Some(subs[rng.choose_index(subs.len()).expect("subs non-empty by construction")])
}

// ---------------------------------------------------------------------------
// Flagship escort shield (旗艦援護 / かばう)
// ---------------------------------------------------------------------------

/// Interception probability for a defending fleet's formation, as a percentage.
///
/// Rates from the official wiki (`攻撃対象の選択`): 単縦陣 45%, 複縦/梯形/単横 60%,
/// 輪形/警戒 75%. Combined-fleet formation IDs (11–14) and any unknown ID return
/// `None` — combined-fleet interception is out of scope until combined sortie
/// exists, and an unknown formation never intercepts.
fn escort_shield_rate(formation_id: i64) -> Option<i64> {
    match formation_id {
        1 => Some(45),         // 単縦陣
        2 | 4 | 5 => Some(60), // 複縦陣 / 梯形陣 / 単横陣
        3 | 6 => Some(75),     // 輪形陣 / 警戒陣
        _ => None,
    }
}

/// Resolve 旗艦援護 (かばう): when an attack targets the defending fleet's
/// flagship (`target_idx == 0`), a healthy escort may intercept it and take the
/// damage instead. Returns the interceptor's index, or `None` when no
/// interception occurs.
///
/// Eligibility (no RNG): the flagship must be the target, the formation must
/// have a defined rate, and at least one escort must be non-flagship, alive, in
/// green health (小破未満, HP > 75%), and of the flagship's category (surface-like
/// ships protect surface-like flagships, submarines protect submarine flagships).
/// The flagship's own HP does **not** gate — interception fires even when the
/// flagship is 大破.
///
/// RNG draw order (deterministic): only when all eligibility holds does this draw
/// one `roll_range` for the probability, then one `choose_index` for the
/// interceptor on success. Eligibility-fail paths consume no RNG, mirroring
/// `choose_index`'s "no draw when empty" invariant.
pub(crate) fn select_escort_shield(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defenders: &[BattleRuntimeShip],
    target_idx: usize,
    defender_formation_id: i64,
) -> Option<usize> {
    if target_idx != 0 {
        return None;
    }
    let rate = escort_shield_rate(defender_formation_id)?;

    // Surface-like (incl. PT / installation) protects surface-like; submarine
    // protects submarine. The two categories are complementary.
    let flagship_is_submarine = target_class(codex, &defenders[0]).is_submarine();

    let eligible: Vec<usize> = defenders
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, ship)| {
            ship.is_alive()
                && ship.hp() * 4 > ship.ship.api_maxhp * 3
                && target_class(codex, ship).is_submarine() == flagship_is_submarine
        })
        .map(|(idx, _)| idx)
        .collect();
    if eligible.is_empty() {
        return None;
    }

    if rng.roll_range(0, 100) >= rate {
        return None;
    }
    let pick = rng.choose_index(eligible.len())?;
    Some(eligible[pick])
}

// ---------------------------------------------------------------------------
// Target classification
// ---------------------------------------------------------------------------

/// Determine the [`TargetClass`] of a ship.
pub(crate) fn target_class(codex: &Codex, ship: &BattleRuntimeShip) -> TargetClass {
    if matches!(ship_type(codex, ship), Some(KcShipType::SS | KcShipType::SSV)) {
        return TargetClass::Submarine;
    }

    if let Some(name) = ship_mst(codex, ship).map(|mst| mst.api_name.as_str()) {
        if is_pt_target_name(name) {
            return TargetClass::PtBoat;
        }
        if is_installation_target_name(name) {
            return TargetClass::Installation;
        }
    }

    TargetClass::SurfaceShip
}

/// Check whether a ship name matches PT-boat markers.
pub(crate) fn is_pt_target_name(name: &str) -> bool {
    PT_TARGET_NAME_MARKERS.iter().any(|marker| name.contains(marker))
}

/// Check whether a ship name matches installation markers.
pub(crate) fn is_installation_target_name(name: &str) -> bool {
    INSTALLATION_TARGET_NAME_MARKERS.iter().any(|marker| name.contains(marker))
}

// ---------------------------------------------------------------------------
// Ship master-data helpers
// ---------------------------------------------------------------------------

/// Look up the master ship definition.
pub(crate) fn ship_mst<'a>(
    codex: &'a Codex,
    ship: &'a BattleRuntimeShip,
) -> Option<&'a ApiMstShip> {
    codex.find::<ApiMstShip>(&ship.ship.api_ship_id).ok()
}

/// Get the ship type (stype) of a runtime ship.
pub(crate) fn ship_type(codex: &Codex, ship: &BattleRuntimeShip) -> Option<KcShipType> {
    ship_mst(codex, ship).and_then(|mst| KcShipType::n(mst.api_stype as i32))
}

// ---------------------------------------------------------------------------
// Equipment queries
// ---------------------------------------------------------------------------

/// Check whether the ship carries any equipment of the given type.
pub(crate) fn has_slotitem_type(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    wanted: KcSlotItemType3,
) -> bool {
    ship.slot_items.iter().any(|slot_item| {
        codex
            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
            .ok()
            .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
            == Some(wanted)
    })
}

/// Check whether the ship carries a specific equipment by master ID.
pub(crate) fn has_slotitem_id(ship: &BattleRuntimeShip, wanted: i64) -> bool {
    ship.slot_items.iter().any(|slot_item| slot_item.api_slotitem_id == wanted)
}

// ---------------------------------------------------------------------------
// Phase eligibility — per-ship checks
// ---------------------------------------------------------------------------

/// Whether a single ship may participate in the opening torpedo phase.
pub(crate) fn can_opening_torpedo_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() || ship.ship.api_raisou[0] <= 0 {
        return false;
    }

    match ship_type(codex, ship) {
        // CLT always fires opening torpedo (with base torpedo, checked above).
        Some(KcShipType::CLT) => true,
        // Submarines fire a preemptive torpedo natively from Lv 10; below that
        // they need a 甲標的 (special submarine) equipped, like any other ship.
        Some(KcShipType::SS | KcShipType::SSV) => {
            ship.ship.api_lv >= 10
                || has_slotitem_type(codex, ship, KcSlotItemType3::SpecialSubmarineVessel)
        }
        // Any other ship type opens torpedo only with an equipped 甲標的.
        _ => has_slotitem_type(codex, ship, KcSlotItemType3::SpecialSubmarineVessel),
    }
}

/// Whether a single ship may participate in the closing torpedo phase.
pub(crate) fn can_closing_torpedo_ship(_codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    // Base torpedo stat is the sole eligibility gate (wikiwiki 戦闘について): any
    // ship type with `api_raisou[0] > 0` participates — including BBs with a base
    // torpedo (Bismarck drei, 金剛型第三改装, Гангут, …) that a ship-type whitelist
    // wrongly excluded — while ship types with base torpedo 0 (DE, LHA, most BB,
    // carriers) fall out naturally.
    if ship.is_sunk() || ship.ship.api_raisou[0] <= 0 {
        return false;
    }

    // Chūha (HP ≤ 50%) ships cannot fire closing torpedo.
    if ship.hp() * 2 <= ship.ship.api_maxhp {
        return false;
    }

    true
}

/// Whether a single ship may fire in day shelling.
pub(crate) fn can_shell_day_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() {
        return false;
    }

    match ship_type(codex, ship) {
        Some(KcShipType::SS | KcShipType::SSV) => false,
        Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
            total_attack_plane_count(codex, std::slice::from_ref(ship)) > 0
        }
        _ => true,
    }
}

/// Whether a single ship may attack at night.
pub(crate) fn can_attack_night_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() {
        return false;
    }

    match ship_type(codex, ship) {
        Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
            // 夜間作戦航空要員, or a built-in 夜戦特性 (the exempt CVs), plus a carrier plane.
            let has_personnel = has_slotitem_id(ship, 258) || has_slotitem_id(ship, 259);
            let is_exempt =
                crate::simulation::night::EXEMPT_NIGHT_CV_IDS.contains(&ship.ship.api_ship_id);
            (has_personnel || is_exempt)
                && ship.slot_items.iter().any(|slot_item| {
                    codex
                        .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
                        .ok()
                        .is_some_and(|mst| is_air_combat_type(mst.api_type[2]))
                })
        }
        Some(KcShipType::SS | KcShipType::SSV) => false,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Attack capability for phase
// ---------------------------------------------------------------------------

/// Determine what kinds of targets a ship can engage in a given phase.
pub(crate) fn attack_capability_for_phase(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    phase: BattlePhase,
) -> AttackCapability {
    match phase {
        BattlePhase::OpeningTorpedo | BattlePhase::ClosingTorpedo => {
            if ship.is_alive() && ship.ship.api_raisou[0] > 0 {
                AttackCapability::SurfaceOnly
            } else {
                AttackCapability::CannotAttack
            }
        }
        BattlePhase::DayShelling => {
            if !can_shell_day_ship(codex, ship) {
                AttackCapability::CannotAttack
            } else if can_attack_submarine_day_shelling(codex, ship) {
                AttackCapability::BothPreferSubmarine
            } else {
                AttackCapability::SurfaceOnly
            }
        }
        BattlePhase::NightShelling => {
            if !can_attack_night_ship(codex, ship) {
                AttackCapability::CannotAttack
            } else if can_attack_submarine_night_shelling(codex, ship) {
                AttackCapability::BothPreferSubmarine
            } else {
                AttackCapability::SurfaceOnly
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ASW eligibility
// ---------------------------------------------------------------------------

/// Whether a ship can attack submarines during day shelling.
pub(crate) fn can_attack_submarine_day_shelling(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() || ship.ship.api_taisen[0] <= 0 {
        return false;
    }

    match ship_type(codex, ship) {
        Some(
            KcShipType::DE
            | KcShipType::DD
            | KcShipType::CL
            | KcShipType::CLT
            | KcShipType::CT
            | KcShipType::AO,
        ) => true,
        Some(
            KcShipType::BBV | KcShipType::CAV | KcShipType::AV | KcShipType::LHA | KcShipType::CVL,
        ) => has_active_asw_aircraft(codex, ship),
        _ => false,
    }
}

/// Whether a ship can attack submarines during night shelling.
pub(crate) fn can_attack_submarine_night_shelling(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() || ship.ship.api_taisen[0] <= 0 {
        return false;
    }

    match ship_type(codex, ship) {
        Some(
            KcShipType::DE
            | KcShipType::DD
            | KcShipType::CL
            | KcShipType::CLT
            | KcShipType::CT
            | KcShipType::AO,
        ) => true,
        Some(KcShipType::CV | KcShipType::CVL | KcShipType::CVB) => {
            can_attack_night_ship(codex, ship) && has_active_asw_aircraft(codex, ship)
        }
        _ => false,
    }
}

/// Whether a ship has at least one active ASW-capable aircraft.
pub(crate) fn has_active_asw_aircraft(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(slot_item, onslot)| {
        let Some(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok() else {
            return false;
        };
        matches!(
            KcSlotItemType3::n(mst.api_type[2]),
            Some(
                KcSlotItemType3::AutoGyro
                    | KcSlotItemType3::AntiSubmarinePatrol
                    | KcSlotItemType3::SeaBasedBomber
                    | KcSlotItemType3::LargeFlyingBoat
            )
        ) && onslot > 0
    })
}

/// Check if a ship can perform OASW (opening anti-submarine warfare).
pub(crate) fn can_opening_asw(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() {
        return false;
    }
    let asw = ship.ship.api_taisen[0];
    let stype = ship_type(codex, ship);

    match stype {
        // DE: ASW >= 60
        Some(KcShipType::DE) => {
            asw >= 60
                && (has_slotitem_type(codex, ship, KcSlotItemType3::Sonar)
                    || has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar))
        }
        // DD/CL/CT/CLT/AO: ASW >= 100 + sonar
        Some(
            KcShipType::DD | KcShipType::CL | KcShipType::CT | KcShipType::CLT | KcShipType::AO,
        ) => {
            asw >= 100
                && (has_slotitem_type(codex, ship, KcSlotItemType3::Sonar)
                    || has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar))
        }
        // CVL: ASW >= 65 + has ASW aircraft
        Some(KcShipType::CVL) => asw >= 65 && has_active_asw_aircraft(codex, ship),
        // CVB: ASW >= 100 + has ASW aircraft
        Some(KcShipType::CVB) => asw >= 100 && has_active_asw_aircraft(codex, ship),
        // BBV: ASW >= 100 + large sonar + ASW aircraft
        Some(KcShipType::BBV) => {
            asw >= 100
                && has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar)
                && has_active_asw_aircraft(codex, ship)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Fleet-level phase eligibility checks
// ---------------------------------------------------------------------------

/// Whether any ship in the fleet can fire an opening torpedo.
pub(crate) fn can_opening_torpedo(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
    ships.iter().any(|ship| can_opening_torpedo_ship(codex, ship))
}

/// Whether any ship in the fleet can fire a closing torpedo.
pub(crate) fn can_closing_torpedo(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
    ships.iter().any(|ship| can_closing_torpedo_ship(codex, ship))
}

/// Whether any ship in the fleet is still alive.
pub fn any_alive(ships: &[BattleRuntimeShip]) -> bool {
    ships.iter().any(BattleRuntimeShip::is_alive)
}

/// Whether the fleet contains any battleship-class ship (FBB, BB, BBV, XBB).
/// Does not filter by alive status — checks battle-start presence.
pub(crate) fn fleet_has_bb_class(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
    ships.iter().any(|ship| {
        matches!(
            ship_type(codex, ship),
            Some(KcShipType::FBB | KcShipType::BB | KcShipType::BBV | KcShipType::XBB)
        )
    })
}

// ---------------------------------------------------------------------------
// Aircraft counting helpers
// ---------------------------------------------------------------------------

/// Count total attack-capable aircraft across a fleet (dive bombers, torpedo
/// bombers, sea-based bombers, jets).
pub(crate) fn total_attack_plane_count(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
    ships
        .iter()
        .flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
        .filter(|(slot_item, onslot)| {
            *onslot > 0
                && codex
                    .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
                    .ok()
                    .is_some_and(|mst| is_airstrike_attack_type(mst.api_type[2]))
        })
        .map(|(_, onslot)| onslot)
        .sum()
}

/// Whether the slot-item type is any air-combat type (fighters, bombers, recon, jets).
pub(crate) fn is_air_combat_type(slotitem_type: i64) -> bool {
    matches!(
        KcSlotItemType3::n(slotitem_type),
        Some(
            KcSlotItemType3::CarrierBasedFighter
                | KcSlotItemType3::CarrierBasedDiveBomber
                | KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::CarrierBasedRecon
                | KcSlotItemType3::CarrierBasedRecon2
                | KcSlotItemType3::SeaBasedBomber
                | KcSlotItemType3::SeaBasedRecon
                | KcSlotItemType3::SeaplaneFighter
                | KcSlotItemType3::JetFighter
                | KcSlotItemType3::JetFighterBomber
                | KcSlotItemType3::JetAttacker
                | KcSlotItemType3::JetRecon
        )
    )
}

/// Whether the slot-item type is an airstrike-capable attack type.
pub(crate) fn is_airstrike_attack_type(slotitem_type: i64) -> bool {
    matches!(
        KcSlotItemType3::n(slotitem_type),
        Some(
            KcSlotItemType3::CarrierBasedDiveBomber
                | KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::SeaBasedBomber
                | KcSlotItemType3::JetFighterBomber
                | KcSlotItemType3::JetAttacker
        )
    )
}

// ---------------------------------------------------------------------------
// Slot-item master-data helper
// ---------------------------------------------------------------------------

/// Look up the master slot-item definition.
pub(crate) fn slotitem_mst<'a>(
    codex: &'a Codex,
    slot_item: &'a KcApiSlotItem,
) -> Option<&'a ApiMstSlotitem> {
    codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()
}

// ---------------------------------------------------------------------------
// Display helpers — day / ASW / night type predicates
// ---------------------------------------------------------------------------

/// Choose the display damage value for the battle animation log.
/// Enemy defenders show raw pre-protection damage for the overkill visual effect.
/// Friendly defenders show actual dealt (capped) damage so the client tracks HP correctly.
pub(crate) fn display_damage(defender: &BattleRuntimeShip, raw: i64, dealt: i64) -> i64 {
    if !defender.is_friendly {
        raw
    } else {
        dealt
    }
}

/// Whether the slot type is shown in day surface attack displays.
pub(crate) fn is_day_surface_display_type(slot_type: KcSlotItemType3) -> bool {
    DAY_SURFACE_DISPLAY_TYPES.contains(&slot_type)
}

/// Whether a slot item qualifies for ASW display.
pub(crate) fn is_asw_display_slotitem(codex: &Codex, slot_item: &KcApiSlotItem) -> bool {
    let Some(mst) = slotitem_mst(codex, slot_item) else {
        return false;
    };
    let Some(slot_type) = KcSlotItemType3::n(mst.api_type[2]) else {
        return false;
    };

    ASW_DISPLAY_TYPES.contains(&slot_type)
        || (slot_type == KcSlotItemType3::CarrierBasedTorpedoBomber && mst.api_tais > 0)
}

/// Collect ASW-eligible slot-item master IDs from a ship.
pub(crate) fn collect_asw_display_ids(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<i64> {
    ship.slot_items
        .iter()
        .filter(|slot_item| is_asw_display_slotitem(codex, slot_item))
        .map(|slot_item| slot_item.api_slotitem_id)
        .collect()
}

/// Whether the slot type counts as a main gun for night battle formulas.
// TODO(#0): used by night battle display helpers
#[expect(dead_code)]
pub(crate) fn is_night_main_gun_type(slot_type: KcSlotItemType3) -> bool {
    NIGHT_MAIN_GUN_TYPES.contains(&slot_type)
}

/// Whether the slot type counts as a secondary gun for night battle formulas.
// TODO(#0): used by night battle display helpers
#[expect(dead_code)]
pub(crate) fn is_night_secondary_gun_type(slot_type: KcSlotItemType3) -> bool {
    NIGHT_SECONDARY_GUN_TYPES.contains(&slot_type)
}

/// Whether the slot type counts as a torpedo for night battle formulas.
// TODO(#0): used by night battle display helpers
#[expect(dead_code)]
pub(crate) fn is_night_torpedo_type(slot_type: KcSlotItemType3) -> bool {
    NIGHT_TORPEDO_TYPES.contains(&slot_type)
}

/// Whether the slot type is a radar.
// TODO(#0): used by night battle display helpers
#[expect(dead_code)]
pub(crate) fn is_radar_type(slot_type: KcSlotItemType3) -> bool {
    RADAR_DISPLAY_TYPES.contains(&slot_type)
}

/// Collect slot-item IDs matching a custom predicate on `(slot_type, mst)`.
pub(crate) fn collect_matching_slot_ids(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    matcher: impl Fn(KcSlotItemType3, &ApiMstSlotitem) -> bool,
) -> Vec<i64> {
    ship.slot_items
        .iter()
        .filter_map(|slot_item| {
            let mst = slotitem_mst(codex, slot_item)?;
            let slot_type = KcSlotItemType3::n(mst.api_type[2])?;
            matcher(slot_type, mst).then_some(slot_item.api_slotitem_id)
        })
        .collect()
}

/// Return the first ID or `[-1]` if empty (the "no equipment" sentinel).
pub(crate) fn first_or_default(ids: Vec<i64>) -> Vec<i64> {
    if ids.is_empty() {
        vec![-1]
    } else {
        vec![ids[0]]
    }
}

/// Extend `target` with items from `source` up to `limit` total entries.
pub(crate) fn extend_limit(target: &mut Vec<i64>, source: &[i64], limit: usize) {
    for id in source {
        if target.len() >= limit {
            break;
        }
        target.push(*id);
    }
}

/// Compute the display slot-item IDs for a day attack.
pub(crate) fn day_attack_display_ids(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    is_submarine_target: bool,
) -> Vec<i64> {
    if is_submarine_target {
        let asw_ids = collect_asw_display_ids(codex, ship);
        if !asw_ids.is_empty() {
            return first_or_default(asw_ids);
        }
    }

    let surface_ids = collect_matching_slot_ids(codex, ship, |slot_type, _mst| {
        is_day_surface_display_type(slot_type)
    });
    first_or_default(surface_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::{BattlePhase, BattleRuntimeShip, TargetClass};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    #[test]
    fn day_shelling_destroyer_prefers_submarine_targets() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

        let attacker = BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50));
        let defenders = vec![
            BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50)),
            BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
        ];
        let mut rng = crate::random::SeededRng::new(7);

        let target_idx = select_random_target_index(
            &codex,
            &mut rng,
            &attacker,
            &defenders,
            BattlePhase::DayShelling,
        )
        .unwrap();

        assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::Submarine);
    }

    #[test]
    fn day_shelling_battleship_ignores_submarine_targets() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

        let attacker = BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50));
        let defenders = vec![
            BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
            BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50)),
        ];
        let mut rng = crate::random::SeededRng::new(7);

        let target_idx = select_random_target_index(
            &codex,
            &mut rng,
            &attacker,
            &defenders,
            BattlePhase::DayShelling,
        )
        .unwrap();

        assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::SurfaceShip);
    }

    #[test]
    fn target_taxonomy_classifies_pt_and_installation_targets_explicitly() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let pt_mst = ship_mst_id_by_name(&codex, "PT小鬼群");
        let installation_mst = ship_mst_id_by_name(&codex, "飛行場姫");

        let surface = BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50));
        let submarine = BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50));

        let mut pt = sample_ship(&codex, dd_mst, 50);
        pt.ship.api_ship_id = pt_mst;
        let pt = BattleRuntimeShip::from(pt);

        let mut installation = sample_ship(&codex, dd_mst, 50);
        installation.ship.api_ship_id = installation_mst;
        let installation = BattleRuntimeShip::from(installation);

        assert_eq!(target_class(&codex, &surface), TargetClass::SurfaceShip);
        assert_eq!(target_class(&codex, &submarine), TargetClass::Submarine);
        assert_eq!(target_class(&codex, &pt), TargetClass::PtBoat);
        assert_eq!(target_class(&codex, &installation), TargetClass::Installation);
    }

    #[test]
    fn surface_only_targeting_keeps_pt_targets_in_surface_bucket() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let pt_mst = ship_mst_id_by_name(&codex, "PT小鬼群");

        let attacker = BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50));
        let mut pt = sample_ship(&codex, dd_mst, 50);
        pt.ship.api_ship_id = pt_mst;
        let defenders = vec![
            BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
            BattleRuntimeShip::from(pt),
        ];
        let mut rng = crate::random::SeededRng::new(13);

        let target_idx = select_random_target_index(
            &codex,
            &mut rng,
            &attacker,
            &defenders,
            BattlePhase::DayShelling,
        )
        .unwrap();

        assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::PtBoat);
    }

    #[test]
    fn torpedo_targeting_keeps_installations_in_surface_bucket_for_now() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let installation_mst = ship_mst_id_by_name(&codex, "飛行場姫");

        let attacker = BattleRuntimeShip::from(sample_ship(&codex, clt_mst, 50));
        let mut installation = sample_ship(&codex, dd_mst, 50);
        installation.ship.api_ship_id = installation_mst;
        let defenders = vec![
            BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
            BattleRuntimeShip::from(installation),
        ];
        let mut rng = crate::random::SeededRng::new(17);

        let target_idx = select_random_target_index(
            &codex,
            &mut rng,
            &attacker,
            &defenders,
            BattlePhase::ClosingTorpedo,
        )
        .unwrap();

        assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::Installation);
    }

    #[test]
    fn day_shelling_display_ids_skip_non_attack_equipment_like_night_recon() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let night_recon_mst_id = slotitem_mst_id_by_name(&codex, "九八式水上偵察機(夜偵)");
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);

        let mut ship = sample_ship(&codex, bb_mst, 50);
        ship.slot_items =
            vec![slotitem_with_mst_id(night_recon_mst_id), slotitem_with_mst_id(main_gun_mst_id)];
        let runtime_ship = BattleRuntimeShip::from(ship);

        assert_eq!(day_attack_display_ids(&codex, &runtime_ship, false), vec![main_gun_mst_id]);
    }

    #[test]
    fn day_asw_display_ids_ignore_night_recon_when_valid_asw_equipment_exists() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);
        let night_recon_mst_id = slotitem_mst_id_by_name(&codex, "九八式水上偵察機(夜偵)");
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let mut ship = sample_ship(&codex, bbv_mst, 50);
        ship.slot_items =
            vec![slotitem_with_mst_id(night_recon_mst_id), slotitem_with_mst_id(sonar_mst_id)];
        let runtime_ship = BattleRuntimeShip::from(ship);

        assert_eq!(day_attack_display_ids(&codex, &runtime_ship, true), vec![sonar_mst_id]);
    }

    #[test]
    fn torpedo_targeting_ignores_submarines() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

        let attacker = BattleRuntimeShip::from(sample_ship(&codex, clt_mst, 50));
        let mixed_defenders = vec![
            BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
            BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50)),
        ];
        let submarine_only = vec![BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50))];
        let mut rng = crate::random::SeededRng::new(11);

        let target_idx = select_random_target_index(
            &codex,
            &mut rng,
            &attacker,
            &mixed_defenders,
            BattlePhase::ClosingTorpedo,
        )
        .unwrap();
        assert_eq!(target_class(&codex, &mixed_defenders[target_idx]), TargetClass::SurfaceShip);
        assert!(
            select_random_target_index(
                &codex,
                &mut rng,
                &attacker,
                &submarine_only,
                BattlePhase::OpeningTorpedo,
            )
            .is_none()
        );
    }

    #[test]
    fn oasw_requires_sufficient_asw_and_sonar() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        // DD with ASW 100 + sonar → can OASW
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.ship.api_taisen[0] = 100;
        ship.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
        let rt = BattleRuntimeShip::from(ship);
        assert!(can_opening_asw(&codex, &rt));

        // DD with ASW 99 + sonar → cannot OASW
        let mut ship2 = sample_ship(&codex, dd_mst, 99);
        ship2.ship.api_taisen[0] = 99;
        ship2.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
        let rt2 = BattleRuntimeShip::from(ship2);
        assert!(!can_opening_asw(&codex, &rt2));

        // DD with ASW 100 but no sonar → cannot OASW
        let mut ship3 = sample_ship(&codex, dd_mst, 99);
        ship3.ship.api_taisen[0] = 100;
        ship3.slot_items = vec![];
        let rt3 = BattleRuntimeShip::from(ship3);
        assert!(!can_opening_asw(&codex, &rt3));
    }

    #[test]
    fn oasw_de_threshold_is_60() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let de_mst = first_ship_mst_by_type(&codex, KcShipType::DE);
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let mut ship = sample_ship(&codex, de_mst, 50);
        ship.ship.api_taisen[0] = 60;
        ship.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
        let rt = BattleRuntimeShip::from(ship);
        assert!(can_opening_asw(&codex, &rt));

        let mut ship2 = sample_ship(&codex, de_mst, 50);
        ship2.ship.api_taisen[0] = 59;
        ship2.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
        let rt2 = BattleRuntimeShip::from(ship2);
        assert!(!can_opening_asw(&codex, &rt2));
    }

    #[test]
    fn oasw_targets_submarines_only() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let defenders = vec![
            BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50)),
            BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50)),
        ];
        let mut rng = crate::random::SeededRng::new(42);

        // Should always select index 1 (the submarine), never index 0 (the DD)
        for _ in 0..10 {
            let idx = select_submarine_target(&codex, &mut rng, &defenders).unwrap();
            assert_eq!(idx, 1, "OASW should only target submarines");
        }
    }

    #[test]
    fn closing_torpedo_rejects_chuha_ship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Healthy DD → eligible
        let mut healthy = sample_ship(&codex, dd_mst, 50);
        healthy.ship.api_raisou[0] = 50;
        let rt_healthy = BattleRuntimeShip::from(healthy);
        assert!(can_closing_torpedo_ship(&codex, &rt_healthy));

        // Chūha exact boundary: hp = maxhp/2 → rejected
        let mut chuha = sample_ship(&codex, dd_mst, 50);
        chuha.ship.api_raisou[0] = 50;
        chuha.ship.api_maxhp = 10;
        chuha.ship.api_nowhp = 5;
        let rt_chuha = BattleRuntimeShip::new(chuha, false, true);
        assert!(!can_closing_torpedo_ship(&codex, &rt_chuha), "hp=5, maxhp=10 → chūha");

        // Shōha (still > 50%): hp = maxhp/2 + 1 → eligible
        let mut shoha = sample_ship(&codex, dd_mst, 50);
        shoha.ship.api_raisou[0] = 50;
        shoha.ship.api_maxhp = 10;
        shoha.ship.api_nowhp = 6;
        let rt_shoha = BattleRuntimeShip::new(shoha, false, true);
        assert!(can_closing_torpedo_ship(&codex, &rt_shoha), "hp=6, maxhp=10 → shōha");

        // Odd maxhp boundary: maxhp=7, hp=3 → 3*2=6 ≤ 7 → chūha
        let mut odd_chuha = sample_ship(&codex, dd_mst, 50);
        odd_chuha.ship.api_raisou[0] = 50;
        odd_chuha.ship.api_maxhp = 7;
        odd_chuha.ship.api_nowhp = 3;
        let rt_odd_chuha = BattleRuntimeShip::new(odd_chuha, false, true);
        assert!(!can_closing_torpedo_ship(&codex, &rt_odd_chuha), "hp=3, maxhp=7 → chūha");

        // Odd maxhp boundary: maxhp=7, hp=4 → 4*2=8 > 7 → eligible
        let mut odd_shoha = sample_ship(&codex, dd_mst, 50);
        odd_shoha.ship.api_raisou[0] = 50;
        odd_shoha.ship.api_maxhp = 7;
        odd_shoha.ship.api_nowhp = 4;
        let rt_odd_shoha = BattleRuntimeShip::new(odd_shoha, false, true);
        assert!(can_closing_torpedo_ship(&codex, &rt_odd_shoha), "hp=4, maxhp=7 → shōha");

        // Zero hp: rejected by is_sunk() pre-check
        let mut sunk = sample_ship(&codex, dd_mst, 50);
        sunk.ship.api_raisou[0] = 50;
        sunk.ship.api_maxhp = 10;
        sunk.ship.api_nowhp = 0;
        let rt_sunk = BattleRuntimeShip::new(sunk, false, true);
        assert!(!can_closing_torpedo_ship(&codex, &rt_sunk));

        // Zero raisou: rejected by pre-existing filter
        let mut zero_raisou = sample_ship(&codex, dd_mst, 50);
        zero_raisou.ship.api_raisou[0] = 0;
        let rt_zero_raisou = BattleRuntimeShip::from(zero_raisou);
        assert!(!can_closing_torpedo_ship(&codex, &rt_zero_raisou));

        // A stock BB has base torpedo 0, so the stat gate excludes it (a BB *with*
        // base torpedo is covered by closing_torpedo_includes_battleship_with_base_torpedo).
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let bb = sample_ship(&codex, bb_mst, 50);
        let rt_bb = BattleRuntimeShip::from(bb);
        assert!(!can_closing_torpedo_ship(&codex, &rt_bb));
    }

    #[test]
    fn opening_torpedo_accepts_chuha_ship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);

        // Chūha CLT with positive raisou → eligible for opening torpedo (damage-agnostic)
        let mut chuha_clt = sample_ship(&codex, clt_mst, 50);
        chuha_clt.ship.api_raisou[0] = 50;
        chuha_clt.ship.api_maxhp = 10;
        chuha_clt.ship.api_nowhp = 3;
        let rt = BattleRuntimeShip::new(chuha_clt, false, true);
        assert!(can_opening_torpedo_ship(&codex, &rt), "opening torpedo ignores damage state");
    }

    /// U3.2: a submarine fires opening torpedo natively only from Lv 10. Below
    /// that, it is ineligible without a 甲標的.
    #[test]
    fn opening_torpedo_submarine_requires_level_10() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

        let mut low = sample_ship(&codex, ss_mst, 9);
        low.ship.api_raisou[0] = 50;
        let rt_low = BattleRuntimeShip::new(low, false, true);
        assert!(
            !can_opening_torpedo_ship(&codex, &rt_low),
            "SS Lv 9 without 甲標的 must not open-torpedo"
        );

        let mut at10 = sample_ship(&codex, ss_mst, 10);
        at10.ship.api_raisou[0] = 50;
        let rt_10 = BattleRuntimeShip::new(at10, false, true);
        assert!(can_opening_torpedo_ship(&codex, &rt_10), "SS Lv 10 opens torpedo equipment-free");
    }

    /// U3.1 + U3.2: a sub-Lv-10 submarine equipped with a 甲標的 is still eligible
    /// — the equipment path is preserved alongside the level gate.
    #[test]
    fn opening_torpedo_low_level_submarine_with_minisub() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let minisub = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SpecialSubmarineVessel);

        let mut low = sample_ship(&codex, ss_mst, 5);
        low.ship.api_raisou[0] = 50;
        low.slot_items = vec![slotitem_with_mst_id(minisub)];
        let rt = BattleRuntimeShip::new(low, false, true);
        assert!(
            can_opening_torpedo_ship(&codex, &rt),
            "Lv 5 SS with 甲標的 opens torpedo (equipment path)"
        );
    }

    #[test]
    fn display_damage_enemy_defender_returns_raw_overkill() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Enemy defender (is_friendly=false, is_sortie=true)
        let mut input = sample_ship(&codex, dd_mst, 50);
        input.ship.api_nowhp = 50;
        let enemy = BattleRuntimeShip::new(input, false, true);

        let display = display_damage(&enemy, 200, 50);
        assert_eq!(display, 200, "enemy defender should show raw damage (overkill)");
    }

    #[test]
    fn display_damage_friendly_defender_returns_dealt() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Friendly defender (is_friendly=true, is_sortie=true)
        let mut input = sample_ship(&codex, dd_mst, 50);
        input.ship.api_nowhp = 100;
        let friendly = BattleRuntimeShip::new(input, true, true);

        let display = display_damage(&friendly, 200, 30);
        assert_eq!(display, 30, "friendly defender should show dealt damage (actual HP change)");
    }

    #[test]
    fn display_damage_practice_returns_dealt_for_both_sides() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Enemy in practice (is_friendly=false, is_sortie=false) — still returns raw
        let mut e_input = sample_ship(&codex, dd_mst, 50);
        e_input.ship.api_nowhp = 50;
        let enemy_practice = BattleRuntimeShip::new(e_input, false, false);
        assert_eq!(
            display_damage(&enemy_practice, 200, 50),
            200,
            "practice enemy should still show raw (overkill display)"
        );

        // Friendly in practice (is_friendly=true, is_sortie=false)
        let mut f_input = sample_ship(&codex, dd_mst, 50);
        f_input.ship.api_nowhp = 100;
        let friendly_practice = BattleRuntimeShip::new(f_input, true, false);
        assert_eq!(
            display_damage(&friendly_practice, 100, 80),
            80,
            "practice friendly should show dealt"
        );
    }

    // ── Flagship escort shield (旗艦援護 / かばう) ──────────────────────

    /// Build a defending runtime ship with explicit current/max HP.
    fn rt_ship(codex: &Codex, mst_id: i64, nowhp: i64, maxhp: i64) -> BattleRuntimeShip {
        let mut input = sample_ship(codex, mst_id, 50);
        input.ship.api_maxhp = maxhp;
        input.ship.api_nowhp = nowhp;
        BattleRuntimeShip::new(input, false, true)
    }

    #[test]
    fn escort_shield_rate_table_matches_formations() {
        assert_eq!(escort_shield_rate(1), Some(45)); // 単縦陣
        assert_eq!(escort_shield_rate(2), Some(60)); // 複縦陣
        assert_eq!(escort_shield_rate(4), Some(60)); // 梯形陣
        assert_eq!(escort_shield_rate(5), Some(60)); // 単横陣
        assert_eq!(escort_shield_rate(3), Some(75)); // 輪形陣
        assert_eq!(escort_shield_rate(6), Some(75)); // 警戒陣
        // Combined-fleet (11-14) and unknown formations are out of scope.
        assert_eq!(escort_shield_rate(0), None);
        assert_eq!(escort_shield_rate(11), None);
        assert_eq!(escort_shield_rate(14), None);
    }

    /// Covers AE2. Interception fires at the formation rate (単縦 45%, 輪形 75%),
    /// deterministically under a fixed seed.
    #[test]
    fn flagship_shield_fires_at_formation_rate() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        // Flagship (index 0) + one healthy surface escort.
        let defenders = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 30, 30)];

        let trials = 4000;
        let count = |formation: i64| {
            let mut rng = crate::random::SeededRng::new(7);
            (0..trials)
                .filter(|_| {
                    select_escort_shield(&codex, &mut rng, &defenders, 0, formation).is_some()
                })
                .count()
        };

        let single_line = count(1); // 単縦陣 45%
        let diamond = count(3); // 輪形陣 75%
        assert!((1600..=2000).contains(&single_line), "単縦 ~45%: got {single_line}/{trials}");
        assert!((2800..=3200).contains(&diamond), "輪形 ~75%: got {diamond}/{trials}");
        assert!(diamond > single_line, "higher formation rate must intercept more often");
    }

    /// Covers AE3. No escort in green health → never intercepts, seed-independent
    /// (eligibility fails before any RNG draw).
    #[test]
    fn no_shield_when_escorts_below_green() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        // Escort at 50% HP (小破): 50*4=200 is not > 100*3=300 → ineligible.
        let defenders = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 50, 100)];
        for seed in [1u64, 7, 999] {
            let mut rng = crate::random::SeededRng::new(seed);
            assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 0, 3), None);
        }
    }

    /// Covers AE4. A submarine escort cannot protect a surface flagship.
    #[test]
    fn no_shield_on_type_mismatch() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let defenders = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, ss, 30, 30)];
        for seed in [1u64, 7, 999] {
            let mut rng = crate::random::SeededRng::new(seed);
            assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 0, 3), None);
        }
    }

    /// Green health is strictly above 75% max HP: 75% is 小破 (ineligible), 76% green.
    #[test]
    fn green_health_boundary_is_strictly_above_75_percent() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Exactly 75%: 75*4 = 300 == 100*3 → not strictly greater → ineligible.
        let at_75 = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 75, 100)];
        let mut rng = crate::random::SeededRng::new(7);
        assert_eq!(select_escort_shield(&codex, &mut rng, &at_75, 0, 3), None);

        // 76%: 76*4 = 304 > 300 → eligible → fires on some rolls.
        let at_76 = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 76, 100)];
        let mut rng = crate::random::SeededRng::new(7);
        let fired = (0..200)
            .filter(|_| select_escort_shield(&codex, &mut rng, &at_76, 0, 3).is_some())
            .count();
        assert!(fired > 0, "a 76% escort must be eligible and intercept on some rolls");
    }

    /// Non-flagship targets never intercept and must not consume RNG (KTD-5).
    #[test]
    fn non_flagship_target_consumes_no_rng() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let defenders = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 30, 30)];

        let mut rng = crate::random::SeededRng::new(5);
        assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 1, 1), None);
        // No draw consumed: the next roll matches a fresh rng's first roll.
        let next = rng.roll_range(0, 1_000_000);
        let fresh = crate::random::SeededRng::new(5).roll_range(0, 1_000_000);
        assert_eq!(next, fresh, "non-flagship target must not draw RNG");
    }

    /// R5: a submarine escort protects a submarine flagship.
    #[test]
    fn submarine_flagship_protected_by_submarine_escort() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let defenders = vec![rt_ship(&codex, ss, 30, 30), rt_ship(&codex, ss, 30, 30)];
        let mut rng = crate::random::SeededRng::new(7);
        let fired = (0..200)
            .filter(|_| select_escort_shield(&codex, &mut rng, &defenders, 0, 3).is_some())
            .count();
        assert!(fired > 0, "a submarine escort must protect a submarine flagship");
    }

    /// R4: with more than one eligible escort, the interceptor is chosen
    /// uniformly at random via `choose_index` — every eligible index must be
    /// reachable, and the flagship (index 0) is never selected.
    #[test]
    fn shield_picks_among_multiple_eligible_escorts() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        // Flagship + two healthy surface escorts: indices 1 and 2 both eligible.
        let defenders = vec![
            rt_ship(&codex, dd, 30, 30),
            rt_ship(&codex, dd, 30, 30),
            rt_ship(&codex, dd, 30, 30),
        ];
        let mut rng = crate::random::SeededRng::new(7);
        let mut seen = [false; 3];
        for _ in 0..4000 {
            if let Some(idx) = select_escort_shield(&codex, &mut rng, &defenders, 0, 3) {
                seen[idx] = true;
            }
        }
        assert!(seen[1], "escort index 1 must be selectable");
        assert!(seen[2], "escort index 2 must be selectable");
        assert!(!seen[0], "the flagship (index 0) must never be the interceptor");
    }

    // ── Closing torpedo eligibility (plan 002 U2: base-torpedo stat is the gate,
    //    no ship-type whitelist) ───────────────────────────────────────────────

    fn closing_torpedo_ship(
        codex: &Codex,
        ty: KcShipType,
        raisou: i64,
        nowhp: i64,
        maxhp: i64,
    ) -> BattleRuntimeShip {
        let mst = first_ship_mst_by_type(codex, ty);
        let mut input = sample_ship(codex, mst, 80);
        input.ship.api_raisou[0] = raisou;
        input.ship.api_nowhp = nowhp;
        input.ship.api_maxhp = maxhp;
        BattleRuntimeShip::new(input, true, true)
    }

    /// U1 / 5.9: a carrier with no attack planes cannot shell in day battle;
    /// loading a bomber makes it eligible. (Complements the fighter-only
    /// integration test in shelling.rs, which keeps planes loaded but non-attacking.)
    #[test]
    fn day_carrier_without_attack_planes_cannot_shell() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cv_mst = first_ship_mst_by_type(&codex, KcShipType::CV);
        let bomber_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);

        // No planes loaded → excluded.
        let mut empty_input = sample_ship(&codex, cv_mst, 99);
        empty_input.slot_items = vec![];
        empty_input.ship.api_onslot = [0; 5];
        let empty = BattleRuntimeShip::from(empty_input);
        assert!(!can_shell_day_ship(&codex, &empty), "CV with no attack planes cannot shell");

        // A loaded bomber → eligible.
        let mut armed_input = sample_ship(&codex, cv_mst, 99);
        armed_input.slot_items = vec![slotitem_with_mst_id(bomber_id)];
        armed_input.ship.api_onslot = [12, 0, 0, 0, 0];
        let armed = BattleRuntimeShip::from(armed_input);
        assert!(can_shell_day_ship(&codex, &armed), "CV with a loaded bomber can shell");
    }

    /// R3 / U2: a battleship — excluded by the old ship-type whitelist — with a
    /// base torpedo stat now participates in closing torpedo.
    #[test]
    fn closing_torpedo_includes_battleship_with_base_torpedo() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb = closing_torpedo_ship(&codex, KcShipType::BB, 50, 90, 90);
        assert!(
            can_closing_torpedo_ship(&codex, &bb),
            "a BB with base torpedo > 0 must fire closing torpedo (no type whitelist)"
        );
    }
}
