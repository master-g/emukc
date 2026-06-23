//! Opening anti-submarine warfare (OASW) phase simulation.

use emukc_model::codex::Codex;

use crate::damage::calculate_asw_damage;
use crate::random::BattleRng;
use crate::targeting::{can_opening_asw, day_attack_display_ids, select_submarine_target};
use crate::types::{BattleHougeki, BattleRuntimeShip, EngagementType};

/// Simulate the opening ASW phase (先制対潜).
pub(crate) fn simulate_opening_taisen(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
) -> Option<BattleHougeki> {
    let mut at_eflag = Vec::new();
    let mut at_list = Vec::new();
    let mut at_type = Vec::new();
    let mut df_list = Vec::new();
    let mut si_list = Vec::new();
    let mut cl_list = Vec::new();
    let mut damage = Vec::new();

    // Friendly OASW attacks
    for (idx, ship) in friendly.iter_mut().enumerate() {
        if !can_opening_asw(codex, ship) {
            continue;
        }
        let Some(target_idx) = select_submarine_target(codex, rng, enemy) else {
            continue;
        };
        let raw = calculate_asw_damage(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
        );
        let (raw_dmg, dealt) = enemy[target_idx].apply_damage(rng, raw, target_idx);
        ship.damage_dealt += dealt;
        let display = crate::targeting::display_damage(&enemy[target_idx], raw_dmg, dealt);

        at_eflag.push(0);
        at_list.push(idx as i64);
        at_type.push(7); // ASW attack type
        df_list.push(vec![target_idx as i64]);
        si_list.push(day_attack_display_ids(codex, ship, true));
        cl_list.push(vec![1]);
        damage.push(vec![display]);
    }

    // Enemy OASW attacks
    for (idx, ship) in enemy.iter_mut().enumerate() {
        if !can_opening_asw(codex, ship) {
            continue;
        }
        let Some(target_idx) = select_submarine_target(codex, rng, friendly) else {
            continue;
        };
        let raw = calculate_asw_damage(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
        );
        let (_, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
        ship.damage_dealt += dealt;

        at_eflag.push(1);
        at_list.push(idx as i64);
        at_type.push(7);
        df_list.push(vec![target_idx as i64]);
        si_list.push(day_attack_display_ids(codex, ship, true));
        cl_list.push(vec![1]);
        damage.push(vec![dealt]);
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
    fn oasw_fires_in_day_battle_when_conditions_met() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_taisen[0] = 100;
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];

        let mut enemy = sample_ship(&codex, ss_mst, 50);
        enemy.ship.api_soukou[0] = 5;
        enemy.ship.api_nowhp = 30;
        enemy.ship.api_maxhp = 30;

        let context = BattleContext {
            god_mode: false,
            one_hit_kill: false,
            battle_type: BattleType::Normal,
            is_sortie: true,
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            friend_ships: vec![friend],
            enemy_ships: vec![enemy],
        };

        let result =
            crate::simulation::simulate_day(&codex, context, &mut crate::random::SeededRng::new(1));
        assert_eq!(result.packet.opening_taisen_flag, 1);
        assert!(result.packet.opening_taisen.is_some());

        let taisen = result.packet.opening_taisen.unwrap();
        assert_eq!(taisen.api_at_eflag, vec![0]);
        assert_eq!(taisen.api_at_type, vec![7]);
        assert!(taisen.api_damage[0][0] >= 1);
    }
}
