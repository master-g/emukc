//! An enemy air raid on the air base, flown while the fleet is on its way.
//!
//! Follows `KC3Kai/kancolle-replay`'s `simLBRaid` (`kcsim.js`) and
//! `LandBase.airPowerDefend` (`kcships.js`); the plan `2026-10-09-003` lists
//! each figure with its line. Contact and the high-altitude modifier are left
//! out.

use std::collections::BTreeMap;

use emukc_model::{
    codex::Codex,
    kc2::{KcSlotItemType3, start2::ApiMstSlotitem},
};
use serde::Serialize;

use crate::accuracy::{HitOutcome, roll_strike_on_base};
use crate::damage::apply_cap;
use crate::random::BattleRng;
use crate::targeting::{is_air_combat_type, is_airstrike_attack_type};
use crate::types::{
    AirSquadronInput, AirState, BattleKoukuStage1, BattleKoukuStage3, BattleRuntimeShip,
    BattleSquadronPlane, DamageCell,
};

use super::kouku::{AIRSTRIKE_HIT_PERCENT, attack_plane_from, calculate_fighter_power};

/// What an air base can take; it is never brought below 1.
const BASE_HP: i64 = 200;
/// Where a strike on the base stops growing linearly, as for any air strike.
const DAMAGE_CAP: f64 = 170.0;

/// One air corps of the raided area.
#[derive(Debug, Clone)]
pub struct AirRaidBase {
    /// The air corps, by its id within the area.
    pub base_rid: i64,
    /// Whether it is ordered to defend: only then do its squadrons fly.
    pub defending: bool,
    /// Its squadrons, in squadron order.
    pub squadrons: Vec<AirSquadronInput>,
}

/// `api_destruction_battle`: the raid as the client plays it.
#[expect(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleAirRaid {
    pub api_formation: [i64; 3],
    pub api_ship_ke: Vec<i64>,
    pub api_ship_lv: Vec<i64>,
    pub api_e_nowhps: Vec<i64>,
    pub api_e_maxhps: Vec<i64>,
    #[serde(rename = "api_eSlot")]
    pub api_e_slot: Vec<Vec<i64>>,
    pub api_f_nowhps: Vec<i64>,
    pub api_f_maxhps: Vec<i64>,
    pub api_air_base_attack: BattleAirRaidAttack,
    /// 1 stores lost, 2 stores and aircraft on the ground, 3 aircraft only, 4 nothing. The
    /// simulation leaves it at 4; what was lost is the caller's to decide.
    pub api_lost_kind: i64,
    /// What each base took, in base order. Not part of the packet.
    #[serde(skip)]
    pub base_damage: Vec<i64>,
}

/// The one air battle of a raid.
#[expect(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleAirRaidAttack {
    pub api_stage_flag: [i64; 3],
    /// The defending bases by their place in the area, from 1, then the enemy ships that
    /// launched. Without a defender the first is null.
    pub api_plane_from: (Option<Vec<i64>>, Vec<i64>),
    /// The aircraft each defending base sent up, keyed by its place in the area.
    pub api_map_squadron_plane: Option<BTreeMap<String, Vec<BattleSquadronPlane>>>,
    pub api_stage1: BattleKoukuStage1,
    pub api_stage2: Option<()>,
    pub api_stage3: BattleKoukuStage3,
}

/// 制空値 of a defending air corps: a land fighter adds its interception and twice its
/// anti-bomber figure, and the best reconnaissance aircraft of the corps multiplies the sum.
fn defence_power(codex: &Codex, base: &AirRaidBase) -> i64 {
    let mut power = 0;
    let mut modifier = 1.0_f64;
    for squadron in base.squadrons.iter().filter(|squadron| squadron.count > 0) {
        let Ok(mst) = codex.find::<ApiMstSlotitem>(&squadron.mst_id) else {
            continue;
        };
        let type3 = KcSlotItemType3::n(mst.api_type[2]);
        let mut stat = mst.api_tyku as f64;
        if type3 == Some(KcSlotItemType3::LocalFighter) {
            // 迎撃 is carried in the evasion field, 対爆 in the accuracy field.
            stat += (mst.api_houk + 2 * mst.api_houm) as f64;
        }
        power += (stat * (squadron.count as f64).sqrt()).floor() as i64;
        let scouting = match type3 {
            Some(KcSlotItemType3::SeaBasedRecon | KcSlotItemType3::LargeFlyingBoat) => {
                match mst.api_saku {
                    9.. => 1.16,
                    8 => 1.13,
                    _ => 1.1,
                }
            }
            Some(KcSlotItemType3::CarrierBasedRecon | KcSlotItemType3::CarrierBasedRecon2) => {
                if mst.api_saku >= 9 {
                    1.3
                } else {
                    1.2
                }
            }
            Some(KcSlotItemType3::LandBasedRecon) => {
                if mst.api_houm >= 3 {
                    1.23
                } else {
                    1.18
                }
            }
            _ => 1.0,
        };
        modifier = modifier.max(scouting);
    }
    (power as f64 * modifier).floor() as i64
}

