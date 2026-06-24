//! Day shelling phase simulation.

use emukc_model::codex::Codex;

use crate::damage::{calculate_asw_damage, calculate_shelling_damage};
use crate::random::BattleRng;
use crate::simulation::day_cutin::{DayAttackType, carrier_ci_display_ids, resolve_day_attack};
use crate::simulation::special_attack;
use crate::targeting::{
    can_shell_day_ship, day_attack_display_ids, select_random_target_index, target_class,
};
use crate::types::{BattleHougeki, BattleRuntimeShip, ShellingParams, SiListId};

/// Maximum ships per fleet (single fleet, not combined). Caps the special-attack skip array.
const MAX_FLEET_SIZE: usize = 6;

/// Simulate one side's shelling attacks in a day battle.
pub(crate) fn simulate_shelling_side(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    params: &ShellingParams,
) -> Option<BattleHougeki> {
    let fleet_los = attackers.iter().map(|s| s.ship.api_sakuteki[0].max(0)).sum();

    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut at_type = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut damage = Vec::new();
    let mut special_attack_skip = [false; MAX_FLEET_SIZE];

    // Try flagship special attack before normal shelling loop
    if let Some(resolved) =
        special_attack::try_special_attack(codex, rng, attackers, params.formation_id)
    {
        let result = special_attack::execute_special_attack(
            codex, rng, attackers, defenders, resolved, params,
        );
        at_eflag.extend(result.hougeki.api_at_eflag);
        at_list.extend(result.hougeki.api_at_list);
        at_type.extend(result.hougeki.api_at_type);
        df_list.extend(result.hougeki.api_df_list);
        si_list.extend(result.hougeki.api_si_list);
        cl_list.extend(result.hougeki.api_cl_list);
        damage.extend(result.hougeki.api_damage);
        for &i in &result.participant_indices {
            debug_assert!(
                i < MAX_FLEET_SIZE,
                "special_attack participant index {i} exceeds MAX_FLEET_SIZE; combined fleets need a wider skip array"
            );
            if i < MAX_FLEET_SIZE {
                special_attack_skip[i] = true;
            }
        }
    }

    for (idx, ship) in attackers.iter_mut().enumerate() {
        if idx < MAX_FLEET_SIZE && special_attack_skip[idx] {
            continue;
        }
        if !can_shell_day_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, defenders, params.phase)
        else {
            continue;
        };
        let is_asw_attack = target_class(codex, &defenders[target_idx]).is_submarine();

        if is_asw_attack {
            let raw = calculate_asw_damage(
                codex,
                rng,
                ship,
                &defenders[target_idx],
                params.formation_id,
                params.engagement,
            );
            let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
            if !params.attacker_is_enemy {
                ship.damage_dealt += dealt;
            }
            let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);

            push_attack(
                &mut at_eflag,
                &mut at_list,
                &mut at_type,
                &mut df_list,
                &mut si_list,
                &mut cl_list,
                &mut damage,
                params.attacker_is_enemy,
                idx,
                7,
                vec![target_idx as i64],
                SiListId::num_from_i64(&day_attack_display_ids(codex, ship, true)),
                vec![display],
            );
        } else {
            let resolved = resolve_day_attack(codex, rng, ship, params.air_state, fleet_los, idx);

            let ci_mult = if resolved.damage_multiplier != 1.0 {
                Some(resolved.damage_multiplier)
            } else {
                None
            };

            if resolved.hit_count == 2 {
                // DoubleAttack: 2 hits on the same target
                let mut damages = Vec::with_capacity(2);
                for _ in 0..2 {
                    let raw = calculate_shelling_damage(
                        codex,
                        rng,
                        ship,
                        &defenders[target_idx],
                        params.formation_id,
                        params.engagement,
                        ci_mult,
                    );
                    let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
                    if !params.attacker_is_enemy {
                        ship.damage_dealt += dealt;
                    }
                    damages.push(crate::targeting::display_damage(
                        &defenders[target_idx],
                        raw_dmg,
                        dealt,
                    ));
                }
                push_attack(
                    &mut at_eflag,
                    &mut at_list,
                    &mut at_type,
                    &mut df_list,
                    &mut si_list,
                    &mut cl_list,
                    &mut damage,
                    params.attacker_is_enemy,
                    idx,
                    resolved.at_type as i64,
                    vec![target_idx as i64; 2],
                    SiListId::text_from_i64(&day_attack_display_ids(codex, ship, false)),
                    damages,
                );
            } else {
                let raw = calculate_shelling_damage(
                    codex,
                    rng,
                    ship,
                    &defenders[target_idx],
                    params.formation_id,
                    params.engagement,
                    ci_mult,
                );
                let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
                if !params.attacker_is_enemy {
                    ship.damage_dealt += dealt;
                }
                let display =
                    crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
                let display_ids = if resolved.at_type == DayAttackType::CarrierCI {
                    SiListId::text_from_i64(&carrier_ci_display_ids(
                        codex,
                        ship,
                        resolved.carrier_sub.expect("CarrierCI must have sub-type"),
                    ))
                } else if resolved.at_type != DayAttackType::Normal {
                    // Artillery spotting CI (at_type 3-6)
                    SiListId::text_from_i64(&day_attack_display_ids(codex, ship, false))
                } else {
                    SiListId::num_from_i64(&day_attack_display_ids(codex, ship, false))
                };
                push_attack(
                    &mut at_eflag,
                    &mut at_list,
                    &mut at_type,
                    &mut df_list,
                    &mut si_list,
                    &mut cl_list,
                    &mut damage,
                    params.attacker_is_enemy,
                    idx,
                    resolved.at_type as i64,
                    vec![target_idx as i64],
                    display_ids,
                    vec![display],
                );
            }
        }
    }

    (!at_list.is_empty()).then_some(BattleHougeki {
        api_at_eflag: at_eflag,
        api_at_list: at_list,
        api_at_type: at_type,
        api_df_list: df_list,
        api_si_list: si_list,
        api_cl_list: cl_list,
        api_damage: damage,
    })
}

