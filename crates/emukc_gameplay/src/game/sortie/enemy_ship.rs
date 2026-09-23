use emukc_battle::BattleShipInput;
use emukc_crypto::rng;
use emukc_model::{
    codex::{
        Codex,
        map::{EnemyComposition, EnemyFleetDefinition, MapDefinition, MapVariantDefinition},
    },
    kc2::{KcApiShip, KcApiSlotItem, UserHQRank, level, start2::ApiMstShip},
};
use tracing::warn;

use crate::err::GameplayError;

/// Fallback enemy ship: Abyssal DD I-class (駆逐イ級).
/// Used when map data is missing enemy fleet definitions.
const FALLBACK_ENEMY_SHIP_ID: i64 = 1501;

/// The enemy a sortie meets at one cell, decided once.
pub(super) struct EnemyEncounter {
    pub(super) ships: Vec<BattleShipInput>,
    pub(super) formation_id: i64,
    pub(super) level: i64,
    pub(super) rank: String,
    pub(super) deck_name: String,
}

/// Build the encounter at `cell_no` from the composition the route locked, or a
/// freshly rolled one when none was locked.
///
/// The formation is the composition's own: a cell lists one per composition, so
/// the first entry of that list belongs to the first composition only. A
/// composition without one falls back to the cell's first, then to 単縦陣.
pub(super) fn build_enemy_encounter(
    codex: &Codex,
    definition: &MapDefinition,
    stage: &MapVariantDefinition,
    cell_no: i64,
    locked: Option<&EnemyComposition>,
) -> Result<EnemyEncounter, GameplayError> {
    let enemy_fleet = resolve_sortie_enemy_fleet(definition.map_id, stage, cell_no);
    let composition = locked
        .cloned()
        .or_else(|| select_random_enemy_composition(&enemy_fleet))
        .unwrap_or_else(|| fallback_enemy_composition(cell_no));
    let formation_id =
        composition.formation.or_else(|| enemy_fleet.formations.first().copied()).unwrap_or(1);

    let level = (definition.level.max(1) * 5 + enemy_fleet.cell_no).max(1);
    let ship_ids = if composition.ship_ids.is_empty() {
        vec![FALLBACK_ENEMY_SHIP_ID]
    } else {
        composition.ship_ids.clone()
    };
    let ships = ship_ids
        .into_iter()
        .map(|ship_id| build_sortie_enemy_ship(codex, ship_id, level))
        .collect::<Result<Vec<_>, GameplayError>>()?;

    Ok(EnemyEncounter {
        ships,
        formation_id,
        level,
        rank: UserHQRank::RearAdmiral.get_name().to_string(),
        deck_name: format!("{}海域敵艦隊", definition.name),
    })
}

pub(super) fn build_sortie_enemy_ship(
    codex: &Codex,
    ship_id: i64,
    enemy_level: i64,
) -> Result<BattleShipInput, GameplayError> {
    if let Some((mut api_ship, slot_items)) = codex.new_enemy_ship(ship_id) {
        let exp_now = level::ship_level_required_exp(enemy_level.min(99));
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        api_ship.api_lv = enemy_level;
        api_ship.api_exp = [exp_now, next_exp, 0];
        return Ok(BattleShipInput {
            ship: api_ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        });
    }

    if let Some((mut api_ship, slot_items)) = codex.new_ship(ship_id) {
        warn!(ship_id, "enemy bootstrap data missing; using ship_extra fallback for sortie enemy",);
        let exp_now = level::ship_level_required_exp(enemy_level.min(99));
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        api_ship.api_lv = enemy_level;
        api_ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut api_ship, &slot_items, false)?;
        for (idx, slot_item) in slot_items.iter().take(5).enumerate() {
            api_ship.api_slot[idx] = slot_item.api_slotitem_id;
        }
        return Ok(BattleShipInput {
            ship: api_ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        });
    }

    let mst =
        codex.manifest.find_ship(ship_id).ok_or_else(|| {
            warn!(
                ship_id,
                "enemy bootstrap data missing and no manifest entry found for sortie enemy",
            );
            GameplayError::ManifestNotFound(ship_id)
        })?;
    Ok(build_manifest_only_sortie_enemy_ship(mst, ship_id, enemy_level))
}

#[derive(Debug)]
struct ManifestOnlyEnemyStats {
    sortno: i64,
    hp: [i64; 2],
    firepower: [i64; 2],
    torpedo: [i64; 2],
    aa: [i64; 2],
    armor: [i64; 2],
    asw: [i64; 2],
    luck: [i64; 2],
    range: i64,
    backs: i64,
    fuel: i64,
    bull: i64,
    missing_fields: Vec<&'static str>,
}

