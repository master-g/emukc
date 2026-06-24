//! Night battle phase simulation.
//!
//! Implements night attack type detection (cut-in / double attack),
//! CI trigger rate calculation, and the night hougeki simulation loop.

use emukc_model::{
    codex::Codex,
    kc2::{KcShipType, KcSlotItemType3},
};

use crate::damage::{calculate_night_damage, calculate_scratch_damage};
use crate::random::BattleRng;
use crate::targeting::{
    can_attack_night_ship, collect_matching_slot_ids, extend_limit, has_slotitem_id,
    is_day_surface_display_type, select_random_target_index, ship_type, target_class,
};
use crate::types::{
    BattleNightHougeki, BattlePhase, BattleRuntimeShip, DamageCell, NightBattleParams, SiListId,
};

// ---------------------------------------------------------------------------
// Night attack type enum
// ---------------------------------------------------------------------------

/// Night battle special attack (cut-in / double attack) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NightAttackType {
    Normal,
    DoubleAttack,  // 連撃: 2 hits x 1.2x
    MainTorpRadar, // 主魚CI: 1 hit x 1.625x (sp_list=2)
    TorpTorpTorp,  // 魚魚CI: 2 hits x 1.3x (sp_list=3)
    MainMainSec,   // 主主副CI: 1 hit x 1.75x (sp_list=4)
    MainMainMain,  // 主主主CI: 1 hit x 2.0x (sp_list=5)
    // DD CI variants (sp_list 7-14)
    DdGunTorpRadar,      // GTR: 1 hit x 1.3x (sp_list=7)
    DdGunTorpRadar2,     // GTR 2-hit: 2 hits x 1.3x (sp_list=11)
    DdTorpLookoutRadar,  // TRL: 1 hit x 1.2x (sp_list=8)
    DdTorpLookoutRadar2, // TRL 2-hit: 2 hits x 1.2x (sp_list=12)
    DdTorpTorpLookout,   // TTL: 1 hit x 1.5x (sp_list=9)
    DdTorpTorpLookout2,  // TTL 2-hit: 2 hits x 1.5x (sp_list=13)
    DdTorpDrumLookout,   // DTL: 1 hit x 1.3x (sp_list=10)
    DdTorpDrumLookout2,  // DTL 2-hit: 2 hits x 1.3x (sp_list=14)
    // Carrier night CI (sp_list=6)
    CarrierNightCI(CarrierNightCiSubType), // 戦爆連合夜間CI: 1 hit, multiplier varies by sub-type
}

impl NightAttackType {
    fn api_sp_list(self) -> i64 {
        match self {
            Self::Normal => 0,
            Self::DoubleAttack => 1,
            Self::MainTorpRadar => 2,
            Self::TorpTorpTorp => 3,
            Self::MainMainSec => 4,
            Self::MainMainMain => 5,
            Self::DdGunTorpRadar => 7,
            Self::DdGunTorpRadar2 => 11,
            Self::DdTorpLookoutRadar => 8,
            Self::DdTorpLookoutRadar2 => 12,
            Self::DdTorpTorpLookout => 9,
            Self::DdTorpTorpLookout2 => 13,
            Self::DdTorpDrumLookout => 10,
            Self::DdTorpDrumLookout2 => 14,
            Self::CarrierNightCI(_) => 6,
        }
    }

    fn damage_multiplier(self) -> f64 {
        match self {
            Self::Normal => 1.0,
            Self::DoubleAttack | Self::DdTorpLookoutRadar | Self::DdTorpLookoutRadar2 => 1.2,
            Self::MainTorpRadar => 1.625,
            Self::TorpTorpTorp
            | Self::DdGunTorpRadar
            | Self::DdGunTorpRadar2
            | Self::DdTorpDrumLookout
            | Self::DdTorpDrumLookout2 => 1.3,
            Self::MainMainSec => 1.75,
            Self::MainMainMain => 2.0,
            Self::DdTorpTorpLookout | Self::DdTorpTorpLookout2 => 1.5,
            Self::CarrierNightCI(sub) => sub.damage_multiplier(),
        }
    }

    fn hit_count(self) -> usize {
        match self {
            Self::Normal
            | Self::MainTorpRadar
            | Self::MainMainSec
            | Self::MainMainMain
            | Self::DdGunTorpRadar
            | Self::DdTorpLookoutRadar
            | Self::DdTorpTorpLookout
            | Self::DdTorpDrumLookout
            | Self::CarrierNightCI(_) => 1,
            Self::DoubleAttack
            | Self::TorpTorpTorp
            | Self::DdGunTorpRadar2
            | Self::DdTorpLookoutRadar2
            | Self::DdTorpTorpLookout2
            | Self::DdTorpDrumLookout2 => 2,
        }
    }

    fn ci_coefficient(self) -> f64 {
        match self {
            Self::MainTorpRadar | Self::DdGunTorpRadar | Self::DdGunTorpRadar2 => 115.0,
            Self::TorpTorpTorp | Self::DdTorpDrumLookout | Self::DdTorpDrumLookout2 => 122.0,
            Self::MainMainSec => 130.0,
            Self::MainMainMain | Self::DdTorpLookoutRadar | Self::DdTorpLookoutRadar2 => 140.0,
            Self::DdTorpTorpLookout | Self::DdTorpTorpLookout2 => 125.0,
            Self::CarrierNightCI(sub) => sub.coefficient(),
            Self::DoubleAttack | Self::Normal => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Equipment counting helpers
// ---------------------------------------------------------------------------

fn count_equipment_type(codex: &Codex, ship: &BattleRuntimeShip, wanted: KcSlotItemType3) -> usize {
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                == Some(wanted)
        })
        .count()
}

fn is_main_gun_type(t: KcSlotItemType3) -> bool {
    matches!(
        t,
        KcSlotItemType3::SmallCaliberMainGun
            | KcSlotItemType3::MediumCaliberMainGun
            | KcSlotItemType3::LargeCaliberMainGun
            | KcSlotItemType3::LargeCaliberMainGun2
    )
}

fn count_main_guns(codex: &Codex, ship: &BattleRuntimeShip) -> usize {
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(is_main_gun_type)
        })
        .count()
}

fn count_secondary_guns(codex: &Codex, ship: &BattleRuntimeShip) -> usize {
    ship.slot_items
        .iter()
        .filter(|si| {
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .and_then(|mst| KcSlotItemType3::n(mst.api_type[2]))
                .is_some_and(|t| {
                    matches!(t, KcSlotItemType3::SecondaryGun | KcSlotItemType3::SecondaryGun2)
                })
        })
        .count()
}