#[allow(clippy::too_many_arguments)]
fn push_attack(
    at_eflag: &mut Vec<i64>,
    at_list: &mut Vec<i64>,
    at_type: &mut Vec<i64>,
    df_list: &mut Vec<Vec<i64>>,
    si_list: &mut Vec<Vec<SiListId>>,
    cl_list: &mut Vec<Vec<i64>>,
    damage: &mut Vec<Vec<i64>>,
    attacker_is_enemy: bool,
    attacker_idx: usize,
    attack_type: i64,
    targets: Vec<i64>,
    display_ids: Vec<SiListId>,
    damages: Vec<i64>,
) {
    at_eflag.push(i64::from(attacker_is_enemy));
    at_list.push(attacker_idx as i64);
    at_type.push(attack_type);
    df_list.push(targets);
    si_list.push(display_ids);
    cl_list.push(vec![1; damages.len()]);
    damage.push(damages);
}

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::types::{BattleContext, BattleType, EngagementType};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    #[test]
    fn fighter_only_carrier_does_not_shell_in_day_battle() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let fighter_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);

        let mut carrier = sample_ship(&codex, carrier_mst, 50);
        carrier.slot_items = vec![slotitem_with_mst_id(fighter_id)];
        carrier.ship.api_onslot = [18, 0, 0, 0, 0];
        let bb = sample_ship(&codex, bb_mst, 50);
        let enemy = sample_ship(&codex, dd_mst, 50);

        let simulation = crate::simulation::simulate_day(
            &codex,
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: false,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![carrier, bb],
                enemy_ships: vec![enemy],
            },
            &mut crate::random::SeededRng::new(1),
        );

        // Verify no friendly shelling attack came from index 0 (carrier)
        // regardless of which side goes first due to fleet speed
        let all_at_eflags: Vec<i64> = simulation
            .packet
            .hougeki1
            .iter()
            .chain(simulation.packet.hougeki2.iter())
            .flat_map(|h| h.api_at_eflag.iter().copied())
            .collect();
        let all_at_lists: Vec<i64> = simulation
            .packet
            .hougeki1
            .iter()
            .chain(simulation.packet.hougeki2.iter())
            .flat_map(|h| h.api_at_list.iter().copied())
            .collect();
        // The carrier (index 0 in friendly fleet) should never appear as attacker
        // when eflag=0 (friendly). BB (index 1) should be the one shelling.
        let friendly_attacks: Vec<i64> = all_at_eflags
            .iter()
            .zip(all_at_lists.iter())
            .filter(|(ef, _)| **ef == 0)
            .map(|(_, idx)| *idx)
            .collect();
        assert!(
            !friendly_attacks.contains(&0),
            "carrier with only fighters should not shell: {friendly_attacks:?}"
        );
    }

    #[test]
    fn day_ci_produces_nonzero_at_type() {
        use crate::simulation::day_cutin::{
            DayAttackType, detect_day_attack_type, resolve_day_attack,
        };
        use crate::types::AirState;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let ap_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::ArmorPiercingShell);
        let seaplane_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedRecon);

        // BB: 2 main guns + AP shell + seaplane
        let mut bb = sample_ship(&codex, bb_mst, 99);
        bb.slot_items = vec![
            slotitem_with_mst_id(main_gun_id),
            slotitem_with_mst_id(main_gun_id),
            slotitem_with_mst_id(ap_id),
            slotitem_with_mst_id(seaplane_id),
        ];
        bb.ship.api_onslot = [0, 0, 0, 1, 0];
        let rt = crate::types::BattleRuntimeShip::from(bb);

        // Detection should succeed
        let detected = detect_day_attack_type(&codex, &rt, Some(&AirState::Supremacy));
        assert_eq!(detected, Some(DayAttackType::MainApMainCI));

        // Trigger roll with supremacy and flagship bonus should succeed
        let fleet_los = rt.ship.api_sakuteki[0].max(0);

        // Find a seed that triggers CI
        let mut resolved = None;
        for seed in 0..100u64 {
            let r = resolve_day_attack(
                &codex,
                &mut crate::random::SeededRng::new(seed),
                &rt.clone(),
                Some(&AirState::Supremacy),
                fleet_los,
                0, // flagship
            );
            if r.at_type != DayAttackType::Normal {
                resolved = Some(r);
                break;
            }
        }
        let resolved = resolved.expect("at least one seed should trigger CI");
        assert!(
            resolved.at_type != DayAttackType::Normal,
            "resolved should be CI or DoubleAttack, got {:?}",
            resolved.at_type
        );
    }

    #[test]
    fn special_attack_skip_marks_participants_and_spares_others() {
        // Mirror the production loop in `simulate_shelling_side`: when a special attack
        // produces participant indices 0/2/4, the skip array must mark exactly those
        // slots as true. Indices 1/3/5 (and any future slot) must remain attackable.
        const MAX_FLEET_SIZE: usize = super::MAX_FLEET_SIZE;
        let participant_indices = vec![0_usize, 2, 4];

        let mut special_attack_skip = [false; MAX_FLEET_SIZE];
        for &i in &participant_indices {
            if i < MAX_FLEET_SIZE {
                special_attack_skip[i] = true;
            }
        }

        for (idx, &skip) in special_attack_skip.iter().enumerate() {
            let should_skip = participant_indices.contains(&idx);
            assert_eq!(skip, should_skip, "idx {idx}: expected skip={should_skip}, got {skip}",);
        }
    }
}