fn build_manifest_only_sortie_enemy_ship(
    mst: &ApiMstShip,
    ship_id: i64,
    enemy_level: i64,
) -> BattleShipInput {
    let fallback = manifest_only_enemy_stats(mst);
    if fallback.missing_fields.is_empty() {
        warn!(ship_id, "enemy bootstrap data missing; using manifest-only sortie enemy fallback",);
    } else {
        warn!(
            ship_id,
            missing_fields = ?fallback.missing_fields,
            "enemy bootstrap data missing; using degraded manifest-only sortie enemy fallback",
        );
    }
    let exp_now = level::ship_level_required_exp(enemy_level.min(99));
    let (_, next_exp) = level::exp_to_ship_level(exp_now);
    let hp = fallback.hp;
    let api_ship = KcApiShip {
        api_id: 0,
        api_sortno: fallback.sortno,
        api_ship_id: ship_id,
        api_lv: enemy_level,
        api_exp: [exp_now, next_exp, 0],
        api_nowhp: hp[0].max(1),
        api_maxhp: hp[0].max(1),
        api_soku: mst.api_soku,
        api_leng: fallback.range,
        api_slot: [-1; 5],
        api_onslot: [0; 5],
        api_slot_ex: 0,
        api_onslot_max: None,
        api_kyouka: [0; 7],
        api_backs: fallback.backs,
        api_fuel: fallback.fuel,
        api_bull: fallback.bull,
        api_slotnum: mst.api_slot_num,
        api_ndock_time: 0,
        api_ndock_item: [0, 0],
        api_srate: 0,
        api_cond: 49,
        api_karyoku: fallback.firepower,
        api_raisou: fallback.torpedo,
        api_taiku: fallback.aa,
        api_soukou: fallback.armor,
        api_kaihi: [0, 0],
        api_taisen: fallback.asw,
        api_sakuteki: [0, 0],
        api_lucky: fallback.luck,
        api_locked: 0,
        api_locked_equip: 0,
        api_sally_area: 0,
        api_sp_effect_items: None,
    };

    BattleShipInput {
        ship: api_ship,
        slot_items: Vec::<KcApiSlotItem>::new(),
        effect_list: vec![0],
        married: false,
    }
}

fn manifest_only_enemy_stats(mst: &ApiMstShip) -> ManifestOnlyEnemyStats {
    let mut missing_fields = Vec::new();
    let _ = manifest_onslot_or_default(mst.api_maxeq, "api_maxeq", &mut missing_fields);
    ManifestOnlyEnemyStats {
        sortno: mst.api_sortno.unwrap_or(mst.api_sort_id),
        hp: manifest_pair_or_default(mst.api_taik, [1, 1], "api_taik", &mut missing_fields),
        firepower: manifest_pair_or_default(mst.api_houg, [0, 0], "api_houg", &mut missing_fields),
        torpedo: manifest_pair_or_default(mst.api_raig, [0, 0], "api_raig", &mut missing_fields),
        aa: manifest_pair_or_default(mst.api_tyku, [0, 0], "api_tyku", &mut missing_fields),
        armor: manifest_pair_or_default(mst.api_souk, [0, 0], "api_souk", &mut missing_fields),
        asw: manifest_single_pair_or_default(mst.api_tais, [0, 0], "api_tais", &mut missing_fields),
        luck: manifest_pair_or_default(mst.api_luck, [0, 0], "api_luck", &mut missing_fields),
        range: mst.api_leng.unwrap_or(-1),
        backs: mst.api_backs.unwrap_or(-1),
        fuel: mst.api_fuel_max.unwrap_or(0),
        bull: mst.api_bull_max.unwrap_or(0),
        missing_fields,
    }
}

fn manifest_pair_or_default(
    value: Option<[i64; 2]>,
    default: [i64; 2],
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 2] {
    value.unwrap_or_else(|| {
        missing_fields.push(field);
        default
    })
}

fn manifest_single_pair_or_default(
    value: Option<[i64; 1]>,
    default: [i64; 2],
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 2] {
    value.map(|[stat]| [stat, stat]).unwrap_or_else(|| {
        missing_fields.push(field);
        default
    })
}

fn manifest_onslot_or_default(
    value: Option<[i64; 5]>,
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 5] {
    value.unwrap_or_else(|| {
        missing_fields.push(field);
        [0; 5]
    })
}