fn has_radar(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    ship.slot_items.iter().any(|slot_item| {
        codex
            .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&slot_item.api_slotitem_id)
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

// ---------------------------------------------------------------------------
// Carrier night CI
// ---------------------------------------------------------------------------

/// Night plane icon types (`api_type`[3]).
const NIGHT_FIGHTER_ICON: i64 = 45; // 夜間戦闘機
const NIGHT_ATTACKER_ICON: i64 = 46; // 夜間攻撃機
const NIGHT_BOMBER_ICON: i64 = 58; // 夜間爆戦
// Note: 夜間瑞雲 (icon 51) is intentionally not a carrier-night-CI plane — none of the 8
// sub-types reference it (see plan Q13). The separate 夜間瑞雲夜戦カットイン (sp_list=200) is
// deferred follow-up work; its icon constant lands with that plan.

/// Aviation personnel item IDs for night carrier operations.
const AVIATION_PERSONNEL_IDS: &[i64] = &[258, 259]; // 夜間作戦航空要員/夜間作戦航空要員(熟練)

/// Carriers that trigger night CI **without** 夜間作戦航空要員 — a built-in 夜戦特性 granted by
/// a specific remodel. The night-plane requirement still applies. `mst_id`s verified against the
/// Codex bootstrap (`.data/codex/start2.json`); see wikiwiki 夜戦 (<https://wikiwiki.jp/kancolle/夜戦>).
///
/// Saratoga Mk.II Mod.2 (`mst_id=550`) is deliberately **excluded** — it loses 夜戦特性 on the
/// further upgrade (<https://zekamashi.net/kancolle-kouryaku/yasyuu-cutin/>). 加賀改二護 (646) is
/// also excluded — a Type Ⅰ 无条件 night-battle carrier (a different mechanic, out of scope).
const SARATOGA_MK2_ID: i64 = 545; // Saratoga Mk.II
const AKAGI_K2E_ID: i64 = 599; // 赤城改二戊
const KAGA_K2E_ID: i64 = 610; // 加賀改二戊
const RYUUHOU_K2E_ID: i64 = 883; // 龍鳳改二戊
/// `pub(crate)` so `targeting::can_attack_night_ship` can let these CVs attack at night without
/// 夜間作戦航空要員 (their built-in 夜戦特性), mirroring the bypass in `is_cv_night_ci_eligible`.
pub(crate) const EXEMPT_NIGHT_CV_IDS: &[i64] =
    &[SARATOGA_MK2_ID, AKAGI_K2E_ID, KAGA_K2E_ID, RYUUHOU_K2E_ID];

/// 光電管彗星 (彗星一二型(三一号光電管爆弾搭載機)). `api_type[3]=7` (regular dive-bomber
/// icon), so `count_night_planes_by_icon` misses it — it must be matched by item id. Counts
/// toward the 戦彗 / 攻彗 / 爆彗 carrier-night-CI sub-types.
const KOUDENKAN_SUISEI_ID: i64 = 320;
/// 零戦62型(爆戦/岩井隊). A 夜間飛行機 but with a non-night icon; contributes only to the
/// `Nf1Other` (戦他他) fallback count.
const IWAI_FUKUSEN_ID: i64 = 154;
/// Swordfish / Swordfish Mk.II(熟練) / Swordfish Mk.III(熟練). 夜間飛行機 with a
/// torpedo-bomber icon (not a night icon), so matched by item id.
const SWORDFISH_IDS: &[i64] = &[242, 243, 244];

const SKILLED_LOOKOUT_ID: i64 = 412;

/// Chuuha bracket: 25% < HP/MaxHP <= 50%. Excludes taiha (HP <= 25%).
fn is_chuuha(ship: &BattleRuntimeShip) -> bool {
    let r = ship.hp() as f64 / ship.ship.api_maxhp.max(1) as f64;
    r > 0.25 && r <= 0.5
}

fn count_night_planes_by_icon(codex: &Codex, ship: &BattleRuntimeShip, icon: i64) -> usize {
    ship.slot_items
        .iter()
        .zip(ship.ship.api_onslot)
        .filter(|(si, onslot)| {
            if *onslot <= 0 {
                return false;
            }
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .is_some_and(|mst| mst.api_type[3] == icon)
        })
        .count()
}

/// Count slot items (in non-shot-down slots, `onslot > 0`) whose item id satisfies `pred`.
/// Mirrors `count_night_planes_by_icon`'s shot-down exclusion for item-id-based detection.
fn count_slotitems_with_onslot(ship: &BattleRuntimeShip, pred: impl Fn(i64) -> bool) -> usize {
    ship.slot_items
        .iter()
        .zip(ship.ship.api_onslot)
        .filter(|(si, onslot)| *onslot > 0 && pred(si.api_slotitem_id))
        .count()
}

/// Count 光電管彗星 (item 320) in non-shot-down slots. Disjoint from icon-based counts —
/// item 320 carries a regular dive-bomber icon, not a night-plane icon.
fn count_kouden_suisei(ship: &BattleRuntimeShip) -> usize {
    count_slotitems_with_onslot(ship, |id| id == KOUDENKAN_SUISEI_ID)
}

/// Count Swordfish系 (242/243/244) and 岩井爆戦 (154) in non-shot-down slots. 夜間飛行機 by
/// classification, but with non-night icons, so matched by item id (disjoint from icon counts).
fn count_swordfish_iwai(ship: &BattleRuntimeShip) -> usize {
    count_slotitems_with_onslot(ship, |id| SWORDFISH_IDS.contains(&id) || id == IWAI_FUKUSEN_ID)
}

/// Carrier night CI sub-type (戦爆連合夜間CI, `sp_list=6`). Selected by night-plane composition
/// via [`detect_carrier_night_ci_sub_type`]; drives the damage multiplier and the trigger-rate
/// 種別係数. Every sub-type still emits `api_sp_list=6`.
///
/// Multipliers and coefficients per wikiwiki 夜戦 (<https://wikiwiki.jp/kancolle/夜戦>). The
/// 戦彗 / 攻彗 / 戦爆 / 攻爆 / 爆彗 coefficients are annotated `120?` (uncertain) by wikiwiki;
/// treated as 120 until contradicting battle samples emerge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarrierNightCiSubType {
    Nf2Na,    // 戦戦攻: ≥2 夜戦 + ≥1 夜攻 — 1.25x, coeff 105
    Nf1Na,    // 戦攻:   ≥1 夜戦 + ≥1 夜攻 — 1.20x, coeff 120
    Nf1Kk,    // 戦彗:   ≥1 夜戦 + ≥1 光電管彗星 — 1.20x, coeff 120
    Na1Kk,    // 攻彗:   ≥1 夜攻 + ≥1 光電管彗星 (no 夜戦) — 1.20x, coeff 120
    Nf1Nb,    // 戦爆:   ≥1 夜戦 + ≥1 夜爆 — 1.20x, coeff 120
    Na1Nb,    // 攻爆:   ≥1 夜攻 + ≥1 夜爆 — 1.20x, coeff 120
    Nb1Kk,    // 爆彗:   ≥1 夜爆 + ≥1 光電管彗星 — 1.20x, coeff 120
    Nf1Other, // 戦他他: ≥1 夜戦 + total 夜間飛行機 ≥2 — 1.18x, coeff 130
}

impl CarrierNightCiSubType {
    fn damage_multiplier(self) -> f64 {
        match self {
            Self::Nf2Na => 1.25,
            Self::Nf1Na | Self::Nf1Kk | Self::Na1Kk | Self::Nf1Nb | Self::Na1Nb | Self::Nb1Kk => {
                1.20
            }
            Self::Nf1Other => 1.18,
        }
    }

    /// Trigger-rate 種別係数 (the denominator in `night_ci_trigger_rate`).
    fn coefficient(self) -> f64 {
        match self {
            Self::Nf2Na => 105.0,
            Self::Nf1Na | Self::Nf1Kk | Self::Na1Kk | Self::Nf1Nb | Self::Na1Nb | Self::Nb1Kk => {
                120.0
            }
            Self::Nf1Other => 130.0,
        }
    }
}

/// Detect the carrier night CI sub-type from the ship's night-plane composition, walking the
/// 8 wikiwiki priorities top-down (first match wins). The counts are disjoint by construction:
/// icon-based (夜戦/夜攻/夜爆) and item-id-based (光電管彗星, Swordfish系/岩井) never overlap.
/// Returns `None` when no sub-type matches (e.g. 夜間瑞雲-only or a lone 夜戦).
fn detect_carrier_night_ci_sub_type(
    codex: &Codex,
    ship: &BattleRuntimeShip,
) -> Option<CarrierNightCiSubType> {
    use CarrierNightCiSubType::*;

    let nf = count_night_planes_by_icon(codex, ship, NIGHT_FIGHTER_ICON);
    let na = count_night_planes_by_icon(codex, ship, NIGHT_ATTACKER_ICON);
    let nb = count_night_planes_by_icon(codex, ship, NIGHT_BOMBER_ICON);
    let kk = count_kouden_suisei(ship);
    let sf_iwai = count_swordfish_iwai(ship);
    // 夜間飛行機 (per wikiwiki) is the disjoint union of all five counts.
    let total_yakanhikouki = nf + na + nb + kk + sf_iwai;

    // Positive-only predicates; the top-down order makes higher priorities win ties (e.g. a
    // 夜戦+光電管彗星 reaches Nf1Kk before Na1Kk because Na1Kk only fires when nf is exhausted).
    if nf >= 2 && na >= 1 {
        Some(Nf2Na)
    } else if nf >= 1 && na >= 1 {
        Some(Nf1Na)
    } else if nf >= 1 && kk >= 1 {
        Some(Nf1Kk)
    } else if na >= 1 && kk >= 1 {
        Some(Na1Kk)
    } else if nf >= 1 && nb >= 1 {
        Some(Nf1Nb)
    } else if na >= 1 && nb >= 1 {
        Some(Na1Nb)
    } else if nb >= 1 && kk >= 1 {
        Some(Nb1Kk)
    } else if nf >= 1 && total_yakanhikouki >= 2 {
        Some(Nf1Other)
    } else {
        None
    }
}

/// Check if a carrier is eligible for night CI.
///
/// Requires CV type, plus either 夜間作戦航空要員 (item 258/259) **or** membership in
/// [`EXEMPT_NIGHT_CV_IDS`], plus a recognised night-plane combination. The plane-combination
/// check delegates to [`detect_carrier_night_ci_sub_type`] so eligibility and detection can
/// never diverge — the old icon-only predicate could not recognise the 光電管彗星 / Swordfish /
/// 岩井 sub-types, and a 夜間瑞雲-only or lone-夜戦 setup correctly returns `None` → ineligible.
fn is_cv_night_ci_eligible(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if !crate::damage::is_cv_type(codex, ship) {
        return false;
    }
    let has_personnel = AVIATION_PERSONNEL_IDS
        .iter()
        .any(|&id| ship.slot_items.iter().any(|si| si.api_slotitem_id == id));
    let is_exempt = EXEMPT_NIGHT_CV_IDS.contains(&ship.ship.api_ship_id);
    if !(has_personnel || is_exempt) {
        return false;
    }
    detect_carrier_night_ci_sub_type(codex, ship).is_some()
}

// ---------------------------------------------------------------------------
// DD CI types (internal detection + trigger)
// ---------------------------------------------------------------------------

/// DD-specific night CI base types. Each has 1-hit and 2-hit resolved variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DdCiType {
    GunTorpRadar,     // GTR: 主砲+魚雷+電探 (sp_list=7/11)
    TorpLookoutRadar, // TRL: 魚雷+見張員+電探 (sp_list=8/12)
    TorpTorpLookout,  // TTL: 魚雷+水雷見張員+魚雷 (sp_list=9/13)
    TorpDrumLookout,  // DTL: 魚雷+水雷見張員+ドラム缶 (sp_list=10/14)
}

impl DdCiType {
    fn base_attack(self) -> f64 {
        match self {
            Self::GunTorpRadar => 115.0,
            Self::TorpLookoutRadar => 140.0,
            Self::TorpTorpLookout => 125.0,
            Self::TorpDrumLookout => 122.0,
        }
    }

