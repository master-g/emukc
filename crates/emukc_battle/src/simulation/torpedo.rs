//! Torpedo phase simulation (opening torpedo + closing torpedo / raigeki).

use emukc_model::codex::Codex;

use crate::damage::calculate_torpedo_damage;
use crate::random::BattleRng;
use crate::targeting::{
    can_closing_torpedo_ship, can_opening_torpedo_ship, select_random_target_index,
};
use crate::types::{
    BattleOpeningAttack, BattlePhase, BattleRaigeki, BattleRuntimeShip, EngagementType,
    TorpedoAttackerSide, TorpedoHit,
};

/// Simulate the opening torpedo phase.
pub(crate) fn simulate_opening_torpedo(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
) -> Option<BattleOpeningAttack> {
    let fleet_size = friendly.len().max(enemy.len());
    let mut payload = BattleOpeningAttack::blank(fleet_size);
    let mut happened = false;

    for (idx, ship) in friendly.iter_mut().enumerate() {
        if !can_opening_torpedo_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, enemy, BattlePhase::OpeningTorpedo)
        else {
            continue;
        };
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
            BattlePhase::OpeningTorpedo,
        );
        let (raw_dmg, dealt) = enemy[target_idx].apply_damage(rng, raw, target_idx);
        ship.damage_dealt += dealt;
        let display = crate::targeting::display_damage(&enemy[target_idx], raw_dmg, dealt);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Friendly,
            TorpedoHit {
                attacker_index: idx,
                defender_index: target_idx,
                damage: display,
            },
        );
        happened = true;
    }

    for (idx, ship) in enemy.iter_mut().enumerate() {
        if !can_opening_torpedo_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, friendly, BattlePhase::OpeningTorpedo)
        else {
            continue;
        };
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
            BattlePhase::OpeningTorpedo,
        );
        let (raw_dmg, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
        ship.damage_dealt += dealt;
        let display = crate::targeting::display_damage(&friendly[target_idx], raw_dmg, dealt);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: idx,
                defender_index: target_idx,
                damage: display,
            },
        );
        happened = true;
    }

    happened.then_some(payload)
}

/// Simulate the closing torpedo (raigeki) phase.
pub(crate) fn simulate_raigeki(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
) -> Option<BattleRaigeki> {
    let fleet_size = friendly.len().max(enemy.len());
    let mut payload = BattleRaigeki::blank(fleet_size);
    let mut happened = false;

    for (idx, ship) in friendly.iter_mut().enumerate() {
        if !can_closing_torpedo_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, enemy, BattlePhase::ClosingTorpedo)
        else {
            continue;
        };
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
            BattlePhase::ClosingTorpedo,
        );
        let (raw_dmg, dealt) = enemy[target_idx].apply_damage(rng, raw, target_idx);
        ship.damage_dealt += dealt;
        let display = crate::targeting::display_damage(&enemy[target_idx], raw_dmg, dealt);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Friendly,
            TorpedoHit {
                attacker_index: idx,
                defender_index: target_idx,
                damage: display,
            },
        );
        happened = true;
    }

    for (idx, ship) in enemy.iter_mut().enumerate() {
        if !can_closing_torpedo_ship(codex, ship) {
            continue;
        }
        let Some(target_idx) =
            select_random_target_index(codex, rng, ship, friendly, BattlePhase::ClosingTorpedo)
        else {
            continue;
        };
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
            BattlePhase::ClosingTorpedo,
        );
        let (raw_dmg, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
        ship.damage_dealt += dealt;
        let display = crate::targeting::display_damage(&friendly[target_idx], raw_dmg, dealt);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: idx,
                defender_index: target_idx,
                damage: display,
            },
        );
        happened = true;
    }

    happened.then_some(payload)
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::SeededRng;
    use crate::test_utils::*;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;

    #[test]
    fn only_opening_torpedo_capable_ship_participates() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let dd = sample_ship(&codex, dd_mst, 50);
        let clt = sample_ship(&codex, clt_mst, 50);
        let enemy = sample_ship(&codex, bb_mst, 50);

        let simulation = crate::simulation::simulate_day(
            &codex,
            crate::types::BattleContext {
                battle_type: crate::types::BattleType::Normal,
                is_sortie: false,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![dd, clt],
                enemy_ships: vec![enemy],
            },
            &mut crate::random::SeededRng::new(1),
        );

        let opening = simulation.packet.opening_attack.unwrap();
        assert!(opening.api_frai_list_items[0].is_none());
        assert!(opening.api_frai_list_items[1].is_some());
    }

    #[test]
    fn opening_torpedo_friendly_damage_uses_fydam_list_items() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let mut friendly = vec![BattleRuntimeShip::from(sample_ship(&codex, clt_mst, 50))];
        let mut enemy = vec![BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50))];
        let mut rng = SeededRng::new(1);

        let opening = simulate_opening_torpedo(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            1,
            1,
            EngagementType::SameCourse,
        )
        .unwrap();

        let dealt = opening.api_edam[0];
        assert!(dealt > 0);
        assert_eq!(opening.api_frai_list_items[0], Some(vec![0]));
        assert_eq!(opening.api_fydam_list_items[0], Some(vec![dealt]));
        assert_eq!(opening.api_eydam_list_items[0], None);
        assert_eq!(enemy[0].hp(), enemy[0].ship.api_nowhp - dealt);
    }

    #[test]
    fn opening_torpedo_enemy_damage_uses_eydam_list_items() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt_mst = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let mut friendly = vec![BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50))];
        let mut enemy = vec![BattleRuntimeShip::from(sample_ship(&codex, clt_mst, 50))];
        let mut rng = SeededRng::new(1);

        let opening = simulate_opening_torpedo(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            1,
            1,
            EngagementType::SameCourse,
        )
        .unwrap();

        let dealt = opening.api_fdam[0];
        assert!(dealt > 0);
        assert_eq!(opening.api_erai_list_items[0], Some(vec![0]));
        assert_eq!(opening.api_eydam_list_items[0], Some(vec![dealt]));
        assert_eq!(opening.api_fydam_list_items[0], None);
        assert_eq!(friendly[0].hp(), friendly[0].ship.api_nowhp - dealt);
    }

    #[test]
    fn closing_torpedo_friendly_damage_uses_fydam() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let mut friendly = vec![BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50))];
        let mut enemy = vec![BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50))];
        let mut rng = SeededRng::new(1);

        let raigeki = simulate_raigeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            1,
            1,
            EngagementType::SameCourse,
        )
        .unwrap();

        let dealt = raigeki.api_edam[0];
        assert!(dealt > 0);
        assert_eq!(raigeki.api_frai[0], 0);
        assert_eq!(raigeki.api_fydam[0], dealt);
        assert_eq!(raigeki.api_eydam[0], 0);
        assert_eq!(enemy[0].hp(), enemy[0].ship.api_nowhp - dealt);
    }

    #[test]
    fn closing_torpedo_enemy_damage_uses_eydam() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);

        let mut friendly = vec![BattleRuntimeShip::from(sample_ship(&codex, bb_mst, 50))];
        let mut enemy = vec![BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 50))];
        let mut rng = SeededRng::new(1);

        let raigeki = simulate_raigeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            1,
            1,
            EngagementType::SameCourse,
        )
        .unwrap();

        let dealt = raigeki.api_fdam[0];
        assert!(dealt > 0);
        assert_eq!(raigeki.api_erai[0], 0);
        assert_eq!(raigeki.api_eydam[0], dealt);
        assert_eq!(raigeki.api_fydam[0], 0);
        assert_eq!(friendly[0].hp(), friendly[0].ship.api_nowhp - dealt);
    }
}