pub(super) fn resolve_sortie_enemy_fleet(
    map_id: i64,
    variant: &MapVariantDefinition,
    cell_no: i64,
) -> EnemyFleetDefinition {
    if let Some(enemy_fleet) = variant.enemy_fleets.get(&cell_no) {
        return enemy_fleet.clone();
    }

    warn!(
        map_id,
        cell_no, "missing enemy fleet definition for sortie cell; using fallback composition",
    );
    fallback_enemy_fleet(cell_no)
}

fn fallback_enemy_fleet(cell_no: i64) -> EnemyFleetDefinition {
    EnemyFleetDefinition {
        cell_no,
        battle_kind: 1,
        formations: vec![1],
        compositions: vec![fallback_enemy_composition(cell_no)],
    }
}

pub(super) fn fallback_enemy_composition(cell_no: i64) -> EnemyComposition {
    EnemyComposition {
        comp_id: format!("fallback:{cell_no}"),
        weight: 1,
        ship_ids: vec![FALLBACK_ENEMY_SHIP_ID],
        formation: Some(1),
        raw_ship_names: Vec::new(),
    }
}

pub(super) fn select_random_enemy_composition(
    enemy_fleet: &EnemyFleetDefinition,
) -> Option<EnemyComposition> {
    if enemy_fleet.compositions.is_empty() {
        return None;
    }

    let total_weight = enemy_fleet
        .compositions
        .iter()
        .map(|composition| composition.weight.max(1) as u64)
        .sum::<u64>();
    if total_weight == 0 {
        return enemy_fleet.compositions.first().cloned();
    }

    let roll = rng::u64(0..total_weight);
    select_enemy_composition_for_roll(enemy_fleet, roll).cloned()
}

