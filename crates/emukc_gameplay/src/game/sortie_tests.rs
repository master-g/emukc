use super::*;
use crate::game::battle::sortie::{
    enemy_slot_ids, pending_battle, run_day_battle, run_sp_midnight_battle,
};
use crate::game::map_route::{FleetRouteContext, FleetRouteShipEntry};
use crate::prelude::*;
use emukc_bootstrap::prelude::build_final_map_catalog_from_repo_assets;
use emukc_db::{
    entity::profile::{map_record, material as profile_material, ship as profile_ship},
    prelude::new_mem_db,
    sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    },
};
use emukc_model::{
    codex::{
        Codex,
        map::{EnemyFleetDefinition, MapDefinition, MapVariantDefinition},
    },
    kc2::level,
    prelude::{ApiMstShip, Kc3rdEnemyShip, Kc3rdEnemyShipSlotInfo},
};
use emukc_time::chrono::Utc;
use std::collections::{BTreeMap, HashMap};

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

fn weaken_for_midnight(mut ship: BattleShipInput) -> BattleShipInput {
    ship.ship.api_karyoku[0] = 1;
    ship.ship.api_raisou[0] = 0;
    ship.ship.api_soukou[0] = 200;
    ship
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

fn successful_boss_snapshot() -> SortieBattleResultSnapshot {
    SortieBattleResultSnapshot {
        friendly_ship_ids: vec![],
        enemy_ship_ids: vec![],
        friendly_nowhps: vec![],
        enemy_ship_types: vec![],
        enemy_nowhps: vec![],
        win_rank: "S".to_string(),
        get_exp: 0,
        member_lv: 0,
        member_exp: 0,
        get_base_exp: 0,
        mvp: 0,
        get_ship_exp: vec![],
        get_exp_lvup: vec![],
        quest_name: String::new(),
        quest_level: 0,
        enemy_level: 0,
        enemy_rank: String::new(),
        enemy_deck_name: String::new(),
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
    assert_eq!(enemy_slot_ids(&enemy), [1519, 525, -1, -1, -1]);
}

#[test]
fn build_sortie_enemy_ship_drops_enemy_slots_missing_from_manifest() {
    let mut codex = enemy_test_codex();
    let enemy_extra = codex.enemy_ship_extra.get_mut(&19991).unwrap();
    enemy_extra.slots[0].item_id = 999999;

    let enemy = build_sortie_enemy_ship(&codex, 19991, 45).unwrap();
    assert_eq!(enemy.ship.api_onslot, [0, 6, 0, 0, 0]);
    assert_eq!(enemy_slot_ids(&enemy), [-1, 525, -1, -1, -1]);
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
    assert_eq!(enemy_slot_ids(&enemy), [1519, 525, -1, -1, -1]);
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
    assert_eq!(enemy_slot_ids(&enemy), enemy_slot_ids(&expected));
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

#[tokio::test]
async fn sortie_midnight_battle_updates_pending_snapshot() {
    use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
    let store = &*GLOBAL_SORTIE_STORE;
    store.clear();

    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex.clone());
    let profile_id = 42;

    let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
    let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));
    let session = run_day_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![friend.clone()],
                enemy_ships: vec![enemy.clone()],
            },
        },
    );

    assert_eq!(session.packet.midnight_flag, 1);
    store.insert_pending_result(
        profile_id,
        SortieBattleResultSnapshot {
            friendly_ship_ids: session.friendly_ship_ids.clone(),
            enemy_ship_ids: session.enemy_ship_ids.clone(),
            friendly_nowhps: session.friendly.iter().map(|f| f.hp().max(0)).collect(),
            enemy_ship_types: session
                .enemy_ship_ids
                .iter()
                .map(|&id| codex.find::<ApiMstShip>(&id).map(|m| m.api_stype).unwrap_or(0))
                .collect(),
            enemy_nowhps: session.packet.enemy_nowhps.clone(),
            win_rank: session.outcome.win_rank.to_string(),
            get_exp: 0,
            member_lv: 1,
            member_exp: 0,
            get_base_exp: 30,
            mvp: session.outcome.mvp,
            get_ship_exp: vec![],
            get_exp_lvup: vec![],
            quest_name: "test".to_string(),
            quest_level: 1,
            enemy_level: 1,
            enemy_rank: "Test".to_string(),
            enemy_deck_name: "Test".to_string(),
        },
    );

    let response = context.sortie_midnight_battle(profile_id).await.unwrap();
    assert_eq!(response.api_deck_id, 1);
    assert!(response.api_hougeki.is_some());

    let updated_snapshot = store.take_pending_result(profile_id).unwrap();
    assert!(!updated_snapshot.win_rank.is_empty());
    assert!(updated_snapshot.mvp >= 1);

    let stored = pending_battle(store, profile_id).unwrap();
    assert_eq!(stored.packet.midnight_flag, 0);

    let _ = take_day_battle_result(store, profile_id);
    store.clear();
}

