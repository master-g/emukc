//! An air corps' attack on the enemy fleet, flown before the fleets meet.
//!
//! The three stages are those of the carrier air battle in [`super::kouku`]
//! and are kept at its level of detail: every attack hits, the anti-air stage
//! is the same linear estimate and a target is any enemy still afloat. What is
//! particular to an air corps follows `KC3Kai/kancolle-replay`'s `kcsim.js`
//! (`LBASPhase`, `airstrikeLBAS`) and `kcships.js` (`LandBase.airPower`); the
//! plan `2026-10-08-006` lists each formula with its line and what was left out.

use emukc_model::{
    codex::Codex,
    kc2::{KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::damage::{apply_cap, calculate_defense_power, resolve_damage};
use crate::random::BattleRng;
use crate::targeting::{is_airstrike_attack_type, target_class};
use crate::types::{
    AirCorpsInput, AirState, BattleAirBaseAttack, BattleKouku, BattleKoukuStage1,
    BattleKoukuStage2, BattleKoukuStage3, BattleRuntimeShip, BattleSquadronPlane, DamageCell,
    TargetClass,
};

use super::kouku::{
    apply_plane_losses, attack_plane_from, calculate_fighter_power, total_plane_count,
};

/// Where an air corps strike stops growing linearly (`lbasDmgCap`).
const DAMAGE_CAP: f64 = 220.0;
/// What a squadron's strength counts for under the square root.
const SLOT_MODIFIER: f64 = 1.8;
/// 陸上攻撃機 and 大型陸上機 strike at this share before the cap…
const LAND_ATTACKER_PRE_CAP: f64 = 0.8;
/// …and 陸上攻撃機 at this multiple after it.
const LAND_ATTACKER_POST_CAP: f64 = 1.8;

fn is_land_attacker(type3: i64) -> bool {
    matches!(
        KcSlotItemType3::n(type3),
        Some(KcSlotItemType3::LandBasedAttacker | KcSlotItemType3::LargeLandBasedAircraft)
    )
}

/// Whether a squadron of this equipment attacks ships at all; the rest only
/// fight for the air.
fn strikes(type3: i64) -> bool {
    is_land_attacker(type3) || is_airstrike_attack_type(type3)
}

/// 制空値 of an air corps on a sortie: each squadron's anti-air, with a land
/// fighter's interception counted at one and a half, times the root of its
/// strength.
fn fighter_power(codex: &Codex, corps: &AirCorpsInput) -> i64 {
    corps
        .squadrons
        .iter()
        .filter(|squadron| squadron.count > 0)
        .filter_map(|squadron| {
            let mst = codex.find::<ApiMstSlotitem>(&squadron.mst_id).ok()?;
            let mut base = mst.api_tyku as f64;
            if KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::LocalFighter) {
                // 迎撃 is carried in the evasion field.
                base += mst.api_houk as f64 * 1.5;
            }
            Some((base * (squadron.count as f64).sqrt()).floor() as i64)
        })
        .sum()
}

/// The figure a squadron strikes this target with: bombs or torpedoes,
/// depending on what it flies and whether the target stands on land.
fn strike_stat(mst: &ApiMstSlotitem, on_land: bool) -> f64 {
    let bombs = mst.api_baku.max(0) as f64;
    let torpedoes = mst.api_raig.max(0) as f64;
    if is_land_attacker(mst.api_type[2]) {
        if on_land {
            bombs
        } else {
            torpedoes
        }
    } else if KcSlotItemType3::n(mst.api_type[2])
        == Some(KcSlotItemType3::CarrierBasedTorpedoBomber)
    {
        if on_land {
            (torpedoes / 2.0).floor()
        } else {
            torpedoes
        }
    } else {
        bombs
    }
}

/// Attack power of one squadron before the target's armour:
/// `25 + stat × √(1.8 × strength)`, a land attacker's taken at 0.8, capped at
/// 220, and 陸上攻撃機's then multiplied by 1.8.
fn strike_power(mst: &ApiMstSlotitem, count: i64, on_land: bool) -> f64 {
    let mut power = 25.0 + strike_stat(mst, on_land) * (SLOT_MODIFIER * count as f64).sqrt();
    if is_land_attacker(mst.api_type[2]) {
        power *= LAND_ATTACKER_PRE_CAP;
    }
    let mut power = apply_cap(power, DAMAGE_CAP) as f64;
    if KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::LandBasedAttacker) {
        power *= LAND_ATTACKER_POST_CAP;
    }
    power
}

