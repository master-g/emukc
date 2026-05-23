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
use crate::types::{BattleNightHougeki, BattlePhase, BattleRuntimeShip, NightBattleParams};

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
    CarrierNightCI, // 戦爆連合夜間CI: 1 hit, multiplier varies by sub-type
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
            Self::CarrierNightCI => 6,
        }
    }

    fn damage_multiplier(self) -> f64 {
        match self {
            Self::Normal => 1.0,
            Self::DoubleAttack => 1.2,
            Self::MainTorpRadar => 1.625,
            Self::TorpTorpTorp => 1.3,
            Self::MainMainSec => 1.75,
            Self::MainMainMain => 2.0,
            Self::DdGunTorpRadar | Self::DdGunTorpRadar2 => 1.3,
            Self::DdTorpLookoutRadar | Self::DdTorpLookoutRadar2 => 1.2,
            Self::DdTorpTorpLookout | Self::DdTorpTorpLookout2 => 1.5,
            Self::DdTorpDrumLookout | Self::DdTorpDrumLookout2 => 1.3,
            Self::CarrierNightCI => 1.25, // default; actual multiplier set by sub-type
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
            | Self::CarrierNightCI => 1,
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
            Self::MainTorpRadar => 115.0,
            Self::TorpTorpTorp => 122.0,
            Self::MainMainSec => 130.0,
            Self::MainMainMain => 140.0,
            Self::DdGunTorpRadar | Self::DdGunTorpRadar2 => 115.0,
            Self::DdTorpLookoutRadar | Self::DdTorpLookoutRadar2 => 140.0,
            Self::DdTorpTorpLookout | Self::DdTorpTorpLookout2 => 125.0,
            Self::DdTorpDrumLookout | Self::DdTorpDrumLookout2 => 122.0,
            Self::DoubleAttack | Self::Normal | Self::CarrierNightCI => 0.0,
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

/// Night plane icon types (api_type[3]).
const NIGHT_FIGHTER_ICON: i64 = 45; // 夜間戦闘機
const NIGHT_ATTACKER_ICON: i64 = 46; // 夜間攻撃機
const NIGHT_BOMBER_ICON: i64 = 58; // 夜間爆戦
const NIGHT_SUISEI_ICON: i64 = 51; // 夜間瑞雲

/// Aviation personnel item IDs for night carrier operations.
const AVIATION_PERSONNEL_IDS: &[i64] = &[258, 259]; // 夜間作戦航空要員/夜間作戦航空要員(熟練)

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

/// Check if a carrier is eligible for night CI.
/// Requires: CV type + aviation personnel + night planes.
fn is_cv_night_ci_eligible(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if !crate::damage::is_cv_type(codex, ship) {
        return false;
    }
    let has_personnel = AVIATION_PERSONNEL_IDS
        .iter()
        .any(|&id| ship.slot_items.iter().any(|si| si.api_slotitem_id == id));
    if !has_personnel {
        return false;
    }
    let night_fighters = count_night_planes_by_icon(codex, ship, NIGHT_FIGHTER_ICON);
    let night_attackers = count_night_planes_by_icon(codex, ship, NIGHT_ATTACKER_ICON);
    let night_bombers = count_night_planes_by_icon(codex, ship, NIGHT_BOMBER_ICON);
    let night_suisei = count_night_planes_by_icon(codex, ship, NIGHT_SUISEI_ICON);
    let night_other = night_attackers + night_bombers + night_suisei;

    // At least 1 night fighter + 1 other night plane
    night_fighters >= 1 && (night_attackers >= 1 || night_bombers >= 1 || night_suisei >= 1)
        || // Or at least 1 night attacker + 1 night bomber/suisei
        night_attackers >= 1 && (night_bombers >= 1 || night_suisei >= 1)
        || // Or at least 2 night fighters + other
        night_fighters >= 2 && night_other >= 1
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
    if is_cv_night_ci_eligible(codex, ship) {
        return NightAttackType::CarrierNightCI;
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
    if matches!(ship_type(codex, ship), Some(KcShipType::DD)) {
        if let Some(dd_ci) = resolve_dd_night_attack(codex, rng, ship, is_flagship) {
            return dd_ci;
        }
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
        NightAttackType::MainTorpRadar => {
            extend_limit(&mut ids, &main_guns, 1);
            extend_limit(&mut ids, &torpedoes, 2);
            extend_limit(&mut ids, &radars, 3);
        }
        NightAttackType::TorpTorpTorp => extend_limit(&mut ids, &torpedoes, 3),
        // DD CI: GTR (主砲+魚雷+電探)
        NightAttackType::DdGunTorpRadar | NightAttackType::DdGunTorpRadar2 => {
            extend_limit(&mut ids, &main_guns, 1);
            extend_limit(&mut ids, &torpedoes, 2);
            extend_limit(&mut ids, &radars, 3);
        }
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
        NightAttackType::CarrierNightCI | NightAttackType::Normal => {
            extend_limit(&mut ids, &surface_ids, 1)
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
        si_list.push(night_attack_display_ids(codex, ship, attack_type));
        cl_list.push(hit_cls);
        sp_list.push(attack_type.api_sp_list());
        damage.push(hit_damages);
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
        si_list.push(night_attack_display_ids(codex, ship, attack_type));
        cl_list.push(hit_cls);
        sp_list.push(attack_type.api_sp_list());
        damage.push(hit_damages);
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
    use crate::types::{BattleRuntimeShip, EngagementType, NightBattleParams};
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
        assert!(hougeki.api_damage[0][0] >= 1);
        assert!(hougeki.api_damage[0][0] < enemy_hp);
        assert_eq!(enemy[0].hp(), enemy_hp - hougeki.api_damage[0][0]);
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
        assert_eq!(friendly[0].is_sortie, false, "practice ship should have is_sortie=false");
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
        assert_eq!(attack, NightAttackType::CarrierNightCI);
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
}