#[tokio::test]
async fn sortie_sp_midnight_battle_runs_night_only() {
    use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
    let store = &*GLOBAL_SORTIE_STORE;
    store.clear();

    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let profile_id = 84;

    let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
    let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));

    let (day_session, night_session) = run_sp_midnight_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![friend.clone()],
                enemy_ships: vec![enemy.clone()],
            },
        },
        1,
    );

    // Day packet should have no combat phases (sp_midnight skips day battle)
    assert!(day_session.packet.kouku.is_none());
    assert!(day_session.packet.hougeki1.is_none());
    assert!(day_session.packet.opening_taisen.is_none());
    assert_eq!(day_session.packet.hourai_flag, [0, 0, 0, 0]);

    // Night battle should have run
    assert!(night_session.packet.hougeki.is_some());
    assert_eq!(night_session.profile_id, profile_id);

    // The stored session should have been updated with night results
    let stored = pending_battle(store, profile_id).unwrap();
    assert_eq!(stored.packet.midnight_flag, 0); // no further midnight allowed

    clear_pending_sortie_runtime_state(store, profile_id);
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

#[tokio::test]
async fn maelstrom_drains_ship_resource_without_touching_profile_materials() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex);
    let account = context.sign_up("maelstrom-loss", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "maelstrom-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let ship = context.add_ship(profile_id, 951).await.unwrap();

    let ship_before =
        profile_ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();
    let materials_before = profile_material::Entity::find()
        .filter(profile_material::Column::ProfileId.eq(profile_id))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();

    let cell = MapCellDefinition {
        cell_no: 99,
        color_no: 3,
        event_id: 3,
        event_kind: 0,
        next_cells: vec![],
        node_label: Some("H".to_string()),
        master_cell_id: None,
        distance: None,
    };

    let (itemget, happening) = resolve_non_battle_node_effect(
        &context.0,
        &context.1,
        profile_id,
        &cell,
        &[ship_before.clone()],
    )
    .await
    .unwrap();
    assert!(itemget.is_none());
    let happening = happening.expect("maelstrom should produce a happening response");
    assert_eq!(happening.resource_type, 1);
    assert!(happening.amount > 0);

    let ship_after =
        profile_ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();
    let materials_after = profile_material::Entity::find()
        .filter(profile_material::Column::ProfileId.eq(profile_id))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(materials_after, materials_before);
    assert_eq!(ship_after.ammo, ship_before.ammo);
    assert!(ship_after.fuel < ship_before.fuel);
    assert_eq!(ship_before.fuel - ship_after.fuel, happening.amount);
}