/// Fly one attack of `corps` against `enemy`, taking the losses out of both.
pub(crate) fn simulate_air_base_attack(
    codex: &Codex,
    corps: &mut AirCorpsInput,
    enemy: &mut [BattleRuntimeShip],
    rng: &mut impl BattleRng,
) -> BattleAirBaseAttack {
    let api_squadron_plane = corps
        .squadrons
        .iter()
        .filter(|squadron| squadron.count > 0)
        .map(|squadron| BattleSquadronPlane {
            api_mst_id: squadron.mst_id,
            api_count: squadron.count,
        })
        .collect();

    let friend_planes: i64 = corps.squadrons.iter().map(|squadron| squadron.count).sum();
    let enemy_planes = total_plane_count(codex, enemy);
    let air_state =
        AirState::from_power(fighter_power(codex, corps), calculate_fighter_power(codex, enemy));

    // Stage 1: the fight for the air, every squadron losing the same share.
    let (f_min, f_max) = air_state.stage1_friendly_loss_ratio();
    let (e_min, e_max) = air_state.stage1_enemy_loss_ratio();
    let f_ratio = rng.random_f64_range(f_min, f_max);
    let e_ratio = rng.random_f64_range(e_min, e_max);
    let mut stage1_f_lost = 0;
    for squadron in &mut corps.squadrons {
        let lost = (squadron.count as f64 * f_ratio).floor() as i64;
        squadron.count -= lost;
        stage1_f_lost += lost;
    }
    let stage1_e_lost = (enemy_planes as f64 * e_ratio).floor() as i64;
    apply_plane_losses(codex, enemy, stage1_e_lost);

    // Stage 2: the fleet's anti-air fire on the squadrons that come to strike.
    let enemy_aa: f64 = enemy.iter().map(|ship| ship.ship.api_taiku[0].max(0) as f64).sum();
    let mut stage2_f_count = 0;
    let mut stage2_f_lost = 0;
    for squadron in &mut corps.squadrons {
        let striking = codex
            .find::<ApiMstSlotitem>(&squadron.mst_id)
            .is_ok_and(|mst| strikes(mst.api_type[2]));
        if !striking || squadron.count <= 0 {
            continue;
        }
        let lost =
            ((enemy_aa / 400.0) * squadron.count as f64).floor().min(squadron.count as f64) as i64;
        stage2_f_count += squadron.count;
        stage2_f_lost += lost;
        squadron.count -= lost;
    }

    // Stage 3: each striking squadron picks one enemy still afloat.
    let mut api_edam = vec![0_i64; enemy.len()];
    let mut api_erai_flag = vec![0_i64; enemy.len()];
    let mut api_ebak_flag = vec![0_i64; enemy.len()];
    let mut struck = false;
    for squadron in &corps.squadrons {
        if squadron.count <= 0 {
            continue;
        }
        let Ok(mst) = codex.find::<ApiMstSlotitem>(&squadron.mst_id) else {
            continue;
        };
        if !strikes(mst.api_type[2]) {
            continue;
        }
        let afloat: Vec<usize> = enemy
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.is_alive())
            .map(|(index, _)| index)
            .collect();
        let Some(pick) = rng.choose_index(afloat.len()) else {
            break;
        };
        let target = afloat[pick];
        let on_land = target_class(codex, &enemy[target]) == TargetClass::Installation;

        if strike_stat(mst, on_land) <= 0.0 {
            continue;
        }
        let land_attacker = is_land_attacker(mst.api_type[2]);
        let power = strike_power(mst, squadron.count, on_land);
        let defense = calculate_defense_power(rng, enemy[target].ship.api_soukou[0]);
        let damage = resolve_damage(rng, power, defense, enemy[target].hp());
        struck = true;
        if damage > 0 {
            let (raw, dealt) = enemy[target].apply_damage(rng, damage, target);
            api_edam[target] += crate::targeting::display_damage(&enemy[target], raw, dealt);
        }
        // A land attacker drops bombs on what stands on land and torpedoes on
        // what floats; the client draws the one whose flag is set.
        let torpedo = !on_land
            && (land_attacker
                || KcSlotItemType3::n(mst.api_type[2])
                    == Some(KcSlotItemType3::CarrierBasedTorpedoBomber));
        if torpedo {
            api_erai_flag[target] = 1;
        } else {
            api_ebak_flag[target] = 1;
        }
    }

    BattleAirBaseAttack {
        api_base_id: corps.base_rid,
        api_stage_flag: [1, i64::from(stage2_f_count > 0), i64::from(struck)],
        api_squadron_plane,
        remaining: corps
            .squadrons
            .iter()
            .map(|squadron| (squadron.squadron_id, squadron.count))
            .collect(),
        kouku: BattleKouku {
            api_plane_from: [Vec::new(), attack_plane_from(codex, enemy)],
            api_stage1: BattleKoukuStage1 {
                api_f_count: friend_planes,
                api_f_lostcount: stage1_f_lost,
                api_e_count: enemy_planes,
                api_e_lostcount: stage1_e_lost,
                api_disp_seiku: air_state.api_disp_seiku(),
                api_touch_plane: [-1, -1],
            },
            api_stage2: BattleKoukuStage2 {
                api_f_count: stage2_f_count,
                api_f_lostcount: stage2_f_lost,
                api_e_count: 0,
                api_e_lostcount: 0,
            },
            api_stage3: BattleKoukuStage3 {
                api_frai: Vec::new(),
                api_erai: Vec::new(),
                api_fbak: Vec::new(),
                api_ebak: Vec::new(),
                api_frai_flag: Vec::new(),
                api_erai_flag,
                api_fbak_flag: Vec::new(),
                api_ebak_flag,
                api_fcl_flag: Vec::new(),
                api_ecl_flag: vec![0; enemy.len()],
                api_fdam: Vec::new(),
                api_edam: api_edam.into_iter().map(DamageCell::Plain).collect(),
                api_f_sp_list: Vec::new(),
                api_e_sp_list: vec![None; enemy.len()],
            },
            api_stage3_combined: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::AirSquadronInput;
    use emukc_model::kc2::types::KcShipType;

    /// 一式陸攻: 雷装 10, 爆装 12, 対空 2.
    const LAND_ATTACKER: i64 = 169;
    /// 雷電: 対空 6, 迎撃 2.
    const LOCAL_FIGHTER: i64 = 175;

    fn corps(squadrons: &[(i64, i64)]) -> AirCorpsInput {
        AirCorpsInput {
            base_rid: 1,
            waves: 1,
            squadrons: (1..)
                .zip(squadrons)
                .map(|(squadron_id, &(mst_id, count))| AirSquadronInput {
                    squadron_id,
                    mst_id,
                    count,
                })
                .collect(),
        }
    }

    fn lone_enemy(codex: &Codex, ship_type: KcShipType) -> Vec<BattleRuntimeShip> {
        let mut enemy = sample_ship(codex, first_ship_mst_by_type(codex, ship_type), 1);
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_taiku[0] = 0;
        enemy.ship.api_onslot = [0; 5];
        vec![BattleRuntimeShip::new(enemy, false, true)]
    }

    #[test]
    fn a_land_attacker_strikes_at_the_sources_figures() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mst = codex.manifest.find_slotitem(LAND_ATTACKER).unwrap();
        assert_eq!((mst.api_raig, mst.api_baku), (10, 12), "the figures below assume these");

        // 25 + 10 × √32.4 = 81.9, × 0.8 = 65.5 → 65, × 1.8.
        assert!((strike_power(mst, 18, false) - 117.0).abs() < 1e-9);
        // On land it bombs: 25 + 12 × √32.4 = 93.3, × 0.8 = 74.6 → 74, × 1.8.
        assert!((strike_power(mst, 18, true) - 133.2).abs() < 1e-9);
        // Fewer aircraft strike less: 25 + 10 × √1.8 = 38.4, × 0.8 = 30.7 → 30, × 1.8.
        assert!((strike_power(mst, 1, false) - 54.0).abs() < 1e-9);
    }

    #[test]
    fn the_cap_bends_a_strike_at_220() {
        // 300 over the cap keeps √80 of the excess.
        assert_eq!(apply_cap(300.0, DAMAGE_CAP), 228);
    }

    #[test]
    fn an_air_corps_counts_a_land_fighters_interception() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fighter = codex.manifest.find_slotitem(LOCAL_FIGHTER).unwrap();
        let expected_fighter = ((fighter.api_tyku as f64 + fighter.api_houk as f64 * 1.5)
            * 18.0_f64.sqrt())
        .floor() as i64;
        // 一式陸攻's 対空 2 × √18 = 8.
        assert_eq!(
            fighter_power(&codex, &corps(&[(LAND_ATTACKER, 18), (LOCAL_FIGHTER, 18)])),
            8 + expected_fighter
        );
    }

    #[test]
    fn a_strike_damages_one_enemy_and_reports_what_flew() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut enemy = lone_enemy(&codex, KcShipType::DD);
        let mut corps = corps(&[(LAND_ATTACKER, 18), (LOCAL_FIGHTER, 18)]);
        let mut rng = crate::random::SeededRng::new(7);

        let attack = simulate_air_base_attack(&codex, &mut corps, &mut enemy, &mut rng);

        assert_eq!(attack.api_base_id, 1);
        assert_eq!(attack.api_squadron_plane.len(), 2);
        assert_eq!(attack.api_squadron_plane[0].api_count, 18);
        assert_eq!(attack.api_stage_flag, [1, 1, 1]);
        assert_eq!(attack.kouku.api_stage1.api_f_count, 36);
        assert_eq!(
            attack.kouku.api_stage2.api_f_count, corps.squadrons[0].count,
            "only the bombers"
        );
        assert_eq!(attack.kouku.api_stage3.api_erai_flag, [1], "torpedoes on a ship afloat");
        assert_eq!(attack.kouku.api_stage3.api_ebak_flag, [0]);
        assert!(enemy[0].hp() < 500, "the destroyer was hit");
        assert_eq!(
            attack.remaining,
            [(1, corps.squadrons[0].count), (2, corps.squadrons[1].count)],
            "what is left is what the caller writes back"
        );
    }

    #[test]
    fn fighters_alone_fight_for_the_air_and_strike_nothing() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut enemy = lone_enemy(&codex, KcShipType::DD);
        let mut corps = corps(&[(LOCAL_FIGHTER, 18)]);
        let mut rng = crate::random::SeededRng::new(7);

        let attack = simulate_air_base_attack(&codex, &mut corps, &mut enemy, &mut rng);

        assert_eq!(attack.api_stage_flag, [1, 0, 0]);
        assert_eq!(enemy[0].hp(), 500);
    }

    /// Pointed at one cell twice, an air corps attacks twice, the second time
    /// with what the first left it.
    #[test]
    fn two_waves_fly_in_turn_and_carry_their_losses() {
        use crate::types::{BattleContext, BattleType};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut enemy = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 1);
        enemy.ship.api_nowhp = 5000;
        enemy.ship.api_maxhp = 5000;
        enemy.ship.api_taiku[0] = 200;
        let friend = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 1);

        let mut context =
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]);
        context.air_corps = vec![AirCorpsInput {
            waves: 2,
            ..corps(&[(LAND_ATTACKER, 18)])
        }];
        let mut rng = crate::random::SeededRng::new(7);

        let packet = crate::execute_day(&codex, context, &mut rng).packet;

        assert_eq!(packet.air_base_attack.len(), 2);
        let after_first = packet.air_base_attack[0].remaining[0].1;
        assert!(after_first < 18, "anti-air fire took some");
        assert_eq!(packet.air_base_attack[1].api_squadron_plane[0].api_count, after_first);
    }
}