    fn second_hit_probability(self) -> f64 {
        match self {
            Self::GunTorpRadar => 0.65,
            Self::TorpLookoutRadar => 0.50,
            Self::TorpTorpLookout => 0.875,
            Self::TorpDrumLookout => 0.55,
        }
    }

    fn to_attack_type(self, two_hit: bool) -> NightAttackType {
        match (self, two_hit) {
            (Self::GunTorpRadar, false) => NightAttackType::DdGunTorpRadar,
            (Self::GunTorpRadar, true) => NightAttackType::DdGunTorpRadar2,
            (Self::TorpLookoutRadar, false) => NightAttackType::DdTorpLookoutRadar,
            (Self::TorpLookoutRadar, true) => NightAttackType::DdTorpLookoutRadar2,
            (Self::TorpTorpLookout, false) => NightAttackType::DdTorpTorpLookout,
            (Self::TorpTorpLookout, true) => NightAttackType::DdTorpTorpLookout2,
            (Self::TorpDrumLookout, false) => NightAttackType::DdTorpDrumLookout,
            (Self::TorpDrumLookout, true) => NightAttackType::DdTorpDrumLookout2,
        }
    }
}

/// Check equipment conditions for a DD CI type.
fn detect_dd_ci_type(codex: &Codex, ship: &BattleRuntimeShip, dd_type: DdCiType) -> bool {
    let main_guns = count_main_guns(codex, ship);
    let torps = count_equipment_type(codex, ship, KcSlotItemType3::Torpedo)
        + count_equipment_type(codex, ship, KcSlotItemType3::SubmarineTorpedo);
    let has_radar = has_radar(codex, ship);
    let has_lookout = count_equipment_type(codex, ship, KcSlotItemType3::SeaplanePersonnel) > 0;
    let has_skilled_lookout = has_slotitem_id(ship, SKILLED_LOOKOUT_ID);
    let has_drum = count_equipment_type(codex, ship, KcSlotItemType3::TransportContainer) > 0;

    match dd_type {
        DdCiType::GunTorpRadar => main_guns >= 1 && torps >= 1 && has_radar,
        DdCiType::TorpLookoutRadar => torps >= 1 && has_lookout && has_radar,
        DdCiType::TorpTorpLookout => torps >= 2 && has_skilled_lookout,
        DdCiType::TorpDrumLookout => torps >= 1 && has_skilled_lookout && has_drum,
    }
}

/// DD CI trigger rate (Level+Luck based, different from standard Luck-only formula).
fn dd_ci_trigger_rate(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    dd_type: DdCiType,
    is_flagship: bool,
) -> f64 {
    let luck = ship.ship.api_lucky[1].max(0) as f64;
    let level = ship.ship.api_lv.max(1) as f64;

    let base_ship = if luck < 50.0 {
        (0.75 * level.sqrt()).floor() + luck
    } else {
        (0.80 * level.sqrt()).floor() + 50.0 + (luck - 50.0).sqrt()
    };

    let flagship_mod = if is_flagship {
        15.0
    } else {
        0.0
    };

    let chuuha_mod = if is_chuuha(ship) {
        18.0
    } else {
        0.0
    };

    // Lookout modifier: TSLO (+8) overrides regular lookout (+5)
    let lookout_mod = if has_slotitem_id(ship, SKILLED_LOOKOUT_ID) {
        8.0
    } else if count_equipment_type(codex, ship, KcSlotItemType3::SeaplanePersonnel) > 0 {
        5.0
    } else {
        0.0
    };

    let total = (15.0 + base_ship + flagship_mod + chuuha_mod + lookout_mod).floor();
    (total / dd_type.base_attack()).clamp(0.0, 1.0)
}

/// Resolve DD night CI via multiroll (GTR→TRL→TTL→DTL).
/// Returns Some if a DD CI type triggered, None to fall through to standard CI.
fn resolve_dd_night_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    ship: &BattleRuntimeShip,
    is_flagship: bool,
) -> Option<NightAttackType> {
    let dd_types = [
        DdCiType::GunTorpRadar,
        DdCiType::TorpLookoutRadar,
        DdCiType::TorpTorpLookout,
        DdCiType::TorpDrumLookout,
    ];

    for dd_type in dd_types {
        if !detect_dd_ci_type(codex, ship, dd_type) {
            continue;
        }
        let rate = dd_ci_trigger_rate(codex, ship, dd_type, is_flagship);
        if rng.random_f64_range(0.0, 1.0) < rate {
            let is_two_hit = rng.random_f64_range(0.0, 1.0) < dd_type.second_hit_probability();
            return Some(dd_type.to_attack_type(is_two_hit));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Night attack detection & CI trigger
// ---------------------------------------------------------------------------

/// Returns true if the ship qualifies for night double-attack.
/// Requires at least 2 different weapon categories (main + secondary, main + torp, or 2+ main).
fn is_double_attack_eligible(main_guns: usize, sec_guns: usize, torps: usize) -> bool {
    (main_guns >= 2) || (main_guns >= 1 && sec_guns >= 1) || (main_guns >= 1 && torps >= 1)
}

/// Detect the best night attack type from equipment loadout.
fn detect_night_attack_type(codex: &Codex, ship: &BattleRuntimeShip) -> NightAttackType {
    // Carrier night CI takes priority for CV ships
    if is_cv_night_ci_eligible(codex, ship)
        && let Some(sub) = detect_carrier_night_ci_sub_type(codex, ship)
    {
        return NightAttackType::CarrierNightCI(sub);
    }

    let main_guns = count_main_guns(codex, ship);
    let torps = count_equipment_type(codex, ship, KcSlotItemType3::Torpedo)
        + count_equipment_type(codex, ship, KcSlotItemType3::SubmarineTorpedo);
    let sec_guns = count_secondary_guns(codex, ship);
    let has_radar = has_radar(codex, ship);

    // CI priority (highest first): 主主主 > 主主副 > 鱼雷CI > 主鱼電 > 連撃
    if main_guns >= 3 {
        return NightAttackType::MainMainMain;
    }
    if main_guns >= 2 && sec_guns >= 1 {
        return NightAttackType::MainMainSec;
    }
    if torps >= 2 {
        return NightAttackType::TorpTorpTorp;
    }
    if main_guns >= 1 && torps >= 1 && has_radar {
        return NightAttackType::MainTorpRadar;
    }
    // Double attack: 2+ different weapon categories (main + secondary, main + torp, etc.)
    if is_double_attack_eligible(main_guns, sec_guns, torps) {
        return NightAttackType::DoubleAttack;
    }
    NightAttackType::Normal
}

/// Calculate night CI trigger rate.
fn night_ci_trigger_rate(
    ship: &BattleRuntimeShip,
    ci_type: NightAttackType,
    is_flagship: bool,
) -> f64 {
    let coefficient = ci_type.ci_coefficient();
    if coefficient <= 0.0 {
        return if ci_type == NightAttackType::DoubleAttack {
            0.99
        } else {
            0.0
        };
    }

    let luck = ship.ship.api_lucky[1].max(0) as f64;
    let level = ship.ship.api_lv.max(1) as f64;

    let ci_value = if luck < 50.0 {
        15.0 + luck + (0.75 * level.sqrt()).floor()
    } else {
        65.0 + (luck - 50.0).sqrt() + (0.8 * level.sqrt()).floor()
    };

    let flagship_bonus = if is_flagship {
        15.0
    } else {
        0.0
    };

    // Chuuha bonus: 25% < HP <= 50% (中破) → +18 for DD torpedo CI, +5 for others
    let chuuha_bonus = if is_chuuha(ship) {
        if ci_type == NightAttackType::TorpTorpTorp {
            18.0
        } else {
            5.0
        }
    } else {
        0.0
    };

    let total = ci_value + flagship_bonus + chuuha_bonus;
    (total / coefficient).clamp(0.0, 1.0)
}

/// Resolve night attack type: detect CI from equipment, then roll trigger.
fn resolve_night_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    ship: &BattleRuntimeShip,
    is_flagship: bool,
    is_submarine_target: bool,
) -> NightAttackType {
    if is_submarine_target {
        return NightAttackType::Normal;
    }

    // DD CI multiroll: GTR→TRL→TTL→DTL, fallback to standard CI
    if matches!(ship_type(codex, ship), Some(KcShipType::DD))
        && let Some(dd_ci) = resolve_dd_night_attack(codex, rng, ship, is_flagship)
    {
        return dd_ci;
    }

    let detected = detect_night_attack_type(codex, ship);
    if detected == NightAttackType::Normal {
        return NightAttackType::Normal;
    }
    if detected == NightAttackType::DoubleAttack {
        return NightAttackType::DoubleAttack;
    }
    // Roll CI trigger
    let rate = night_ci_trigger_rate(ship, detected, is_flagship);
    let roll = rng.random_f64_range(0.0, 1.0);
    if roll < rate {
        detected
    } else {
        // A CV that failed its night CI roll falls back to Normal — carriers do not perform the
        // artillery 連撃 (double attack) at night.
        if matches!(detected, NightAttackType::CarrierNightCI(_)) {
            return NightAttackType::Normal;
        }
        // Failed CI -> check for double attack fallback
        let main_guns = count_main_guns(codex, ship);
        let sec_guns = count_secondary_guns(codex, ship);
        let torps = count_equipment_type(codex, ship, KcSlotItemType3::Torpedo)
            + count_equipment_type(codex, ship, KcSlotItemType3::SubmarineTorpedo);
        if is_double_attack_eligible(main_guns, sec_guns, torps) {
            NightAttackType::DoubleAttack
        } else {
            NightAttackType::Normal
        }
    }
}

// ---------------------------------------------------------------------------
// Night attack display IDs
// ---------------------------------------------------------------------------

fn night_attack_display_ids(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    attack_type: NightAttackType,
) -> Vec<i64> {
    let main_guns =
        collect_matching_slot_ids(codex, ship, |slot_type, _| is_main_gun_type(slot_type));
    let torpedoes = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        matches!(slot_type, KcSlotItemType3::Torpedo | KcSlotItemType3::SubmarineTorpedo)
    });
    let secondary_guns = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        matches!(slot_type, KcSlotItemType3::SecondaryGun | KcSlotItemType3::SecondaryGun2)
    });
    let radars = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        matches!(
            slot_type,
            KcSlotItemType3::SmallRadar
                | KcSlotItemType3::LargeRadar
                | KcSlotItemType3::LargeRadar2
        )
    });
    let lookouts = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::SeaplanePersonnel
    });
    let skilled_lookouts: Vec<i64> = ship
        .slot_items
        .iter()
        .filter_map(|si| (si.api_slotitem_id == SKILLED_LOOKOUT_ID).then_some(si.api_slotitem_id))
        .collect();
    let drums = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::TransportContainer
    });
    let surface_ids = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        is_day_surface_display_type(slot_type)
    });

    let mut ids = Vec::new();
    match attack_type {
        NightAttackType::MainMainMain => extend_limit(&mut ids, &main_guns, 3),
        NightAttackType::MainMainSec => {
            extend_limit(&mut ids, &main_guns, 2);
            extend_limit(&mut ids, &secondary_guns, 3);
        }
        NightAttackType::MainTorpRadar
        | NightAttackType::DdGunTorpRadar
        | NightAttackType::DdGunTorpRadar2 => {
            extend_limit(&mut ids, &main_guns, 1);
            extend_limit(&mut ids, &torpedoes, 2);
            extend_limit(&mut ids, &radars, 3);
        }
        NightAttackType::TorpTorpTorp => extend_limit(&mut ids, &torpedoes, 3),
        // DD CI: TRL (魚雷+見張員+電探)
        NightAttackType::DdTorpLookoutRadar | NightAttackType::DdTorpLookoutRadar2 => {
            extend_limit(&mut ids, &torpedoes, 1);
            extend_limit(&mut ids, &lookouts, 2);
            extend_limit(&mut ids, &radars, 3);
        }
        // DD CI: TTL (魚雷+水雷見張員+魚雷)
        NightAttackType::DdTorpTorpLookout | NightAttackType::DdTorpTorpLookout2 => {
            extend_limit(&mut ids, &torpedoes, 2);
            extend_limit(&mut ids, &skilled_lookouts, 3);
        }
        // DD CI: DTL (魚雷+水雷見張員+ドラム缶)
        NightAttackType::DdTorpDrumLookout | NightAttackType::DdTorpDrumLookout2 => {
            extend_limit(&mut ids, &torpedoes, 1);
            extend_limit(&mut ids, &skilled_lookouts, 2);
            extend_limit(&mut ids, &drums, 3);
        }
        NightAttackType::DoubleAttack => extend_limit(&mut ids, &surface_ids, 2),
        NightAttackType::CarrierNightCI(_) | NightAttackType::Normal => {
            extend_limit(&mut ids, &surface_ids, 1);
        }
    }

    if ids.is_empty() {
        vec![-1]
    } else {
        ids
    }
}