#[test]
fn eligible_sortie_ship_drops_skip_limited_and_non_victory_results() {
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 3,
        cells: vec![],
        routing_rules: BTreeMap::new(),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::from([(
            1,
            vec![
                emukc_model::codex::map::ShipDropDefinition {
                    ship_id: 1,
                    raw_ship_name: "睦月".to_string(),
                    tags: Vec::new(),
                },
                emukc_model::codex::map::ShipDropDefinition {
                    ship_id: 2,
                    raw_ship_name: "如月".to_string(),
                    tags: vec!["limited".to_string()],
                },
                emukc_model::codex::map::ShipDropDefinition {
                    ship_id: 999999,
                    raw_ship_name: "missing".to_string(),
                    tags: Vec::new(),
                },
            ],
        )]),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };

    let eligible = eligible_sortie_ship_drops(&codex, &variant, 1, "S");
    assert_eq!(eligible.len(), 1);
    assert_eq!(eligible[0].ship_id, 1);
    assert!(eligible_sortie_ship_drops(&codex, &variant, 1, "C").is_empty());
}

fn empty_stage() -> MapStageDefinition {
    MapStageDefinition::default()
}

#[test]
fn route_predicate_matches_ship_set_variants() {
    fn route_entry(
        ship_id: i64,
        ship_type: i64,
        speed: i64,
        slotitem_types: &[i64],
    ) -> FleetRouteShipEntry {
        FleetRouteShipEntry {
            ship_id,
            ship_type,
            speed,
            slotitem_types: slotitem_types.iter().copied().collect(),
        }
    }

    let context = FleetRouteContext {
        fleet_size: 3,
        visited_cell_ids: BTreeSet::new(),
        ship_ids: BTreeSet::from([526, 6001, 6002]),
        flagship_ship_id: Some(526),
        flagship_ship_type: Some(7),
        ship_type_counts: BTreeMap::from([(2, 2), (7, 1)]),
        ship_entries: vec![
            route_entry(526, 7, 10, &[]),
            route_entry(6001, 2, 10, &[]),
            route_entry(6002, 2, 10, &[]),
        ],
        min_speed: 10,
        los_total: 20,
        total_drums: 0,
        ..Default::default()
    };

    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::ContainsShipSet {
                ship_types: vec![1],
                ship_ids: vec![526],
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::OnlyShipSet {
                ship_types: vec![2],
                ship_ids: vec![526],
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::ShipSetCount {
                ship_types: vec![2],
                ship_ids: vec![526],
                op: RouteOperator::Eq,
                value: 3,
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::FlagshipShipId {
                ship_ids: vec![526],
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
}

#[test]
fn route_predicate_matches_visited_equipment_and_speed_qualified_predicates() {
    let context = FleetRouteContext {
        fleet_size: 4,
        visited_cell_ids: BTreeSet::from([1, 4]),
        ship_ids: BTreeSet::from([9001, 9002, 9003, 9004]),
        flagship_ship_id: Some(9001),
        flagship_ship_type: Some(3),
        ship_type_counts: BTreeMap::from([(3, 1), (8, 2), (11, 1)]),
        ship_entries: vec![
            FleetRouteShipEntry {
                ship_id: 9001,
                ship_type: 3,
                speed: 10,
                slotitem_types: BTreeSet::from([12]),
            },
            FleetRouteShipEntry {
                ship_id: 9002,
                ship_type: 8,
                speed: 5,
                slotitem_types: BTreeSet::new(),
            },
            FleetRouteShipEntry {
                ship_id: 9003,
                ship_type: 8,
                speed: 5,
                slotitem_types: BTreeSet::new(),
            },
            FleetRouteShipEntry {
                ship_id: 9004,
                ship_type: 11,
                speed: 10,
                slotitem_types: BTreeSet::new(),
            },
        ],
        min_speed: 5,
        los_total: 20,
        total_drums: 0,
        ..Default::default()
    };

    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::VisitedNode {
                cell_nos: vec![4],
                visited: true,
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::VisitedNode {
                cell_nos: vec![7],
                visited: false,
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::EquipmentCount {
                slotitem_types: vec![12, 13, 93],
                op: RouteOperator::Eq,
                value: 1,
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::FlagshipShipType {
                ship_types: vec![3],
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
    assert!(matches!(
        route_predicate_matches(
            &RoutePredicate::ShipSetSpeedCount {
                ship_types: vec![8],
                ship_ids: vec![],
                speed_op: RouteOperator::Lte,
                speed_class: SpeedClass::Slow,
                op: RouteOperator::Gte,
                value: 2,
            },
            &context,
            &empty_stage(),
        ),
        crate::game::map_route::RoutePredicateEval::Matched
    ));
}

#[test]
fn route_rules_prefer_executable_predicates_over_static_next_cells() {
    let current = MapCellDefinition {
        cell_no: 1,
        color_no: 4,
        event_id: 4,
        event_kind: 1,
        next_cells: vec![2, 3],
        node_label: None,
        master_cell_id: None,
        distance: None,
    };
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 3,
        cells: vec![current.clone()],
        routing_rules: BTreeMap::from([(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::ContainsShipType {
                        ship_types: vec![13],
                    },
                    raw_text: "潜水艦を含む".to_string(),
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 1,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: "それ以外".to_string(),
                },
            ],
        )]),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };
    let context = FleetRouteContext {
        fleet_size: 4,
        visited_cell_ids: BTreeSet::new(),
        ship_ids: BTreeSet::new(),
        flagship_ship_id: None,
        flagship_ship_type: None,
        ship_type_counts: BTreeMap::from([(2, 4)]),
        ship_entries: vec![
            FleetRouteShipEntry::default(),
            FleetRouteShipEntry::default(),
            FleetRouteShipEntry::default(),
            FleetRouteShipEntry::default(),
        ],
        min_speed: 10,
        los_total: 20,
        total_drums: 0,
        ..Default::default()
    };

    let next = evaluate_route_destination(&current, &variant, &context, None).unwrap();
    assert_eq!(next, 3);
}

#[test]
fn route_rules_use_unique_unconditional_fallback_when_predicate_is_unknown() {
    let current = MapCellDefinition {
        cell_no: 1,
        color_no: 4,
        event_id: 4,
        event_kind: 1,
        next_cells: vec![2, 3],
        node_label: None,
        master_cell_id: None,
        distance: None,
    };
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 3,
        cells: vec![current.clone()],
        routing_rules: BTreeMap::from([(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Unknown {
                        raw_text: "ランダム".to_string(),
                    },
                    raw_text: "ランダム".to_string(),
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 1,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: "それ以外".to_string(),
                },
            ],
        )]),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };

    let next = evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
        .unwrap();
    assert_eq!(next, 3);
}

#[test]
fn weighted_route_selection_uses_weights() {
    let weights = BTreeMap::from([(2, 45_u64), (3, 55_u64)]);
    assert_eq!(select_route_target_for_roll(&weights, 0), Some(2));
    assert_eq!(select_route_target_for_roll(&weights, 44), Some(2));
    assert_eq!(select_route_target_for_roll(&weights, 45), Some(3));
    assert_eq!(select_route_target_for_roll(&weights, 99), Some(3));
}

#[test]
fn selected_route_is_accepted_when_all_rules_are_unknown() {
    let current = MapCellDefinition {
        cell_no: 1,
        color_no: 4,
        event_id: 4,
        event_kind: 1,
        next_cells: vec![2, 3],
        node_label: None,
        master_cell_id: None,
        distance: None,
    };
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 3,
        cells: vec![current.clone()],
        routing_rules: BTreeMap::from([(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Unknown {
                        raw_text: "能動分岐".to_string(),
                    },
                    raw_text: "能動分岐".to_string(),
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 1,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Unknown {
                        raw_text: "能動分岐".to_string(),
                    },
                    raw_text: "能動分岐".to_string(),
                },
            ],
        )]),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };

    let next =
        evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), Some(3))
            .unwrap();
    assert_eq!(next, 3);
}

