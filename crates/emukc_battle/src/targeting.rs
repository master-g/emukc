//! Target selection, eligibility checks, and display helpers for battle phases.

use emukc_model::{
    codex::{Codex, gear_bonus::GearBonusGear},
    kc2::{
        KcApiSlotItem, KcShipType, KcSlotItemType3,
        start2::{ApiMstShip, ApiMstSlotitem},
    },
};

use crate::combined::CombinedFleetRole;
use crate::random::BattleRng;
use crate::simulation::always_oasw::{ABYSSAL_ALWAYS_OASW_IDS, PLAYER_ALWAYS_OASW_IDS};
use crate::simulation::installation_types::{
    INSTALLATION_BOMBERS, INSTALLATION_TYPES, LAST_SOURCE_ABYSSAL_ID,
};
use crate::types::{AttackCapability, BattlePhase, BattleRuntimeShip, TargetClass};

// ---------------------------------------------------------------------------
// Constants — type-category lists used by display / eligibility helpers
// ---------------------------------------------------------------------------

/// Equipment a day surface attack names in `api_si_list`.
///
/// Guns only, on purpose. Whatever lands in `api_si_list[0]` is loaded by
/// `CutinAttack` as a `btxt_flat` name plate whenever the client resolves the
/// attack to `PhaseAttackNormal`, and aircraft have no name plate upstream --
/// probed 2026-09-22: of the whole carrier air group only the night-capable
/// machines (F6F-3N, Swordfish, TBM-3D, 試製 夜間瑞雲 …) return 200, and those
/// are reached through the night carrier cut-in, not through this list. The
/// archived `102 -> btxt_flat` incident is exactly a 水上爆撃機 arriving here.
///
/// Nothing is lost by leaving them out: a carrier resolves to
/// `PhaseAttackKansaiki`, which is constructed with `showTelop = false` and
/// never reads `api_si_list` at all.
const DAY_SURFACE_DISPLAY_TYPES: &[KcSlotItemType3] = &[
    KcSlotItemType3::SmallCaliberMainGun,
    KcSlotItemType3::MediumCaliberMainGun,
    KcSlotItemType3::LargeCaliberMainGun,
    KcSlotItemType3::SecondaryGun,
    KcSlotItemType3::LargeCaliberMainGun2,
    KcSlotItemType3::SecondaryGun2,
];

const MAIN_GUN_TYPES: &[KcSlotItemType3] = &[
    KcSlotItemType3::SmallCaliberMainGun,
    KcSlotItemType3::MediumCaliberMainGun,
    KcSlotItemType3::LargeCaliberMainGun,
    KcSlotItemType3::LargeCaliberMainGun2,
];

const SECONDARY_GUN_TYPES: &[KcSlotItemType3] =
    &[KcSlotItemType3::SecondaryGun, KcSlotItemType3::SecondaryGun2];

const TORPEDO_TYPES: &[KcSlotItemType3] =
    &[KcSlotItemType3::Torpedo, KcSlotItemType3::SubmarineTorpedo];

const RADAR_DISPLAY_TYPES: &[KcSlotItemType3] =
    &[KcSlotItemType3::SmallRadar, KcSlotItemType3::LargeRadar, KcSlotItemType3::LargeRadar2];

const PT_TARGET_NAME_MARKERS: &[&str] = &["PT小鬼群", "Schnellboot小鬼群"];
const INSTALLATION_TARGET_NAME_MARKERS: &[&str] =
    &["砲台", "飛行場", "港湾", "離島", "集積地", "泊地", "要塞", "トーチカ"];

// ---------------------------------------------------------------------------
// Core targeting — random target selection
// ---------------------------------------------------------------------------

/// Draw a target from `candidates` as `choiceWProtect` does (`kcsim.js:2295`): one draw,
/// up to `rerolls` more while the ship drawn has no searchlight, one more when she stands in
/// the front half of a fleet in 警戒陣, then かばう
/// ([`select_escort_shield`]). Returns the ship hit and whether the client shows かばう: an
/// enemy escort-deck ship covering the main deck's flagship takes the hit unshown
/// (`DIDPROTECT = false`). `include_escort` lets the escort deck protect, as the source's
/// caller says.
pub(crate) fn choice_with_protect(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defenders: &[BattleRuntimeShip],
    candidates: &[usize],
    rerolls: u8,
    include_escort: bool,
    formation_id: i64,
) -> Option<(usize, bool)> {
    let pick = candidates[rng.choose_index(candidates.len())?];
    let pick = reroll_toward_searchlight(codex, rng, defenders, candidates, pick, rerolls);
    // A ship in the front half of a fleet in 警戒陣 is drawn again, once.
    let pick = if defenders[pick].in_vanguard_front() {
        candidates[rng.roll_range(0, candidates.len() as i64) as usize]
    } else {
        pick
    };
    Some(match select_escort_shield(codex, rng, defenders, pick, include_escort, formation_id) {
        Some(escort) => {
            let ship = &defenders[escort];
            (escort, ship.is_friendly || !ship.is_escort_deck())
        }
        None => (pick, false),
    })
}

/// How many times the fleet a ship is in draws a night target again for her searchlight:
/// 1 for a searchlight, 2 for a large one (`hasSearchlight`).
// ponytail: the source keeps whichever of the two comes last in her slots; this takes the
// larger.
pub(crate) fn searchlight_rerolls(codex: &Codex, ship: &BattleRuntimeShip) -> u8 {
    if has_slotitem_type(codex, ship, KcSlotItemType3::LargeSearchlight) {
        2
    } else if has_slotitem_type(codex, ship, KcSlotItemType3::Searchlight) {
        1
    } else {
        0
    }
}

fn reroll_toward_searchlight(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defenders: &[BattleRuntimeShip],
    candidates: &[usize],
    mut pick: usize,
    rerolls: u8,
) -> usize {
    for _ in 0..rerolls {
        if searchlight_rerolls(codex, &defenders[pick]) > 0 {
            break;
        }
        pick = candidates[rng.roll_range(0, candidates.len() as i64) as usize];
    }
    pick
}

/// The ships an attacker may fire at in `phase`: the surface ships or the submarines,
/// whichever she prefers and can reach.
pub(crate) fn target_candidates(
    codex: &Codex,
    attacker: &BattleRuntimeShip,
    defenders: &[BattleRuntimeShip],
    phase: BattlePhase,
) -> Vec<usize> {
    let alive_targets = defenders
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.is_alive())
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    if alive_targets.is_empty() {
        return Vec::new();
    }

    let shuns_land = match phase {
        BattlePhase::DayShelling => shuns_installations(codex, attacker),
        // Neither torpedo is aimed at land (`torpedoPhase`: `if (!alive[i].isInstall)`).
        BattlePhase::OpeningTorpedo | BattlePhase::ClosingTorpedo => true,
        BattlePhase::NightShelling => false,
    };
    // A submarine shells nothing but land (`shellPhaseTarget`).
    // ponytail: the source also leaves her out of the order when the other side has no land
    // target (`orderByRange`'s `includeSubs`); with nothing here to fire at she fires nothing
    // either, and only the random draws differ (the order's shuffle, and her deck roll against
    // an enemy combined fleet).
    let land_only =
        phase == BattlePhase::DayShelling && target_class(codex, attacker).is_submarine();
    let surface_like_targets = alive_targets
        .iter()
        .copied()
        .filter(|idx| {
            let class = target_class(codex, &defenders[*idx]);
            let land = class == TargetClass::Installation;
            class.is_surface_like() && !(shuns_land && land) && (land || !land_only)
        })
        .collect::<Vec<_>>();
    let submarine_targets = alive_targets
        .iter()
        .copied()
        .filter(|idx| target_class(codex, &defenders[*idx]).is_submarine())
        .collect::<Vec<_>>();

    let candidates = match attack_capability_for_phase(codex, attacker, phase) {
        AttackCapability::CannotAttack => Vec::new(),
        AttackCapability::SurfaceOnly => surface_like_targets,
        AttackCapability::BothPreferSubmarine => {
            if submarine_targets.is_empty() {
                surface_like_targets
            } else {
                submarine_targets
            }
        }
        AttackCapability::BothPreferSurface => {
            if surface_like_targets.is_empty() {
                submarine_targets
            } else {
                surface_like_targets
            }
        }
    };
    // An anti-PT ship's guns go for a PT imp first (`shellPhaseTarget`, `nightPhaseTarget`).
    // ponytail: by day the source still lets any ship cover the flagship, as here; at night it
    // draws the cover from the PT imps alone, which differs only when the flagship is one.
    match phase {
        BattlePhase::DayShelling | BattlePhase::NightShelling => {
            crate::pt_imp::prefer_pt(codex, attacker, defenders, candidates)
        }
        BattlePhase::OpeningTorpedo | BattlePhase::ClosingTorpedo => candidates,
    }
}

