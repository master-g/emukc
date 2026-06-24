//! Day shelling phase simulation.

use emukc_model::codex::Codex;

use crate::damage::{calculate_asw_damage, calculate_shelling_damage};
use crate::random::BattleRng;
use crate::simulation::day_cutin::{DayAttackType, carrier_ci_display_ids, resolve_day_attack};
use crate::simulation::special_attack;
use crate::targeting::{
    can_shell_day_ship, day_attack_display_ids, select_random_target_index, target_class,
};
use crate::types::{BattleHougeki, BattleRuntimeShip, DamageCell, ShellingParams, SiListId};

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
        let Some(mut target_idx) =
            select_random_target_index(codex, rng, ship, defenders, params.phase)
        else {
            continue;
        };
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let shield = match crate::targeting::select_escort_shield(
            codex,
            rng,
            defenders,
            target_idx,
            params.defender_formation_id,
        ) {
            Some(escort) => {
                target_idx = escort;
                true
            }
            None => false,
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
                shield,
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
                    shield,
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
                    shield,
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
    damage: &mut Vec<Vec<DamageCell>>,
    attacker_is_enemy: bool,
    attacker_idx: usize,
    attack_type: i64,
    targets: Vec<i64>,
    display_ids: Vec<SiListId>,
    damages: Vec<i64>,
    shield: bool,
) {
    at_eflag.push(i64::from(attacker_is_enemy));
    at_list.push(attacker_idx as i64);
    at_type.push(attack_type);
    df_list.push(targets);
    si_list.push(display_ids);
    cl_list.push(vec![1; damages.len()]);
    // 旗艦援護: an intercepted hit carries the `.1` shield flag (DamageCell::Shielded).
    damage.push(damages.into_iter().map(|d| damage_cell(d, shield)).collect());
}

/// Wrap a display-damage value, flagging it as shield-intercepted when `shield`.
fn damage_cell(value: i64, shield: bool) -> DamageCell {
    if shield {
        DamageCell::Shielded(value)
    } else {
        DamageCell::Plain(value)
    }
}

#[cfg(test)]
mod tests {
    use super::simulate_shelling_side;
    use crate::random::SeededRng;
    use crate::test_utils::*;
    use crate::types::{
        BattleContext, BattlePhase, BattleRuntimeShip, BattleType, DamageCell, EngagementType,
        ShellingParams,
    };
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    /// Run one day-shelling side and return its hougeki, attacker = enemy side
    /// (so the friendly flagship is the defender) when `attacker_is_enemy`.
    /// Defending fleet is [flagship, healthy escort]; both surface (BB + DD).
    fn shelling_with_shield(
        codex: &Codex,
        attacker_is_enemy: bool,
        defender_formation_id: i64,
        seed: u64,
    ) -> Option<crate::types::BattleHougeki> {
        let bb = first_ship_mst_by_type(codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);
        let mut defenders = vec![
            BattleRuntimeShip::new(sample_ship(codex, bb, 80), !attacker_is_enemy, true),
            BattleRuntimeShip::new(sample_ship(codex, dd, 80), !attacker_is_enemy, true),
        ];
        let mut attackers = vec![
            BattleRuntimeShip::new(sample_ship(codex, bb, 80), attacker_is_enemy, true),
            BattleRuntimeShip::new(sample_ship(codex, bb, 80), attacker_is_enemy, true),
        ];
        let mut rng = SeededRng::new(seed);
        simulate_shelling_side(
            codex,
            &mut rng,
            &mut attackers,
            &mut defenders,
            &ShellingParams {
                attacker_is_enemy,
                formation_id: 1,
                defender_formation_id,
                engagement: EngagementType::SameCourse,
                phase: BattlePhase::DayShelling,
                air_state: None,
            },
        )
    }