#[test]
fn fallback_rule_does_not_compete_with_matching_specific_rule() {
    let current = MapCellDefinition {
        cell_no: 1,
        color_no: 4,
        event_id: 4,
        event_kind: 1,
        next_cells: vec![2, 3],
        node_label: None,
        master_cell_id: None,
        distance: None,
    };
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 3,
        cells: vec![current.clone()],
        routing_rules: BTreeMap::from([(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::ContainsShipType {
                        ship_types: vec![13],
                    },
                    raw_text: "潜水艦を含む".to_string(),
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 1,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: "それ以外".to_string(),
                },
            ],
        )]),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };
    let context = FleetRouteContext {
        fleet_size: 4,
        visited_cell_ids: BTreeSet::new(),
        ship_ids: BTreeSet::new(),
        flagship_ship_id: None,
        flagship_ship_type: None,
        ship_type_counts: BTreeMap::from([(13, 1)]),
        ship_entries: vec![
            FleetRouteShipEntry {
                ship_id: 1601,
                ship_type: 13,
                speed: 10,
                slotitem_types: BTreeSet::new(),
            },
            FleetRouteShipEntry::default(),
            FleetRouteShipEntry::default(),
            FleetRouteShipEntry::default(),
        ],
        min_speed: 10,
        los_total: 20,
        total_drums: 0,
        ..Default::default()
    };

    let next = evaluate_route_destination(&current, &variant, &context, None).unwrap();
    assert_eq!(next, 2);
}

