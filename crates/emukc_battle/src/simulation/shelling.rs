//! Day shelling phase simulation.

use emukc_model::codex::Codex;

use crate::damage::{calculate_asw_damage, calculate_shelling_damage};
use crate::random::BattleRng;
use crate::targeting::{
    can_shell_day_ship, day_attack_display_ids, select_random_target_index, target_class,
};
use crate::types::{BattleHougeki, BattleRuntimeShip, ShellingParams};

/// Simulate one side's shelling attacks in a day battle.
pub(crate) fn simulate_shelling_side(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    params: &ShellingParams,
) -> Option<BattleHougeki> {
    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut at_type = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut damage = Vec::new();

    for (idx, ship) in attackers.iter_mut().enumerate() {
        if !can_shell_day_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, defenders, params.phase)
        else {
            continue;
        };
        let is_asw_attack = target_class(codex, &defenders[target_idx]).is_submarine();
        let raw = if is_asw_attack {
            calculate_asw_damage(
                codex,
                rng,
                ship,
                &defenders[target_idx],
                params.formation_id,
                params.engagement,
            )
        } else {
            calculate_shelling_damage(
                codex,
                rng,
                ship,
                &defenders[target_idx],
                params.formation_id,
                params.engagement,
            )
        };
        let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
        if !params.attacker_is_enemy {
            ship.damage_dealt += dealt;
        }
        let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);

        at_eflag.push(i64::from(params.attacker_is_enemy));
        at_list.push(idx as i64);
        at_type.push(if is_asw_attack {
            7
        } else {
            0
        });
        df_list.push(vec![target_idx as i64]);
        si_list.push(day_attack_display_ids(codex, ship, is_asw_attack));
        cl_list.push(vec![1]);
        damage.push(vec![display]);
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
}
