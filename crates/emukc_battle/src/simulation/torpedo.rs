//! Torpedo phase simulation (opening torpedo + closing torpedo / raigeki).

use emukc_model::codex::Codex;

use crate::accuracy::{Aim, AttackKind, roll_attack};
use crate::config::BattlePhaseKind;
use crate::damage::{calculate_torpedo_damage, torpedo_attack_power};
use crate::random::BattleRng;
use crate::simulation::volley::{Volley, Volleys};
use crate::targeting::{
    can_closing_torpedo_ship, can_opening_torpedo_ship, choice_with_protect, deck_candidates,
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
    let mut payload = BattleOpeningAttack::blank(friendly.len().max(enemy.len()));
    let hits = torpedo_phase(
        codex,
        rng,
        friendly,
        enemy,
        (friendly_formation_id, enemy_formation_id),
        engagement,
        BattlePhase::OpeningTorpedo,
    );
    let happened = !hits.is_empty();
    for (side, hit) in hits {
        payload.record_torpedo_hit(side, hit);
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
    let mut payload = BattleRaigeki::blank(friendly.len().max(enemy.len()));
    let hits = torpedo_phase(
        codex,
        rng,
        friendly,
        enemy,
        (friendly_formation_id, enemy_formation_id),
        engagement,
        BattlePhase::ClosingTorpedo,
    );
    let happened = !hits.is_empty();
    for (side, hit) in hits {
        payload.record_torpedo_hit(side, hit);
    }
    happened.then_some(payload)
}

/// Both sides' torpedoes in `phase`, the friendly side's first. Every ship that
/// could fire when the phase began fires: what the enemy sends is settled from
/// the enemy as it was, not as the friendly torpedoes resolved just before
/// leave it.
fn torpedo_phase(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    (friendly_formation, enemy_formation): (i64, i64),
    engagement: EngagementType,
    phase: BattlePhase,
) -> Vec<(TorpedoAttackerSide, TorpedoHit)> {
    let (kind, can_fire): (_, fn(&Codex, &BattleRuntimeShip) -> bool) = match phase {
        BattlePhase::OpeningTorpedo => (BattlePhaseKind::OpeningTorpedo, can_opening_torpedo_ship),
        BattlePhase::ClosingTorpedo => (BattlePhaseKind::ClosingTorpedo, can_closing_torpedo_ship),
        BattlePhase::DayShelling | BattlePhase::NightShelling => {
            unreachable!("{phase:?} is not a torpedo phase")
        }
    };
    let volleys = Volleys::opening(kind, friendly, enemy);
    let friendly_at_start = volleys.friendly.attackers_in(friendly).to_vec();
    let enemy_at_start = volleys.enemy.attackers_in(enemy).to_vec();

    let friendly_hits = salvo(
        codex,
        rng,
        &friendly_at_start,
        (friendly, enemy),
        &volleys.friendly,
        Aim::new(AttackKind::Torpedo, friendly_formation, enemy_formation),
        (engagement, phase, can_fire),
    );
    let enemy_hits = salvo(
        codex,
        rng,
        &enemy_at_start,
        (enemy, friendly),
        &volleys.enemy,
        Aim::new(AttackKind::Torpedo, enemy_formation, friendly_formation),
        (engagement, phase, can_fire),
    );
    let mut hits: Vec<_> =
        friendly_hits.into_iter().map(|hit| (TorpedoAttackerSide::Friendly, hit)).collect();
    hits.extend(enemy_hits.into_iter().map(|hit| (TorpedoAttackerSide::Enemy, hit)));
    hits
}

/// One side's torpedoes: each ship of `at_start` (the volley's attackers as the
/// phase began) that `can_fire` fires at the volley's targets, and the damage is
/// credited to the same ship in `afloat`. `aim` carries the two formations,
/// the attacker's first.
fn salvo(
    codex: &Codex,
    rng: &mut impl BattleRng,
    at_start: &[BattleRuntimeShip],
    (afloat, defending): (&mut [BattleRuntimeShip], &mut [BattleRuntimeShip]),
    volley: &Volley,
    aim: Aim,
    (engagement, phase, can_fire): (
        EngagementType,
        BattlePhase,
        fn(&Codex, &BattleRuntimeShip) -> bool,
    ),
) -> Vec<TorpedoHit> {
    let (attackers, targets) = volley.split(afloat, defending);
    let mut hits = Vec::new();
    for (idx, ship) in at_start.iter().enumerate() {
        if !can_fire(codex, ship) {
            continue;
        }
        // The closing torpedo picks an enemy combined fleet's deck first; the opening
        // one draws from both decks, which `deck_candidates` does without a roll.
        let candidates = deck_candidates(codex, rng, ship, targets, phase);
        // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
        let Some((target_idx, shield)) =
            choice_with_protect(codex, rng, targets, &candidates, 0, false, aim.defender_formation)
        else {
            continue;
        };
        // A fifth of the torpedo's power goes into its accuracy.
        let power = torpedo_attack_power(
            codex,
            ship,
            &targets[target_idx],
            aim.attacker_formation,
            engagement,
            phase,
        );
        let outcome = roll_attack(
            codex,
            rng,
            ship,
            &targets[target_idx],
            aim.with_flat((power / 5.0).floor()),
        );
        let raw = calculate_torpedo_damage(codex, rng, ship, &targets[target_idx], power, outcome);
        let landed = targets[target_idx].take_hit(rng, raw, shield);
        attackers[idx].damage_dealt += landed.dealt;
        hits.push(TorpedoHit {
            attacker_index: volley.attacker_at(idx),
            defender_index: volley.target_at(target_idx),
            damage: landed.cell,
            cl: outcome.cl(),
        });
    }
    hits
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