#[test]
fn cell_zero_uses_explicit_start_rules_before_static_next_cells() {
    let current = MapCellDefinition {
        cell_no: 0,
        color_no: 0,
        event_id: 0,
        event_kind: 0,
        next_cells: vec![1, 2],
        node_label: Some("Start".to_string()),
        master_cell_id: None,
        distance: None,
    };
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 2,
        cells: vec![
            current.clone(),
            MapCellDefinition {
                cell_no: 1,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("A".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 2,
                color_no: 5,
                event_id: 5,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("C".to_string()),
                master_cell_id: None,
                distance: None,
            },
        ],
        routing_rules: BTreeMap::from([(
            0,
            vec![RouteRule {
                from_cell_no: 0,
                to_cell_no: 2,
                priority: 0,
                weight: None,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text: "出撃".to_string(),
            }],
        )]),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };

    let next = evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
        .unwrap();
    assert_eq!(next, 2);
}

#[test]
fn ambiguous_cell_zero_without_rules_is_rejected() {
    let current = MapCellDefinition {
        cell_no: 0,
        color_no: 0,
        event_id: 0,
        event_kind: 0,
        next_cells: vec![1, 2],
        node_label: Some("Start".to_string()),
        master_cell_id: None,
        distance: None,
    };
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 2,
        cells: vec![current.clone()],
        routing_rules: BTreeMap::new(),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: vec!["missing_start_routes".to_string()],
    };

    let error = evaluate_route_destination(&current, &variant, &FleetRouteContext::default(), None)
        .unwrap_err();
    assert!(error.to_string().contains("explicit start routing rules"));
}