pub(super) fn select_enemy_composition_for_roll(
    enemy_fleet: &EnemyFleetDefinition,
    mut roll: u64,
) -> Option<&EnemyComposition> {
    for composition in &enemy_fleet.compositions {
        let weight = composition.weight.max(1) as u64;
        if roll < weight {
            return Some(composition);
        }
        roll -= weight;
    }

    enemy_fleet.compositions.last()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use emukc_model::prelude::{Kc3rdEnemyShip, Kc3rdEnemyShipSlotInfo};

    use crate::game::battle::response::enemy_slot_ids;

    use super::*;
    use std::collections::BTreeMap;

    fn test_codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex").unwrap()
    }

    fn make_variant_with_enemy(
        cell_no: i64,
        enemy_fleet: EnemyFleetDefinition,
    ) -> MapVariantDefinition {
        let mut variant = empty_variant();
        variant.enemy_fleets.insert(cell_no, enemy_fleet);
        variant
    }

    fn empty_variant() -> MapVariantDefinition {
        MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 5,
            cells: vec![],
            routing_rules: BTreeMap::new(),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        }
    }

    fn make_definition(level: i64, name: &str) -> MapDefinition {
        MapDefinition {
            map_id: 11,
            maparea_id: 1,
            mapinfo_no: 1,
            name: name.to_string(),
            level,
            sally_flag: vec![],
            is_event: false,
            reset_policy: Default::default(),
            airbase_count: None,
            gauge_type: None,
            gauge_type_e: None,
            gauge_count: None,
            required_defeat_count: None,
            max_hp: None,
            default_variant: String::new(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::new(),
        }
    }

    fn composition(comp_id: &str, weight: i64, ship_ids: Vec<i64>) -> EnemyComposition {
        EnemyComposition {
            comp_id: comp_id.to_string(),
            weight,
            ship_ids,
            formation: Some(1),
            raw_ship_names: Vec::new(),
        }
    }

    // --- Happy path tests ---

    #[test]
    fn resolve_sortie_enemy_fleet_returns_correct_fleet_when_cell_has_data() {
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 3,
            battle_kind: 2,
            formations: vec![3, 4],
            compositions: vec![
                composition("comp_a", 1, vec![1501, 1502]),
                composition("comp_b", 2, vec![1503]),
            ],
        };
        let variant = make_variant_with_enemy(3, enemy_fleet.clone());

        let result = resolve_sortie_enemy_fleet(11, &variant, 3);
        assert_eq!(result.cell_no, 3);
        assert_eq!(result.battle_kind, 2);
        assert_eq!(result.formations, vec![3, 4]);
        assert_eq!(result.compositions.len(), 2);
        assert_eq!(result.compositions[0].ship_ids, vec![1501, 1502]);
        assert_eq!(result.compositions[1].weight, 2);
    }

    #[test]
    fn select_enemy_composition_for_roll_is_deterministic_by_weight() {
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 1,
            battle_kind: 1,
            formations: vec![1],
            compositions: vec![
                composition("light", 1, vec![1501]),
                composition("medium", 2, vec![1502]),
                composition("heavy", 3, vec![1503]),
            ],
        };

        // total_weight = 1 + 2 + 3 = 6
        // roll 0 → light (weight 1, 0 < 1)
        let c = select_enemy_composition_for_roll(&enemy_fleet, 0).unwrap();
        assert_eq!(c.comp_id, "light");

        // roll 1 → medium (1 >= 1, roll becomes 0; 0 < 2)
        let c = select_enemy_composition_for_roll(&enemy_fleet, 1).unwrap();
        assert_eq!(c.comp_id, "medium");

        // roll 3 → heavy (3 >= 1 → roll=2, 2 >= 2 → roll=0, 0 < 3)
        let c = select_enemy_composition_for_roll(&enemy_fleet, 3).unwrap();
        assert_eq!(c.comp_id, "heavy");

        // roll 5 → last (heavy) via fallthrough
        let c = select_enemy_composition_for_roll(&enemy_fleet, 5).unwrap();
        assert_eq!(c.comp_id, "heavy");
    }

    #[test]
    fn build_sortie_enemy_ships_creates_correct_enemy_list() {
        let codex = test_codex();

        let definition = make_definition(5, "鎮守府");
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 2,
            battle_kind: 1,
            formations: vec![1],
            compositions: vec![composition("std", 1, vec![1501, 1501, 1501])],
        };
        let comp = enemy_fleet.compositions[0].clone();
        let variant = make_variant_with_enemy(2, enemy_fleet);

        let encounter =
            build_enemy_encounter(&codex, &definition, &variant, 2, Some(&comp)).unwrap();
        let (ships, enemy_level, enemy_rank, deck_name) =
            (encounter.ships, encounter.level, encounter.rank, encounter.deck_name);

        assert_eq!(ships.len(), 3);
        // All ships should be the fallback DD I-class (1501)
        for ship in &ships {
            assert_eq!(ship.ship.api_ship_id, 1501);
        }
        // level = max(5, 1) * 5 + 2 = 27
        assert_eq!(enemy_level, 27);
        assert_eq!(enemy_rank, "少将");
        assert_eq!(deck_name, "鎮守府海域敵艦隊");
    }

    // --- Edge case tests ---

    #[test]
    fn resolve_sortie_enemy_fleet_fallback_uses_abyssal_dd_not_friendly_ids() {
        let variant = empty_variant();

        let result = resolve_sortie_enemy_fleet(11, &variant, 99);
        assert_eq!(result.compositions.len(), 1);

        let comp = &result.compositions[0];
        assert_eq!(comp.ship_ids, vec![1501]);
        // 1501 is Abyssal DD I-class, not a friendly ship ID (friendly IDs are <= ~700)
        assert!(comp.ship_ids[0] >= 1500, "fallback should use abyssal ship IDs, not friendly IDs");
    }

    #[test]
    fn fallback_enemy_composition_uses_abyssal_dd_id_1501() {
        let comp = fallback_enemy_composition(7);
        assert_eq!(comp.comp_id, "fallback:7");
        assert_eq!(comp.weight, 1);
        assert_eq!(comp.ship_ids, vec![1501]);
        assert_eq!(comp.formation, Some(1));
    }

    #[test]
    fn build_sortie_enemy_ships_uses_fallback_when_ship_ids_empty() {
        let codex = test_codex();

        let definition = make_definition(1, "test");
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 1,
            battle_kind: 1,
            formations: vec![1],
            compositions: vec![composition("empty", 1, vec![])],
        };
        let comp = enemy_fleet.compositions[0].clone();
        assert!(comp.ship_ids.is_empty());
        let variant = make_variant_with_enemy(1, enemy_fleet);

        let ships =
            build_enemy_encounter(&codex, &definition, &variant, 1, Some(&comp)).unwrap().ships;

        assert_eq!(ships.len(), 1);
        assert_eq!(ships[0].ship.api_ship_id, 1501);
    }

    #[test]
    fn build_sortie_enemy_ship_uses_new_enemy_ship_for_abyssal_id() {
        let codex = test_codex();

        // 1501 is Abyssal DD I-class — should be in enemy_ship_extra
        let result = build_sortie_enemy_ship(&codex, 1501, 30).unwrap();
        assert_eq!(result.ship.api_ship_id, 1501);
        assert_eq!(result.ship.api_lv, 30);
        assert!(result.ship.api_nowhp > 0);
        // enemy ships from new_enemy_ship should have valid HP and stats
        assert!(result.ship.api_maxhp > 0);
    }

    #[test]
    fn build_sortie_enemy_ship_uses_new_ship_for_friendly_id_as_fallback() {
        let codex = test_codex();

        // 1 is 睦月 (Mutsuki), a friendly ship — not in enemy_ship_extra,
        // so it falls through to the new_ship path
        let ship_id = 1;
        assert!(
            codex.new_enemy_ship(ship_id).is_none(),
            "friendly ship should not be in enemy_ship_extra"
        );

        let result = build_sortie_enemy_ship(&codex, ship_id, 50).unwrap();
        assert_eq!(result.ship.api_ship_id, ship_id);
        assert_eq!(result.ship.api_lv, 50);
        assert!(result.ship.api_nowhp > 0);
        // The friendly-ship-fallback path populates slot items
        assert!(!result.slot_items.is_empty());
    }

    #[test]
    fn select_enemy_composition_for_roll_returns_none_for_empty_compositions() {
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 1,
            battle_kind: 1,
            formations: vec![1],
            compositions: vec![],
        };
        assert!(select_enemy_composition_for_roll(&enemy_fleet, 0).is_none());
    }

    fn formed(comp_id: &str, formation: Option<i64>) -> EnemyComposition {
        EnemyComposition {
            formation,
            ..composition(comp_id, 1, vec![1501])
        }
    }

    fn encounter_formation(
        formations: Vec<i64>,
        compositions: Vec<EnemyComposition>,
        locked: Option<&EnemyComposition>,
    ) -> i64 {
        let codex = test_codex();
        let variant = make_variant_with_enemy(
            4,
            EnemyFleetDefinition {
                cell_no: 4,
                battle_kind: 1,
                formations,
                compositions,
            },
        );
        build_enemy_encounter(&codex, &make_definition(5, "test"), &variant, 4, locked)
            .unwrap()
            .formation_id
    }

    /// AE2: three compositions formed 1/3/4; locking the second fights in 3.
    #[test]
    fn encounter_formation_is_the_locked_compositions_own() {
        let comps = vec![formed("a", Some(1)), formed("b", Some(3)), formed("c", Some(4))];
        assert_eq!(encounter_formation(vec![1, 3, 4], comps.clone(), Some(&comps[1])), 3);
    }

    /// AE3: without its own formation a composition takes the cell's first, and
    /// with neither it is 単縦陣.
    #[test]
    fn encounter_formation_falls_back_to_the_cell_then_line_ahead() {
        let bare = formed("bare", None);
        assert_eq!(encounter_formation(vec![2, 5], vec![bare.clone()], Some(&bare)), 2);
        assert_eq!(encounter_formation(vec![], vec![bare.clone()], Some(&bare)), 1);
    }

    /// A cell with no enemy data fights the fallback 駆逐イ級 in 単縦陣.
    #[test]
    fn encounter_without_cell_data_is_the_fallback_destroyer() {
        let codex = test_codex();
        let encounter =
            build_enemy_encounter(&codex, &make_definition(5, "test"), &empty_variant(), 7, None)
                .unwrap();
        assert_eq!(encounter.formation_id, 1);
        assert_eq!(
            encounter.ships.iter().map(|s| s.ship.api_ship_id).collect::<Vec<_>>(),
            vec![FALLBACK_ENEMY_SHIP_ID]
        );
    }

    // --- Integration tests ---

    #[test]
    fn end_to_end_resolve_select_build_from_variant() {
        let codex = test_codex();

        let mut variant = empty_variant();
        variant.enemy_fleets.insert(
            3,
            EnemyFleetDefinition {
                cell_no: 3,
                battle_kind: 1,
                formations: vec![1, 2],
                compositions: vec![
                    composition("patrol", 1, vec![1501, 1501]),
                    composition("main", 2, vec![1501, 1501, 1501]),
                ],
            },
        );

        let definition = make_definition(3, "南西諸島");

        // Resolve fleet for cell 3
        let fleet = resolve_sortie_enemy_fleet(11, &variant, 3);
        assert_eq!(fleet.cell_no, 3);
        assert_eq!(fleet.compositions.len(), 2);

        // Deterministically select the heavier composition (roll=1 selects "main")
        let comp = select_enemy_composition_for_roll(&fleet, 1).unwrap();
        assert_eq!(comp.comp_id, "main");
        assert_eq!(comp.ship_ids.len(), 3);

        // Build enemy ships
        let encounter =
            build_enemy_encounter(&codex, &definition, &variant, 3, Some(comp)).unwrap();
        let (ships, level, rank, deck_name) =
            (encounter.ships, encounter.level, encounter.rank, encounter.deck_name);

        assert_eq!(ships.len(), 3);
        for ship in &ships {
            assert_eq!(ship.ship.api_ship_id, 1501);
            assert_eq!(ship.ship.api_lv, level);
        }
        // level = max(3,1)*5 + 3 = 18
        assert_eq!(level, 18);
        assert_eq!(rank, "少将");
        assert_eq!(deck_name, "南西諸島海域敵艦隊");
    }

    #[test]
    fn fallback_enemy_ship_ids_all_buildable_via_new_enemy_ship() {
        let codex = test_codex();

        // Verify 1501 (Abyssal DD I-class) can be built via new_enemy_ship
        let result = codex.new_enemy_ship(1501);
        assert!(result.is_some(), "Abyssal DD I-class (1501) should exist in enemy_ship_extra");

        let (ship, _slot_items) = result.unwrap();
        assert_eq!(ship.api_ship_id, 1501);
        assert!(ship.api_nowhp > 0);
        assert!(ship.api_maxhp > 0);

        // Also verify build_sortie_enemy_ship succeeds for 1501
        let battle_ship = build_sortie_enemy_ship(&codex, 1501, 1).unwrap();
        assert_eq!(battle_ship.ship.api_ship_id, 1501);
        assert!(battle_ship.ship.api_nowhp > 0);
    }

    fn enemy_test_codex() -> Codex {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let enemy_ship_id = 19991;
        let enemy_slot_id = 1519;
        codex.manifest.api_mst_ship.push(ApiMstShip {
            api_id: enemy_ship_id,
            api_name: "enemy-test".to_string(),
            api_yomi: "enemy-test".to_string(),
            api_stype: 7,
            api_ctype: 1,
            api_soku: 10,
            api_slot_num: 2,
            api_sort_id: enemy_ship_id,
            api_sortno: Some(enemy_ship_id),
            api_taik: Some([45, 45]),
            api_houg: Some([35, 35]),
            api_raig: Some([10, 10]),
            api_tyku: Some([40, 40]),
            api_souk: Some([20, 20]),
            api_tais: Some([30]),
            api_luck: Some([5, 5]),
            api_maxeq: Some([18, 6, 0, 0, 0]),
            api_leng: Some(2),
            api_backs: Some(4),
            api_fuel_max: Some(0),
            api_bull_max: Some(0),
            ..ApiMstShip::default()
        });
        codex.enemy_ship_extra.insert(
            enemy_ship_id,
            Kc3rdEnemyShip {
                api_id: enemy_ship_id,
                name: "enemy-test".to_string(),
                yomi: "enemy-test".to_string(),
                stype: 7,
                ctype: 1,
                hp: 45,
                firepower: 35,
                torpedo: 10,
                aa: 40,
                armor: 20,
                evasion: 12,
                asw: 30,
                los: 18,
                luck: 5,
                speed: 10,
                range: 2,
                rarity: 4,
                backs: 4,
                slot_num: 2,
                maxeq: [18, 6, 0, 0, 0],
                slots: vec![
                    Kc3rdEnemyShipSlotInfo {
                        item_id: enemy_slot_id,
                        onslot: 18,
                    },
                    Kc3rdEnemyShipSlotInfo {
                        item_id: 525,
                        onslot: 6,
                    },
                ],
            },
        );
        codex
    }

    fn manifest_only_test_codex(mst: ApiMstShip) -> Codex {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        codex.manifest.api_mst_ship.retain(|ship| ship.api_id != mst.api_id);
        codex.ship_extra.remove(&mst.api_id);
        codex.enemy_ship_extra.remove(&mst.api_id);
        codex.manifest.api_mst_ship.push(mst);
        codex
    }

    fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items, false).unwrap();
        BattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        }
    }

    #[test]
    fn build_sortie_enemy_ship_prefers_enemy_bootstrap_stats_and_slots() {
        let codex = enemy_test_codex();
        let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
        assert_eq!(enemy.ship.api_ship_id, 19991);
        assert_eq!(enemy.ship.api_nowhp, 45);
        assert_eq!(enemy.ship.api_karyoku, [35, 35]);
        assert_eq!(enemy.ship.api_taiku, [40, 40]);
        assert_eq!(enemy.ship.api_taisen, [30, 30]);
        assert_eq!(enemy.ship.api_onslot, [18, 6, 0, 0, 0]);
        assert_eq!(enemy_slot_ids(&enemy.ship, &enemy.slot_items), [1519, 525, -1, -1, -1]);
    }

    #[test]
    fn build_sortie_enemy_ship_drops_enemy_slots_missing_from_manifest() {
        let mut codex = enemy_test_codex();
        let enemy_extra = codex.enemy_ship_extra.get_mut(&19991).unwrap();
        enemy_extra.slots[0].item_id = 999999;

        let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
        assert_eq!(enemy.ship.api_onslot, [0, 6, 0, 0, 0]);
        assert_eq!(enemy_slot_ids(&enemy.ship, &enemy.slot_items), [-1, 525, -1, -1, -1]);
    }

    #[test]
    fn build_sortie_enemy_ship_uses_enemy_bootstrap_when_manifest_entry_is_missing() {
        let mut codex = enemy_test_codex();
        codex.manifest.api_mst_ship.retain(|ship| ship.api_id != 19991);
        assert!(codex.manifest.find_ship(19991).is_none());
        assert!(codex.new_ship(19991).is_none());

        let (bootstrap_ship, bootstrap_slots) = codex.new_enemy_ship(19991).unwrap();
        assert_eq!(bootstrap_ship.api_sortno, 19991);
        assert_eq!(bootstrap_ship.api_fuel, 0);
        assert_eq!(bootstrap_ship.api_bull, 0);
        assert_eq!(bootstrap_ship.api_onslot, [18, 6, 0, 0, 0]);
        assert_eq!(
            bootstrap_slots.iter().map(|slot| slot.api_slotitem_id).collect::<Vec<_>>(),
            vec![1519, 525]
        );

        let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
        assert_eq!(enemy.ship.api_ship_id, 19991);
        assert_eq!(enemy.ship.api_sortno, 19991);
        assert_eq!(enemy.ship.api_fuel, 0);
        assert_eq!(enemy.ship.api_bull, 0);
        assert_eq!(enemy.ship.api_nowhp, 45);
        assert_eq!(enemy.ship.api_karyoku, [35, 35]);
        assert_eq!(enemy.ship.api_taiku, [40, 40]);
        assert_eq!(enemy.ship.api_taisen, [30, 30]);
        assert_eq!(enemy.ship.api_onslot, [18, 6, 0, 0, 0]);
        assert_eq!(enemy_slot_ids(&enemy.ship, &enemy.slot_items), [1519, 525, -1, -1, -1]);
    }

    #[test]
    fn build_sortie_enemy_ship_falls_back_to_ship_extra_data_when_enemy_bootstrap_is_missing() {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let ship_id = 518;
        codex.enemy_ship_extra.remove(&ship_id);
        assert!(codex.new_enemy_ship(ship_id).is_none());

        let expected = sample_ship(&codex, ship_id, 55);
        let enemy = build_sortie_enemy_ship(&codex, ship_id, 55).unwrap();
        assert_eq!(enemy.ship.api_ship_id, ship_id);
        assert_eq!(enemy.ship.api_lv, 55);
        assert_eq!(enemy.ship.api_nowhp, expected.ship.api_nowhp);
        assert_eq!(enemy.ship.api_karyoku, expected.ship.api_karyoku);
        assert_eq!(enemy.ship.api_kaihi, expected.ship.api_kaihi);
        assert_eq!(enemy.ship.api_taisen, expected.ship.api_taisen);
        assert_eq!(enemy.ship.api_lucky, expected.ship.api_lucky);
        assert_eq!(enemy.ship.api_onslot, expected.ship.api_onslot);
        assert_eq!(
            enemy_slot_ids(&enemy.ship, &enemy.slot_items),
            enemy_slot_ids(&expected.ship, &expected.slot_items)
        );
    }

    #[test]
    fn build_sortie_enemy_ship_keeps_common_abyssals_buildable_without_enemy_bootstrap() {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        for ship_id in [1501, 1505, 1538] {
            codex.enemy_ship_extra.remove(&ship_id);
            assert!(codex.new_enemy_ship(ship_id).is_none());
            assert!(codex.new_ship(ship_id).is_none());

            let mst = codex.manifest.find_ship(ship_id).unwrap();
            let enemy = build_sortie_enemy_ship(&codex, ship_id, 45).unwrap();
            assert_eq!(enemy.ship.api_ship_id, ship_id);
            assert_eq!(enemy.ship.api_lv, 45);
            assert_eq!(enemy.ship.api_sortno, mst.api_sortno.unwrap_or(mst.api_sort_id));
            assert_eq!(enemy.ship.api_slotnum, mst.api_slot_num);
            assert_eq!(enemy.ship.api_nowhp, mst.api_taik.unwrap_or([1, 1])[0].max(1));
            assert_eq!(enemy.ship.api_karyoku, mst.api_houg.unwrap_or([0, 0]));
            assert_eq!(enemy.ship.api_taiku, mst.api_tyku.unwrap_or([0, 0]));
            assert_eq!(
                enemy.ship.api_taisen,
                mst.api_tais.map(|[stat]| [stat, stat]).unwrap_or([0, 0]),
            );
            assert_eq!(enemy.ship.api_lucky, mst.api_luck.unwrap_or([0, 0]));
            assert_eq!(enemy.ship.api_onslot, [0; 5]);
            assert!(enemy.slot_items.is_empty());
        }
    }

    #[test]
    fn build_sortie_enemy_ship_manifest_fallback_uses_available_manifest_stats() {
        let ship_id = 29991;
        let codex = manifest_only_test_codex(ApiMstShip {
            api_id: ship_id,
            api_name: "enemy-manifest-only".to_string(),
            api_yomi: "enemy-manifest-only".to_string(),
            api_stype: 7,
            api_ctype: 1,
            api_soku: 10,
            api_slot_num: 2,
            api_sort_id: ship_id,
            api_taik: Some([45, 45]),
            api_houg: Some([35, 35]),
            api_raig: Some([10, 10]),
            api_tyku: Some([40, 40]),
            api_souk: Some([20, 20]),
            api_tais: Some([30]),
            api_luck: Some([5, 5]),
            api_maxeq: Some([18, 6, 0, 0, 0]),
            api_leng: Some(2),
            api_backs: Some(4),
            api_fuel_max: Some(0),
            api_bull_max: Some(0),
            ..ApiMstShip::default()
        });

        let enemy = build_sortie_enemy_ship(&codex, ship_id, 45).unwrap();
        assert_eq!(enemy.ship.api_ship_id, ship_id);
        assert_eq!(enemy.ship.api_sortno, ship_id);
        assert_eq!(enemy.ship.api_nowhp, 45);
        assert_eq!(enemy.ship.api_karyoku, [35, 35]);
        assert_eq!(enemy.ship.api_raisou, [10, 10]);
        assert_eq!(enemy.ship.api_taiku, [40, 40]);
        assert_eq!(enemy.ship.api_soukou, [20, 20]);
        assert_eq!(enemy.ship.api_taisen, [30, 30]);
        assert_eq!(enemy.ship.api_lucky, [5, 5]);
        assert_eq!(enemy.ship.api_onslot, [0; 5]);
        assert!(enemy.slot_items.is_empty());
    }

    #[test]
    fn weighted_enemy_composition_selection_uses_weights() {
        let enemy_fleet = EnemyFleetDefinition {
            cell_no: 3,
            battle_kind: 1,
            formations: vec![1],
            compositions: vec![
                EnemyComposition {
                    comp_id: "light".to_string(),
                    weight: 1,
                    ship_ids: vec![501],
                    formation: Some(1),
                    raw_ship_names: Vec::new(),
                },
                EnemyComposition {
                    comp_id: "heavy".to_string(),
                    weight: 3,
                    ship_ids: vec![502],
                    formation: Some(1),
                    raw_ship_names: Vec::new(),
                },
            ],
        };

        assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 0).unwrap().comp_id, "light",);
        assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 1).unwrap().comp_id, "heavy",);
        assert_eq!(select_enemy_composition_for_roll(&enemy_fleet, 3).unwrap().comp_id, "heavy",);
    }

    #[test]
    fn fallback_enemy_fleet_is_only_used_when_catalog_data_is_missing() {
        let mut variant = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 5,
            cells: vec![],
            routing_rules: HashMap::new().into_iter().collect(),
            enemy_fleets: HashMap::new().into_iter().collect(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };
        variant.enemy_fleets.insert(
            2,
            EnemyFleetDefinition {
                cell_no: 2,
                battle_kind: 1,
                formations: vec![2],
                compositions: vec![EnemyComposition {
                    comp_id: "real".to_string(),
                    weight: 1,
                    ship_ids: vec![501, 502],
                    formation: Some(2),
                    raw_ship_names: Vec::new(),
                }],
            },
        );

        let real = resolve_sortie_enemy_fleet(11, &variant, 2);
        assert_eq!(real.formations, vec![2]);
        assert_eq!(real.compositions[0].ship_ids, vec![501, 502]);

        let fallback = resolve_sortie_enemy_fleet(11, &variant, 7);
        assert_eq!(fallback.compositions[0].ship_ids, vec![1501]);
    }
}
