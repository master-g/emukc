//! Day shelling phase simulation.

use crate::accuracy::{Aim, AttackKind, roll_attack};
use crate::damage::{calculate_asw_damage, calculate_shelling_damage};
use crate::random::BattleRng;
use crate::simulation::day_attack::DayAttackKind;
use crate::simulation::day_cutin::{day_accuracy_modifier, resolve_day_attack};
use crate::simulation::special_attack;
use crate::targeting::{any_alive, can_shell_day_ship, select_random_target_index, target_class};
use crate::types::{
    AirState, BattleHougeki, BattlePhase, BattleRuntimeShip, DamageCell, EngagementType,
    ShellingParams,
};
use emukc_model::codex::Codex;
use emukc_model::kc2::start2::ApiMstSlotitem;
use std::ops::Range;

/// One round of day shelling: who fires, at whom, and in what order.
pub(crate) struct ShellingRound<'a> {
    /// The friendly ships that fire this round. The enemy fires at the whole
    /// friendly force whichever deck this is.
    pub friendly_deck: Range<usize>,
    /// The enemy ships that fire this round and are fired at.
    pub enemy_deck: Range<usize>,
    /// Longest range first, as in a first round; fleet position otherwise.
    pub by_range: bool,
    pub friendly_formation: i64,
    pub enemy_formation: i64,
    pub engagement: EngagementType,
    pub air_state: Option<&'a AirState>,
}

/// Simulate one round of day shelling. The two sides fire ship by ship in
/// turn, the friendly side first: its first ship, the enemy's first, its
/// second, and so on down the two orders. Every index written is a position in
/// the whole fleet, whichever deck the round belongs to.
pub(crate) fn simulate_shelling_round(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    round: &ShellingRound,
    special_attack_used: &mut [bool; 2],
) -> Option<BattleHougeki> {
    let friendly_params = ShellingParams {
        attacker_is_enemy: false,
        formation_id: round.friendly_formation,
        defender_formation_id: round.enemy_formation,
        engagement: round.engagement,
        phase: BattlePhase::DayShelling,
        air_state: round.air_state,
    };
    let enemy_params = ShellingParams {
        attacker_is_enemy: true,
        formation_id: round.enemy_formation,
        defender_formation_id: round.friendly_formation,
        ..friendly_params
    };
    let (friendly_at, enemy_at) = (round.friendly_deck.start, round.enemy_deck.start);

    let friendly_los = fleet_los(&friendly[round.friendly_deck.clone()]);
    let enemy_los = fleet_los(&enemy[round.enemy_deck.clone()]);
    // The orders are fixed when the round begins; a ship sunk before its turn
    // comes loses it.
    let can_shell = |ship: &BattleRuntimeShip| can_shell_day_ship(codex, ship);
    let friendly_order =
        firing_order(codex, rng, &friendly[round.friendly_deck.clone()], round.by_range, can_shell);
    let enemy_order =
        firing_order(codex, rng, &enemy[round.enemy_deck.clone()], round.by_range, can_shell);

    let mut hougeki = BattleHougeki::default();
    let [friendly_special, enemy_special] = special_attack_used;
    for turn in 0..friendly_order.len().max(enemy_order.len()) {
        if let Some(&idx) = friendly_order.get(turn) {
            let fired = shell_turn(
                codex,
                rng,
                &mut friendly[round.friendly_deck.clone()],
                idx,
                &mut enemy[round.enemy_deck.clone()],
                &friendly_params,
                friendly_los,
                friendly_special,
            );
            append_turn(&mut hougeki, fired, friendly_at, enemy_at);
        }
        if !any_alive(&enemy[round.enemy_deck.clone()]) {
            break;
        }
        if let Some(&idx) = enemy_order.get(turn) {
            let fired = shell_turn(
                codex,
                rng,
                &mut enemy[round.enemy_deck.clone()],
                idx,
                friendly,
                &enemy_params,
                enemy_los,
                enemy_special,
            );
            append_turn(&mut hougeki, fired, enemy_at, 0);
        }
        if !any_alive(friendly) {
            break;
        }
    }

    (!hougeki.api_at_list.is_empty()).then_some(hougeki)
}

fn fleet_los(ships: &[BattleRuntimeShip]) -> i64 {
    ships.iter().map(|s| s.ship.api_sakuteki[0].max(0)).sum()
}