/// [`target_candidates`], but against both decks of an enemy combined fleet an attack first
/// picks a deck and draws from that deck alone (`kancolle-replay` `69097abc`):
///
/// - a shell (`doShellC`) goes for the escort deck first on a roll under 0.39 and the main
///   deck first otherwise, an anti-PT ship for the deck that alone has PT imps, and for the
///   other deck when the first has nothing to fire at;
/// - a closing torpedo (`torpedoPhase` with `combinedAll`) goes for the main deck on a roll
///   under 0.35 and the escort deck otherwise, with no roll when one deck has nothing.
///
/// Anything else, or a single fleet, draws from them all.
pub(crate) fn deck_candidates(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attacker: &BattleRuntimeShip,
    defenders: &[BattleRuntimeShip],
    phase: BattlePhase,
) -> Vec<usize> {
    let escort_start =
        defenders.iter().position(|ship| ship.enemy_deck == Some(CombinedFleetRole::Escort));
    let Some(escort_start) = escort_start.filter(|&at| at > 0) else {
        return target_candidates(codex, attacker, defenders, phase);
    };
    let main = || target_candidates(codex, attacker, &defenders[..escort_start], phase);
    let escort = || {
        let candidates = target_candidates(codex, attacker, &defenders[escort_start..], phase);
        candidates.into_iter().map(|at| at + escort_start).collect::<Vec<_>>()
    };
    match phase {
        BattlePhase::DayShelling => {
            let mut escort_first = rng.random_f64_range(0.0, 1.0) < 0.39;
            // An anti-PT ship goes for the deck that alone has PT imps, whatever the roll.
            if attacker.pt_accuracy_flat > 0 {
                let has_pt = |deck: &[BattleRuntimeShip]| {
                    deck.iter().any(|ship| ship.is_alive() && crate::pt_imp::is_pt(codex, ship))
                };
                match (has_pt(&defenders[..escort_start]), has_pt(&defenders[escort_start..])) {
                    (true, false) => escort_first = false,
                    (false, true) => escort_first = true,
                    _ => {}
                }
            }
            let (first, fallback) = if escort_first {
                (escort(), main())
            } else {
                (main(), escort())
            };
            if first.is_empty() {
                fallback
            } else {
                first
            }
        }
        BattlePhase::ClosingTorpedo => {
            let (main, escort) = (main(), escort());
            if main.is_empty() || escort.is_empty() {
                [main, escort].concat()
            } else if rng.random_f64_range(0.0, 1.0) < 0.35 {
                main
            } else {
                escort
            }
        }
        _ => target_candidates(codex, attacker, defenders, phase),
    }
}

/// A carrier whose dive bombers may not attack a land target leaves one out of her day
/// shelling (`shellPhaseTarget`): she carries a dive or jet bomber, flown or not
/// (`hasDivebomber`), and no bomber the source lets attack land (`canShellInstall`) has planes
/// left while some dive or jet bomber does (`CV.canShellInstallation`).
// ponytail: carrier-style shelling is `is_cv_type`, as the shelling formula has it; the source
// also counts an oiler flying bombers (速吸改).
fn shuns_installations(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if !crate::damage::is_cv_type(codex, ship) {
        return false;
    }
    let is_bomber = |item: &KcApiSlotItem| {
        slotitem_mst(codex, item).is_some_and(|mst| {
            matches!(
                KcSlotItemType3::n(mst.api_type[2]),
                Some(KcSlotItemType3::CarrierBasedDiveBomber | KcSlotItemType3::JetFighterBomber)
            )
        })
    };
    let flown = || ship.slot_items.iter().zip(ship.ship.api_onslot).filter(|(_, count)| *count > 0);
    ship.slot_items.iter().any(is_bomber)
        && !flown().any(|(item, _)| INSTALLATION_BOMBERS.contains(&item.api_slotitem_id))
        && flown().any(|(item, _)| is_bomber(item))
}

/// The ships afloat that are not submarines, whatever the attacker would rather fire at:
/// the ships of a special attack take no other (`shellPhaseTarget` with no submarines to
/// choose from).
pub(crate) fn surface_candidates(codex: &Codex, defenders: &[BattleRuntimeShip]) -> Vec<usize> {
    afloat_where(defenders, |ship| !target_class(codex, ship).is_submarine())
}

/// The submarines afloat.
pub(crate) fn submarine_candidates(codex: &Codex, defenders: &[BattleRuntimeShip]) -> Vec<usize> {
    afloat_where(defenders, |ship| target_class(codex, ship).is_submarine())
}

fn afloat_where(
    defenders: &[BattleRuntimeShip],
    keep: impl Fn(&BattleRuntimeShip) -> bool,
) -> Vec<usize> {
    defenders
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.is_alive() && keep(ship))
        .map(|(idx, _)| idx)
        .collect()
}

// ---------------------------------------------------------------------------
// Flagship escort shield (旗艦援護 / かばう)
// ---------------------------------------------------------------------------

/// Interception probability for a defending fleet's formation, as a percentage.
///
/// Rates from the official wiki (`攻撃対象の選択`): 単縦陣 45%, 複縦/梯形/単横 60%,
/// 輪形/警戒 75%. An unknown formation never intercepts.
///
/// The four 警戒航行序列 (11–14) take [`COMBINED_SHIELD_RATE`], which is a fitted
/// default rather than a wiki figure — see that constant.
fn escort_shield_rate(formation_id: i64) -> Option<i64> {
    match formation_id {
        1 => Some(45),                         // 単縦陣
        2 | 4 | 5 => Some(60),                 // 複縦陣 / 梯形陣 / 単横陣
        3 | 6 => Some(75),                     // 輪形陣 / 警戒陣
        11..=14 => Some(COMBINED_SHIELD_RATE), // 警戒航行序列
        _ => None,
    }
}

/// Interception rate for the four 警戒航行序列, as a percentage.
///
/// Unlike every other rate in [`escort_shield_rate`], this one is **not
/// verified**: every かばう cell in the wikiwiki combined-fleet tables is `?`.
/// It is the fallback `KC3Kai`'s simulator applies — `kcsim.js:2315` indexes a
/// rate table by formation id, ids 11–14 fall off its end, and the `if (!rate)`
/// branch substitutes `.6`. Treat it as a fitted default, not observed game
/// data. It is still closer to the game than never intercepting at all, which
/// is what this function did for these formations before combined sortie
/// existed.
const COMBINED_SHIELD_RATE: i64 = 60;

/// Resolve 旗艦援護 (かばう): when an attack targets the defending fleet's
/// flagship (`target_idx == 0`), a healthy escort may intercept it and take the
/// damage instead. Returns the interceptor's index, or `None` when no
/// interception occurs.
///
/// In the order `choiceWProtect` takes: the flagship must be the target and not
/// a land target, and the formation must have a defined rate, before any draw.
/// Then one `roll_range` for the rate, cast whether or not anyone could protect.
/// Only on success are the protectors gathered: non-flagship, alive, in green
/// health (小破未満, HP > 75%), not a land target, of the flagship's category
/// (ships at sea protect ships at sea, submarines protect submarine flagships),
/// and in the escort deck only when `include_escort`. None → no interception and
/// no further draw; otherwise one `choose_index` picks the interceptor.
/// The flagship's own HP does **not** gate — interception fires even when the
/// flagship is 大破.
pub(crate) fn select_escort_shield(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defenders: &[BattleRuntimeShip],
    target_idx: usize,
    include_escort: bool,
    defender_formation_id: i64,
) -> Option<usize> {
    // Only a fleet flagship is shielded, and never deck 2's (`kcsim.js:2309`
    // returns early on `target.isescort`). By day the two decks share one vector
    // with deck 1 first, so deck 2's flagship sits at `escort_start`; at night
    // deck 2 fights alone and its flagship is at 0, which the deck tag catches.
    if target_idx != 0 || defenders[0].is_escort_deck() {
        return None;
    }
    // A land target is neither shielded nor a shield (`kcsim.js:2309`, `:2317`).
    let flagship = target_class(codex, &defenders[0]);
    if flagship == TargetClass::Installation {
        return None;
    }
    let rate = escort_shield_rate(defender_formation_id)?;
    if rng.roll_range(0, 100) >= rate {
        return None;
    }

    // A ship at sea (incl. PT) protects one at sea; submarine protects
    // submarine.
    let flagship_is_submarine = flagship.is_submarine();

    let eligible: Vec<usize> = defenders
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, ship)| {
            // Deck 2 shields deck 1's flagship only when the caller opts in
            // (`includeEscort`: the opening ASW and night surface draws).
            // `is_escort_deck` is false for every ship in a single-fleet battle,
            // so this filter is a no-op there.
            let class = target_class(codex, ship);
            (include_escort || !ship.is_escort_deck())
                && ship.is_alive()
                && ship.hp() * 4 > ship.ship.api_maxhp * 3
                && class != TargetClass::Installation
                && class.is_submarine() == flagship_is_submarine
        })
        .map(|(idx, _)| idx)
        .collect();
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

    let id = ship.ship.api_ship_id;
    if installation_type(id).is_some() {
        return TargetClass::Installation;
    }
    if let Some(name) = ship_mst(codex, ship).map(|mst| mst.api_name.as_str()) {
        if is_pt_target_name(name) {
            return TargetClass::PtBoat;
        }
        // A ship newer than the source's table is still judged by her name.
        if id > LAST_SOURCE_ABYSSAL_ID && is_installation_target_name(name) {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                tracing::warn!(
                    "abyssal ship {id} ({name}) is newer than the installation table; \
                     judging her by name. Regenerate it with scripts/gen_installation_types.py."
                );
            });
            return TargetClass::Installation;
        }
    }

    TargetClass::SurfaceShip
}