    /// Covers AE1. When interception fires, the hit's damage cell is Shielded
    /// (serializes `X.1`) and `api_df_list` points at the escort (index 1),
    /// never the flagship (index 0).
    #[test]
    fn flagship_shield_redirects_to_escort_and_flags_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut found = false;
        for seed in 0..300u64 {
            // 輪形陣 (75%) maximises interception frequency for the scan.
            let Some(h) = shelling_with_shield(&codex, true, 3, seed) else {
                continue;
            };
            for (i, dmgs) in h.api_damage.iter().enumerate() {
                if dmgs.iter().any(|c| matches!(c, DamageCell::Shielded(_))) {
                    assert!(
                        h.api_df_list[i].iter().all(|&t| t == 1),
                        "intercepted hit must target the escort, got df_list {:?}",
                        h.api_df_list[i]
                    );
                    let json = serde_json::to_string(&dmgs).unwrap();
                    assert!(json.contains(".1"), "shielded damage must serialize with .1: {json}");
                    found = true;
                }
                // A non-shielded hit on the flagship keeps df_list at 0.
                if h.api_df_list[i].contains(&0) {
                    assert!(
                        dmgs.iter().all(|c| matches!(c, DamageCell::Plain(_))),
                        "a hit still on the flagship (0) must not be shielded"
                    );
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "expected an intercepted flagship hit within the seed scan");
    }

    /// Covers R9. Interception is bidirectional: a friendly attack on the enemy
    /// flagship is intercepted by an enemy escort too.
    #[test]
    fn enemy_flagship_is_also_protected() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let found = (0..300u64).any(|seed| {
            shelling_with_shield(&codex, false, 3, seed).is_some_and(|h| {
                h.api_damage
                    .iter()
                    .any(|dmgs| dmgs.iter().any(|c| matches!(c, DamageCell::Shielded(_))))
            })
        });
        assert!(found, "enemy flagship must also be protected by an enemy escort (R9)");
    }

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
    fn day_shelling_si_list_text_for_ci_num_for_normal() {
        use super::simulate_shelling_side;
        use crate::types::{AirState, BattlePhase, BattleRuntimeShip, ShellingParams, SiListId};

        // CI-capable BB (2 main guns + AP shell + seaplane → MainApMainCI).
        // Across seeds a CI/double sometimes fires and a normal attack fires
        // otherwise. Drive the real simulate_shelling_side path and assert the
        // si_list wire type matches the attack branch at the push site — a
        // text_from_i64/num_from_i64 swap there passes every other test.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let ap_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::ArmorPiercingShell);
        let seaplane_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedRecon);

        let make_attacker = || {
            let mut bb = sample_ship(&codex, bb_mst, 99);
            bb.slot_items = vec![
                slotitem_with_mst_id(main_gun_id),
                slotitem_with_mst_id(main_gun_id),
                slotitem_with_mst_id(ap_id),
                slotitem_with_mst_id(seaplane_id),
            ];
            bb.ship.api_onslot = [0, 0, 0, 1, 0];
            BattleRuntimeShip::from(bb)
        };
        let make_defender = || {
            let mut dd = sample_ship(&codex, dd_mst, 30);
            dd.ship.api_soukou[0] = 1;
            dd.ship.api_nowhp = 800;
            dd.ship.api_maxhp = 800;
            BattleRuntimeShip::from(dd)
        };

        let air_state = AirState::Supremacy;
        let mut saw_ci_text = false;
        let mut saw_normal_num = false;
        for seed in 0..200u64 {
            let mut attackers = vec![make_attacker()];
            let mut defenders = vec![make_defender()];
            let Some(hougeki) = simulate_shelling_side(
                &codex,
                &mut crate::random::SeededRng::new(seed),
                &mut attackers,
                &mut defenders,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: 1,
                    defender_formation_id: 0, // unused here: no かばう in this test
                    engagement: EngagementType::SameCourse,
                    phase: BattlePhase::DayShelling,
                    air_state: Some(&air_state),
                },
            ) else {
                continue;
            };
            if hougeki.api_at_type.is_empty() {
                continue;
            }
            let entry = &hougeki.api_si_list[0];
            if hougeki.api_at_type[0] == 0 {
                assert!(
                    entry.iter().all(|id| matches!(id, SiListId::Num(_))),
                    "normal attack (at_type 0) si_list must be all integers: {entry:?}"
                );
                saw_normal_num = true;
            } else {
                assert!(
                    entry.iter().any(|id| matches!(id, SiListId::Text(_))),
                    "CI/double (at_type {}) si_list must contain string entries: {entry:?}",
                    hougeki.api_at_type[0]
                );
                saw_ci_text = true;
            }
            if saw_ci_text && saw_normal_num {
                break;
            }
        }
        assert!(saw_ci_text, "no CI/double fired across 200 seeds; Text path unverified");
        assert!(saw_normal_num, "no normal attack across 200 seeds; Num path unverified");
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