/// The ships of one side that `can_fire`, in the order they will: longest
/// range first with equal ranges in random order, or simply down the line.
pub(crate) fn firing_order(
    codex: &Codex,
    rng: &mut impl BattleRng,
    ships: &[BattleRuntimeShip],
    by_range: bool,
    can_fire: impl Fn(&BattleRuntimeShip) -> bool,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..ships.len()).filter(|&idx| can_fire(&ships[idx])).collect();
    if by_range {
        for last in (1..order.len()).rev() {
            order.swap(last, rng.roll_range(0, last as i64 + 1) as usize);
        }
        // Stable, so the shuffle is what orders ships of one range.
        order.sort_by_key(|&idx| std::cmp::Reverse(shelling_range(codex, &ships[idx])));
    }
    order
}

/// A ship's range: its own or that of the longest piece it carries.
fn shelling_range(codex: &Codex, ship: &BattleRuntimeShip) -> i64 {
    ship.slot_items
        .iter()
        .filter_map(|item| codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok())
        .map(|mst| mst.api_leng)
        .fold(ship.ship.api_leng, i64::max)
}

/// Add what one ship fired to the round, lifting the indices of its own slice
/// and of its targets' slice to positions in the whole fleets.
fn append_turn(
    round: &mut BattleHougeki,
    mut fired: BattleHougeki,
    attacker_at: usize,
    defender_at: usize,
) {
    for attacker in &mut fired.api_at_list {
        *attacker += attacker_at as i64;
    }
    for defender in fired.api_df_list.iter_mut().flatten().filter(|d| **d >= 0) {
        *defender += defender_at as i64;
    }
    round.api_at_eflag.extend(fired.api_at_eflag);
    round.api_at_list.extend(fired.api_at_list);
    round.api_at_type.extend(fired.api_at_type);
    round.api_df_list.extend(fired.api_df_list);
    round.api_si_list.extend(fired.api_si_list);
    round.api_cl_list.extend(fired.api_cl_list);
    round.api_damage.extend(fired.api_damage);
}

/// One side shelling down its line with no answer from the other.
#[cfg(test)]
pub(crate) fn simulate_shelling_side(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    defenders: &mut [BattleRuntimeShip],
    params: &ShellingParams,
) -> Option<BattleHougeki> {
    let los = fleet_los(attackers);
    let mut hougeki = BattleHougeki::default();
    let mut special = false;
    for idx in 0..attackers.len() {
        let fired = shell_turn(codex, rng, attackers, idx, defenders, params, los, &mut special);
        append_turn(&mut hougeki, fired, 0, 0);
    }
    (!hougeki.api_at_list.is_empty()).then_some(hougeki)
}

