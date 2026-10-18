//! Torpedo phase simulation (opening torpedo + closing torpedo / raigeki).

use emukc_model::codex::Codex;

use crate::accuracy::{Aim, AttackKind, roll_attack};
use crate::damage::{calculate_torpedo_damage, torpedo_attack_power};
use crate::random::BattleRng;
use crate::targeting::{
    can_closing_torpedo_ship, can_opening_torpedo_ship, choice_with_protect, deck_candidates,
    target_candidates,
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

    // Every ship that could fire when the phase began fires: what the enemy
    // sends is settled from the enemy as it was, not as the friendly torpedoes
    // resolved just before leave it.
    let enemy_at_start = enemy.to_vec();

    for (idx, ship) in friendly.iter_mut().enumerate() {
        // 第1艦隊 never torpedoes in a combined battle, opening or closing.
        // (Upstream has a narrow exception — a handful of ships can open-torpedo
        // from the main deck, `canOpTorpMain` in `kcsim.js:1811` — which is not
        // modelled here.) No-op outside a combined battle, and ahead of every
        // RNG draw.
        if ship.is_main_deck() {
            continue;
        }
        if !can_opening_torpedo_ship(codex, ship) {
            continue;
        }
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let candidates = target_candidates(codex, ship, enemy, BattlePhase::OpeningTorpedo);
        let Some((target_idx, shield)) =
            choice_with_protect(codex, rng, enemy, &candidates, 0, false, enemy_formation_id)
        else {
            continue;
        };
        // A fifth of the torpedo's power goes into its accuracy.
        let power = torpedo_attack_power(
            codex,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
            BattlePhase::OpeningTorpedo,
        );
        let outcome = roll_attack(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            Aim::new(AttackKind::Torpedo, friendly_formation_id, enemy_formation_id)
                .with_flat((power / 5.0).floor()),
        );
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
            BattlePhase::OpeningTorpedo,
            outcome,
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
                cl: outcome.cl(),
                shield,
            },
        );
        happened = true;
    }

    for (idx, ship) in enemy_at_start.iter().enumerate() {
        if !can_opening_torpedo_ship(codex, ship) {
            continue;
        }
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let candidates = target_candidates(codex, ship, friendly, BattlePhase::OpeningTorpedo);
        let Some((target_idx, shield)) =
            choice_with_protect(codex, rng, friendly, &candidates, 0, false, friendly_formation_id)
        else {
            continue;
        };
        // A fifth of the torpedo's power goes into its accuracy.
        let power = torpedo_attack_power(
            codex,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
            BattlePhase::OpeningTorpedo,
        );
        let outcome = roll_attack(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            Aim::new(AttackKind::Torpedo, enemy_formation_id, friendly_formation_id)
                .with_flat((power / 5.0).floor()),
        );
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
            BattlePhase::OpeningTorpedo,
            outcome,
        );
        let (raw_dmg, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
        enemy[idx].damage_dealt += dealt;
        let display = crate::targeting::display_damage(&friendly[target_idx], raw_dmg, dealt);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: idx,
                defender_index: target_idx,
                damage: display,
                cl: outcome.cl(),
                shield,
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

    // Every ship that could fire when the phase began fires: what the enemy
    // sends is settled from the enemy as it was, not as the friendly torpedoes
    // resolved just before leave it.
    let enemy_at_start = enemy.to_vec();

    for (idx, ship) in friendly.iter_mut().enumerate() {
        // 第1艦隊 does not torpedo — see the opening phase above.
        if ship.is_main_deck() {
            continue;
        }
        if !can_closing_torpedo_ship(codex, ship) {
            continue;
        }
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let candidates = deck_candidates(codex, rng, ship, enemy, BattlePhase::ClosingTorpedo);
        let Some((target_idx, shield)) =
            choice_with_protect(codex, rng, enemy, &candidates, 0, false, enemy_formation_id)
        else {
            continue;
        };
        // A fifth of the torpedo's power goes into its accuracy.
        let power = torpedo_attack_power(
            codex,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
            BattlePhase::ClosingTorpedo,
        );
        let outcome = roll_attack(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            Aim::new(AttackKind::Torpedo, friendly_formation_id, enemy_formation_id)
                .with_flat((power / 5.0).floor()),
        );
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            friendly_formation_id,
            engagement,
            BattlePhase::ClosingTorpedo,
            outcome,
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
                cl: outcome.cl(),
                shield,
            },
        );
        happened = true;
    }

    for (idx, ship) in enemy_at_start.iter().enumerate() {
        // Of an enemy combined fleet only the escort fleet closes with
        // torpedoes. No-op for an enemy single fleet, whose ships have no deck.
        if ship.is_main_deck() {
            continue;
        }
        if !can_closing_torpedo_ship(codex, ship) {
            continue;
        }
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let candidates = deck_candidates(codex, rng, ship, friendly, BattlePhase::ClosingTorpedo);
        let Some((target_idx, shield)) =
            choice_with_protect(codex, rng, friendly, &candidates, 0, false, friendly_formation_id)
        else {
            continue;
        };
        // A fifth of the torpedo's power goes into its accuracy.
        let power = torpedo_attack_power(
            codex,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
            BattlePhase::ClosingTorpedo,
        );
        let outcome = roll_attack(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            Aim::new(AttackKind::Torpedo, enemy_formation_id, friendly_formation_id)
                .with_flat((power / 5.0).floor()),
        );
        let raw = calculate_torpedo_damage(
            codex,
            rng,
            ship,
            &friendly[target_idx],
            enemy_formation_id,
            engagement,
            BattlePhase::ClosingTorpedo,
            outcome,
        );
        let (raw_dmg, dealt) = friendly[target_idx].apply_damage(rng, raw, target_idx);
        enemy[idx].damage_dealt += dealt;
        let display = crate::targeting::display_damage(&friendly[target_idx], raw_dmg, dealt);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: idx,
                defender_index: target_idx,
                damage: display,
                cl: outcome.cl(),
                shield,
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
    use crate::types::DamageCell;
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
            crate::types::BattleContext::head_on(
                crate::types::BattleType::Normal,
                false,
                vec![dd, clt],
                vec![enemy],
            ),
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

        let dealt = opening.api_edam[0].amount();
        assert!(dealt > 0);
        assert_eq!(opening.api_frai_list_items[0], Some(vec![0]));
        assert_eq!(opening.api_fydam_list_items[0], Some(vec![DamageCell::Plain(dealt)]));
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

        let dealt = opening.api_fdam[0].amount();
        assert!(dealt > 0);
        assert_eq!(opening.api_erai_list_items[0], Some(vec![0]));
        assert_eq!(opening.api_eydam_list_items[0], Some(vec![DamageCell::Plain(dealt)]));
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

        let dealt = raigeki.api_edam[0].amount();
        assert!(dealt > 0);
        assert_eq!(raigeki.api_frai[0], 0);
        assert_eq!(raigeki.api_fydam[0], DamageCell::Plain(dealt));
        assert_eq!(raigeki.api_eydam[0], DamageCell::Plain(0));
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

        let dealt = raigeki.api_fdam[0].amount();
        assert!(dealt > 0);
        assert_eq!(raigeki.api_erai[0], 0);
        assert_eq!(raigeki.api_eydam[0], DamageCell::Plain(dealt));
        assert_eq!(raigeki.api_fydam[0], DamageCell::Plain(0));
        assert_eq!(friendly[0].hp(), friendly[0].ship.api_nowhp - dealt);
    }

    /// `torpedoPhase` draws the closing torpedo through `choiceWProtect` as it does the
    /// opening one, so an escort covers the flagship there too.
    #[test]
    fn closing_torpedo_flagship_can_be_shielded() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        let shielded = (0..200u64).any(|seed| {
            let mut friendly = vec![
                BattleRuntimeShip::new(sample_ship(&codex, bb, 80), true, true),
                BattleRuntimeShip::new(sample_ship(&codex, dd, 80), true, true),
            ];
            let mut enemy = vec![
                BattleRuntimeShip::new(sample_ship(&codex, clt, 80), false, true),
                BattleRuntimeShip::new(sample_ship(&codex, clt, 80), false, true),
            ];
            let mut rng = SeededRng::new(seed);
            let Some(raigeki) = simulate_raigeki(
                &codex,
                &mut rng,
                &mut friendly,
                &mut enemy,
                3,
                1,
                EngagementType::SameCourse,
            ) else {
                return false;
            };
            raigeki.api_eydam.iter().zip(&raigeki.api_erai).any(|(cell, &target)| {
                let shielded = matches!(cell, DamageCell::Shielded(_));
                assert!(!shielded || target == 1, "an intercepted torpedo hits the escort");
                shielded
            })
        });
        assert!(shielded, "expected an intercepted closing-torpedo hit on the flagship");
    }

    /// Covers AE5. An intercepted opening-torpedo hit on the friendly flagship
    /// carries the `.1` shield flag on the attacker's `api_eydam_list_items`
    /// entry, and `api_erai_list_items` points at the escort (index 1), not 0.
    #[test]
    fn opening_torpedo_flagship_shield_flags_eydam() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let bb = first_ship_mst_by_type(&codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut found = false;
        for seed in 0..400u64 {
            // Friendly defenders: surface flagship + healthy surface escort.
            let mut friendly = vec![
                BattleRuntimeShip::new(sample_ship(&codex, bb, 80), true, true),
                BattleRuntimeShip::new(sample_ship(&codex, dd, 80), true, true),
            ];
            // Enemy attackers: torpedo-capable CLTs.
            let mut enemy = vec![
                BattleRuntimeShip::new(sample_ship(&codex, clt, 80), false, true),
                BattleRuntimeShip::new(sample_ship(&codex, clt, 80), false, true),
            ];
            let mut rng = SeededRng::new(seed);
            // friendly_formation_id = 3 (輪形陣, 75%) maximises interception.
            let Some(op) = simulate_opening_torpedo(
                &codex,
                &mut rng,
                &mut friendly,
                &mut enemy,
                3,
                1,
                EngagementType::SameCourse,
            ) else {
                continue;
            };
            for (i, cell) in op.api_eydam_list_items.iter().enumerate() {
                let Some(dmgs) = cell else {
                    continue;
                };
                if dmgs.iter().any(|c| matches!(c, DamageCell::Shielded(_))) {
                    assert_eq!(
                        op.api_erai_list_items[i],
                        Some(vec![1]),
                        "intercepted torpedo must target the escort, not the flagship"
                    );
                    let json = serde_json::to_string(dmgs).unwrap();
                    assert!(json.contains(".1"), "shielded eydam must serialize with .1: {json}");
                    found = true;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "expected an intercepted opening-torpedo hit on the flagship");
    }

    /// Torpedoes are in the water together: an enemy the friendly salvo sinks
    /// has already fired its own.
    #[test]
    fn an_enemy_sunk_by_the_friendly_salvo_still_fires() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_raisou[0] = 300;
        let mut foe = sample_ship(&codex, dd_mst, 1);
        foe.ship.api_raisou[0] = 50;

        let mut sunk = 0;
        for seed in 0..20 {
            let mut friendly = vec![BattleRuntimeShip::from(friend.clone())];
            let mut enemy = vec![BattleRuntimeShip::from(foe.clone())];
            let raigeki = simulate_raigeki(
                &codex,
                &mut SeededRng::new(seed),
                &mut friendly,
                &mut enemy,
                1,
                1,
                crate::types::EngagementType::SameCourse,
            )
            .unwrap();
            if enemy[0].is_sunk() {
                sunk += 1;
                assert_eq!(raigeki.api_erai[0], 0, "seed {seed}: the sunk ship fired at F1");
            }
        }
        assert!(sunk > 0, "no seed sank the enemy, so nothing was tested");
    }

    /// A ship of `mst`, standing in for `ship_id` with a hull nothing here sinks.
    fn stand_in(codex: &Codex, mst: i64, ship_id: i64) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst, 50);
        ship.ship.api_ship_id = ship_id;
        ship.ship.api_nowhp = 4000;
        ship.ship.api_maxhp = 4000;
        BattleRuntimeShip::from(ship)
    }

    /// `torpedoPhase` leaves land targets out of both sides' targets, opening and closing
    /// alike (`if (!alive[i].isInstall)`): only the torpedo cruiser beside one is aimed at.
    #[test]
    fn a_torpedo_never_goes_for_a_land_target() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let clt = first_ship_mst_by_type(&codex, KcShipType::CLT);
        let land = ship_mst_id_by_name(&codex, "砲台小鬼");
        let mut gun_battery = stand_in(&codex, clt, land);
        // It fires no torpedoes of its own, so only the cruisers' are counted.
        gun_battery.ship.api_raisou[0] = 0;
        let fleet = || vec![gun_battery.clone(), stand_in(&codex, clt, clt)];
        let eng = EngagementType::SameCourse;
        let mut aimed = 0;
        for seed in 0..30 {
            let (mut friendly, mut enemy) = (fleet(), fleet());
            let rng = &mut SeededRng::new(seed);
            let opening =
                simulate_opening_torpedo(&codex, rng, &mut friendly, &mut enemy, 1, 1, eng)
                    .unwrap();
            let raigeki =
                simulate_raigeki(&codex, rng, &mut friendly, &mut enemy, 1, 1, eng).unwrap();
            let targets = opening
                .api_frai_list_items
                .iter()
                .chain(&opening.api_erai_list_items)
                .flatten()
                .flatten()
                .chain(raigeki.api_frai.iter().chain(&raigeki.api_erai).filter(|&&at| at >= 0));
            for &at in targets {
                assert_eq!(at, 1, "seed {seed}: a torpedo went for the land target");
                aimed += 1;
            }
        }
        assert_eq!(aimed, 30 * 4, "each torpedo cruiser fires in both phases");

        // At land targets alone there is nothing to fire at.
        let mut friendly = vec![stand_in(&codex, clt, clt)];
        let mut enemy = vec![gun_battery];
        let rng = &mut SeededRng::new(1);
        assert!(
            simulate_opening_torpedo(&codex, rng, &mut friendly, &mut enemy, 1, 1, eng).is_none()
        );
        assert!(simulate_raigeki(&codex, rng, &mut friendly, &mut enemy, 1, 1, eng).is_none());
    }
}
