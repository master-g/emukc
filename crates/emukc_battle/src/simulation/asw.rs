//! Opening anti-submarine warfare (OASW) phase simulation.

use emukc_model::codex::Codex;

use crate::accuracy::{Aim, AttackKind, roll_attack};
use crate::damage::calculate_asw_damage;
use crate::random::BattleRng;
use crate::simulation::day_attack::DayAttackKind;
use crate::simulation::shelling::firing_order;
use crate::targeting::{can_opening_asw, select_submarine_target};
use crate::types::{BattleHougeki, BattleRuntimeShip, EngagementType};

/// Simulate the opening ASW phase (先制対潜). The ships of both sides that can
/// open with ASW fire in turn, longest range first, the friendly side first.
pub(crate) fn simulate_opening_taisen(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
) -> Option<BattleHougeki> {
    // 第1艦隊 does not open with ASW in a combined battle; only the escort
    // deck does. `is_main_deck` is false for every single-fleet ship.
    let friendly_order = firing_order(codex, rng, friendly, true, |ship| {
        !ship.is_main_deck() && can_opening_asw(codex, ship)
    });
    let enemy_order = firing_order(codex, rng, enemy, true, |ship| can_opening_asw(codex, ship));

    let mut hougeki = BattleHougeki::default();
    for turn in 0..friendly_order.len().max(enemy_order.len()) {
        if let Some(&idx) = friendly_order.get(turn) {
            let aim = Aim::new(AttackKind::Asw, friendly_formation_id, enemy_formation_id);
            asw_turn(codex, rng, friendly, idx, enemy, aim, engagement, false, &mut hougeki);
        }
        if let Some(&idx) = enemy_order.get(turn) {
            let aim = Aim::new(AttackKind::Asw, enemy_formation_id, friendly_formation_id);
            asw_turn(codex, rng, enemy, idx, friendly, aim, engagement, true, &mut hougeki);
        }
    }

    (!hougeki.api_at_list.is_empty()).then_some(hougeki)
}

/// One ship's opening ASW attack, if it is still afloat and has a submarine to
/// attack.
#[expect(clippy::too_many_arguments)]
fn asw_turn(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    idx: usize,
    defenders: &mut [BattleRuntimeShip],
    aim: Aim,
    engagement: EngagementType,
    attacker_is_enemy: bool,
    hougeki: &mut BattleHougeki,
) {
    let ship = &mut attackers[idx];
    if !can_opening_asw(codex, ship) {
        return;
    }
    let Some(target_idx) = select_submarine_target(codex, rng, defenders) else {
        return;
    };
    let outcome = roll_attack(codex, rng, ship, &defenders[target_idx], aim);
    let raw = calculate_asw_damage(
        codex,
        rng,
        ship,
        &defenders[target_idx],
        aim.attacker_formation,
        engagement,
        outcome,
    );
    let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
    ship.damage_dealt += dealt;
    let shown = if attacker_is_enemy {
        dealt
    } else {
        crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt)
    };

    hougeki.record_day_attack(
        DayAttackKind::Asw(codex, ship),
        attacker_is_enemy,
        idx,
        vec![target_idx as i64],
        vec![shown.into()],
        vec![outcome.cl()],
    );
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

    #[test]
    fn both_sides_open_with_asw_in_turn() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let mut hunter = sample_ship(&codex, dd_mst, 99);
        hunter.ship.api_taisen[0] = 100;
        hunter.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
        let mut submarine = sample_ship(&codex, ss_mst, 50);
        submarine.ship.api_soukou[0] = 400;
        let fleet = || vec![hunter.clone(), hunter.clone(), submarine.clone()];

        let result = crate::simulation::simulate_day(
            &codex,
            BattleContext::head_on(BattleType::Normal, true, fleet(), fleet()),
            &mut crate::random::SeededRng::new(1),
        );

        assert_eq!(result.packet.opening_taisen.unwrap().api_at_eflag, [0, 1, 0, 1]);
    }
}