#[test]
fn start_source_cells_include_nonzero_route_cell_roots() {
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 14,
        cells: vec![
            MapCellDefinition {
                cell_no: 0,
                color_no: 0,
                event_id: 0,
                event_kind: 0,
                next_cells: vec![1, 2],
                node_label: Some("Start".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 1,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("A".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 2,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("B".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 13,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![14],
                node_label: Some("M".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 14,
                color_no: 5,
                event_id: 5,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("N".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 22,
                color_no: 0,
                event_id: 0,
                event_kind: 0,
                next_cells: vec![13],
                node_label: Some("Start".to_string()),
                master_cell_id: None,
                distance: None,
            },
        ],
        routing_rules: BTreeMap::new(),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };

    let sources =
        start_source_cells(&variant).into_iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

    assert_eq!(sources, vec![0, 22]);
}

#[tokio::test]
async fn first_gauge_clear_switches_map_variant_without_finishing_map() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex);
    let account = context.sign_up("variant-switch", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "variant-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    if let Ok(record) = find_map_record_impl(&context.0, profile_id, 73).await {
        let mut am = record.into_active_model();
        am.cleared = ActiveValue::Set(false);
        am.unlocked = ActiveValue::Set(true);
        am.last_cleared_at = ActiveValue::Set(None);
        am.last_reset_at = ActiveValue::Set(Some(now));
        am.defeat_count = ActiveValue::Set(Some(2));
        am.current_hp = ActiveValue::Set(None);
        am.gauge_index = ActiveValue::Set(1);
        assign_stage_id(&mut am, Some("pre_p_unlock".to_string()));
        am.selected_rank = ActiveValue::Set(map_record::SelectedRank::NotSet);
        am.event_state = ActiveValue::Set(None);
        am.update(&context.0).await.unwrap();
    } else {
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(73),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(Some(2)),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
        }
        .insert(&context.0)
        .await
        .unwrap();
    }

    let definition = context.1.maps.map_definition(73).unwrap().clone();
    assert_eq!(definition.default_variant, "pre_p_unlock");
    assert_eq!(definition.gauge_count, Some(2));
    let variant = definition.variant("pre_p_unlock").unwrap().clone();
    assert_eq!(variant.required_defeat_count, Some(3));
    assert_eq!(variant.clear_to_variant_key.as_deref(), Some("post_p_unlock"));
    let snapshot = successful_boss_snapshot();

    assert_eq!(
        apply_sortie_map_result(&context.0, profile_id, &definition, &variant, true, &snapshot)
            .await
            .unwrap(),
        0
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(73))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();
    assert!(!record.cleared);
    assert_eq!(record.defeat_count, Some(0));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.stage_id.as_deref(), Some("post_p_unlock"));
    assert!(record.last_cleared_at.is_none());
}

#[tokio::test]
async fn start_sortie_returns_post_p_unlock_layout_after_first_gauge_clear() {
    let db = new_mem_db().await.unwrap();
    let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    codex.maps =
        build_final_map_catalog_from_repo_assets("../../.data/temp", &codex.manifest).unwrap();
    let context = (db, codex);
    let account = context.sign_up("variant-layout", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "variant-layout-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    if let Ok(record) = find_map_record_impl(&context.0, profile_id, 73).await {
        let mut am = record.into_active_model();
        am.cleared = ActiveValue::Set(false);
        am.unlocked = ActiveValue::Set(true);
        am.last_cleared_at = ActiveValue::Set(None);
        am.last_reset_at = ActiveValue::Set(Some(now));
        am.defeat_count = ActiveValue::Set(Some(2));
        am.current_hp = ActiveValue::Set(None);
        am.gauge_index = ActiveValue::Set(1);
        assign_stage_id(&mut am, Some("pre_p_unlock".to_string()));
        am.selected_rank = ActiveValue::Set(map_record::SelectedRank::NotSet);
        am.event_state = ActiveValue::Set(None);
        am.update(&context.0).await.unwrap();
    } else {
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(73),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(Some(2)),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
        }
        .insert(&context.0)
        .await
        .unwrap();
    }

    let definition = context.1.maps.map_definition(73).unwrap().clone();
    let variant = definition.variant("pre_p_unlock").unwrap().clone();
    let snapshot = successful_boss_snapshot();
    apply_sortie_map_result(&context.0, profile_id, &definition, &variant, true, &snapshot)
        .await
        .unwrap();

    let ship = context.add_ship(profile_id, 951).await.unwrap();
    context.update_fleet_ships(profile_id, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

    let response = context.start_sortie(profile_id, 1, 7, 3, 1).await.unwrap();
    let cell_nos = response.cell_data.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

    assert!(cell_nos.iter().any(|cell_no| *cell_no > 16));
    assert!(cell_nos.contains(&25));
    assert_eq!(response.cell_data.first().map(|cell| cell.cell_no), Some(0));
    assert_eq!(response.cell_data.last().map(|cell| cell.cell_no), Some(25));
}

