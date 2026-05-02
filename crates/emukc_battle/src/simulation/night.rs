//! Night battle phase simulation.
//!
//! Implements night attack type detection (cut-in / double attack),
//! CI trigger rate calculation, and the night hougeki simulation loop.

use emukc_model::{codex::Codex, kc2::KcSlotItemType3};

use crate::damage::{calculate_night_damage, calculate_scratch_damage};
use crate::random::BattleRng;
use crate::targeting::{
    can_attack_night_ship, collect_matching_slot_ids, extend_limit, is_day_surface_display_type,
    is_night_main_gun_type, is_night_secondary_gun_type, is_night_torpedo_type, is_radar_type,
    select_random_target_index, target_class,
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
    MainMainMain,  // 主主主CI: 1 hit x 2.0x
    MainMainSec,   // 主主副CI: 1 hit x 1.75x
    TorpTorpTorp,  // 鱼鱼鱼CI: 2 hits x 1.3x
    MainTorpRadar, // 主鱼電CI: 1 hit x 1.625x
}

impl NightAttackType {
    fn api_sp_list(self) -> i64 {
        match self {
            Self::Normal => 0,
            Self::DoubleAttack => 1,
            Self::MainMainMain => 2,
            Self::MainMainSec => 3,
            Self::TorpTorpTorp => 4,
            Self::MainTorpRadar => 5,
        }
    }

    fn damage_multiplier(self) -> f64 {
        match self {
            Self::Normal => 1.0,
            Self::DoubleAttack => 1.2,
            Self::MainMainMain => 2.0,
            Self::MainMainSec => 1.75,
            Self::TorpTorpTorp => 1.3,
            Self::MainTorpRadar => 1.625,
        }
    }

    fn hit_count(self) -> usize {
        match self {
            Self::Normal | Self::MainMainMain | Self::MainMainSec | Self::MainTorpRadar => 1,
            Self::DoubleAttack | Self::TorpTorpTorp => 2,
        }
    }

    fn ci_coefficient(self) -> f64 {
        match self {
            Self::TorpTorpTorp => 122.0,
            Self::MainTorpRadar => 115.0,
            Self::MainMainSec => 130.0,
            Self::MainMainMain => 140.0,
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
// Night attack detection & CI trigger
// ---------------------------------------------------------------------------

/// Detect the best night attack type from equipment loadout.
fn detect_night_attack_type(codex: &Codex, ship: &BattleRuntimeShip) -> NightAttackType {
    let main_guns = count_main_guns(codex, ship);
    let torps = count_equipment_type(codex, ship, KcSlotItemType3::Torpedo)
        + count_equipment_type(codex, ship, KcSlotItemType3::SubmarineTorpedo);
    let sec_guns = count_secondary_guns(codex, ship);
    let has_radar = has_radar(codex, ship);

    // CI priority (highest first): 主主主 > 主主副 > 主鱼電 > 鱼鱼鱼 > 連撃
    if main_guns >= 3 {
        return NightAttackType::MainMainMain;
    }
    if main_guns >= 2 && sec_guns >= 1 {
        return NightAttackType::MainMainSec;
    }
    if main_guns >= 1 && torps >= 1 && has_radar {
        return NightAttackType::MainTorpRadar;
    }
    if torps >= 2 {
        return NightAttackType::TorpTorpTorp;
    }
    // Double attack: 2+ different weapon categories (main + secondary, main + torp, etc.)
    if (main_guns >= 2) || (main_guns >= 1 && sec_guns >= 1) || (main_guns >= 1 && torps >= 1) {
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

    let luck = ship.ship.api_lucky[0].max(0) as f64;
    let level = ship.ship.api_lv.max(1) as f64;

    let ci_value = if luck < 50.0 {
        15.0 + luck + (0.75 * level.sqrt()).floor()
    } else {
        65.0 + (luck - 50.0).sqrt() + (0.8 * level.sqrt()).floor()
    };

    let modifier = if is_flagship {
        15.0
    } else {
        0.0
    };
    // Chuuha modifier omitted for simplicity (would need HP check)

    let total = ci_value + modifier;
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
    let detected = detect_night_attack_type(codex, ship);
    if detected == NightAttackType::Normal {
        return NightAttackType::Normal;
    }
    if detected == NightAttackType::DoubleAttack {
        // Double attack has ~99% trigger
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
        if (main_guns >= 2) || (main_guns >= 1 && sec_guns >= 1) || (main_guns >= 1 && torps >= 1) {
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
        NightAttackType::DoubleAttack => extend_limit(&mut ids, &surface_ids, 2),
        NightAttackType::Normal => extend_limit(&mut ids, &surface_ids, 1),
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
                let base =
                    calculate_night_damage(codex, rng, ship, &enemy[target_idx], params.air_state);
                (base as f64 * multiplier).floor() as i64
            };
            let (_, dealt) = enemy[target_idx].apply_damage(rng, raw, target_idx);
            total_dealt += dealt;
            hit_damages.push(dealt);
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

        for _ in 0..hits {
            let raw = if is_submarine {
                calculate_scratch_damage(rng, friendly[target_idx].hp().max(1))
            } else {
                let base = calculate_night_damage(
                    codex,
                    rng,
                    ship,
                    &friendly[target_idx],
                    params.air_state,
                );
                (base as f64 * multiplier).floor() as i64
            };
            let (_, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
            hit_damages.push(dealt);
            hit_cls.push(1i64);
        }

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

    let _ = (params.friendly_formation_id, params.enemy_formation_id, params.engagement);
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
    use crate::random::BattleRng;
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
        low_luck_ship.ship.api_lucky = [10, 99];
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

        assert_eq!(hougeki.api_sp_list[0], 4, "torpedo CI sp_list should be 4");
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
            vec![BattleRuntimeShip::from(carrier)],
            vec![BattleRuntimeShip::from(enemy)],
            1,
            1,
            EngagementType::SameCourse,
            None,
            &mut rng,
        );

        let hougeki = simulation.packet.hougeki.unwrap();
        assert!(hougeki.api_at_eflag.iter().all(|flag| *flag == 1));
    }
}