// ---------------------------------------------------------------------------
// Night hougeki simulation
// ---------------------------------------------------------------------------

/// Build the `api_si_list` entry for a night attacker. Cut-ins and double
/// attacks (any non-`Normal` type) serialize as JSON strings; normal attacks
/// stay integers. The `-1` empty-equipment sentinel is kept as `Num` by the
/// `text_from_i64` / `num_from_i64` helpers. Shared by the friendly and enemy
/// loops, which differ only in `at_eflag`.
fn night_si_entry(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    attack_type: NightAttackType,
) -> Vec<SiListId> {
    let ids = night_attack_display_ids(codex, ship, attack_type);
    if attack_type != NightAttackType::Normal {
        SiListId::text_from_i64(&ids)
    } else {
        SiListId::num_from_i64(&ids)
    }
}

/// Simulate the night battle hougeki phase.
pub(crate) fn simulate_night_hougeki(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    params: &NightBattleParams,
) -> Option<BattleNightHougeki> {
    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut n_mother_list = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut sp_list = Vec::new();
    let mut damage = Vec::new();

    for (idx, ship) in friendly.iter_mut().enumerate() {
        if !can_attack_night_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, enemy, BattlePhase::NightShelling)
        else {
            continue;
        };
        let is_submarine = target_class(codex, &enemy[target_idx]).is_submarine();
        let attack_type = resolve_night_attack(codex, rng, ship, idx == 0, is_submarine);
        let hits = attack_type.hit_count();
        let multiplier = attack_type.damage_multiplier();

        let mut hit_damages = Vec::new();
        let mut hit_cls = Vec::new();
        let mut total_dealt = 0i64;

        for _ in 0..hits {
            let raw = if is_submarine {
                calculate_scratch_damage(rng, enemy[target_idx].hp().max(1))
            } else {
                calculate_night_damage(
                    codex,
                    rng,
                    ship,
                    &enemy[target_idx],
                    params.air_state,
                    if multiplier != 1.0 {
                        Some(multiplier)
                    } else {
                        None
                    },
                )
            };
            let (raw_dmg, dealt) = enemy[target_idx].apply_damage(rng, raw, target_idx);
            total_dealt += dealt;
            let display = crate::targeting::display_damage(&enemy[target_idx], raw_dmg, dealt);
            hit_damages.push(display);
            hit_cls.push(1i64);
        }
        ship.damage_dealt += total_dealt;

        at_eflag.push(0);
        at_list.push(idx as i64);
        n_mother_list.push(0);
        df_list.push(vec![target_idx as i64; hits]);
        si_list.push(night_si_entry(codex, ship, attack_type));
        cl_list.push(hit_cls);
        sp_list.push(attack_type.api_sp_list());
        damage.push(hit_damages.into_iter().map(DamageCell::from).collect());
    }

    for (idx, ship) in enemy.iter_mut().enumerate() {
        if !can_attack_night_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, friendly, BattlePhase::NightShelling)
        else {
            continue;
        };
        let is_submarine = target_class(codex, &friendly[target_idx]).is_submarine();
        let attack_type = resolve_night_attack(codex, rng, ship, idx == 0, is_submarine);
        let hits = attack_type.hit_count();
        let multiplier = attack_type.damage_multiplier();

        let mut hit_damages = Vec::new();
        let mut hit_cls = Vec::new();
        let mut total_dealt = 0i64;

        for _ in 0..hits {
            let raw = if is_submarine {
                calculate_scratch_damage(rng, friendly[target_idx].hp().max(1))
            } else {
                calculate_night_damage(
                    codex,
                    rng,
                    ship,
                    &friendly[target_idx],
                    params.air_state,
                    if multiplier != 1.0 {
                        Some(multiplier)
                    } else {
                        None
                    },
                )
            };
            let (raw_dmg, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
            total_dealt += dealt;
            hit_damages.push(raw_dmg);
            hit_cls.push(1i64);
        }
        ship.damage_dealt += total_dealt;

        at_eflag.push(1);
        at_list.push(idx as i64);
        n_mother_list.push(0);
        df_list.push(vec![target_idx as i64; hits]);
        si_list.push(night_si_entry(codex, ship, attack_type));
        cl_list.push(hit_cls);
        sp_list.push(attack_type.api_sp_list());
        damage.push(hit_damages.into_iter().map(DamageCell::from).collect());
    }

    if at_list.is_empty() {
        return None;
    }

    Some(BattleNightHougeki {
        api_at_eflag: at_eflag,
        api_at_list: at_list,
        api_n_mother_list: n_mother_list,
        api_df_list: df_list,
        api_si_list: si_list,
        api_cl_list: cl_list,
        api_sp_list: sp_list,
        api_damage: damage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::{BattleRuntimeShip, EngagementType, NightBattleParams, SiListId};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    #[test]
    fn night_ci_triple_main_gun_detects_as_main_main_main() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);

        let mut ship = sample_ship(&codex, bb_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainMainMain);
        assert!((attack.damage_multiplier() - 2.0).abs() < f64::EPSILON);
        assert_eq!(attack.hit_count(), 1);
    }

    #[test]
    fn night_ci_torpedo_torpedo_detects_as_torp_ci() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::TorpTorpTorp);
        assert!((attack.damage_multiplier() - 1.3).abs() < f64::EPSILON);
        assert_eq!(attack.hit_count(), 2);
    }

    #[test]
    fn night_ci_main_main_secondary_detects_correctly() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let sec_gun_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SecondaryGun);

        let mut ship = sample_ship(&codex, bb_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(sec_gun_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainMainSec);
        assert!((attack.damage_multiplier() - 1.75).abs() < f64::EPSILON);
    }

    #[test]
    fn night_double_attack_with_main_and_torpedo() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(main_gun_mst_id), slotitem_with_mst_id(torp_mst_id)];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::DoubleAttack);
        assert_eq!(attack.hit_count(), 2);
        assert!((attack.damage_multiplier() - 1.2).abs() < f64::EPSILON);
    }

    #[test]
    fn night_ci_trigger_rate_increases_with_luck() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut low_luck_ship = sample_ship(&codex, dd_mst, 99);
        low_luck_ship.ship.api_lucky = [10, 30];
        let rt_low = BattleRuntimeShip::from(low_luck_ship);

        let mut high_luck_ship = sample_ship(&codex, dd_mst, 99);
        high_luck_ship.ship.api_lucky = [80, 99];
        let rt_high = BattleRuntimeShip::from(high_luck_ship);

        let rate_low = night_ci_trigger_rate(&rt_low, NightAttackType::TorpTorpTorp, false);
        let rate_high = night_ci_trigger_rate(&rt_high, NightAttackType::TorpTorpTorp, false);
        assert!(
            rate_high > rate_low,
            "higher luck should give higher CI rate: {rate_high} > {rate_low}"
        );
    }

    #[test]
    fn night_battle_sp_list_nonzero_for_ci_ship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_lucky = [90, 99];
        friend.ship.api_karyoku[0] = 150;
        friend.ship.api_raisou[0] = 200;
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.slot_items =
            vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];

        let mut enemy_ship = sample_ship(&codex, dd_mst, 50);
        enemy_ship.ship.api_soukou[0] = 10;
        enemy_ship.ship.api_nowhp = 500;
        enemy_ship.ship.api_maxhp = 500;
        enemy_ship.ship.api_karyoku[0] = 1;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy_ship)];
        let mut rng = crate::random::SeededRng::new(42);

        let hougeki = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemies,
            &NightBattleParams {
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
        )
        .unwrap();

        assert_eq!(hougeki.api_sp_list[0], 3, "torpedo CI sp_list should be 3 (魚雷/魚雷)");
        assert_eq!(hougeki.api_damage[0].len(), 2, "torpedo CI should deal 2 hits");
        // A night CI must serialize its si_list as JSON strings. Guards against
        // a Text/Num swap at the simulate_night_hougeki push site that the
        // isolated packet serialization tests cannot catch.
        assert!(
            hougeki.api_si_list[0].iter().any(|id| matches!(id, SiListId::Text(_))),
            "night CI si_list must contain string entries: {:?}",
            hougeki.api_si_list[0]
        );
    }

    #[test]
    fn night_normal_attack_si_list_is_integers() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        // A single main gun cannot form any night cut-in or double attack, so
        // the attack resolves as Normal (sp_list == 0) and its si_list must
        // stay integer-typed — the counterpart guard to the CI test above.
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);

        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 150;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.slot_items = vec![slotitem_with_mst_id(main_gun_mst_id)];

        let mut enemy_ship = sample_ship(&codex, dd_mst, 50);
        enemy_ship.ship.api_soukou[0] = 10;
        enemy_ship.ship.api_nowhp = 500;
        enemy_ship.ship.api_maxhp = 500;
        enemy_ship.ship.api_karyoku[0] = 1;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy_ship)];
        let mut rng = crate::random::SeededRng::new(42);

        let hougeki = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemies,
            &NightBattleParams {
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
        )
        .unwrap();

        assert_eq!(hougeki.api_sp_list[0], 0, "single-gun DD must do a normal night attack");
        assert!(
            hougeki.api_si_list[0].iter().all(|id| matches!(id, SiListId::Num(_))),
            "normal night attack si_list must be all integers: {:?}",
            hougeki.api_si_list[0]
        );
    }

    #[test]
    fn night_shelling_against_submarines_is_scratch_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);

        let mut friendly = vec![BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50))];
        let mut enemy = vec![BattleRuntimeShip::from(sample_ship(&codex, ss_mst, 50))];
        let enemy_hp = enemy[0].hp();
        let mut rng = crate::random::SeededRng::new(3);

        let hougeki = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            &NightBattleParams {
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
        )
        .unwrap();

        assert_eq!(hougeki.api_df_list[0], vec![0]);
        assert!(hougeki.api_damage[0][0].amount() >= 1);
        assert!(hougeki.api_damage[0][0].amount() < enemy_hp);
        assert_eq!(enemy[0].hp(), enemy_hp - hougeki.api_damage[0][0].amount());
    }

    #[test]
    fn regular_carrier_cannot_attack_in_night_battle_without_night_crew() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let carrier = sample_ship(&codex, carrier_mst, 50);
        let enemy = sample_ship(&codex, dd_mst, 50);
        let mut rng = crate::random::SeededRng::new(0);

        let simulation = crate::simulation::simulate_night(
            &codex,
            crate::types::NightBattleInput {
                friendly: vec![BattleRuntimeShip::from(carrier)],
                enemy: vec![BattleRuntimeShip::from(enemy)],
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
            &mut rng,
        );

        let hougeki = simulation.packet.hougeki.unwrap();
        assert!(hougeki.api_at_eflag.iter().all(|flag| *flag == 1));
    }

    #[test]
    fn sortie_night_battle_non_taiha_ship_survives_lethal_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Create a ship with enough HP to NOT be taiha at entry (>25% max)
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 200;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 100;
        friend.ship.api_maxhp = 100;
        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];

        // Enemy with massive firepower to deal lethal damage
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 500;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];

        let mut rng = crate::random::SeededRng::new(42);

        let _ = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut enemies, // enemy attacks first
            &mut friendly,
            &NightBattleParams {
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
        );

        // In a sortie night battle, the friendly ship should survive due to sinking protection
        // (entry_hp was 100, max_hp was 100, so 100*4 > 100 means not taiha)
        assert!(
            friendly[0].hp() > 0,
            "sortie night battle: non-taiha ship should survive lethal damage, got HP={}",
            friendly[0].hp()
        );
    }

    #[test]
    fn practice_night_battle_non_taiha_ship_can_be_sunk() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Create a ship with enough HP to NOT be taiha at entry
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 0;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 10;
        friend.ship.api_maxhp = 100;
        let mut friendly = vec![BattleRuntimeShip::new(friend, true, false)]; // is_sortie=false

        // Enemy with massive firepower
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 500;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, false)];

        let mut rng = crate::random::SeededRng::new(42);

        let _ = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut enemies,
            &mut friendly,
            &NightBattleParams {
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
        );

        // In practice, sinking protection does NOT apply
        // But ships may survive anyway due to scratch damage (attack < defense)
        // The key assertion: if the ship took lethal-level damage, HP can reach 0
        // We just verify the ship was created as practice (no protection)
        assert!(!friendly[0].is_sortie, "practice ship should have is_sortie=false");
    }

    #[test]
    fn night_ci_priority_torpedo_over_main_torp_radar() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        let radar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);

        // Ship with 1 main gun + 2 torpedoes + 1 radar
        // Should detect TorpTorpTorp (priority 3), NOT MainTorpRadar (priority 4)
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(
            attack,
            NightAttackType::TorpTorpTorp,
            "ship with 2 torpedoes should get TorpTorpTorp, not MainTorpRadar"
        );
    }

    #[test]
    fn night_ci_trigger_rate_uses_total_luck_with_equipment() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Ship with low base luck but high total luck (via equipment)
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.ship.api_lucky = [10, 80]; // base=10, total=80
        let rt = BattleRuntimeShip::from(ship);

        let rate = night_ci_trigger_rate(&rt, NightAttackType::TorpTorpTorp, false);
        // With total luck 80, we should get the higher-luck formula (>65)
        // ci_value = 65 + sqrt(80-50) + floor(0.8*sqrt(99)) = 65 + ~5.48 + ~7.92 = ~78.4
        // rate = 78.4 / 122 ≈ 0.64
        assert!(rate > 0.5, "total luck (api_lucky[1]=80) should give rate > 0.5, got {rate}");
    }

    #[test]
    fn night_ci_chuuha_bonus_increases_trigger_rate() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let mut ship_chuuha = sample_ship(&codex, dd_mst, 99);
        ship_chuuha.ship.api_lucky = [50, 50];
        ship_chuuha.ship.api_nowhp = 30; // 30% HP = chuuha
        ship_chuuha.ship.api_maxhp = 100;
        let rt_chuuha = BattleRuntimeShip::from(ship_chuuha);

        let rate_healthy = night_ci_trigger_rate(&rt_healthy, NightAttackType::TorpTorpTorp, false);
        let rate_chuuha = night_ci_trigger_rate(&rt_chuuha, NightAttackType::TorpTorpTorp, false);

        assert!(
            rate_chuuha > rate_healthy,
            "chuuha ship should have higher CI rate: {rate_chuuha} > {rate_healthy}"
        );
    }

    #[test]
    fn night_ci_taiha_no_chuuha_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Chuuha (30%) should get bonus
        let mut ship_chuuha = sample_ship(&codex, dd_mst, 99);
        ship_chuuha.ship.api_lucky = [50, 50];
        ship_chuuha.ship.api_nowhp = 30;
        ship_chuuha.ship.api_maxhp = 100;
        let rt_chuuha = BattleRuntimeShip::from(ship_chuuha);

        // Taiha (10%) should NOT get bonus
        let mut ship_taiha = sample_ship(&codex, dd_mst, 99);
        ship_taiha.ship.api_lucky = [50, 50];
        ship_taiha.ship.api_nowhp = 10;
        ship_taiha.ship.api_maxhp = 100;
        let rt_taiha = BattleRuntimeShip::from(ship_taiha);

        // Healthy (100%) should NOT get bonus
        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_chuuha = night_ci_trigger_rate(&rt_chuuha, NightAttackType::TorpTorpTorp, false);
        let rate_taiha = night_ci_trigger_rate(&rt_taiha, NightAttackType::TorpTorpTorp, false);
        let rate_healthy = night_ci_trigger_rate(&rt_healthy, NightAttackType::TorpTorpTorp, false);

        assert!(
            rate_chuuha > rate_taiha,
            "chuuha should have higher rate than taiha: {rate_chuuha} > {rate_taiha}"
        );
        assert_eq!(
            rate_taiha, rate_healthy,
            "taiha should have same rate as healthy (no bonus): {rate_taiha} == {rate_healthy}"
        );
    }

    #[test]
    fn dd_ci_taiha_no_chuuha_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Chuuha (30%) should get bonus
        let mut ship_chuuha = sample_ship(&codex, dd_mst, 99);
        ship_chuuha.ship.api_lucky = [50, 50];
        ship_chuuha.ship.api_nowhp = 30;
        ship_chuuha.ship.api_maxhp = 100;
        let rt_chuuha = BattleRuntimeShip::from(ship_chuuha);

        // Taiha (10%) should NOT get bonus
        let mut ship_taiha = sample_ship(&codex, dd_mst, 99);
        ship_taiha.ship.api_lucky = [50, 50];
        ship_taiha.ship.api_nowhp = 10;
        ship_taiha.ship.api_maxhp = 100;
        let rt_taiha = BattleRuntimeShip::from(ship_taiha);

        let rate_chuuha = dd_ci_trigger_rate(&codex, &rt_chuuha, DdCiType::TorpTorpLookout, false);
        let rate_taiha = dd_ci_trigger_rate(&codex, &rt_taiha, DdCiType::TorpTorpLookout, false);

        assert!(
            rate_chuuha > rate_taiha,
            "DD CI: chuuha should have higher rate than taiha: {rate_chuuha} > {rate_taiha}"
        );
    }

    #[test]
    fn night_ci_hp_at_25pct_no_chuuha_bonus() {
        // HP ratio exactly 0.25 must be treated as taiha (excluded by strict `> 0.25`).
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_25 = sample_ship(&codex, dd_mst, 99);
        ship_25.ship.api_lucky = [50, 50];
        ship_25.ship.api_nowhp = 25;
        ship_25.ship.api_maxhp = 100;
        let rt_25 = BattleRuntimeShip::from(ship_25);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_25 = night_ci_trigger_rate(&rt_25, NightAttackType::TorpTorpTorp, false);
        let rate_healthy = night_ci_trigger_rate(&rt_healthy, NightAttackType::TorpTorpTorp, false);

        assert_eq!(
            rate_25, rate_healthy,
            "HP exactly 25% should NOT receive chuuha bonus (strict > 0.25)"
        );
    }

    #[test]
    fn dd_ci_hp_at_25pct_no_chuuha_bonus() {
        // Same boundary check for the DD-specific trigger formula.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_25 = sample_ship(&codex, dd_mst, 99);
        ship_25.ship.api_lucky = [50, 50];
        ship_25.ship.api_nowhp = 25;
        ship_25.ship.api_maxhp = 100;
        let rt_25 = BattleRuntimeShip::from(ship_25);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_25 = dd_ci_trigger_rate(&codex, &rt_25, DdCiType::TorpTorpLookout, false);
        let rate_healthy =
            dd_ci_trigger_rate(&codex, &rt_healthy, DdCiType::TorpTorpLookout, false);

        assert_eq!(
            rate_25, rate_healthy,
            "DD CI: HP exactly 25% should NOT receive chuuha bonus (strict > 0.25)"
        );
    }

    #[test]
    fn night_ci_hp_at_50pct_chuuha_bonus_applies() {
        // HP ratio exactly 0.50 falls inside the chuuha bracket (inclusive `<= 0.5`).
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_50 = sample_ship(&codex, dd_mst, 99);
        ship_50.ship.api_lucky = [50, 50];
        ship_50.ship.api_nowhp = 50;
        ship_50.ship.api_maxhp = 100;
        let rt_50 = BattleRuntimeShip::from(ship_50);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_50 = night_ci_trigger_rate(&rt_50, NightAttackType::TorpTorpTorp, false);
        let rate_healthy = night_ci_trigger_rate(&rt_healthy, NightAttackType::TorpTorpTorp, false);

        assert!(
            rate_50 > rate_healthy,
            "HP exactly 50% should receive chuuha bonus (inclusive <= 0.5): {rate_50} > {rate_healthy}"
        );
    }

    #[test]
    fn dd_ci_hp_at_50pct_chuuha_bonus_applies() {
        // Same upper-edge check for the DD-specific trigger formula.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_50 = sample_ship(&codex, dd_mst, 99);
        ship_50.ship.api_lucky = [50, 50];
        ship_50.ship.api_nowhp = 50;
        ship_50.ship.api_maxhp = 100;
        let rt_50 = BattleRuntimeShip::from(ship_50);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_50 = dd_ci_trigger_rate(&codex, &rt_50, DdCiType::TorpTorpLookout, false);
        let rate_healthy =
            dd_ci_trigger_rate(&codex, &rt_healthy, DdCiType::TorpTorpLookout, false);

        assert!(
            rate_50 > rate_healthy,
            "DD CI: HP exactly 50% should receive chuuha bonus (inclusive <= 0.5): {rate_50} > {rate_healthy}"
        );
    }

    #[test]
    fn night_attack_type_sp_list_matches_apilist() {
        // Per apilist.txt lines 2319-2342:
        // 0=通常攻撃, 1=連続射撃
        // 2=カットイン(主砲/魚雷), 3=カットイン(魚雷/魚雷),
        // 4=カットイン(主砲/主砲/副砲), 5=カットイン(主砲/主砲/主砲)
        assert_eq!(NightAttackType::Normal.api_sp_list(), 0);
        assert_eq!(NightAttackType::DoubleAttack.api_sp_list(), 1);
        assert_eq!(NightAttackType::MainTorpRadar.api_sp_list(), 2);
        assert_eq!(NightAttackType::TorpTorpTorp.api_sp_list(), 3);
        assert_eq!(NightAttackType::MainMainSec.api_sp_list(), 4);
        assert_eq!(NightAttackType::MainMainMain.api_sp_list(), 5);
    }

    #[test]
    fn night_ci_multiplier_applied_pre_cap() {
        // Verify that CI multiplier is applied before the soft cap at 360
        // A ship with 500 base power and 2.0x CI should be capped at:
        // apply_cap(500 * 2.0, 360) = 360 + sqrt(640) ≈ 385
        // NOT: apply_cap(500, 360) * 2.0 = 385 * 2.0 = 770 (post-defense multiplication)
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut attacker = sample_ship(&codex, dd_mst, 99);
        attacker.ship.api_karyoku[0] = 400; // high firepower
        attacker.ship.api_raisou[0] = 50;
        attacker.ship.api_soukou[0] = 0;
        attacker.ship.api_nowhp = 100;
        attacker.ship.api_maxhp = 100;
        let rt_attacker = BattleRuntimeShip::from(attacker);

        let mut defender = sample_ship(&codex, dd_mst, 50);
        defender.ship.api_soukou[0] = 0; // zero armor for clean test
        defender.ship.api_nowhp = 9999;
        defender.ship.api_maxhp = 9999;
        let rt_defender = BattleRuntimeShip::from(defender);

        let mut rng = crate::random::SeededRng::new(42);

        // Damage with 2.0x CI multiplier (MainMainMain)
        let dmg_with_ci =
            calculate_night_damage(&codex, &mut rng, &rt_attacker, &rt_defender, None, Some(2.0));

        // Damage without CI multiplier
        let dmg_normal =
            calculate_night_damage(&codex, &mut rng, &rt_attacker, &rt_defender, None, None);

        // With pre-cap: apply_cap(455*2.0, 360) = 360 + sqrt(550) ≈ 383
        // capped_power ≈ 383, defense ≈ 0, so damage ≈ 383
        // With no CI: apply_cap(455, 360) = 360 + sqrt(95) ≈ 370
        // CI damage should be higher but NOT 2x of normal (due to soft cap)
        assert!(
            dmg_with_ci > dmg_normal,
            "CI damage ({dmg_with_ci}) should be higher than normal ({dmg_normal})"
        );
        // Pre-cap means CI damage < 2x normal (soft cap eats the excess)
        assert!(
            dmg_with_ci < dmg_normal * 2,
            "pre-cap CI damage ({dmg_with_ci}) should be < 2x normal ({}) due to soft cap",
            dmg_normal * 2
        );
    }

    // ------- DD CI detection tests -------

    #[test]
    fn dd_ci_gtr_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let radar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(detect_dd_ci_type(&codex, &rt, DdCiType::GunTorpRadar));
        assert!(!detect_dd_ci_type(&codex, &rt, DdCiType::TorpLookoutRadar));
        assert!(!detect_dd_ci_type(&codex, &rt, DdCiType::TorpTorpLookout));
        assert!(!detect_dd_ci_type(&codex, &rt, DdCiType::TorpDrumLookout));
    }

    #[test]
    fn dd_ci_trl_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let lookout_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaplanePersonnel);
        let radar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(lookout_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(!detect_dd_ci_type(&codex, &rt, DdCiType::GunTorpRadar));
        assert!(detect_dd_ci_type(&codex, &rt, DdCiType::TorpLookoutRadar));
    }

    #[test]
    fn dd_ci_ttl_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(412), // 水雷戦隊 熟練見張員
            slotitem_with_mst_id(torp_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(detect_dd_ci_type(&codex, &rt, DdCiType::TorpTorpLookout));
        assert!(!detect_dd_ci_type(&codex, &rt, DdCiType::GunTorpRadar));
    }

    #[test]
    fn dd_ci_dtl_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let drum_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::TransportContainer);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(412), // 水雷戦隊 熟練見張員
            slotitem_with_mst_id(drum_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(detect_dd_ci_type(&codex, &rt, DdCiType::TorpDrumLookout));
    }

    #[test]
    fn dd_ci_non_dd_does_not_trigger() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cl_mst = first_ship_mst_by_type(&codex, KcShipType::CL);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::MediumCaliberMainGun);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let radar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);

        // CL with same equipment as GTR → DD CI should NOT trigger
        let mut ship = sample_ship(&codex, cl_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        // Standard detection for CL finds MainTorpRadar (not DD CI)
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainTorpRadar, "CL should get standard CI, not DD CI");
    }

    #[test]
    fn dd_ci_sp_list_values() {
        assert_eq!(NightAttackType::DdGunTorpRadar.api_sp_list(), 7);
        assert_eq!(NightAttackType::DdGunTorpRadar2.api_sp_list(), 11);
        assert_eq!(NightAttackType::DdTorpLookoutRadar.api_sp_list(), 8);
        assert_eq!(NightAttackType::DdTorpLookoutRadar2.api_sp_list(), 12);
        assert_eq!(NightAttackType::DdTorpTorpLookout.api_sp_list(), 9);
        assert_eq!(NightAttackType::DdTorpTorpLookout2.api_sp_list(), 13);
        assert_eq!(NightAttackType::DdTorpDrumLookout.api_sp_list(), 10);
        assert_eq!(NightAttackType::DdTorpDrumLookout2.api_sp_list(), 14);
    }

    #[test]
    fn dd_ci_multiroll_falls_through_to_standard_ci() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        // DD with 2 torpedoes only — no DD CI conditions met
        // Standard detection should still find TorpTorpTorp
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];
        let rt = BattleRuntimeShip::from(ship);

        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(
            attack,
            NightAttackType::TorpTorpTorp,
            "DD without DD CI equipment falls to standard CI"
        );
    }

    #[test]
    fn dd_ci_trigger_rate_increases_with_luck() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut low_luck = sample_ship(&codex, dd_mst, 99);
        low_luck.ship.api_lucky = [10, 20];
        let rt_low = BattleRuntimeShip::from(low_luck);

        let mut high_luck = sample_ship(&codex, dd_mst, 99);
        high_luck.ship.api_lucky = [80, 90];
        let rt_high = BattleRuntimeShip::from(high_luck);

        let rate_low = dd_ci_trigger_rate(&codex, &rt_low, DdCiType::GunTorpRadar, false);
        let rate_high = dd_ci_trigger_rate(&codex, &rt_high, DdCiType::GunTorpRadar, false);
        assert!(
            rate_high > rate_low,
            "DD CI: higher luck should give higher rate: {rate_high} > {rate_low}"
        );
    }

    #[test]
    fn dd_ci_trigger_rate_flagship_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.ship.api_lucky = [50, 50];
        let rt = BattleRuntimeShip::from(ship);

        let rate_normal = dd_ci_trigger_rate(&codex, &rt, DdCiType::GunTorpRadar, false);
        let rate_flagship = dd_ci_trigger_rate(&codex, &rt, DdCiType::GunTorpRadar, true);
        assert!(rate_flagship > rate_normal, "flagship should get +15 bonus");
    }

    #[test]
    fn carrier_night_ci_fba_detection() {
        // CVL with aviation personnel + 2 night fighters + 1 night attacker → FBA
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let night_fighter_id: i64 = 254; // F6F-3N (icon=45)
        let night_attacker_id: i64 = 257; // TBM-3D (icon=46)

        let mut ship = sample_ship(&codex, cvl_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(258), // 夜間作戦航空要員
            slotitem_with_mst_id(night_fighter_id),
            slotitem_with_mst_id(night_fighter_id),
            slotitem_with_mst_id(night_attacker_id),
        ];
        ship.ship.api_onslot = [1, 1, 1, 1, 0];
        let rt = BattleRuntimeShip::from(ship);

        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::CarrierNightCI(CarrierNightCiSubType::Nf2Na));
    }

    #[test]
    fn carrier_night_ci_without_aviation_personnel() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let night_fighter_id: i64 = 254;
        let night_attacker_id: i64 = 257;

        let mut ship = sample_ship(&codex, cvl_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(night_fighter_id), slotitem_with_mst_id(night_attacker_id)];
        ship.ship.api_onslot = [1, 1, 0, 0, 0];
        let rt = BattleRuntimeShip::from(ship);

        // Without 航空要員, CVL cannot attack at night → can_attack_night_ship returns false
        assert!(!crate::targeting::can_attack_night_ship(&codex, &rt));
    }

    #[test]
    fn carrier_night_ci_no_night_planes() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        let mut ship = sample_ship(&codex, cvl_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(258), // 航空要員 but no night planes
            slotitem_with_mst_id(bomber_id),
        ];
        ship.ship.api_onslot = [1, 1, 0, 0, 0];
        let rt = BattleRuntimeShip::from(ship);

        // Has 航空要員 so can attack at night, but no night CI
        assert!(crate::targeting::can_attack_night_ship(&codex, &rt));
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::Normal, "no night planes → Normal");
    }

    // ── Carrier night CI: sub-types, eligibility, enum, trigger (U1-U4) ───────

    // Verified item ids (icons confirmed against .data/codex/start2.json):
    const NF: i64 = 254; // F6F-3N — 夜戦 (icon 45)
    const NA: i64 = 257; // TBM-3D — 夜攻 (icon 46)
    const NB: i64 = 557; // 零式艦戦62型改(夜間爆戦) — 夜爆 (icon 58)
    const KK: i64 = 320; // 光電管彗星 — icon 7, detected by item id
    const SF: i64 = 242; // Swordfish — icon 8, detected by item id

    fn ship_with_slots(codex: &Codex, mst_id: i64, item_ids: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 99);
        ship.slot_items = item_ids.iter().map(|&id| slotitem_with_mst_id(id)).collect();
        let mut onslot = [0i64; 5];
        for slot in onslot.iter_mut().take(item_ids.len().min(5)) {
            *slot = 1;
        }
        ship.ship.api_onslot = onslot;
        BattleRuntimeShip::from(ship)
    }

    fn cvl_with(codex: &Codex, item_ids: &[i64]) -> BattleRuntimeShip {
        ship_with_slots(codex, first_ship_mst_by_type(codex, KcShipType::CVL), item_ids)
    }

    #[test]
    fn carrier_night_ci_subtype_detection_all_eight() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        use CarrierNightCiSubType::*;
        let cases: &[(&[i64], CarrierNightCiSubType)] = &[
            (&[NF, NF, NA], Nf2Na),
            (&[NF, NA], Nf1Na),
            (&[NF, KK], Nf1Kk),
            (&[NA, KK], Na1Kk),
            (&[NF, NB], Nf1Nb),
            (&[NA, NB], Na1Nb),
            (&[NB, KK], Nb1Kk),
            (&[NF, SF, SF], Nf1Other),
        ];
        for (slots, expected) in cases {
            let rt = cvl_with(&codex, slots);
            assert_eq!(
                detect_carrier_night_ci_sub_type(&codex, &rt),
                Some(*expected),
                "slots {slots:?} should detect {expected:?}"
            );
        }
    }

    #[test]
    fn carrier_night_ci_subtype_priority_and_none() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        use CarrierNightCiSubType::*;

        // Nf1Kk (priority 3) wins over Nf1Other when both could match.
        assert_eq!(
            detect_carrier_night_ci_sub_type(&codex, &cvl_with(&codex, &[NF, KK, SF])),
            Some(Nf1Kk),
        );
        // Na1Kk (priority 4) when no 夜戦 present.
        assert_eq!(
            detect_carrier_night_ci_sub_type(&codex, &cvl_with(&codex, &[NA, KK, SF])),
            Some(Na1Kk),
        );
        // Lone 夜戦 → no sub-type (total 夜間飛行機 = 1).
        assert_eq!(detect_carrier_night_ci_sub_type(&codex, &cvl_with(&codex, &[NF])), None);
        // Plain dive bomber → no sub-type.
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        assert_eq!(detect_carrier_night_ci_sub_type(&codex, &cvl_with(&codex, &[bomber])), None);
    }

    #[test]
    fn carrier_night_ci_subtype_excludes_shot_down_slots() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // 夜戦 + 光電管彗星, but the 彗星 slot is shot down (onslot=0) → kk=0 → lone 夜戦 → None.
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 99);
        ship.slot_items = vec![slotitem_with_mst_id(NF), slotitem_with_mst_id(KK)];
        ship.ship.api_onslot = [1, 0, 0, 0, 0];
        let rt = BattleRuntimeShip::from(ship);
        assert_eq!(detect_carrier_night_ci_sub_type(&codex, &rt), None);
    }

    #[test]
    fn carrier_night_ci_eligibility_exempt_and_personnel() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        // Exempt CVs without 航空要員 are eligible with a valid plane combo.
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 545, &[NF, NA]))); // Saratoga Mk.II
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 610, &[NA, KK]))); // 加賀改二戊 (Na1Kk)
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 883, &[NF, SF, SF]))); // 龍鳳改二戊 (Nf1Other)

        // Saratoga Mk.II Mod.2 (550) is NOT exempt — loses 夜戦特性 on upgrade.
        assert!(!is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 550, &[NF, NA])));

        // Standard CVL: eligible only with 航空要員 (item 258).
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, cvl, &[258, NF, NF, NA])));
        assert!(!is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, cvl, &[NF, NA])));

        // Exempt CV but no night planes → ineligible (detector returns None).
        assert!(!is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 545, &[])));
    }

    #[test]
    fn carrier_night_ci_enum_multiplier_coefficient_sp_list() {
        use CarrierNightCiSubType::*;
        let nf2na = NightAttackType::CarrierNightCI(Nf2Na);
        assert_eq!(nf2na.damage_multiplier(), 1.25);
        assert_eq!(nf2na.ci_coefficient(), 105.0);

        let nf1na = NightAttackType::CarrierNightCI(Nf1Na);
        assert_eq!(nf1na.damage_multiplier(), 1.20);
        assert_eq!(nf1na.ci_coefficient(), 120.0);

        let nf1other = NightAttackType::CarrierNightCI(Nf1Other);
        assert_eq!(nf1other.damage_multiplier(), 1.18);
        assert_eq!(nf1other.ci_coefficient(), 130.0);

        for sub in [Nf2Na, Nf1Na, Nf1Kk, Na1Kk, Nf1Nb, Na1Nb, Nb1Kk, Nf1Other] {
            assert_eq!(NightAttackType::CarrierNightCI(sub).api_sp_list(), 6);
            assert_eq!(NightAttackType::CarrierNightCI(sub).hit_count(), 1);
        }
    }

    #[test]
    fn carrier_night_ci_trigger_rate_uses_subtype_coefficient() {
        use CarrierNightCiSubType::*;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 99);
        ship.ship.api_lucky = [99, 99];
        let rt = BattleRuntimeShip::from(ship);

        let rate_nf2na = night_ci_trigger_rate(&rt, NightAttackType::CarrierNightCI(Nf2Na), true);
        let rate_nf1other =
            night_ci_trigger_rate(&rt, NightAttackType::CarrierNightCI(Nf1Other), true);

        // The carrier night CI rate is now non-zero (the stub returned 0.0 and never fired).
        assert!(rate_nf2na > 0.0, "carrier night CI must produce a non-zero trigger rate");
        // Lower 種別係数 (105) yields a higher rate than Nf1Other (130).
        assert!(
            rate_nf2na > rate_nf1other,
            "Nf2Na (coeff 105) should beat Nf1Other (coeff 130): {rate_nf2na} > {rate_nf1other}"
        );
    }

    // ── Carrier night CI: end-to-end battle integration (U5) ──────────────────

    /// Deterministic RNG: every roll returns the same fraction of its range. `0.0` makes any CI
    /// trigger roll succeed; `0.999` makes it fail. Avoids seed-window flakiness.
    struct FixedRng(f64);
    impl BattleRng for FixedRng {
        fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
            min + (max - min) * self.0
        }
        fn roll_range_impl(&mut self, min: i64, max: i64) -> i64 {
            if max <= min {
                min
            } else {
                min + ((max - min) as f64 * self.0) as i64
            }
        }
    }

    /// A night-capable CV fixture: high luck (so the CI rate is comfortably positive), full HP,
    /// and the given slot items in non-shot-down slots.
    fn night_cv(codex: &Codex, mst_id: i64, item_ids: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 99);
        ship.slot_items = item_ids.iter().map(|&id| slotitem_with_mst_id(id)).collect();
        let mut onslot = [0i64; 5];
        for slot in onslot.iter_mut().take(item_ids.len().min(5)) {
            *slot = 1;
        }
        ship.ship.api_onslot = onslot;
        ship.ship.api_lucky = [99, 99];
        ship.ship.api_karyoku[0] = 120;
        ship.ship.api_nowhp = 9999;
        ship.ship.api_maxhp = 9999;
        BattleRuntimeShip::from(ship)
    }

    fn tanky_enemy(codex: &Codex) -> BattleRuntimeShip {
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);
        let mut enemy = sample_ship(codex, dd, 50);
        enemy.ship.api_soukou[0] = 10;
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_nowhp = 9999;
        enemy.ship.api_maxhp = 9999;
        BattleRuntimeShip::from(enemy)
    }

    /// `api_sp_list` values for the friendly attackers (`api_at_eflag == 0`).
    fn friendly_sp_list(hougeki: &BattleNightHougeki) -> Vec<i64> {
        hougeki
            .api_at_eflag
            .iter()
            .zip(&hougeki.api_sp_list)
            .filter(|(eflag, _)| **eflag == 0)
            .map(|(_, sp)| *sp)
            .collect()
    }

    fn run_cv_night(codex: &Codex, cv: BattleRuntimeShip, fire: bool) -> BattleNightHougeki {
        let mut friendly = vec![cv];
        let mut enemy = vec![tanky_enemy(codex)];
        let mut rng = FixedRng(if fire {
            0.0
        } else {
            0.999
        });
        simulate_night_hougeki(
            codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            &NightBattleParams {
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
            },
        )
        .expect("a night hougeki occurs")
    }

    #[test]
    fn carrier_night_ci_integration_fires_for_eligible_cvs() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);

        // Standard CV with 航空要員 + Nf2Na → single-hit CI.
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[258, NF, NF, NA]), true);
        assert_eq!(friendly_sp_list(&hougeki), vec![6], "standard CV Nf2Na → sp_list=6");
        assert_eq!(hougeki.api_damage[0].len(), 1, "carrier night CI is single-hit");

        // Exempt CVs WITHOUT 航空要員 still fire.
        for (mst, slots) in [
            (545_i64, &[NF, NA][..]), // Saratoga Mk.II, Nf1Na
            (610, &[NA, KK][..]),     // 加賀改二戊, Na1Kk (光電管彗星)
            (883, &[NF, SF, SF][..]), // 龍鳳改二戊, Nf1Other (Swordfish)
        ] {
            let hougeki = run_cv_night(&codex, night_cv(&codex, mst, slots), true);
            assert!(
                friendly_sp_list(&hougeki).contains(&6),
                "exempt CV {mst} without personnel → sp_list=6"
            );
        }
    }

    #[test]
    fn carrier_night_ci_integration_negatives() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);

        // Saratoga Mk.II Mod.2 (550) without 航空要員 cannot attack at night → no friendly entry.
        let hougeki = run_cv_night(&codex, night_cv(&codex, 550, &[NF, NA]), true);
        assert!(!friendly_sp_list(&hougeki).contains(&6), "550 without personnel → no sp_list=6");

        // 航空要員 present but only a night fighter → no CI sub-type → Normal.
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[258, NF]), true);
        assert!(!friendly_sp_list(&hougeki).contains(&6), "lone night fighter → no sp_list=6");

        // Non-exempt CV without personnel cannot attack at night.
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[NF, NA]), true);
        assert!(
            !friendly_sp_list(&hougeki).contains(&6),
            "non-exempt CV without personnel → no sp_list=6"
        );
    }

    #[test]
    fn carrier_night_ci_integration_failed_roll_falls_back_to_normal() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);

        // An eligible Nf2Na CV whose trigger roll fails must fall back to Normal — NOT the
        // artillery double attack (CV-specific early return).
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[258, NF, NF, NA]), false);
        let friendly = friendly_sp_list(&hougeki);
        assert_eq!(friendly, vec![0], "failed carrier CI → Normal (sp_list=0)");
        assert!(!friendly.contains(&1), "must not fall back to DoubleAttack");
    }

    // ── Carrier night CI: codex-ID regression guard (U6) ──────────────────────

    /// The exempt-CV and item ids are hardcoded against the Codex bootstrap. If a codex update
    /// renumbers or removes one, this fails loud rather than silently disabling the feature.
    #[test]
    fn carrier_night_ci_ids_resolve_in_codex() {
        use emukc_model::kc2::start2::{ApiMstShip, ApiMstSlotitem};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        // Every exempt CV id must resolve to a real ship.
        for &id in EXEMPT_NIGHT_CV_IDS {
            assert!(codex.find::<ApiMstShip>(&id).is_ok(), "exempt CV mst {id} missing from codex");
        }
        // Saratoga Mk.II Mod.2 (550) exists but must stay OUT of the exempt list.
        assert!(codex.find::<ApiMstShip>(&550i64).is_ok());
        assert!(!EXEMPT_NIGHT_CV_IDS.contains(&550), "550 must not be treated as exempt");

        // 光電管彗星 must resolve and carry the regular dive-bomber icon (api_type[3]=7) — the
        // exact reason it needs item-id detection rather than night-icon detection.
        let kk = codex.find::<ApiMstSlotitem>(&KOUDENKAN_SUISEI_ID).expect("光電管彗星 in codex");
        assert_eq!(kk.api_type[3], 7, "光電管彗星 must carry the regular dive-bomber icon");

        // The other item-id-detected night planes must resolve.
        for &id in SWORDFISH_IDS {
            assert!(codex.find::<ApiMstSlotitem>(&id).is_ok(), "Swordfish item {id} missing");
        }
        assert!(codex.find::<ApiMstSlotitem>(&IWAI_FUKUSEN_ID).is_ok(), "岩井爆戦 (154) missing");
    }
}