#[tokio::test]
async fn hp_gauge_clear_advances_to_next_gauge_before_marking_map_cleared() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex);
    let account = context.sign_up("hp-gauge", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "hp-gauge-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    let definition = MapDefinition {
        map_id: 99011,
        maparea_id: 99,
        mapinfo_no: 11,
        name: "hp gauge".to_string(),
        level: 1,
        sally_flag: vec![],
        is_event: true,
        reset_policy: Default::default(),
        airbase_count: None,
        gauge_type: Some(2),
        gauge_count: Some(2),
        required_defeat_count: None,
        max_hp: Some(1),
        default_variant: String::new(),
        rank_stage_ids: BTreeMap::new(),
        variants: BTreeMap::from([(
            String::new(),
            MapStageDefinition {
                variant_key: String::new(),
                ..Default::default()
            },
        )]),
    };
    let stage = definition.variant("").unwrap().clone();
    map_record::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        map_id: ActiveValue::Set(definition.map_id),
        cleared: ActiveValue::Set(false),
        last_cleared_at: ActiveValue::Set(None),
        last_reset_at: ActiveValue::Set(Some(now)),
        defeat_count: ActiveValue::Set(None),
        current_hp: ActiveValue::Set(Some(1)),
        gauge_index: ActiveValue::Set(1),
        stage_id: ActiveValue::Set(None),
        selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
        event_state: ActiveValue::Set(Some(1)),
        unlocked: ActiveValue::Set(true),
    }
    .insert(&context.0)
    .await
    .unwrap();

    assert_eq!(
        apply_sortie_map_result(
            &context.0,
            profile_id,
            &definition,
            &stage,
            true,
            &successful_boss_snapshot(),
        )
        .await
        .unwrap(),
        0
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(definition.map_id))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();
    assert!(!record.cleared);
    assert_eq!(record.current_hp, Some(1));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.event_state, Some(1));
    assert!(record.last_cleared_at.is_none());
}

#[tokio::test]
async fn hp_gauge_clear_switches_stage_before_marking_map_cleared() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex);
    let account = context.sign_up("hp-stage", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "hp-stage-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    let definition = MapDefinition {
        map_id: 99012,
        maparea_id: 99,
        mapinfo_no: 12,
        name: "hp stage".to_string(),
        level: 1,
        sally_flag: vec![],
        is_event: true,
        reset_policy: Default::default(),
        airbase_count: None,
        gauge_type: Some(2),
        gauge_count: Some(2),
        required_defeat_count: None,
        max_hp: Some(1),
        default_variant: "pre".to_string(),
        rank_stage_ids: BTreeMap::new(),
        variants: BTreeMap::from([
            (
                "pre".to_string(),
                MapStageDefinition {
                    variant_key: "pre".to_string(),
                    clear_to_variant_key: Some("post".to_string()),
                    ..Default::default()
                },
            ),
            (
                "post".to_string(),
                MapStageDefinition {
                    variant_key: "post".to_string(),
                    ..Default::default()
                },
            ),
        ]),
    };
    let stage = definition.variant("pre").unwrap().clone();
    map_record::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        map_id: ActiveValue::Set(definition.map_id),
        cleared: ActiveValue::Set(false),
        last_cleared_at: ActiveValue::Set(None),
        last_reset_at: ActiveValue::Set(Some(now)),
        defeat_count: ActiveValue::Set(None),
        current_hp: ActiveValue::Set(Some(1)),
        gauge_index: ActiveValue::Set(1),
        stage_id: ActiveValue::Set(Some("pre".to_string())),
        selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
        event_state: ActiveValue::Set(Some(1)),
        unlocked: ActiveValue::Set(true),
    }
    .insert(&context.0)
    .await
    .unwrap();

    assert_eq!(
        apply_sortie_map_result(
            &context.0,
            profile_id,
            &definition,
            &stage,
            true,
            &successful_boss_snapshot(),
        )
        .await
        .unwrap(),
        0
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(definition.map_id))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();
    assert!(!record.cleared);
    assert_eq!(record.current_hp, Some(1));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.stage_id.as_deref(), Some("post"));
    assert_eq!(record.event_state, Some(1));
    assert!(record.last_cleared_at.is_none());
}