/// The source's `installtype` of a land target (`kcSHIPDATA.js`): 1 soft-skinned, 2 pillbox,
/// 3 supply depot, 4 isolated island, 5 northern, 6 harbour summer princess.
pub(crate) fn installation_type(mst_id: i64) -> Option<u8> {
    INSTALLATION_TYPES
        .binary_search_by_key(&mst_id, |row| row.0)
        .ok()
        .map(|at| INSTALLATION_TYPES[at].1)
}

/// A fleet of PT boats and nothing else, sunk or not: no fleet special attack is made
/// against one (wikiwiki: 「敵がPT小鬼群のみのマス…では発動しない」; the source's
/// `isNoSpecialNode`).
pub fn is_pt_only(codex: &Codex, fleet: &[BattleRuntimeShip]) -> bool {
    !fleet.is_empty() && fleet.iter().all(|ship| target_class(codex, ship) == TargetClass::PtBoat)
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

/// The ship's equipment as the 装備ボーナス table reads it, in slot order.
pub(crate) fn bonus_gears(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<GearBonusGear> {
    ship.slot_items
        .iter()
        .filter_map(|item| {
            let mst = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok()?;
            Some(GearBonusGear::new(mst, item.api_level))
        })
        .collect()
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

/// How many of the ship's equipment pieces have a type `keep` accepts.
pub(crate) fn count_slotitems(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    keep: impl Fn(KcSlotItemType3) -> bool,
) -> usize {
    ship.slot_items
        .iter()
        .filter_map(|slot_item| slotitem_mst(codex, slot_item))
        .filter_map(|mst| KcSlotItemType3::n(mst.api_type[2]))
        .filter(|&slot_type| keep(slot_type))
        .count()
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

/// Whether a single ship may fire in day shelling. A submarine may with WG42 or a 特型内火艇
/// (`SS.canShell`: `numWG || hasDH3`, which is the source's `B_LC3`, the 特型内火艇 type, and
/// 525/526, of that type too), and then only at a land target.
pub(crate) fn can_shell_day_ship(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() {
        return false;
    }

    match ship_type(codex, ship) {
        Some(KcShipType::SS | KcShipType::SSV) => {
            has_slotitem_id(ship, crate::installation::WG_ID)
                || has_slotitem_type(codex, ship, KcSlotItemType3::SpecialTypeAmphibiousTank)
        }
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
            // A night air attack, or one of the carriers the source marks `nightattack`
            // (the few of ours that shell at night, and the abyssal ones), which attack until
            // heavily damaged (`CV.canNB`).
            crate::simulation::night::can_night_air_attack(codex, ship)
                || ((crate::simulation::night::NIGHT_SHELLING_CV_IDS
                    .contains(&ship.ship.api_ship_id)
                    || crate::simulation::night::abyssal_night_attack(ship) != 0)
                    && ship.hp() * 4 > ship.ship.api_maxhp)
        }
        // `Ship.canNB`; `SS` does not override it.
        _ => ship.hp() * 4 > ship.ship.api_maxhp,
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
            } else if !can_attack_submarine_night_shelling(codex, ship) {
                AttackCapability::SurfaceOnly
            } else if crate::simulation::night::can_night_air_attack(codex, ship) {
                // `nightPhaseTarget`: her aircraft go for the surface ships first.
                AttackCapability::BothPreferSurface
            } else {
                AttackCapability::BothPreferSubmarine
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
    if ship.is_sunk() {
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
        ) => ship.ship.api_taisen[0] > 0,
        // `canASWNight`: a light carrier needs anti-submarine of her own, with or without
        // aircraft for it; a fleet or armoured carrier does not attack a submarine at night.
        Some(KcShipType::CVL) => can_attack_night_ship(codex, ship) && bare_asw(codex, ship) > 0,
        _ => false,
    }
}

/// A ship's anti-submarine without her equipment or what it grants: grown with her level, plus
/// 近代化改修, as `Codex::cal_ship_status` builds it. An abyssal ship's is the enemy data's.
fn bare_asw(codex: &Codex, ship: &BattleRuntimeShip) -> i64 {
    const ASW_MODERNIZATION: usize = 6;
    if let Ok(enemy) = codex.find_enemy_ship_extra(ship.ship.api_ship_id) {
        return enemy.asw;
    }
    codex.find_ship_extra(ship.ship.api_ship_id).map_or(0, |basic| {
        Codex::stat_at_level(ship.ship.api_lv, basic.tais) + ship.ship.api_kyouka[ASW_MODERNIZATION]
    })
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

/// The source's player ships (`isPlayable`, `shared.js:220`) have ids below this.
const PLAYABLE_ID_END: i64 = 1500;
/// 日向改二, who opens with ASW by her own rule (`BBV.canOASW`, `kcships.js:1936-1943`).
const HYUUGA_KAI_NI: i64 = 554;
/// 加賀改二護, a fleet carrier the source rules as a light carrier (`kcSHIPDATA.js` 646).
const KAGA_KAI_NI_GO: i64 = 646;
/// 鈴谷航改二 and 熊野航改二, whose sonar does not lower a light carrier's threshold
/// (`isASWlast`, `kcSHIPDATA.js` 508, 509).
const ASW_LAST_IDS: [i64; 2] = [508, 509];

/// Whether a ship opens with ASW (`canOASW`, `kcships.js`): 日向改二, the light carriers and
/// the escorts have rules of their own; any other ship that may attack a submarine opens with
/// it when the source always lets her, or, being the player's, with 100 anti-submarine and a
/// sonar.
pub(crate) fn can_opening_asw(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if ship.is_sunk() {
        return false;
    }
    let id = ship.ship.api_ship_id;
    let asw = ship.ship.api_taisen[0];
    let sonar = has_slotitem_type(codex, ship, KcSlotItemType3::Sonar)
        || has_slotitem_type(codex, ship, KcSlotItemType3::LargeSonar);
    if id == HYUUGA_KAI_NI {
        // S-51J or S-51J改, or two of カ号観測機, オ号観測機改 and オ号観測機改二.
        let observers =
            ship.slot_items.iter().filter(|item| matches!(item.api_slotitem_id, 69 | 324 | 325));
        return has_slotitem_id(ship, 326) || has_slotitem_id(ship, 327) || observers.count() >= 2;
    }
    if id == KAGA_KAI_NI_GO || ship_type(codex, ship) == Some(KcShipType::CVL) {
        return light_carrier_opening_asw(codex, ship, asw, sonar);
    }
    if ship_type(codex, ship) == Some(KcShipType::DE) {
        // `DE.canOASW`, which does not ask `canASW`.
        let equipment_asw: i64 = ship
            .slot_items
            .iter()
            .filter_map(|item| slotitem_mst(codex, item))
            .map(|mst| mst.api_tais)
            .sum();
        return sonar && asw >= 60 || asw >= 75 && equipment_asw >= 4;
    }
    can_asw(codex, ship)
        && (always_opening_asw(codex, ship) || asw >= 100 && sonar && id < PLAYABLE_ID_END)
}

/// Whether a ship picked to open with ASW still has her shot when her turn comes
/// (`shellPhaseTarget`, `kcsim.js:1086`): `canASW` again, so a light carrier needs a plane left
/// after the air battle, and 鈴谷航改二 / 熊野航改二 hold while a surface ship is afloat.
pub(crate) fn has_opening_asw_shot(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    defenders: &[BattleRuntimeShip],
) -> bool {
    can_asw(codex, ship)
        && (!ASW_LAST_IDS.contains(&ship.ship.api_ship_id)
            || surface_candidates(codex, defenders).is_empty())
}

/// `CVL.canOASW` (`kcships.js:2100-2116`): she must carry a dive or torpedo bomber, autogyro or
/// patrol plane with anti-submarine, whether or not it has aircraft left (her turn asks that,
/// `has_opening_asw_shot`); then the source's
/// always, or a torpedo bomber, autogyro or patrol plane at 7 or more with 50 anti-submarine and
/// a sonar (65 without, or for `ASW_LAST_IDS`), or a sonar and 100.
fn light_carrier_opening_asw(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    asw: i64,
    sonar: bool,
) -> bool {
    let hunters: Vec<(KcSlotItemType3, i64)> = ship
        .slot_items
        .iter()
        .filter_map(|item| slotitem_mst(codex, item))
        .filter(|mst| mst.api_tais > 0)
        .filter_map(|mst| KcSlotItemType3::n(mst.api_type[2]).map(|kind| (kind, mst.api_tais)))
        .filter(|(kind, _)| {
            matches!(
                kind,
                KcSlotItemType3::CarrierBasedDiveBomber
                    | KcSlotItemType3::CarrierBasedTorpedoBomber
                    | KcSlotItemType3::AutoGyro
                    | KcSlotItemType3::AntiSubmarinePatrol
            )
        })
        .collect();
    if hunters.is_empty() {
        return false;
    }
    if always_opening_asw(codex, ship) {
        return true;
    }
    let threshold = if sonar && !ASW_LAST_IDS.contains(&ship.ship.api_ship_id) {
        50
    } else {
        65
    };
    let strong = hunters
        .iter()
        .any(|&(kind, tais)| tais >= 7 && kind != KcSlotItemType3::CarrierBasedDiveBomber);
    strong && asw >= threshold || sonar && asw >= 100
}

/// `alwaysOASW`: a ship the source marks, or a player's ship with 50 anti-submarine or more at
/// level 1, 日向改二 aside (`kcships.js:425`, read from the codex's level-1 figures).
fn always_opening_asw(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    let id = ship.ship.api_ship_id;
    if id < PLAYABLE_ID_END {
        PLAYER_ALWAYS_OASW_IDS.contains(&id)
            || id != HYUUGA_KAI_NI
                && codex.find_ship_extra(id).is_ok_and(|basic| basic.tais[0] >= 50)
    } else {
        ABYSSAL_ALWAYS_OASW_IDS.contains(&id)
    }
}

/// Whether a ship may attack a submarine at all (her class's `canASW`, `kcships.js`), for
/// opening ASW: her own anti-submarine for the escorts, a depth charge for the tenders and
/// 扶桑改二 / 山城改二, aircraft for the aviation ships and light carriers, or for an abyssal one
/// any anti-submarine of her own (an abyssal light carrier always). Day shelling still asks
/// `can_attack_submarine_day_shelling` (#285).
fn can_asw(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    let id = ship.ship.api_ship_id;
    // `CVL.canASW` with `isOASW`, which 加賀改二護 takes too.
    if id == KAGA_KAI_NI_GO || ship_type(codex, ship) == Some(KcShipType::CVL) {
        return id >= PLAYABLE_ID_END || has_asw_plane_aloft(codex, ship);
    }
    let own_asw = || crate::damage::base_asw(codex, ship) > 0.0;
    let depth_charge = || has_slotitem_type(codex, ship, KcSlotItemType3::DepthCharge);
    // `CAV.canASW`.
    let aviation = || {
        if id < PLAYABLE_ID_END {
            has_asw_plane_aloft(codex, ship)
        } else {
            own_asw()
        }
    };
    match ship_type(codex, ship) {
        Some(
            KcShipType::DD
            | KcShipType::CL
            | KcShipType::CLT
            | KcShipType::CT
            | KcShipType::AO
            | KcShipType::DE,
        ) => own_asw(),
        Some(KcShipType::BBV) => matches!(id, 411 | 412) && depth_charge() || aviation(),
        Some(KcShipType::AS | KcShipType::AR) => depth_charge(),
        // 第百一号輸送艦 and her 改.
        Some(KcShipType::LHA) if matches!(id, 727 | 945) => depth_charge(),
        Some(KcShipType::CAV | KcShipType::AV | KcShipType::LHA) => aviation(),
        _ => false,
    }
}

/// Whether a ship has aircraft left in a slot of a kind that attacks submarines (`isASWPlane`)
/// with anti-submarine of its own.
fn has_asw_plane_aloft(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(item, onslot)| {
        onslot > 0
            && slotitem_mst(codex, item).is_some_and(|mst| {
                mst.api_tais > 0
                    && matches!(
                        KcSlotItemType3::n(mst.api_type[2]),
                        Some(
                            KcSlotItemType3::CarrierBasedDiveBomber
                                | KcSlotItemType3::CarrierBasedTorpedoBomber
                                | KcSlotItemType3::SeaBasedBomber
                                | KcSlotItemType3::AutoGyro
                                | KcSlotItemType3::AntiSubmarinePatrol
                                | KcSlotItemType3::LargeFlyingBoat
                        )
                    )
            })
    })
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

/// Whether the slot type is shown in day surface attack displays.
pub(crate) fn is_day_surface_display_type(slot_type: KcSlotItemType3) -> bool {
    DAY_SURFACE_DISPLAY_TYPES.contains(&slot_type)
}

/// Whether the slot type counts as a main gun.
pub(crate) fn is_main_gun_type(slot_type: KcSlotItemType3) -> bool {
    MAIN_GUN_TYPES.contains(&slot_type)
}

/// Whether the slot type counts as a secondary gun.
pub(crate) fn is_secondary_gun_type(slot_type: KcSlotItemType3) -> bool {
    SECONDARY_GUN_TYPES.contains(&slot_type)
}

/// Whether the slot type counts as a torpedo.
pub(crate) fn is_torpedo_type(slot_type: KcSlotItemType3) -> bool {
    TORPEDO_TYPES.contains(&slot_type)
}

/// Whether the slot type is an armour piercing shell.
pub(crate) fn is_ap_shell_type(slot_type: KcSlotItemType3) -> bool {
    slot_type == KcSlotItemType3::ArmorPiercingShell
}

/// Whether the slot type is a radar.
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

/// Display slot-item IDs for a gun-formed attack: main guns first, then
/// secondaries, then whatever `extra` the attack was formed with (the armour
/// piercing shell or the radar), up to `limit`.
///
/// Deliberately narrower than [`is_day_surface_display_type`], which also
/// admits 水上爆撃機 and 艦上攻撃機 -- equipment that can take part in a plain
/// attack but can never form a 連撃 or a gunnery cut-in. The list converges on
/// what the ship actually carries; it is never padded out to the attack's
/// nominal slot count.
pub(crate) fn day_gunnery_display_ids(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    limit: usize,
    extra: Option<fn(KcSlotItemType3) -> bool>,
) -> Vec<i64> {
    let main_guns =
        collect_matching_slot_ids(codex, ship, |slot_type, _mst| is_main_gun_type(slot_type));
    let secondary_guns =
        collect_matching_slot_ids(codex, ship, |slot_type, _mst| is_secondary_gun_type(slot_type));

    let mut ids = Vec::new();
    extend_limit(&mut ids, &main_guns, limit);
    extend_limit(&mut ids, &secondary_guns, limit);
    if let Some(matches_extra) = extra {
        let extras =
            collect_matching_slot_ids(codex, ship, |slot_type, _mst| matches_extra(slot_type));
        extend_limit(&mut ids, &extras, limit);
    }
    if ids.is_empty() {
        vec![-1]
    } else {
        ids
    }
}

/// Compute the display slot-item IDs for a day attack.
pub(crate) fn day_attack_display_ids(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    is_submarine_target: bool,
) -> Vec<i64> {
    // An anti-submarine attack names no equipment. The client resolves it to
    // `PhaseAttackBakurai` or `PhaseAttackKansaiki`, both constructed with
    // `showTelop = false`, so `api_si_list` is never read -- and nothing that
    // forms one has a name plate anyway: probed 2026-09-22, sonars (46/47/132),
    // depth charges and seaplanes (25) all 404 while gun ids return 200. The
    // depth-charge / ASW-plane animation comes from the attacker's own slots
    // and the defender being a submarine, never from `api_si_list`.
    if is_submarine_target {
        return vec![-1];
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
    use crate::types::{BattlePhase, BattleRuntimeShip, BattleShipInput, TargetClass};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    /// Hands out the indices it was given, and panics on one more.
    struct Picks(std::collections::VecDeque<i64>);

    impl BattleRng for Picks {
        fn random_f64_range(&mut self, _min: f64, _max: f64) -> f64 {
            panic!("an unexpected roll")
        }

        fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
            self.0.pop_front().expect("an unexpected roll")
        }
    }

    /// Hands out the rolls it was given, and panics on one more or on an index.
    struct Rolls(std::collections::VecDeque<f64>);

    impl BattleRng for Rolls {
        fn random_f64_range(&mut self, _min: f64, _max: f64) -> f64 {
            self.0.pop_front().expect("an unexpected roll")
        }

        fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
            panic!("an unexpected index")
        }
    }

    /// An enemy combined fleet of two destroyers in each deck, and a destroyer to fire at it.
    fn enemy_combined(codex: &Codex) -> (BattleRuntimeShip, Vec<BattleRuntimeShip>) {
        use crate::combined::CombinedFleetRole::{Escort, Main};
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);
        let ship = || BattleRuntimeShip::new(sample_ship(codex, dd, 50), false, true);
        let decks = [Main, Main, Escort, Escort];
        let defenders = decks.into_iter().map(|deck| ship().in_enemy_deck(deck)).collect();
        (BattleRuntimeShip::from(sample_ship(codex, dd, 50)), defenders)
    }

    /// `doShellC`: against both decks of an enemy combined fleet a shell goes for the escort
    /// deck first on a roll under 0.39 and the main deck first otherwise, and for the other
    /// deck when the first has nothing to fire at.
    #[test]
    fn a_shell_at_an_enemy_combined_fleet_goes_for_the_escort_deck_39_times_in_100() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let (attacker, mut defenders) = enemy_combined(&codex);
        let draw = |defenders: &[BattleRuntimeShip], rolls: &[f64]| {
            let mut rng = Rolls(rolls.iter().copied().collect());
            deck_candidates(&codex, &mut rng, &attacker, defenders, BattlePhase::DayShelling)
        };
        assert_eq!(draw(&defenders, &[0.38]), vec![2, 3]);
        assert_eq!(draw(&defenders, &[0.39]), vec![0, 1]);
        // One deck: no roll at all.
        assert_eq!(draw(&defenders[..2], &[]), vec![0, 1]);

        // The escort deck sunk, the escort-first roll still falls back on the main deck.
        let mut sunk = defenders.clone();
        sunk[2].current_hp = 0;
        sunk[3].current_hp = 0;
        assert_eq!(draw(&sunk, &[0.1]), vec![0, 1]);
        defenders[0].current_hp = 0;
        defenders[1].current_hp = 0;
        assert_eq!(draw(&defenders, &[0.9]), vec![2, 3]);
    }

    /// `torpedoPhase` with `combinedAll`: a closing torpedo at an enemy combined fleet goes for
    /// the main deck on a roll under 0.35 and the escort deck otherwise; with one deck left
    /// there is no roll.
    #[test]
    fn a_closing_torpedo_at_an_enemy_combined_fleet_goes_for_the_main_deck_35_times_in_100() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let (attacker, defenders) = enemy_combined(&codex);
        let draw = |defenders: &[BattleRuntimeShip], rolls: &[f64]| {
            let mut rng = Rolls(rolls.iter().copied().collect());
            deck_candidates(&codex, &mut rng, &attacker, defenders, BattlePhase::ClosingTorpedo)
        };
        assert_eq!(draw(&defenders, &[0.34]), vec![0, 1]);
        assert_eq!(draw(&defenders, &[0.35]), vec![2, 3]);

        let mut sunk = defenders.clone();
        sunk[2].current_hp = 0;
        sunk[3].current_hp = 0;
        assert_eq!(draw(&sunk, &[]), vec![0, 1]);
        // The opening torpedo picks from both.
        let mut rng = Rolls([].into_iter().collect());
        let opening =
            deck_candidates(&codex, &mut rng, &attacker, &defenders, BattlePhase::OpeningTorpedo);
        assert_eq!(opening, vec![0, 1, 2, 3]);
    }

    /// A deck of nothing but land targets has nothing for a torpedo, so the closing torpedo
    /// goes for the other deck with no roll.
    #[test]
    fn a_closing_torpedo_passes_over_a_deck_of_land_targets() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let (attacker, defenders) = enemy_combined(&codex);
        let land = ship_mst_id_by_name(&codex, "砲台小鬼");
        let draw = |defenders: &[BattleRuntimeShip]| {
            let mut rng = Rolls([].into_iter().collect());
            deck_candidates(&codex, &mut rng, &attacker, defenders, BattlePhase::ClosingTorpedo)
        };
        let on_land = |decks: &[usize]| {
            let mut fleet = defenders.clone();
            for &at in decks {
                fleet[at].ship.api_ship_id = land;
            }
            fleet
        };
        assert_eq!(draw(&on_land(&[0, 1])), vec![2, 3]);
        assert_eq!(draw(&on_land(&[2, 3])), vec![0, 1]);
    }

    /// One draw from what `attacker` may fire at in `phase`, with no かばう (formation 0).
    fn draw(
        codex: &Codex,
        rng: &mut impl BattleRng,
        attacker: &BattleRuntimeShip,
        defenders: &[BattleRuntimeShip],
        phase: BattlePhase,
    ) -> Option<usize> {
        let candidates = target_candidates(codex, attacker, defenders, phase);
        choice_with_protect(codex, rng, defenders, &candidates, 0, false, 0).map(|(at, _)| at)
    }

    #[test]
    fn a_searchlight_draws_fire_for_as_many_rerolls_as_its_fleet_has() {
        const SEARCHLIGHT: i64 = 74;
        const LARGE_SEARCHLIGHT: i64 = 140;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ship = |gear: &[i64]| {
            let mut ship = sample_ship(&codex, dd, 50);
            ship.slot_items = gear.iter().copied().map(slotitem_with_mst_id).collect();
            BattleRuntimeShip::from(ship)
        };
        assert_eq!(searchlight_rerolls(&codex, &ship(&[])), 0);
        assert_eq!(searchlight_rerolls(&codex, &ship(&[SEARCHLIGHT])), 1);
        assert_eq!(searchlight_rerolls(&codex, &ship(&[LARGE_SEARCHLIGHT])), 2);
        assert_eq!(searchlight_rerolls(&codex, &ship(&[LARGE_SEARCHLIGHT, SEARCHLIGHT])), 2);

        let attacker = ship(&[]);
        let defenders = [ship(&[]), ship(&[SEARCHLIGHT]), ship(&[])];
        let pick = |rerolls, draws: &[i64]| {
            let mut rng = Picks(draws.iter().copied().collect());
            let candidates =
                target_candidates(&codex, &attacker, &defenders, BattlePhase::NightShelling);
            choice_with_protect(&codex, &mut rng, &defenders, &candidates, rerolls, true, 0)
                .map(|(at, _)| at)
        };
        // With no searchlight on the fleet the one draw stands.
        assert_eq!(pick(0, &[0]), Some(0));
        // A ship without one is drawn again, until the lit one comes up...
        assert_eq!(pick(2, &[0, 1]), Some(1));
        // ...or the rerolls run out.
        assert_eq!(pick(2, &[0, 2, 0]), Some(0));
        // The lit ship drawn first is kept.
        assert_eq!(pick(2, &[1]), Some(1));
        // A special attack's ships draw the same way.
        let mut rng = Picks([2, 1].into_iter().collect());
        let surface = surface_candidates(&codex, &defenders);
        let drawn = choice_with_protect(&codex, &mut rng, &defenders, &surface, 1, true, 0);
        assert_eq!(drawn, Some((1, false)));
    }

    #[test]
    fn a_target_in_the_front_half_of_a_vanguard_is_drawn_again_once() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let defenders: Vec<BattleRuntimeShip> = (1..=6)
            .map(|number| {
                BattleRuntimeShip::from(sample_ship(&codex, dd, 50)).in_vanguard(number, 6)
            })
            .collect();
        let candidates: Vec<usize> = (0..6).collect();
        // Formation 0 keeps かばう out of it.
        let pick = |draws: &[i64]| {
            let mut rng = Picks(draws.iter().copied().collect());
            choice_with_protect(&codex, &mut rng, &defenders, &candidates, 0, true, 0)
                .map(|(at, _)| at)
        };
        // The second ship stands in front: she is drawn again.
        assert_eq!(pick(&[1, 4]), Some(4));
        // Drawn again into the front, she stays.
        assert_eq!(pick(&[0, 2]), Some(2));
        // The fifth stands behind: one draw, no more.
        assert_eq!(pick(&[4]), Some(4));
    }

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

        let target_idx =
            draw(&codex, &mut rng, &attacker, &defenders, BattlePhase::DayShelling).unwrap();

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

        let target_idx =
            draw(&codex, &mut rng, &attacker, &defenders, BattlePhase::DayShelling).unwrap();

        assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::SurfaceShip);
    }

    #[test]
    fn the_installation_tables_follow_the_source() {
        use crate::simulation::installation_types::*;
        // What `scripts/gen_installation_types.py` prints: (installtype, ships, sum of their ids).
        for (kind, ships, sum) in [
            (1, 66, 120_844),
            (2, 16, 33_857),
            (3, 62, 124_795),
            (4, 11, 18_469),
            (5, 3, 5_178),
            (6, 16, 31_340),
        ] {
            let ids = || INSTALLATION_TYPES.iter().filter(|row| row.1 == kind).map(|row| row.0);
            assert_eq!((ids().count(), ids().sum::<i64>()), (ships, sum), "installtype {kind}");
        }
        assert!(INSTALLATION_TYPES.is_sorted_by_key(|row| row.0));
        for (ids, ships, sum) in [
            (SUPPLY_DEPOT_IDS, 8, 17_132),
            (ANCHORAGE_IDS, 6, 10_905),
            (DOCK_IDS, 6, 10_977),
            (SUMMER_BATTLESHIP_IDS, 3, 5_091),
            (SUMMER_CRUISER_IDS, 3, 5_118),
            (INSTALLATION_BOMBERS, 14, 4_648),
            (FIGHTER_BOMBERS, 5, 1_367),
        ] {
            assert_eq!((ids.len(), ids.iter().sum::<i64>()), (ships, sum));
        }
        assert_eq!(LAST_SOURCE_ABYSSAL_ID, 2397);
    }

    #[test]
    fn a_land_target_is_whatever_the_sources_table_says() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let class = |name: &str| {
            let mut ship = sample_ship(&codex, dd, 50);
            ship.ship.api_ship_id = ship_mst_id_by_name(&codex, name);
            target_class(&codex, &BattleRuntimeShip::from(ship))
        };
        // Land targets whose names carry none of the old markers.
        assert_eq!(class("北方棲姫"), TargetClass::Installation);
        assert_eq!(class("中枢棲姫"), TargetClass::Installation);
        assert_eq!(class("対空小鬼"), TargetClass::Installation);
        // Ships whose names do, and which the source keeps at sea.
        assert_eq!(class("浮遊要塞"), TargetClass::SurfaceShip);
        assert_eq!(class("泊地棲姫"), TargetClass::SurfaceShip);
        assert_eq!(class("集積地棲姫III バカンスmode"), TargetClass::SurfaceShip);
        assert_eq!(class("砲台小鬼"), TargetClass::Installation);
        assert_eq!(installation_type(ship_mst_id_by_name(&codex, "砲台小鬼")), Some(2));
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

        let target_idx =
            draw(&codex, &mut rng, &attacker, &defenders, BattlePhase::DayShelling).unwrap();

        assert_eq!(target_class(&codex, &defenders[target_idx]), TargetClass::PtBoat);
    }

    #[test]
    fn a_carrier_with_dive_bombers_that_cannot_hit_land_does_not_shell_a_land_target() {
        const TYPE99_DIVE_BOMBER: i64 = 23;
        const JU87C_KAI: i64 = 64;
        const SB2C_3: i64 = 420;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cv = first_ship_mst_by_type(&codex, KcShipType::CV);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let attack_plane =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);
        let carrier = |gear: &[(i64, i64)]| {
            let mut ship = sample_ship(&codex, cv, 50);
            ship.slot_items = gear.iter().map(|(id, _)| slotitem_with_mst_id(*id)).collect();
            ship.ship.api_onslot = [0; 5];
            for (at, (_, count)) in gear.iter().enumerate() {
                ship.ship.api_onslot[at] = *count;
            }
            BattleRuntimeShip::from(ship)
        };
        let mut land = sample_ship(&codex, dd, 50);
        land.ship.api_ship_id = ship_mst_id_by_name(&codex, "砲台小鬼");
        let land = BattleRuntimeShip::from(land);
        let sea = BattleRuntimeShip::from(sample_ship(&codex, dd, 50));
        let fleet = [land.clone(), sea];
        let aims = |ship: &BattleRuntimeShip, defenders: &[BattleRuntimeShip]| {
            target_candidates(&codex, ship, defenders, BattlePhase::DayShelling)
        };

        let type99 = carrier(&[(TYPE99_DIVE_BOMBER, 18)]);
        assert_eq!(aims(&type99, &fleet), vec![1]);
        // With only the land target left she does not fire, and draws nothing for it.
        assert_eq!(aims(&type99, std::slice::from_ref(&land)), Vec::<usize>::new());
        let mut rng = crate::random::SeededRng::new(5);
        let drawn =
            draw(&codex, &mut rng, &type99, std::slice::from_ref(&land), BattlePhase::DayShelling);
        assert_eq!(drawn, None);
        assert_eq!(rng.roll_range(0, 1000), crate::random::SeededRng::new(5).roll_range(0, 1000));
        // A dive bomber the source lets attack land takes the land target back in.
        assert_eq!(aims(&carrier(&[(JU87C_KAI, 18)]), &fleet), vec![0, 1]);
        assert_eq!(aims(&carrier(&[(SB2C_3, 18)]), &fleet), vec![0, 1]);
        // A land-capable bomber alongside the 九九式 is enough.
        assert_eq!(aims(&carrier(&[(TYPE99_DIVE_BOMBER, 18), (SB2C_3, 12)]), &fleet), vec![0, 1]);
        // ...but not once it has lost all its planes.
        assert_eq!(aims(&carrier(&[(TYPE99_DIVE_BOMBER, 18), (SB2C_3, 0)]), &fleet), vec![1]);
        // A 九九式 shot down to nothing leaves her attack planes free to strike the land.
        let shot_down = carrier(&[(TYPE99_DIVE_BOMBER, 0), (attack_plane, 18)]);
        assert_eq!(aims(&shot_down, &fleet), vec![0, 1]);
        // Attack planes alone were never kept off it.
        assert_eq!(aims(&carrier(&[(attack_plane, 18)]), &fleet), vec![0, 1]);
    }

    /// A torpedo goes neither for a submarine nor for land (`torpedoPhase`).
    #[test]
    fn a_torpedo_at_a_submarine_and_a_land_target_has_nothing_to_aim_at() {
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

        let drawn = draw(&codex, &mut rng, &attacker, &defenders, BattlePhase::ClosingTorpedo);

        assert_eq!(drawn, None);
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

    /// R3/R4: an anti-submarine attack names no display equipment. Whatever the
    /// server puts in `api_si_list[0]` is loaded by `CutinAttack` as a
    /// `btxt_flat` name plate, and nothing that forms an anti-submarine attack
    /// has one upstream -- sonars, depth charges and seaplanes all 404.
    #[test]
    fn day_asw_display_ids_report_no_equipment() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);
        let night_recon_mst_id = slotitem_mst_id_by_name(&codex, "九八式水上偵察機(夜偵)");
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);

        let mut ship = sample_ship(&codex, bbv_mst, 50);
        ship.slot_items = vec![
            slotitem_with_mst_id(night_recon_mst_id),
            slotitem_with_mst_id(sonar_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
        ];
        let runtime_ship = BattleRuntimeShip::from(ship);

        assert_eq!(day_attack_display_ids(&codex, &runtime_ship, true), vec![-1]);
        // The surface path is unaffected: guns do have name plates.
        assert_eq!(day_attack_display_ids(&codex, &runtime_ship, false), vec![main_gun_mst_id]);
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

        let target_idx =
            draw(&codex, &mut rng, &attacker, &mixed_defenders, BattlePhase::ClosingTorpedo)
                .unwrap();
        assert_eq!(target_class(&codex, &mixed_defenders[target_idx]), TargetClass::SurfaceShip);
        assert!(
            draw(&codex, &mut rng, &attacker, &submarine_only, BattlePhase::OpeningTorpedo,)
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

    /// A player's ship `mst_id` at level 99 carrying `items`, her anti-submarine set to `asw`.
    fn oasw_ship(codex: &Codex, mst_id: i64, items: &[i64], asw: i64) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 99);
        ship.slot_items = items.iter().copied().map(slotitem_with_mst_id).collect();
        ship.ship.api_taisen[0] = asw;
        BattleRuntimeShip::from(ship)
    }

    // Equipment: 九三式水中聴音機 (46), 九四式爆雷投射機 (44, 対潜 5), 九五式爆雷 (226, 4),
    // 九九式艦爆 (23, 3), 九七式艦攻(九三一空) (82, 7), カ号観測機 (69), オ号観測機改 (324),
    // S-51J (326).

    #[test]
    fn oasw_needs_no_sonar_for_the_ships_the_source_always_lets() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // 五十鈴改二: 対潜 54 at level 1.
        assert!(can_opening_asw(&codex, &oasw_ship(&codex, 141, &[], 60)));
        // 加賀改二護 rules as a light carrier: she needs a bomber with 対潜 even so.
        assert!(can_opening_asw(&codex, &oasw_ship(&codex, 646, &[82], 80)));
        assert!(!can_opening_asw(&codex, &oasw_ship(&codex, 646, &[], 80)));
    }

    #[test]
    fn oasw_de_with_75_and_four_of_equipment_needs_no_sonar() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // 占守改.
        assert!(can_opening_asw(&codex, &oasw_ship(&codex, 376, &[226], 75)));
        assert!(!can_opening_asw(&codex, &oasw_ship(&codex, 376, &[226], 74)));
        assert!(!can_opening_asw(&codex, &oasw_ship(&codex, 376, &[23], 75)));
    }

    #[test]
    fn oasw_light_carrier_thresholds() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ryuujou =
            |items: &[i64], asw| can_opening_asw(&codex, &oasw_ship(&codex, 157, items, asw));
        assert!(ryuujou(&[82, 46], 50));
        assert!(!ryuujou(&[82, 46], 49));
        assert!(ryuujou(&[82], 65));
        assert!(!ryuujou(&[82], 64));
        // A dive bomber lets her try, but only a sonar and 100 then.
        assert!(!ryuujou(&[23, 46], 99));
        assert!(ryuujou(&[23, 46], 100));
        // 鈴谷航改二's sonar leaves her at 65.
        let suzuya = |asw| can_opening_asw(&codex, &oasw_ship(&codex, 508, &[82, 46], asw));
        assert!(!suzuya(64));
        assert!(suzuya(65));
    }

    #[test]
    fn oasw_hyuuga_kai_ni_and_fusou_kai_ni() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let hyuuga = |items: &[i64]| can_opening_asw(&codex, &oasw_ship(&codex, 554, items, 0));
        assert!(hyuuga(&[326]));
        assert!(hyuuga(&[326, 327]));
        assert!(hyuuga(&[69, 324]));
        assert!(!hyuuga(&[69]));
        // 扶桑改二 attacks a submarine with a depth charge, with no aircraft.
        let fusou = |items: &[i64]| can_opening_asw(&codex, &oasw_ship(&codex, 411, items, 100));
        assert!(fusou(&[44, 46]));
        assert!(!fusou(&[46]));
    }

    #[test]
    fn oasw_turn_needs_a_plane_left_and_holds_the_asw_last() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let defender =
            |mst_id| BattleRuntimeShip::new(sample_ship(&codex, mst_id, 50), false, true);
        let submarines = [defender(ss_mst)];
        let with_a_destroyer = [defender(ss_mst), defender(dd_mst)];

        // 龍驤 picked with her 九七式艦攻(九三一空), and the air battle took it all.
        let mut ryuujou = oasw_ship(&codex, 157, &[82, 46], 50);
        assert!(has_opening_asw_shot(&codex, &ryuujou, &submarines));
        ryuujou.ship.api_onslot = [0; 5];
        assert!(can_opening_asw(&codex, &ryuujou));
        assert!(!has_opening_asw_shot(&codex, &ryuujou, &submarines));
        // An abyssal light carrier asks no plane (軽母ヌ級, none left either).
        let (mut nu, slot_items) = codex.new_enemy_ship(1510).unwrap();
        nu.api_onslot = [0; 5];
        let nu = BattleRuntimeShip::new(
            BattleShipInput {
                ship: nu,
                slot_items,
                aircraft_exp: vec![],
                effect_list: vec![0],
                married: false,
            },
            false,
            true,
        );
        assert!(has_opening_asw_shot(&codex, &nu, &submarines));

        // 鈴谷航改二 waits for the surface ships to go.
        let suzuya = oasw_ship(&codex, 508, &[82, 46], 65);
        assert!(has_opening_asw_shot(&codex, &suzuya, &submarines));
        assert!(!has_opening_asw_shot(&codex, &suzuya, &with_a_destroyer));
    }

    #[test]
    fn oasw_abyssal_ships_need_the_source_mark() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let abyssal = |mst_id: i64| {
            let (mut ship, mut slot_items) = codex.new_enemy_ship(mst_id).unwrap();
            ship.api_taisen[0] = 120;
            slot_items.push(slotitem_with_mst_id(46));
            let input = BattleShipInput {
                ship,
                slot_items,
                aircraft_exp: vec![],
                effect_list: vec![0],
                married: false,
            };
            BattleRuntimeShip::new(input, false, true)
        };
        // 駆逐ハ級後期型elite is marked; 駆逐ロ級後期型elite is not, sonar and 120 or no.
        assert!(can_opening_asw(&codex, &abyssal(1623)));
        assert!(!can_opening_asw(&codex, &abyssal(1622)));
    }

    #[test]
    fn the_always_oasw_tables_follow_the_source() {
        // What `scripts/gen_always_oasw.py` prints: ships, sum of their ids.
        let tally = |ids: &[i64]| (ids.len(), ids.iter().sum::<i64>());
        assert_eq!(tally(PLAYER_ALWAYS_OASW_IDS), (24, 15_079));
        assert_eq!(tally(ABYSSAL_ALWAYS_OASW_IDS), (71, 144_657));
    }

    /// The source's 26: its marked 24 and Visby / Visby改 by their level-1 anti-submarine. A
    /// change here means the codex's level-1 figures drifted from the source's `ASWbase`.
    #[test]
    fn the_player_ships_always_opening_with_asw_are_the_sources() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let always: Vec<i64> = codex
            .manifest
            .api_mst_ship
            .iter()
            .map(|mst| mst.api_id)
            .filter(|&id| id < PLAYABLE_ID_END && codex.new_ship(id).is_some())
            .filter(|&id| always_opening_asw(&codex, &oasw_ship(&codex, id, &[], 0)))
            .collect();
        assert_eq!((always.len(), always.iter().sum::<i64>()), (26, 17_208));
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
            let subs = submarine_candidates(&codex, &defenders);
            let (idx, _) =
                choice_with_protect(&codex, &mut rng, &defenders, &subs, 0, true, 0).unwrap();
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

    /// U3.1 / 5.5: a non-submarine, non-CLT ship (a seaplane tender / AV) opens
    /// torpedo only via an equipped 甲標的 — the equipment path is independent of
    /// ship type.
    #[test]
    fn opening_torpedo_seaplane_tender_with_minisub() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let av_mst = first_ship_mst_by_type(&codex, KcShipType::AV);
        let minisub = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SpecialSubmarineVessel);

        // Base torpedo but no 甲標的 → ineligible (AV is not CLT/SS/SSV).
        let mut bare = sample_ship(&codex, av_mst, 99);
        bare.ship.api_raisou[0] = 50;
        let rt_bare = BattleRuntimeShip::new(bare, false, true);
        assert!(
            !can_opening_torpedo_ship(&codex, &rt_bare),
            "an AV without 甲標的 cannot open-torpedo"
        );

        // Equip a 甲標的 → eligible via the equipment path.
        let mut armed = sample_ship(&codex, av_mst, 99);
        armed.ship.api_raisou[0] = 50;
        armed.slot_items = vec![slotitem_with_mst_id(minisub)];
        let rt_armed = BattleRuntimeShip::new(armed, false, true);
        assert!(
            can_opening_torpedo_ship(&codex, &rt_armed),
            "an AV with 甲標的 opens torpedo (equipment path)"
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
        // 警戒航行序列 — no verified rate upstream; 60% is KC3's fallback.
        for id in 11..=14 {
            assert_eq!(escort_shield_rate(id), Some(60), "formation {id}");
        }
        // Anything else never intercepts.
        assert_eq!(escort_shield_rate(0), None);
        assert_eq!(escort_shield_rate(10), None);
        assert_eq!(escort_shield_rate(15), None);
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
                    select_escort_shield(&codex, &mut rng, &defenders, 0, false, formation)
                        .is_some()
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
            assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 0, false, 3), None);
        }
    }

    /// Deck 2 does not shield deck 1's flagship. Upstream (`kcsim.js:2318`)
    /// builds the protector pool with `!isescort` unless the caller opts in,
    /// and neither the day shelling nor the torpedo path does.
    #[test]
    fn escort_deck_does_not_shield_the_main_deck_flagship() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Deck 1 flagship alone, then a healthy deck 2 ship. Were the escort
        // deck eligible, 警戒航行序列 would intercept at 60%.
        let escort = rt_ship(&codex, dd, 30, 30)
            .in_combined_fleet(CombinedType::CarrierTaskForce, CombinedFleetRole::Escort);
        let flagship = rt_ship(&codex, dd, 30, 30)
            .in_combined_fleet(CombinedType::CarrierTaskForce, CombinedFleetRole::Main);
        let defenders = vec![flagship, escort];

        for seed in [1u64, 7, 999] {
            let mut rng = crate::random::SeededRng::new(seed);
            assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 0, false, 11), None);
        }
    }

    /// At night a combined fleet fights with deck 2 alone, so its flagship sits at index 0;
    /// she is still deck 2's flagship, whom `choiceWProtect` returns before the roll.
    #[test]
    fn deck_2_alone_at_night_leaves_its_flagship_uncovered_and_rolls_nothing() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let deck_2 = || {
            rt_ship(&codex, dd, 30, 30)
                .in_combined_fleet(CombinedType::CarrierTaskForce, CombinedFleetRole::Escort)
        };
        let defenders = vec![deck_2(), deck_2()];
        for include_escort in [true, false] {
            let mut rng = Picks(Default::default());
            assert_eq!(
                select_escort_shield(&codex, &mut rng, &defenders, 0, include_escort, 11),
                None
            );
        }
    }

    /// An enemy escort-deck ship covering the main deck's flagship takes the hit, but the
    /// source shows no かばう for it; a friendly one is shown.
    #[test]
    fn an_enemy_escort_deck_cover_is_not_shown() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        // Draw the flagship, roll under the rate, pick the one protector.
        let cover = |defenders: &[BattleRuntimeShip]| {
            let mut rng = Picks([0, 0, 0].into_iter().collect());
            choice_with_protect(&codex, &mut rng, defenders, &[0], 0, true, 3)
        };

        let enemy = vec![
            rt_ship(&codex, dd, 30, 30).in_enemy_deck(CombinedFleetRole::Main),
            rt_ship(&codex, dd, 30, 30).in_enemy_deck(CombinedFleetRole::Escort),
        ];
        assert_eq!(cover(&enemy), Some((1, false)));

        let friendly = [CombinedFleetRole::Main, CombinedFleetRole::Escort].map(|role| {
            BattleRuntimeShip::new(sample_ship(&codex, dd, 50), true, true)
                .in_combined_fleet(CombinedType::CarrierTaskForce, role)
        });
        assert_eq!(cover(&friendly), Some((1, true)));
    }

    /// A caller that passes `includeEscort` (the opening ASW, night surface draws) lets
    /// deck 2 cover deck 1's flagship.
    #[test]
    fn escort_deck_shields_the_main_deck_flagship_when_included() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let escort = rt_ship(&codex, dd, 30, 30)
            .in_combined_fleet(CombinedType::CarrierTaskForce, CombinedFleetRole::Escort);
        let flagship = rt_ship(&codex, dd, 30, 30)
            .in_combined_fleet(CombinedType::CarrierTaskForce, CombinedFleetRole::Main);
        let defenders = vec![flagship, escort];

        let mut rng = crate::random::SeededRng::new(7);
        let fired = (0..4000)
            .filter(|_| select_escort_shield(&codex, &mut rng, &defenders, 0, true, 11) == Some(1))
            .count();
        assert!((2200..=2600).contains(&fired), "警戒航行序列 ~60%: got {fired}/4000");
    }

    /// Deck 2's own flagship is never shielded either — the two decks share one
    /// vector with deck 1 first, so it never sits at index 0 and the existing
    /// flagship guard rejects it.
    #[test]
    fn escort_deck_flagship_is_never_shielded() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        // [deck 1 flagship, deck 1 ship, deck 2 flagship, deck 2 ship].
        let mut defenders = vec![
            rt_ship(&codex, dd, 30, 30),
            rt_ship(&codex, dd, 30, 30),
            rt_ship(&codex, dd, 30, 30),
            rt_ship(&codex, dd, 30, 30),
        ];
        for (idx, ship) in defenders.iter_mut().enumerate() {
            let role = if idx < 2 {
                CombinedFleetRole::Main
            } else {
                CombinedFleetRole::Escort
            };
            *ship = ship.clone().in_combined_fleet(CombinedType::CarrierTaskForce, role);
        }

        // Index 2 is deck 2's flagship.
        for seed in [1u64, 7, 999] {
            let mut rng = crate::random::SeededRng::new(seed);
            assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 2, false, 11), None);
        }
    }

    /// 警戒航行序列 intercept at KC3's 60% fallback, and deck 1 ships do the
    /// protecting.
    #[test]
    fn combined_formations_intercept_at_sixty_percent() {
        use crate::combined::{CombinedFleetRole, CombinedType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut defenders = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 30, 30)];
        for ship in &mut defenders {
            *ship = ship
                .clone()
                .in_combined_fleet(CombinedType::CarrierTaskForce, CombinedFleetRole::Main);
        }

        let trials = 4000;
        for formation in 11..=14 {
            let mut rng = crate::random::SeededRng::new(7);
            let fired = (0..trials)
                .filter(|_| {
                    select_escort_shield(&codex, &mut rng, &defenders, 0, false, formation)
                        .is_some()
                })
                .count();
            assert!(
                (2200..=2600).contains(&fired),
                "formation {formation} ~60%: got {fired}/{trials}"
            );
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
            assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 0, false, 3), None);
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
        assert_eq!(select_escort_shield(&codex, &mut rng, &at_75, 0, false, 3), None);

        // 76%: 76*4 = 304 > 300 → eligible → fires on some rolls.
        let at_76 = vec![rt_ship(&codex, dd, 30, 30), rt_ship(&codex, dd, 76, 100)];
        let mut rng = crate::random::SeededRng::new(7);
        let fired = (0..200)
            .filter(|_| select_escort_shield(&codex, &mut rng, &at_76, 0, false, 3).is_some())
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
        assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 1, false, 1), None);
        // No draw consumed: the next roll matches a fresh rng's first roll.
        let next = rng.roll_range(0, 1_000_000);
        let fresh = crate::random::SeededRng::new(5).roll_range(0, 1_000_000);
        assert_eq!(next, fresh, "non-flagship target must not draw RNG");
    }

    /// `choiceWProtect`: a land target is neither shielded nor a shield.
    #[test]
    fn a_land_target_neither_takes_nor_gives_cover() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let land = || {
            let mut ship = rt_ship(&codex, dd, 30, 30);
            ship.ship.api_ship_id = ship_mst_id_by_name(&codex, "砲台小鬼");
            ship
        };
        let picks = |faces: &[i64]| Picks(faces.iter().copied().collect());

        // A land flagship is not covered, and no die is cast for it.
        let defenders = vec![land(), rt_ship(&codex, dd, 30, 30)];
        assert_eq!(select_escort_shield(&codex, &mut picks(&[]), &defenders, 0, false, 3), None);
        // A healthy land escort does not cover a ship (the roll is still cast).
        let defenders = vec![rt_ship(&codex, dd, 30, 30), land()];
        assert_eq!(select_escort_shield(&codex, &mut picks(&[0]), &defenders, 0, false, 3), None);
        // A ship beside it still does: the first eligible escort is the ship, not the land.
        let defenders = vec![rt_ship(&codex, dd, 30, 30), land(), rt_ship(&codex, dd, 30, 30)];
        assert_eq!(
            select_escort_shield(&codex, &mut picks(&[0, 0]), &defenders, 0, false, 3),
            Some(2)
        );
    }

    /// `choiceWProtect` casts the die for かばう before it looks for a protector, so a
    /// flagship with none beside her still costs one roll.
    #[test]
    fn a_flagship_with_no_protector_still_costs_the_roll() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut land = rt_ship(&codex, dd, 30, 30);
        land.ship.api_ship_id = ship_mst_id_by_name(&codex, "砲台小鬼");
        let defenders = vec![rt_ship(&codex, dd, 30, 30), land];

        let mut rng = Picks([0].into_iter().collect());
        assert_eq!(select_escort_shield(&codex, &mut rng, &defenders, 0, false, 3), None);
        assert!(rng.0.is_empty(), "the roll is cast whether or not anyone could protect");
    }

    /// R5: a submarine escort protects a submarine flagship.
    #[test]
    fn submarine_flagship_protected_by_submarine_escort() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let defenders = vec![rt_ship(&codex, ss, 30, 30), rt_ship(&codex, ss, 30, 30)];
        let mut rng = crate::random::SeededRng::new(7);
        let fired = (0..200)
            .filter(|_| select_escort_shield(&codex, &mut rng, &defenders, 0, false, 3).is_some())
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
            if let Some(idx) = select_escort_shield(&codex, &mut rng, &defenders, 0, false, 3) {
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