/// What the ship at `idx` fires when its turn comes: nothing, one attack, or
/// from the flagship the whole of a special attack, which a side makes once in
/// a battle. Indices are those of the two slices.
#[expect(clippy::too_many_arguments)]
fn shell_turn(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    idx: usize,
    defenders: &mut [BattleRuntimeShip],
    params: &ShellingParams,
    fleet_los: i64,
    special_attack_used: &mut bool,
) -> BattleHougeki {
    // A special attack is decided when the flagship's turn comes and takes
    // that turn; the ships that join it still have their own.
    if idx == 0
        && !*special_attack_used
        && let Some(resolved) =
            special_attack::try_special_attack(codex, rng, attackers, params.formation_id)
    {
        *special_attack_used = true;
        return special_attack::execute_special_attack(
            codex, rng, attackers, defenders, resolved, params,
        );
    }

    let mut hougeki = BattleHougeki::default();
    let ship = &mut attackers[idx];
    if !can_shell_day_ship(codex, ship) {
        return hougeki;
    }
    let Some(mut target_idx) =
        select_random_target_index(codex, rng, ship, defenders, params.phase)
    else {
        return hougeki;
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
        let outcome = roll_attack(
            codex,
            rng,
            ship,
            &defenders[target_idx],
            Aim::new(AttackKind::Asw, params.formation_id, params.defender_formation_id),
        );
        let raw = calculate_asw_damage(
            codex,
            rng,
            ship,
            &defenders[target_idx],
            params.formation_id,
            params.engagement,
            outcome,
        );
        let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
        if !params.attacker_is_enemy {
            ship.damage_dealt += dealt;
        }
        let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);

        hougeki.record_day_attack(
            DayAttackKind::Asw(codex, ship),
            params.attacker_is_enemy,
            idx,
            vec![target_idx as i64],
            vec![damage_cell(display, shield)],
            vec![outcome.cl()],
        );
    } else {
        let resolved = resolve_day_attack(codex, rng, ship, params.air_state, fleet_los, idx);

        let ci_mult = if resolved.damage_multiplier != 1.0 {
            Some(resolved.damage_multiplier)
        } else {
            None
        };

        let aim = Aim::new(AttackKind::Shelling, params.formation_id, params.defender_formation_id)
            .with_modifier(day_accuracy_modifier(resolved.at_type, resolved.carrier_sub));

        if resolved.hit_count == 2 {
            // DoubleAttack: 2 hits on the same target
            let mut damages = Vec::with_capacity(2);
            let mut cls = Vec::with_capacity(2);
            for _ in 0..2 {
                let outcome = roll_attack(codex, rng, ship, &defenders[target_idx], aim);
                cls.push(outcome.cl());
                let raw = calculate_shelling_damage(
                    codex,
                    rng,
                    ship,
                    &defenders[target_idx],
                    params.formation_id,
                    params.engagement,
                    ci_mult,
                    outcome,
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
            hougeki.record_day_attack(
                DayAttackKind::Shelling {
                    codex,
                    ship,
                    at_type: resolved.at_type,
                    carrier_sub: resolved.carrier_sub,
                },
                params.attacker_is_enemy,
                idx,
                vec![target_idx as i64; 2],
                damages.into_iter().map(|d| damage_cell(d, shield)).collect(),
                cls,
            );
        } else {
            let outcome = roll_attack(codex, rng, ship, &defenders[target_idx], aim);
            let raw = calculate_shelling_damage(
                codex,
                rng,
                ship,
                &defenders[target_idx],
                params.formation_id,
                params.engagement,
                ci_mult,
                outcome,
            );
            let (raw_dmg, dealt) = defenders[target_idx].apply_damage(rng, raw, target_idx);
            if !params.attacker_is_enemy {
                ship.damage_dealt += dealt;
            }
            let display = crate::targeting::display_damage(&defenders[target_idx], raw_dmg, dealt);
            hougeki.record_day_attack(
                DayAttackKind::Shelling {
                    codex,
                    ship,
                    at_type: resolved.at_type,
                    carrier_sub: resolved.carrier_sub,
                },
                params.attacker_is_enemy,
                idx,
                vec![target_idx as i64],
                vec![damage_cell(display, shield)],
                vec![outcome.cl()],
            );
        }
    }
    hougeki
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
            BattleContext::head_on(BattleType::Normal, false, vec![carrier, bb], vec![enemy]),
            &mut crate::random::SeededRng::new(1),
        );

        // Verify no friendly shelling attack came from index 0 (carrier)
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

    /// U1 / 5.1 + 5.2: a destroyer with no equipment, or with only a torpedo,
    /// resolves to a Normal day attack (`api_at_type` 0). Day participation is
    /// ship-type gated; a lone torpedo forms no day cut-in and never produces a
    /// non-zero display type.
    #[test]
    fn day_destroyer_resolves_normal_attack_type_zero() {
        use crate::simulation::day_cutin::{
            DayAttackType, detect_day_attack_type, resolve_day_attack,
        };
        use crate::types::AirState;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torpedo_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        // 5.1: bare DD. 5.2: DD carrying only a torpedo.
        let bare = crate::types::BattleRuntimeShip::from(sample_ship(&codex, dd_mst, 99));
        let mut torp_input = sample_ship(&codex, dd_mst, 99);
        torp_input.slot_items = vec![slotitem_with_mst_id(torpedo_id)];
        let torp = crate::types::BattleRuntimeShip::from(torp_input);

        for rt in [&bare, &torp] {
            assert_eq!(
                detect_day_attack_type(&codex, rt, Some(&AirState::Supremacy)),
                None,
                "a DD with no day-cut-in equipment must not detect a cut-in"
            );
            let los = rt.ship.api_sakuteki[0].max(0);
            for seed in 0..50u64 {
                let resolved = resolve_day_attack(
                    &codex,
                    &mut crate::random::SeededRng::new(seed),
                    rt,
                    Some(&AirState::Supremacy),
                    los,
                    0,
                );
                assert_eq!(
                    resolved.at_type,
                    DayAttackType::Normal,
                    "a DD must always resolve to a Normal attack (api_at_type 0)"
                );
            }
        }
        assert_eq!(DayAttackType::Normal as i64, 0, "Normal must serialize as api_at_type 0");
    }

    /// The wire values these variants serialize to are read by the client and,
    /// separately, hardcoded in `emukc_bootstrap` (which cannot depend on this
    /// crate) to decide whether a day carrier cut-in suppresses its name plate.
    /// Pin them here so the two copies cannot drift apart silently.
    #[test]
    fn day_attack_type_discriminants_match_the_protocol() {
        use crate::simulation::day_cutin::DayAttackType;

        assert_eq!(DayAttackType::Normal as i64, 0);
        assert_eq!(DayAttackType::DoubleAttack as i64, 2);
        assert_eq!(DayAttackType::MainSecCI as i64, 3);
        assert_eq!(DayAttackType::MainRadarCI as i64, 4);
        assert_eq!(DayAttackType::MainApSecCI as i64, 5);
        assert_eq!(DayAttackType::MainApMainCI as i64, 6);
        // `CARRIER_CUTIN_ATTACK_TYPE` in emukc_bootstrap/src/battle_rules.rs.
        assert_eq!(DayAttackType::CarrierCI as i64, 7);
    }

    /// The artillery cut-ins that need a radar or an armour piercing shell name
    /// it: the client draws three slots and those pieces are what qualified the
    /// attack. Both have a name plate upstream, so naming them stays covered.
    #[test]
    fn artillery_cutin_names_the_piece_that_qualified_it() {
        use crate::simulation::day_cutin::{DayAttackType, detect_day_attack_type};
        use crate::targeting::{day_gunnery_display_ids, is_ap_shell_type, is_radar_type};
        use crate::types::AirState;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let secondary_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SecondaryGun);
        let radar_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);
        let ap_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::ArmorPiercingShell);
        let seaplane_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedRecon);

        // 主砲 + 副砲 + 電探 + 水偵 resolves to MainRadarCI.
        let mut radar_ship = sample_ship(&codex, bb_mst, 99);
        radar_ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_id),
            slotitem_with_mst_id(secondary_id),
            slotitem_with_mst_id(radar_id),
            slotitem_with_mst_id(seaplane_id),
        ];
        radar_ship.ship.api_onslot = [0, 0, 0, 1, 0];
        let radar_ship = BattleRuntimeShip::from(radar_ship);
        assert_eq!(
            detect_day_attack_type(&codex, &radar_ship, Some(&AirState::Supremacy)),
            Some(DayAttackType::MainRadarCI)
        );
        assert_eq!(
            day_gunnery_display_ids(&codex, &radar_ship, 3, Some(is_radar_type)),
            vec![main_gun_id, secondary_id, radar_id]
        );

        // 主砲 x2 + 徹甲弾 + 水偵 resolves to MainApMainCI.
        let mut ap_ship = sample_ship(&codex, bb_mst, 99);
        ap_ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_id),
            slotitem_with_mst_id(main_gun_id),
            slotitem_with_mst_id(ap_id),
            slotitem_with_mst_id(seaplane_id),
        ];
        ap_ship.ship.api_onslot = [0, 0, 0, 1, 0];
        let ap_ship = BattleRuntimeShip::from(ap_ship);
        assert_eq!(
            detect_day_attack_type(&codex, &ap_ship, Some(&AirState::Supremacy)),
            Some(DayAttackType::MainApMainCI)
        );
        assert_eq!(
            day_gunnery_display_ids(&codex, &ap_ship, 3, Some(is_ap_shell_type)),
            vec![main_gun_id, main_gun_id, ap_id]
        );
    }

    /// R3 end to end: 航空戦艦 with 瑞雲 plus guns. Day cut-ins and 連撃 are gun
    /// attacks, so the seaplane bomber must never appear in their `si_list` --
    /// it used to, through the broad day-surface display set.
    #[test]
    fn day_cutin_si_list_excludes_seaplane_bombers() {
        use crate::types::{AirState, SiListId};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let zuiun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let secondary_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SecondaryGun);

        let make_attacker = || {
            let mut bbv = sample_ship(&codex, bbv_mst, 99);
            bbv.slot_items = vec![
                slotitem_with_mst_id(zuiun_id),
                slotitem_with_mst_id(main_gun_id),
                slotitem_with_mst_id(main_gun_id),
                slotitem_with_mst_id(secondary_id),
            ];
            bbv.ship.api_onslot = [1, 0, 0, 0, 0];
            BattleRuntimeShip::from(bbv)
        };
        let make_defender = || {
            let mut dd = sample_ship(&codex, dd_mst, 30);
            dd.ship.api_soukou[0] = 1;
            dd.ship.api_nowhp = 4000;
            dd.ship.api_maxhp = 4000;
            BattleRuntimeShip::from(dd)
        };

        let air_state = AirState::Supremacy;
        let mut saw_non_normal = false;
        for seed in 0..300u64 {
            let mut attackers = vec![make_attacker()];
            let mut defenders = vec![make_defender()];
            let Some(hougeki) = simulate_shelling_side(
                &codex,
                &mut SeededRng::new(seed),
                &mut attackers,
                &mut defenders,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: 1,
                    defender_formation_id: 0,
                    engagement: EngagementType::SameCourse,
                    phase: BattlePhase::DayShelling,
                    air_state: Some(&air_state),
                },
            ) else {
                continue;
            };
            for (idx, at_type) in hougeki.api_at_type.iter().enumerate() {
                if *at_type == 0 {
                    continue;
                }
                saw_non_normal = true;
                let shown = &hougeki.api_si_list[idx];
                assert!(
                    !shown.iter().any(
                        |id| matches!(id, SiListId::Text(text) if *text == zuiun_id.to_string())
                    ),
                    "at_type {at_type} displayed the seaplane bomber: {shown:?}"
                );
            }
            if saw_non_normal {
                break;
            }
        }
        assert!(saw_non_normal, "no cut-in or 連撃 fired across 300 seeds; assertion unverified");
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

    /// R1: a day-shelling attack against a submarine reports `api_at_type = 0`,
    /// not the client's carrier cut-in type 7. The client's
    /// `_getNormalAttackType` picks the depth-charge or ASW-plane animation
    /// from the defender being a submarine; the server only says "normal".
    /// The `si_list` stays integer-typed because this is not a cut-in.
    #[test]
    fn day_shelling_against_submarine_reports_attack_type_zero() {
        use crate::types::{AirState, SiListId};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss_mst = first_ship_mst_by_type(&codex, KcShipType::SS);
        let sonar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Sonar);

        let make_attacker = || {
            let mut dd = sample_ship(&codex, dd_mst, 99);
            dd.slot_items = vec![slotitem_with_mst_id(sonar_mst_id)];
            BattleRuntimeShip::from(dd)
        };
        let make_defender = || {
            let mut ss = sample_ship(&codex, ss_mst, 50);
            ss.ship.api_soukou[0] = 1;
            ss.ship.api_nowhp = 800;
            ss.ship.api_maxhp = 800;
            BattleRuntimeShip::from(ss)
        };

        let air_state = AirState::Supremacy;
        let mut saw_attack = false;
        for seed in 0..50u64 {
            let mut attackers = vec![make_attacker()];
            let mut defenders = vec![make_defender()];
            let Some(hougeki) = simulate_shelling_side(
                &codex,
                &mut SeededRng::new(seed),
                &mut attackers,
                &mut defenders,
                &ShellingParams {
                    attacker_is_enemy: false,
                    formation_id: 1,
                    defender_formation_id: 0,
                    engagement: EngagementType::SameCourse,
                    phase: BattlePhase::DayShelling,
                    air_state: Some(&air_state),
                },
            ) else {
                continue;
            };
            for (idx, at_type) in hougeki.api_at_type.iter().enumerate() {
                assert_eq!(
                    *at_type, 0,
                    "ASW shelling must report api_at_type 0, got {at_type} (seed {seed})"
                );
                assert!(
                    hougeki.api_si_list[idx].iter().all(|id| matches!(id, SiListId::Num(_))),
                    "ASW shelling si_list must stay integer-typed: {:?}",
                    hougeki.api_si_list[idx]
                );
                saw_attack = true;
            }
        }
        assert!(saw_attack, "no ASW shelling fired across 50 seeds; assertion unverified");
    }
}