#[tokio::test]
async fn final_hp_gauge_clear_marks_map_cleared() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex);
    let account = context.sign_up("hp-final", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "hp-final-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    let definition = MapDefinition {
        map_id: 99013,
        maparea_id: 99,
        mapinfo_no: 13,
        name: "hp final".to_string(),
        level: 1,
        sally_flag: vec![],
        is_event: true,
        reset_policy: Default::default(),
        airbase_count: None,
        gauge_type: Some(2),
        gauge_count: Some(2),
        required_defeat_count: None,
        max_hp: Some(1),
        default_variant: String::new(),
        rank_stage_ids: BTreeMap::new(),
        variants: BTreeMap::from([(
            String::new(),
            MapStageDefinition {
                variant_key: String::new(),
                ..Default::default()
            },
        )]),
    };
    let stage = definition.variant("").unwrap().clone();
    map_record::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        map_id: ActiveValue::Set(definition.map_id),
        cleared: ActiveValue::Set(false),
        last_cleared_at: ActiveValue::Set(None),
        last_reset_at: ActiveValue::Set(Some(now)),
        defeat_count: ActiveValue::Set(None),
        current_hp: ActiveValue::Set(Some(1)),
        gauge_index: ActiveValue::Set(2),
        stage_id: ActiveValue::Set(None),
        selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
        event_state: ActiveValue::Set(Some(1)),
        unlocked: ActiveValue::Set(true),
    }
    .insert(&context.0)
    .await
    .unwrap();

    assert_eq!(
        apply_sortie_map_result(
            &context.0,
            profile_id,
            &definition,
            &stage,
            true,
            &successful_boss_snapshot(),
        )
        .await
        .unwrap(),
        1
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(definition.map_id))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();
    assert!(record.cleared);
    assert_eq!(record.current_hp, Some(0));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.event_state, Some(2));
    assert!(record.last_cleared_at.is_some());
}

#[tokio::test]
async fn clearing_map_1_1_unlocks_dependents_via_cascade() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = (db, codex);
    let account = context.sign_up("cascade-test", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "cascade-tester").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;

    let catalog = active_map_catalog(&context.1);
    let deps = catalog.dependents_of(11);
    assert!(!deps.is_empty(), "1-1 should have dependents");

    // Verify dependents start locked
    for &dep_id in &deps {
        let rec = find_map_record_impl(&context.0, profile_id, dep_id).await.unwrap();
        assert!(!rec.unlocked, "dependent {dep_id} should start locked");
    }

    // Simulate Boss win on 1-1 through the actual cascade
    let definition = catalog.as_ref().map_definition(11).unwrap();
    let stage = definition.stage("").unwrap();
    let snapshot = successful_boss_snapshot();

    let first_clear = apply_sortie_map_result(
        &context.0, profile_id, definition, stage, true, // boss cell
        &snapshot,
    )
    .await
    .unwrap();
    assert_eq!(first_clear, 1, "first clear should return 1");

    let unlocked =
        check_and_unlock_dependencies_impl(&context.0, &context.1, profile_id, 11).await.unwrap();
    assert!(!unlocked.is_empty(), "should unlock at least one map");

    // Verify dependents are now unlocked
    for &dep_id in &deps {
        let rec = find_map_record_impl(&context.0, profile_id, dep_id).await.unwrap();
        assert!(rec.unlocked, "dependent {dep_id} should be unlocked after clearing 1-1");
    }
}
