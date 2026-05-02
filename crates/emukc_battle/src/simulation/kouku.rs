//! Aerial combat (kouku) phase simulation.
//!
//! Implements the three-stage aerial combat model:
//! Stage 1: Fighter combat (air superiority), Stage 2: Anti-air fire,
//! Stage 3: Bombing damage (dive bombing + torpedo bombing).

use emukc_model::{
    codex::Codex,
    kc2::{KcApiSlotItem, KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::damage::{apply_cap, calculate_defense_power, resolve_damage};
use crate::random::BattleRng;
use crate::types::{
    AirState, AirstrikeOutput, BattleKouku, BattleKoukuStage1, BattleKoukuStage2,
    BattleKoukuStage3, BattleRuntimeShip,
};

// ---------------------------------------------------------------------------
// Fighter power & plane count helpers
// ---------------------------------------------------------------------------

fn is_fighter_power_type(slotitem_type: i64) -> bool {
    matches!(
        KcSlotItemType3::n(slotitem_type),
        Some(
            KcSlotItemType3::CarrierBasedFighter
                | KcSlotItemType3::CarrierBasedDiveBomber
                | KcSlotItemType3::CarrierBasedTorpedoBomber
                | KcSlotItemType3::SeaBasedBomber
                | KcSlotItemType3::SeaplaneFighter
                | KcSlotItemType3::JetFighter
                | KcSlotItemType3::JetFighterBomber
                | KcSlotItemType3::JetAttacker
        )
    )
}

pub(crate) fn calculate_fighter_power(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
    ships
        .iter()
        .flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
        .filter_map(|(slot_item, onslot)| {
            if onslot <= 0 {
                return None;
            }
            let mst = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()?;
            if !is_fighter_power_type(mst.api_type[2]) {
                return None;
            }
            let aa = mst.api_tyku.max(0) as f64;
            Some((aa * (onslot as f64).sqrt()).floor() as i64)
        })
        .sum()
}

pub(crate) fn total_plane_count(codex: &Codex, ships: &[BattleRuntimeShip]) -> i64 {
    ships
        .iter()
        .flat_map(|ship| ship.slot_items.iter().zip(ship.ship.api_onslot))
        .filter(|(slot_item, onslot)| {
            *onslot > 0
                && codex
                    .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
                    .ok()
                    .is_some_and(|mst| is_air_combat_type(mst.api_type[2]))
        })
        .map(|(_, onslot)| onslot)
        .sum()
}

pub(crate) fn has_any_air_combat_planes(codex: &Codex, ships: &[BattleRuntimeShip]) -> bool {
    total_plane_count(codex, ships) > 0
}

pub(crate) fn attack_plane_from(codex: &Codex, ships: &[BattleRuntimeShip]) -> Vec<i64> {
    ships
        .iter()
        .enumerate()
        .filter_map(|(idx, ship)| {
            let has_plane =
                ship.slot_items.iter().zip(ship.ship.api_onslot).any(|(slot_item, onslot)| {
                    onslot > 0
                        && codex
                            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
                            .ok()
                            .is_some_and(|mst| is_airstrike_attack_type(mst.api_type[2]))
                });
            has_plane.then_some(idx as i64 + 1)
        })
        .collect()
}

fn first_touch_plane(codex: &Codex, ships: &[BattleRuntimeShip]) -> Option<i64> {
    ships.iter().flat_map(|ship| ship.slot_items.iter()).find_map(|slot_item| {
        codex
            .find::<ApiMstSlotitem>(&slot_item.api_slotitem_id)
            .ok()
            .filter(|mst| {
                matches!(
                    KcSlotItemType3::n(mst.api_type[2]),
                    Some(KcSlotItemType3::CarrierBasedRecon | KcSlotItemType3::CarrierBasedRecon2)
                )
            })
            .map(|mst| mst.api_id)
    })
}

// ---------------------------------------------------------------------------
// Airstrike attack type detection
// ---------------------------------------------------------------------------

fn is_airstrike_attack_type(slotitem_type: i64) -> bool {
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

fn is_air_combat_type(slotitem_type: i64) -> bool {
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

/// Find the ship index with the highest total bombing power (for damage attribution).
fn best_bomber_index(codex: &Codex, ships: &[BattleRuntimeShip]) -> Option<usize> {
    ships
        .iter()
        .enumerate()
        .map(|(idx, ship)| {
            let power: f64 = ship
                .slot_items
                .iter()
                .zip(ship.ship.api_onslot)
                .filter_map(|(si, onslot)| {
                    if onslot <= 0 {
                        return None;
                    }
                    let mst = codex.find::<ApiMstSlotitem>(&si.api_slotitem_id).ok()?;
                    if !is_airstrike_attack_type(mst.api_type[2]) {
                        return None;
                    }
                    let is_torpedo = KcSlotItemType3::n(mst.api_type[2])
                        == Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
                    let stat = if is_torpedo {
                        mst.api_raig.max(0) as f64
                    } else {
                        mst.api_baku.max(0) as f64
                    };
                    Some(stat * (onslot as f64).sqrt())
                })
                .sum();
            (idx, power)
        })
        .filter(|(_, power)| *power > 0.0)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
}

// ---------------------------------------------------------------------------
// Plane loss application
// ---------------------------------------------------------------------------

fn apply_plane_losses(codex: &Codex, ships: &mut [BattleRuntimeShip], mut lostcount: i64) {
    while lostcount > 0 {
        let mut best_slot: Option<(usize, usize, i64)> = None;
        for (ship_idx, ship) in ships.iter().enumerate() {
            for (slot_idx, slot_item) in ship.slot_items.iter().enumerate().take(5) {
                let onslot = ship.ship.api_onslot[slot_idx];
                if onslot <= 0 {
                    continue;
                }
                let Some(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id).ok()
                else {
                    continue;
                };
                if !is_air_combat_type(mst.api_type[2]) {
                    continue;
                }
                if best_slot.is_none_or(|(_, _, current)| onslot > current) {
                    best_slot = Some((ship_idx, slot_idx, onslot));
                }
            }
        }

        let Some((ship_idx, slot_idx, _)) = best_slot else {
            break;
        };
        ships[ship_idx].ship.api_onslot[slot_idx] -= 1;
        lostcount -= 1;
    }
}

// ---------------------------------------------------------------------------
// Single-slot airstrike damage
// ---------------------------------------------------------------------------

/// Calculate airstrike damage for a single bomber slot.
///
/// Uses bomb/torpedo stat × √(onslot) + 25, capped at 170.
fn calculate_single_slot_airstrike_damage(
    codex: &Codex,
    rng: &mut impl BattleRng,
    slot_item: &KcApiSlotItem,
    onslot: i64,
    defender: &BattleRuntimeShip,
) -> i64 {
    if onslot <= 0 {
        return 0;
    }
    let Ok(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id) else {
        return 0;
    };
    if !is_airstrike_attack_type(mst.api_type[2]) {
        return 0;
    }
    let is_torpedo_bomber =
        KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::CarrierBasedTorpedoBomber);
    let stat = if is_torpedo_bomber {
        mst.api_raig.max(0) as f64
    } else {
        mst.api_baku.max(0) as f64
    };
    let bomb_power = stat * (onslot as f64).sqrt();
    if bomb_power <= 0.0 {
        return 0;
    }
    let raw_power = bomb_power + 25.0;
    let capped = apply_cap(raw_power, 170.0) as f64;
    let defense = calculate_defense_power(rng, defender.ship.api_soukou[0]);
    resolve_damage(rng, capped, defense, defender.hp())
}

// ---------------------------------------------------------------------------
// Airstrike phase execution
// ---------------------------------------------------------------------------

fn execute_airstrike_phase(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    is_enemy_side: bool,
    output: &mut AirstrikeOutput,
) {
    // Phase 1: Dive bombing — iterate per bomber slot (non-torpedo types)
    for (ship_idx, ship) in attackers.iter_mut().enumerate() {
        for (slot_idx, slot_item) in ship.slot_items.iter().enumerate() {
            let onslot = ship.ship.api_onslot.get(slot_idx).copied().unwrap_or(0);
            if onslot <= 0 {
                continue;
            }
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id) else {
                continue;
            };
            let Some(type3) = KcSlotItemType3::n(mst.api_type[2]) else {
                continue;
            };
            if !is_airstrike_attack_type(mst.api_type[2]) {
                continue;
            }
            if type3 == KcSlotItemType3::CarrierBasedTorpedoBomber {
                continue;
            }

            let alive_targets: Vec<usize> = defenders
                .iter()
                .enumerate()
                .filter(|(_, s)| s.is_alive())
                .map(|(i, _)| i)
                .collect();
            if alive_targets.is_empty() {
                continue;
            }
            let target_idx = alive_targets[rng.choose_index(alive_targets.len())];
            let damage = calculate_single_slot_airstrike_damage(
                codex,
                rng,
                slot_item,
                onslot,
                &defenders[target_idx],
            );
            if damage > 0 {
                let (_, dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
                output.damage[target_idx] += dealt;
                output.bak_targets[ship_idx] = target_idx as i64;
            }
        }
    }

    // Phase 2: Torpedo bombing — iterate per torpedo bomber slot
    for (ship_idx, ship) in attackers.iter_mut().enumerate() {
        for (slot_idx, slot_item) in ship.slot_items.iter().enumerate() {
            let onslot = ship.ship.api_onslot.get(slot_idx).copied().unwrap_or(0);
            if onslot <= 0 {
                continue;
            }
            let Ok(mst) = codex.find::<ApiMstSlotitem>(&slot_item.api_slotitem_id) else {
                continue;
            };
            if KcSlotItemType3::n(mst.api_type[2])
                != Some(KcSlotItemType3::CarrierBasedTorpedoBomber)
            {
                continue;
            }

            let alive_targets: Vec<usize> = defenders
                .iter()
                .enumerate()
                .filter(|(_, s)| s.is_alive())
                .map(|(i, _)| i)
                .collect();
            if alive_targets.is_empty() {
                continue;
            }
            let target_idx = alive_targets[rng.choose_index(alive_targets.len())];
            let damage = calculate_single_slot_airstrike_damage(
                codex,
                rng,
                slot_item,
                onslot,
                &defenders[target_idx],
            );
            if damage > 0 {
                let (_, dealt) = defenders[target_idx].apply_damage(rng, damage, target_idx);
                output.damage[target_idx] += dealt;
                output.rai_targets[ship_idx] = target_idx as i64;
            }
        }
    }

    // Attribute total damage to best bomber ship (for statistics)
    if !is_enemy_side && let Some(best_idx) = best_bomber_index(codex, attackers) {
        let total: i64 = output.damage.iter().sum();
        attackers[best_idx].damage_dealt += total;
    }
}

// ---------------------------------------------------------------------------
// Full kouku simulation
// ---------------------------------------------------------------------------

pub(crate) fn simulate_kouku(
    codex: &Codex,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    rng: &mut impl BattleRng,
) -> BattleKouku {
    let friend_planes = total_plane_count(codex, friendly);
    let enemy_planes = total_plane_count(codex, enemy);

    let friend_fighter_power = calculate_fighter_power(codex, friendly);
    let enemy_fighter_power = calculate_fighter_power(codex, enemy);
    let air_state = AirState::from_power(friend_fighter_power, enemy_fighter_power);

    // Stage 1: fighter combat — proportional losses based on air state
    let (f_loss_min, f_loss_max) = air_state.stage1_friendly_loss_ratio();
    let (e_loss_min, e_loss_max) = air_state.stage1_enemy_loss_ratio();
    let f_loss_ratio = rng.random_f64_range(f_loss_min, f_loss_max);
    let e_loss_ratio = rng.random_f64_range(e_loss_min, e_loss_max);
    let stage1_f_lost = (friend_planes as f64 * f_loss_ratio).floor() as i64;
    let stage1_e_lost = (enemy_planes as f64 * e_loss_ratio).floor() as i64;

    apply_plane_losses(codex, friendly, stage1_f_lost);
    apply_plane_losses(codex, enemy, stage1_e_lost);

    // Stage 2: anti-air fire — simplified proportional model.
    // NOTE: Real KanColle uses per-ship AA with slot-level shootdowns and fleet AA modifiers.
    // This linear approximation (total_aa / 400 × plane_count) is a known simplification.
    // Should be replaced with per-ship AA calculation before implementing airbattle / ld_airbattle.
    let friend_planes_after_s1 = total_plane_count(codex, friendly);
    let enemy_planes_after_s1 = total_plane_count(codex, enemy);
    let friendly_aa: f64 = friendly.iter().map(|s| s.ship.api_taiku[0].max(0) as f64).sum();
    let enemy_aa: f64 = enemy.iter().map(|s| s.ship.api_taiku[0].max(0) as f64).sum();
    let stage2_f_lost = ((enemy_aa / 400.0) * friend_planes_after_s1 as f64)
        .floor()
        .min(friend_planes_after_s1 as f64) as i64;
    let stage2_e_lost = ((friendly_aa / 400.0) * enemy_planes_after_s1 as f64)
        .floor()
        .min(enemy_planes_after_s1 as f64) as i64;

    apply_plane_losses(codex, friendly, stage2_f_lost);
    apply_plane_losses(codex, enemy, stage2_e_lost);

    // Stage 3: bombing damage
    let mut api_edam = vec![0i64; enemy.len()];
    let mut api_fdam = vec![0i64; friendly.len()];
    let mut api_erai = vec![-1i64; enemy.len()];
    let mut api_ebak = vec![-1i64; enemy.len()];
    let mut api_frai = vec![-1i64; friendly.len()];
    let mut api_fbak = vec![-1i64; friendly.len()];
    let mut api_fcl_flag = vec![0i64; friendly.len()];

    // Stage 3: Per-slot bombing — split into dive bombing and torpedo bombing phases
    // Each bomber slot independently selects a random alive target.
    execute_airstrike_phase(
        codex,
        rng,
        friendly,
        enemy,
        false,
        &mut AirstrikeOutput {
            damage: &mut api_edam,
            bak_targets: &mut api_fbak,
            rai_targets: &mut api_frai,
        },
    );
    execute_airstrike_phase(
        codex,
        rng,
        enemy,
        friendly,
        true,
        &mut AirstrikeOutput {
            damage: &mut api_fdam,
            bak_targets: &mut api_ebak,
            rai_targets: &mut api_erai,
        },
    );
    api_fcl_flag = api_fdam.iter().map(|&d| i64::from(d > 0)).collect();

    BattleKouku {
        api_plane_from: [attack_plane_from(codex, friendly), attack_plane_from(codex, enemy)],
        api_stage1: BattleKoukuStage1 {
            api_f_count: friend_planes,
            api_f_lostcount: stage1_f_lost,
            api_e_count: enemy_planes,
            api_e_lostcount: stage1_e_lost,
            api_disp_seiku: air_state.api_disp_seiku(),
            api_touch_plane: [
                first_touch_plane(codex, friendly).unwrap_or(-1),
                first_touch_plane(codex, enemy).unwrap_or(-1),
            ],
        },
        api_stage2: BattleKoukuStage2 {
            api_f_count: friend_planes_after_s1,
            api_f_lostcount: stage2_f_lost,
            api_e_count: enemy_planes_after_s1,
            api_e_lostcount: stage2_e_lost,
        },
        api_stage3: BattleKoukuStage3 {
            api_frai,
            api_erai,
            api_fbak,
            api_ebak,
            api_fcl_flag,
            api_ecl_flag: api_edam.iter().map(|dam| i64::from(*dam > 0)).collect(),
            api_fdam,
            api_edam,
            api_f_sp_list: vec![None; friendly.len()],
            api_e_sp_list: vec![None; enemy.len()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::BattleRng;
    use crate::test_utils::*;
    use crate::types::BattleRuntimeShip;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    #[test]
    fn fighter_power_calculates_from_equipment_aa_and_slot_count() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fighter_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        let fighter_mst = codex.manifest.find_slotitem(fighter_mst_id).unwrap();
        let aa = fighter_mst.api_tyku;

        let mut ship_input =
            sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 50);
        ship_input.ship.api_onslot = [18, 0, 0, 0, 0];
        ship_input.slot_items = vec![slotitem_with_mst_id(fighter_mst_id)];

        let ships = vec![BattleRuntimeShip::from(ship_input)];
        let power = calculate_fighter_power(&codex, &ships);
        let expected = (aa as f64 * (18.0_f64).sqrt()).floor() as i64;
        assert_eq!(power, expected);
    }

    #[test]
    fn kouku_stage1_reports_nonzero_losses_when_planes_present() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let mut friend = sample_ship(&codex, cvl_mst, 50);
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, cvl_mst, 50);
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        assert!(kouku.api_stage1.api_f_count > 0);
        assert!(kouku.api_stage1.api_e_count > 0);
        let total_f_lost = kouku.api_stage1.api_f_lostcount + kouku.api_stage2.api_f_lostcount;
        let total_e_lost = kouku.api_stage1.api_e_lostcount + kouku.api_stage2.api_e_lostcount;
        assert!(total_f_lost + total_e_lost > 0 || kouku.api_stage1.api_f_count == 0);
    }

    #[test]
    fn kouku_does_not_wipe_all_enemy_planes_unconditionally() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let mut friend = sample_ship(&codex, bb_mst, 50);
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.ship.api_taiku[0] = 10;

        let mut enemy = sample_ship(&codex, cvl_mst, 50);
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);

        let remaining_enemy_planes = total_plane_count(&codex, &enemies);
        assert!(remaining_enemy_planes > 0, "enemy planes should not be fully wiped");
        assert!(kouku.api_stage2.api_e_lostcount < kouku.api_stage2.api_e_count);
    }

    #[test]
    fn kouku_air_state_reflects_fighter_power_balance() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut friend = sample_ship(&codex, cvl_mst, 50);
        let fighter_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        friend.ship.api_onslot = [24, 0, 0, 0, 0];
        friend.slot_items = vec![slotitem_with_mst_id(fighter_mst_id)];
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;

        let mut enemy = sample_ship(&codex, dd_mst, 50);
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;

        let friendly_fp =
            calculate_fighter_power(&codex, &[BattleRuntimeShip::from(friend.clone())]);
        assert!(friendly_fp > 0, "CVL with fighter should have positive fighter power");

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy)];
        let mut rng = crate::random::SeededRng::new(42);

        let kouku = simulate_kouku(&codex, &mut friendly, &mut enemies, &mut rng);
        assert_eq!(kouku.api_stage1.api_disp_seiku, 1); // supremacy
    }
}
