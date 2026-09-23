//! Opening anti-submarine warfare (OASW) phase simulation.

use emukc_model::codex::Codex;

use crate::damage::calculate_asw_damage;
use crate::random::BattleRng;
use crate::simulation::day_attack::DayAttackKind;
use crate::targeting::{can_opening_asw, select_submarine_target};
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
    let mut hougeki = BattleHougeki::default();

    // Friendly OASW attacks
    for (idx, ship) in friendly.iter_mut().enumerate() {
        // 第1艦隊 does not open with ASW in a combined battle; only the escort
        // deck does. `is_main_deck` is false for every single-fleet ship, so
        // this skips nothing outside a combined battle — and because it sits
        // ahead of every RNG draw, it cannot shift the single-fleet stream.
        if ship.is_main_deck() {
            continue;
        }
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

        hougeki.record_day_attack(
            DayAttackKind::Asw(codex, ship),
            false,
            idx,
            vec![target_idx as i64],
            vec![display.into()],
        );
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

        hougeki.record_day_attack(
            DayAttackKind::Asw(codex, ship),
            true,
            idx,
            vec![target_idx as i64],
            vec![dealt.into()],
        );
    }

    (!hougeki.api_at_list.is_empty()).then_some(hougeki)
}

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::types::{BattleContext, BattleType};
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

        let context = BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]);

        let result =
            crate::simulation::simulate_day(&codex, context, &mut crate::random::SeededRng::new(1));
        assert_eq!(result.packet.opening_taisen_flag, 1);
        assert!(result.packet.opening_taisen.is_some());

        let taisen = result.packet.opening_taisen.unwrap();
        assert_eq!(taisen.api_at_eflag, vec![0]);
        // R1: the client hands `api_opening_taisen` to `PhasePreAntiSubmarine`,
        // whose dispatch only knows 0 (normal) and 2 (double); everything else
        // reaches `PhaseAttackDanchaku`, which throws on anything outside
        // {3,4,5,6,200,201}. 7 is the client's carrier cut-in type, not an ASW
        // type. Depth charge vs. ASW-plane animation is the client's own call.
        assert_eq!(taisen.api_at_type, vec![0]);
        assert!(taisen.api_damage[0][0].amount() >= 1);
    }

    /// R1: the enemy side of the OASW loop reports the same attack type. An
    /// enemy escort with a sonar opening on a friendly submarine must produce
    /// `api_at_eflag = 1` entries whose `api_at_type` is 0.
    #[test]
    fn enemy_oasw_reports_attack_type_zero() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let mut friend = sample_ship(&codex, ss_mst, 50);
        friend.ship.api_soukou[0] = 5;
        friend.ship.api_nowhp = 30;
        friend.ship.api_maxhp = 30;

        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_taisen[0] = 100;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 200;
        enemy.ship.api_maxhp = 200;
        enemy.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];

        let context = BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]);

        let result =
            crate::simulation::simulate_day(&codex, context, &mut crate::random::SeededRng::new(1));
        let taisen = result.packet.opening_taisen.expect("enemy OASW must fire");

        let enemy_types: Vec<i64> = taisen
            .api_at_eflag
            .iter()
            .zip(taisen.api_at_type.iter())
            .filter(|(eflag, _)| **eflag == 1)
            .map(|(_, at_type)| *at_type)
            .collect();
        assert!(!enemy_types.is_empty(), "expected at least one enemy OASW entry");
        assert!(
            enemy_types.iter().all(|t| *t == 0),
            "enemy OASW must report api_at_type 0: {enemy_types:?}"
        );
    }
}