/// The land fighter that meets the enemy's next slot: the one with the best anti-bomber
/// figure among the defenders' squadrons at the first slot, from `from` on, that has any.
/// Returns its `(anti-bomber, interception)` and the slot it stood in.
fn interceptor(codex: &Codex, bases: &[AirRaidBase], from: usize) -> Option<((i64, i64), usize)> {
    (from..4).find_map(|slot| {
        bases
            .iter()
            .filter(|base| base.defending)
            .filter_map(|base| base.squadrons.get(slot))
            .filter(|squadron| squadron.count > 0)
            .filter_map(|squadron| codex.find::<ApiMstSlotitem>(&squadron.mst_id).ok())
            .filter(|mst| {
                KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::LocalFighter)
            })
            .map(|mst| (mst.api_houm, mst.api_houk))
            .max()
            .map(|best| (best, slot))
    })
}

/// Fly `enemy`'s raid on `bases`, taking the losses out of both sides' aircraft.
pub fn simulate_air_raid(
    codex: &Codex,
    bases: &mut [AirRaidBase],
    enemy: &mut [BattleRuntimeShip],
    formation: i64,
    rng: &mut impl BattleRng,
) -> BattleAirRaid {
    let defenders: Vec<usize> =
        bases.iter().enumerate().filter(|(_, base)| base.defending).map(|(at, _)| at).collect();
    let api_map_squadron_plane = (!defenders.is_empty()).then(|| {
        defenders
            .iter()
            .map(|&at| {
                let planes = bases[at]
                    .squadrons
                    .iter()
                    .filter(|squadron| squadron.count > 0)
                    .map(|squadron| BattleSquadronPlane {
                        api_mst_id: squadron.mst_id,
                        api_count: squadron.count,
                    })
                    .collect();
                ((at + 1).to_string(), planes)
            })
            .collect()
    });

    let friend_power: i64 = defenders.iter().map(|&at| defence_power(codex, &bases[at])).sum();
    let air_state = AirState::from_power(friend_power, calculate_fighter_power(codex, enemy));

    // Stage 1, the defenders: every squadron loses the same share.
    let (f_min, f_max) = air_state.stage1_friendly_loss_ratio();
    let mut f_count = 0;
    let mut f_lost = 0;
    for &at in &defenders {
        let ratio = rng.random_f64_range(f_min, f_max);
        for squadron in &mut bases[at].squadrons {
            let lost = (squadron.count as f64 * ratio).floor() as i64;
            f_count += squadron.count;
            f_lost += lost;
            squadron.count -= lost;
        }
    }

    // Stage 1, the raiders: each slot of aircraft meets the defenders' next slot.
    let state_modifier = match air_state {
        AirState::Incapability => 1.0,
        AirState::Denial => 4.0,
        AirState::Parity => 6.0,
        AirState::Superiority => 8.0,
        AirState::Supremacy => 10.0,
    };
    let api_plane_from_enemy = attack_plane_from(codex, enemy);
    let mut e_count = 0;
    let mut e_lost = 0;
    for ship in enemy.iter_mut() {
        let mut slot = 0;
        for (item, onslot) in ship.slot_items.iter().zip(ship.ship.api_onslot.iter_mut()) {
            let flies = codex
                .find::<ApiMstSlotitem>(&item.api_slotitem_id)
                .is_ok_and(|mst| is_air_combat_type(mst.api_type[2]));
            if !flies || *onslot <= 0 {
                continue;
            }
            let met = interceptor(codex, bases, slot);
            let (anti_bomber, interception) = met.map_or((0, 0), |(stats, _)| stats);
            slot = met.map_or(4, |(_, at)| at) + 1;
            let anti_bomber = anti_bomber as f64;
            let share = 6.5 * state_modifier
                + 3.5
                    * (anti_bomber
                        + state_modifier * (interception.min(1) as f64)
                        + rng.random_f64_range(0.0, 1.0) * (state_modifier + anti_bomber));
            let lost = ((*onslot as f64 * share / 100.0).ceil() as i64).min(*onslot);
            e_count += *onslot;
            e_lost += lost;
            *onslot -= lost;
        }
    }

    // Stage 3: every slot of bombers left picks a base.
    let mut hp = vec![BASE_HP; bases.len()];
    let mut base_damage = vec![0_i64; bases.len()];
    let mut api_frai_flag = vec![0_i64; bases.len()];
    let mut api_fbak_flag = vec![0_i64; bases.len()];
    for ship in enemy.iter() {
        for (item, onslot) in ship.slot_items.iter().zip(ship.ship.api_onslot) {
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
                continue;
            };
            if onslot <= 0 || !is_airstrike_attack_type(mst.api_type[2]) {
                continue;
            }
            let Some(target) = rng.choose_index(bases.len()) else {
                break;
            };
            let torpedo = KcSlotItemType3::n(mst.api_type[2])
                == Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
            let stat = if torpedo {
                mst.api_raig
            } else {
                mst.api_baku
            };
            if stat <= 0 {
                continue;
            }
            // The base cannot dodge, so only the aircraft's own miss rate counts.
            let outcome = roll_strike_on_base(rng, AIRSTRIKE_HIT_PERCENT);
            if torpedo {
                api_frai_flag[target] = 1;
            } else {
                api_fbak_flag[target] = 1;
            }
            if outcome == HitOutcome::Miss {
                continue;
            }
            let mut power = stat as f64 * (onslot as f64).sqrt() + 25.0;
            if torpedo {
                // A torpedo run does either less or more than a bombing one.
                power *= if rng.roll_range(0, 2) == 0 {
                    0.8
                } else {
                    1.5
                };
            }
            // The base has no armour; it keeps its last point.
            let damage = apply_cap(power, DAMAGE_CAP).min(hp[target] - 1);
            hp[target] -= damage;
            base_damage[target] += damage;
        }
    }

    let none = |len: usize| vec![0_i64; len];
    let no_damage = |len: usize| vec![DamageCell::Plain(0); len];
    BattleAirRaid {
        api_formation: [1, formation, 1],
        api_ship_ke: enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        api_ship_lv: enemy.iter().map(|ship| ship.ship.api_lv).collect(),
        api_e_nowhps: enemy.iter().map(BattleRuntimeShip::hp).collect(),
        api_e_maxhps: enemy.iter().map(|ship| ship.ship.api_maxhp).collect(),
        api_e_slot: enemy
            .iter()
            .map(|ship| {
                let mut slots: Vec<i64> =
                    ship.slot_items.iter().map(|item| item.api_slotitem_id).collect();
                slots.resize(5, -1);
                slots
            })
            .collect(),
        api_f_nowhps: vec![BASE_HP; bases.len()],
        api_f_maxhps: vec![BASE_HP; bases.len()],
        api_air_base_attack: BattleAirRaidAttack {
            api_stage_flag: [1, 0, 1],
            api_plane_from: (
                (!defenders.is_empty())
                    .then(|| defenders.iter().map(|&at| at as i64 + 1).collect()),
                api_plane_from_enemy,
            ),
            api_map_squadron_plane,
            api_stage1: BattleKoukuStage1 {
                api_f_count: f_count,
                api_f_lostcount: f_lost,
                api_e_count: e_count,
                api_e_lostcount: e_lost,
                api_disp_seiku: air_state.api_disp_seiku(),
                api_touch_plane: [-1, -1],
            },
            api_stage2: None,
            api_stage3: BattleKoukuStage3 {
                api_frai: Vec::new(),
                api_erai: Vec::new(),
                api_fbak: Vec::new(),
                api_ebak: Vec::new(),
                api_frai_flag,
                api_erai_flag: none(enemy.len()),
                api_fbak_flag,
                api_ebak_flag: none(enemy.len()),
                api_fcl_flag: none(bases.len()),
                api_ecl_flag: none(enemy.len()),
                api_fdam: base_damage.iter().map(|&damage| DamageCell::Plain(damage)).collect(),
                api_edam: no_damage(enemy.len()),
                api_f_sp_list: Vec::new(),
                api_e_sp_list: Vec::new(),
            },
        },
        api_lost_kind: 4,
        base_damage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::SeededRng;
    use crate::test_utils::{first_ship_mst_by_type, sample_ship, slotitem_with_mst_id};
    use emukc_model::kc2::types::KcShipType;

    /// 雷電: 対空 6, 迎撃 2, 対爆 5.
    const LOCAL_FIGHTER: i64 = 175;
    /// 彩雲: 索敵 9.
    const SAIUN: i64 = 54;

    fn raiders(codex: &Codex) -> Vec<BattleRuntimeShip> {
        let mut carrier = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::CV), 1);
        // 深海猫艦戦改, 深海地獄艦爆改 and 深海復讐艦攻改, thirty of each.
        carrier.slot_items = [1556, 1557, 1558].map(slotitem_with_mst_id).to_vec();
        carrier.ship.api_onslot = [30, 30, 30, 0, 0];
        vec![BattleRuntimeShip::new(carrier, false, true)]
    }

    fn base(defending: bool, squadrons: &[(i64, i64)]) -> AirRaidBase {
        AirRaidBase {
            base_rid: 1,
            defending,
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

    #[test]
    fn an_undefended_base_is_bombed_and_sends_nothing_up() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut bases = vec![base(false, &[(LOCAL_FIGHTER, 18)]), base(false, &[])];
        let raid =
            simulate_air_raid(&codex, &mut bases, &mut raiders(&codex), 3, &mut SeededRng::new(1));

        let attack = &raid.api_air_base_attack;
        assert_eq!(attack.api_plane_from, (None, vec![1]));
        assert!(attack.api_map_squadron_plane.is_none());
        assert_eq!(attack.api_stage1.api_f_count, 0);
        assert_eq!(attack.api_stage1.api_disp_seiku, AirState::Incapability.api_disp_seiku());
        // Nobody up: between 6.5 and 10 of a hundred fall, so 3 of each thirty.
        assert_eq!((attack.api_stage1.api_e_count, attack.api_stage1.api_e_lostcount), (90, 9));
        assert_eq!(raid.base_damage.len(), 2);
        assert!(raid.base_damage.iter().sum::<i64>() > 0);
        assert!(raid.base_damage.iter().all(|&damage| damage < BASE_HP));
        assert_eq!(bases[0].squadrons[0].count, 18, "a base that does not defend loses nothing");
        assert_eq!(
            serde_json::to_value(&raid).unwrap()["api_air_base_attack"]["api_stage2"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn land_fighters_in_command_of_the_air_shoot_the_raid_down() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut bases = vec![
            base(false, &[]),
            base(
                true,
                &[(LOCAL_FIGHTER, 18), (LOCAL_FIGHTER, 18), (LOCAL_FIGHTER, 18), (SAIUN, 4)],
            ),
        ];
        // (6 + 2 + 2×5) × √18 = 76 a squadron, 228 in all, times 1.3 for the 彩雲.
        assert_eq!(defence_power(&codex, &bases[1]), 296);

        let mut enemy = raiders(&codex);
        enemy[0].ship.api_onslot = [4, 30, 30, 0, 0];
        let raid = simulate_air_raid(&codex, &mut bases, &mut enemy, 3, &mut SeededRng::new(1));

        let attack = &raid.api_air_base_attack;
        assert_eq!(attack.api_plane_from.0, Some(vec![2]));
        assert_eq!(attack.api_map_squadron_plane.as_ref().unwrap()["2"].len(), 4);
        assert_eq!(attack.api_stage1.api_disp_seiku, AirState::Supremacy.api_disp_seiku());
        // 6.5×10 + 3.5×(5 + 10 + …) is past a hundred: every slot is emptied.
        assert_eq!(attack.api_stage1.api_e_lostcount, attack.api_stage1.api_e_count);
        assert_eq!(raid.base_damage, vec![0, 0]);
        assert!(bases[1].squadrons.iter().all(|squadron| squadron.count > 0));
    }
}
