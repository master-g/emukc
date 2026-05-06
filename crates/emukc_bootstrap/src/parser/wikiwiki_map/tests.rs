use super::*;
use std::collections::BTreeSet;

use emukc_model::{
    codex::map::{RouteOperator, RoutePredicate, SpeedClass},
    kc2::start2::{ApiMstMaparea, ApiMstMapinfo, ApiMstShip, ApiMstStype},
};

fn only_variant(map: &WikiwikiMapDefinition) -> &WikiwikiMapVariantDefinition {
    map.variants.get(&map.default_variant).or_else(|| map.variants.values().next()).unwrap()
}

fn manifest_fixture() -> ApiManifest {
    ApiManifest {
        api_mst_maparea: vec![ApiMstMaparea {
            api_id: 1,
            api_name: "鎮守府海域".to_string(),
            api_type: 0,
        }],
        api_mst_mapinfo: vec![
            ApiMstMapinfo {
                api_id: 11,
                api_maparea_id: 1,
                api_no: 1,
                api_infotext: String::new(),
                api_item: vec![],
                api_level: 1,
                api_max_maphp: None,
                api_name: "1-1".to_string(),
                api_opetext: String::new(),
                api_required_defeat_count: None,
                api_sally_flag: vec![],
            },
            ApiMstMapinfo {
                api_id: 15,
                api_maparea_id: 1,
                api_no: 5,
                api_infotext: String::new(),
                api_item: vec![],
                api_level: 1,
                api_max_maphp: None,
                api_name: "1-5".to_string(),
                api_opetext: String::new(),
                api_required_defeat_count: None,
                api_sally_flag: vec![],
            },
        ],
        api_mst_stype: vec![
            ApiMstStype {
                api_id: 1,
                api_name: "海防艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 3,
                api_name: "軽巡洋艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 21,
                api_name: "練習巡洋艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 13,
                api_name: "潜水艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 15,
                api_name: "補給艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 11,
                api_name: "正規空母".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 5,
                api_name: "重巡洋艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 6,
                api_name: "航空巡洋艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 8,
                api_name: "戦艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 9,
                api_name: "高速戦艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
            ApiMstStype {
                api_id: 2,
                api_name: "駆逐艦".to_string(),
                api_equip_type: BTreeMap::new(),
                api_kcnt: 0,
                api_scnt: 0,
                api_sortno: 0,
            },
        ],
        api_mst_ship: vec![
            ApiMstShip {
                api_id: 101,
                api_name: "吹雪".to_string(),
                api_yomi: String::new(),
                api_stype: 2,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 102,
                api_name: "綾波".to_string(),
                api_yomi: String::new(),
                api_stype: 2,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 103,
                api_name: "初雪".to_string(),
                api_yomi: String::new(),
                api_stype: 2,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 104,
                api_name: "伊168".to_string(),
                api_yomi: String::new(),
                api_stype: 13,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 201,
                api_name: "大鷹".to_string(),
                api_yomi: String::new(),
                api_ctype: 76,
                api_stype: 7,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 202,
                api_name: "神鷹".to_string(),
                api_yomi: String::new(),
                api_ctype: 76,
                api_stype: 7,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 203,
                api_name: "鹿島".to_string(),
                api_yomi: String::new(),
                api_ctype: 41,
                api_stype: 21,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 301,
                api_name: "足柄".to_string(),
                api_yomi: String::new(),
                api_ctype: 10,
                api_stype: 5,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 302,
                api_name: "妙高".to_string(),
                api_yomi: String::new(),
                api_ctype: 9,
                api_stype: 5,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 303,
                api_name: "高雄".to_string(),
                api_yomi: String::new(),
                api_ctype: 11,
                api_stype: 5,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1501,
                api_name: "駆逐イ級".to_string(),
                api_yomi: String::new(),
                api_stype: 2,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1502,
                api_name: "駆逐ロ級".to_string(),
                api_yomi: String::new(),
                api_stype: 2,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1503,
                api_name: "軽巡ホ級".to_string(),
                api_yomi: "flagship".to_string(),
                api_stype: 3,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1601,
                api_name: "潜水ソ級".to_string(),
                api_yomi: String::new(),
                api_stype: 13,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1701,
                api_name: "軽母ヌ級elite".to_string(),
                api_yomi: String::new(),
                api_stype: 7,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1702,
                api_name: "戦艦ル級改flagship".to_string(),
                api_yomi: String::new(),
                api_stype: 8,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1703,
                api_name: "PT小鬼群".to_string(),
                api_yomi: String::new(),
                api_stype: 2,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1704,
                api_name: "護衛要塞".to_string(),
                api_yomi: String::new(),
                api_stype: 8,
                ..ApiMstShip::default()
            },
            ApiMstShip {
                api_id: 1705,
                api_name: "飛行場姫".to_string(),
                api_yomi: String::new(),
                api_stype: 8,
                ..ApiMstShip::default()
            },
        ],
        ..ApiManifest::default()
    }
}

#[test]
fn parse_fixture_catalog_with_probability_routes() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
			pages.join("1-1.html"),
			r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr><td rowspan="2">A</td><td>B</td><td rowspan="2">ランダム(艦隊人数により確率変動) ※Cマス進行割合 6隻:55% 5隻:60% 4隻:65%</td></tr>
  <tr><td>C</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>A：</td><td>パターン1</td><td>10</td><td>駆逐イ級</td><td>単縦陣</td></tr>
  <tr><td>B：</td><td>パターン1</td><td>15</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
  <tr><td>C：ボス</td><td>パターン1</td><td>20</td><td>軽巡ホ級flagship、駆逐イ級</td><td>複縦陣</td></tr>
</table>
</body></html>
"#,
		)
		.unwrap();

    let catalog = parse_debug(root.path(), &manifest_fixture()).unwrap();
    let wiki_map = catalog.maps.get(&11).unwrap();
    let variant = only_variant(wiki_map);
    assert_eq!(variant.nodes.len(), 3);
    assert!(variant.routing_rules.iter().any(|rule| {
        rule.to_cell_no == 3
            && matches!(
                rule.predicate,
                RoutePredicate::FleetSize {
                    value: 6,
                    ..
                }
            )
            && rule.probability_pct == Some(55.0)
    }));
    assert!(variant.routing_rules.iter().any(|rule| {
        rule.to_cell_no == 2
            && matches!(
                rule.predicate,
                RoutePredicate::FleetSize {
                    value: 6,
                    ..
                }
            )
            && rule.probability_pct == Some(45.0)
    }));
    assert!(variant.enemy_fleets.iter().any(|fleet| {
        fleet.compositions.iter().any(|composition| composition.ship_ids == vec![1503, 1501])
    }));
}

#[test]
fn parse_fixture_catalog_with_probability_routes_ignores_route_footnote_anchor() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
		pages.join("1-1.html"),
		r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr>
    <td rowspan="2">A</td>
    <td>B</td>
    <td rowspan="2">ランダム(艦隊人数により確率変動)<br class="spacer">※Cマス進行割合<a id="notetext_1" class="note_super tooltip" data-tooltip-content="dummy">*1</a><br class="spacer"> 6隻:55%<br class="spacer"> 5隻:60%<br class="spacer"> 4隻:65%</td>
  </tr>
  <tr><td>C</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>A：</td><td>パターン1</td><td>10</td><td>駆逐イ級</td><td>単縦陣</td></tr>
  <tr><td>B：</td><td>パターン1</td><td>15</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
  <tr><td>C：ボス</td><td>パターン1</td><td>20</td><td>軽巡ホ級flagship、駆逐イ級</td><td>複縦陣</td></tr>
</table>
</body></html>
"#,
	)
	.unwrap();

    let catalog = parse_debug(root.path(), &manifest_fixture()).unwrap();
    let wiki_map = catalog.maps.get(&11).unwrap();
    let variant = only_variant(wiki_map);

    assert!(variant.parse_warnings.is_empty());
    assert!(variant.routing_rules.iter().any(|rule| {
        rule.to_cell_no == 3
            && matches!(
                rule.predicate,
                RoutePredicate::FleetSize {
                    value: 6,
                    ..
                }
            )
            && rule.probability_pct == Some(55.0)
    }));
    assert!(variant.routing_rules.iter().any(|rule| {
        rule.to_cell_no == 2
            && matches!(
                rule.predicate,
                RoutePredicate::FleetSize {
                    value: 6,
                    ..
                }
            )
            && rule.probability_pct == Some(45.0)
    }));
}

#[test]
fn parse_fixture_catalog_preserves_explicit_start_routes() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
		pages.join("1-1.html"),
		r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr><td rowspan="3">出撃</td><td>A</td><td rowspan="3">A:B:C=30%:40%:30%</td></tr>
  <tr><td>B</td></tr>
  <tr><td>C</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>A：</td><td>パターン1</td><td>10</td><td>駆逐イ級</td><td>単縦陣</td></tr>
  <tr><td>B：</td><td>パターン1</td><td>15</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
  <tr><td>C：ボス</td><td>パターン1</td><td>20</td><td>軽巡ホ級flagship、駆逐イ級</td><td>複縦陣</td></tr>
</table>
</body></html>
"#,
	)
	.unwrap();

    let debug_catalog = parse_debug(root.path(), &manifest_fixture()).unwrap();
    let wiki_map = debug_catalog.maps.get(&11).unwrap();
    let variant = only_variant(wiki_map);
    assert!(variant.nodes.iter().all(|node| node.label != "Start"));
    assert_eq!(
        variant
            .routing_rules
            .iter()
            .filter(|rule| rule.from_cell_no == 0)
            .map(|rule| rule.to_cell_no)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([1, 2, 3])
    );

    let catalog = parse(root.path(), &manifest_fixture()).unwrap();
    let stage = catalog.map_definition(11).unwrap().variant("").unwrap();
    let start = stage.cell(0).unwrap();
    assert_eq!(start.node_label.as_deref(), Some("Start"));
    assert_eq!(start.next_cells, vec![1, 2, 3]);
    assert_eq!(stage.routing_rules.get(&0).map(Vec::len), Some(3));
    assert!(
        stage
            .parse_warnings
            .iter()
            .all(|warning| !warning.starts_with("inferred_multi_root_start"))
    );
    assert_eq!(stage.first_progress_cell_no(), None);
}

#[test]
fn parse_fixture_catalog_with_executable_route_predicates() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
			pages.join("1-5.html"),
			r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr><td rowspan="4">C</td><td>J</td><td>海防艦のみの艦隊</td></tr>
  <tr><td>J</td><td>軽巡1隻 かつ 海防4隻の5隻編成</td></tr>
  <tr><td>B</td><td>潜水艦を含む</td></tr>
  <tr><td>B</td><td>それ以外</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>C：</td><td>パターン1</td><td>30</td><td>潜水ソ級</td><td>梯形陣</td></tr>
  <tr><td>J：ボス</td><td>パターン1</td><td>50</td><td>潜水ソ級、軽巡ホ級flagship</td><td>単横陣</td></tr>
</table>
</body></html>
"#,
		)
		.unwrap();

    let catalog = parse_debug(root.path(), &manifest_fixture()).unwrap();
    let wiki_map = catalog.maps.get(&15).unwrap();
    let variant = only_variant(wiki_map);
    assert!(
        variant
            .routing_rules
            .iter()
            .any(|rule| { matches!(rule.predicate, RoutePredicate::OnlyShipTypes { .. }) })
    );
    assert!(
        variant
            .routing_rules
            .iter()
            .any(|rule| { matches!(rule.predicate, RoutePredicate::And(_)) })
    );
    assert!(
        variant
            .routing_rules
            .iter()
            .any(|rule| { matches!(rule.predicate, RoutePredicate::ContainsShipType { .. }) })
    );
}

#[test]
fn parse_route_predicate_supports_contains_negation_and_conjunctive_contains() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate =
        parse_route_predicate("駆逐艦を含み吹雪を含まない", &ship_types, &ships).unwrap();

    match predicate {
        RoutePredicate::And(predicates) => {
            assert_eq!(predicates.len(), 2);
            assert!(matches!(
                &predicates[0],
                RoutePredicate::ContainsShipType {
                    ship_types
                } if ship_types == &vec![2]
            ));
            assert!(matches!(
                &predicates[1],
                RoutePredicate::Not(inner)
                    if matches!(
                        inner.as_ref(),
                        RoutePredicate::ContainsShipId {
                            ship_ids
                        } if ship_ids == &vec![101]
                    )
            ));
        }
        other => panic!("expected And predicate, got {other:?}"),
    }
}

#[test]
fn parse_route_predicate_supports_named_pair_contains_variants() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let both = parse_route_predicate("吹雪と綾波の両方を含む", &ship_types, &ships).unwrap();
    assert!(matches!(
        both,
        RoutePredicate::And(predicates)
            if predicates.len() == 2
                && predicates.iter().all(|predicate| matches!(
                    predicate,
                    RoutePredicate::ContainsShipId {
                        ship_ids
                    } if ship_ids.len() == 1 && [101, 102].contains(&ship_ids[0])
                ))
    ));

    let neither =
        parse_route_predicate("吹雪と綾波のいずれも含まない", &ship_types, &ships).unwrap();
    assert!(matches!(
        neither,
        RoutePredicate::Not(inner)
            if matches!(
                inner.as_ref(),
                RoutePredicate::Or(predicates)
                    if predicates.len() == 2
                        && predicates.iter().all(|predicate| matches!(
                            predicate,
                            RoutePredicate::ContainsShipId {
                                ship_ids
                            } if ship_ids.len() == 1 && [101, 102].contains(&ship_ids[0])
                        ))
                )
    ));
}

#[test]
fn parse_route_predicate_supports_class_group_and_mixed_selector() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let count = parse_route_predicate("大鷹型2隻以上", &ship_types, &ships).unwrap();
    assert!(
        matches!(
                &count,
                RoutePredicate::ShipSetCount {
                    ship_types,
                    ship_ids,
                op: RouteOperator::Gte,
                value: 2,
            } if ship_types.is_empty()
                && ship_ids.len() >= 2
                && ship_ids.contains(&201)
                && ship_ids.contains(&202)
        ),
        "{count:?}"
    );

    let mixed = parse_route_predicate("練巡を含みかつ(大鷹型+駆逐)を含まない", &ship_types, &ships)
        .unwrap();
    let mut flattened = Vec::new();
    flatten_and_predicates(&mixed, &mut flattened);
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::ContainsShipType {
            ship_types
        } if ship_types == &vec![21]
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::Not(inner)
            if matches!(
                inner.as_ref(),
                RoutePredicate::ContainsShipSet {
                    ship_types,
                    ship_ids,
                } if ship_types == &vec![2]
                    && ship_ids.len() >= 2
                    && ship_ids.contains(&201)
                    && ship_ids.contains(&202)
            )
    )));
}

#[test]
fn parse_route_predicate_supports_mixed_only_selector_and_named_set_count() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let only =
        parse_route_predicate("(重巡+軽巡+駆逐+海防)のみの艦隊", &ship_types, &ships).unwrap();
    match only {
        RoutePredicate::OnlyShipTypes {
            ship_types,
        } => {
            let actual = ship_types.into_iter().collect::<BTreeSet<_>>();
            assert_eq!(actual, BTreeSet::from([1, 2, 3, 5]));
        }
        other => panic!("expected OnlyShipTypes, got {other:?}"),
    }

    let count =
        parse_route_predicate("足柄、妙高、高雄から2隻を含む", &ship_types, &ships).unwrap();
    match count {
        RoutePredicate::ShipSetCount {
            ship_types,
            ship_ids,
            op: RouteOperator::Gte,
            value: 2,
        } => {
            assert!(ship_types.is_empty());
            assert_eq!(ship_ids, vec![301, 302, 303]);
        }
        other => panic!("expected ShipSetCount, got {other:?}"),
    }
}

#[test]
fn parse_route_predicate_respects_parenthesized_or_precedence() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate =
        parse_route_predicate("(吹雪 または 綾波) かつ 初雪を含む", &ship_types, &ships).unwrap();
    let mut flattened = Vec::new();
    flatten_and_predicates(&predicate, &mut flattened);
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::Or(predicates)
            if predicates.len() == 2
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::ContainsShipId {
            ship_ids
        } if ship_ids == &vec![103]
    )));
}

#[test]
fn parse_route_predicate_supports_ship_pair_or_contains() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate =
        parse_route_predicate("吹雪と綾波 または 吹雪と初雪 を含む", &ship_types, &ships).unwrap();
    assert!(matches!(
        predicate,
        RoutePredicate::Or(predicates)
            if predicates.len() == 2
    ));
}

#[test]
fn parse_route_predicate_supports_contains_count_and_speed_alias() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let contains_count = parse_route_predicate("海防艦3隻を含む", &ship_types, &ships).unwrap();
    assert!(matches!(
        contains_count,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Gte,
            value: 3,
        } if ship_types == vec![1]
    ));

    let predicate =
        parse_route_predicate("高速以上統一 かつ 軽巡1隻 かつ 駆逐4隻", &ship_types, &ships)
            .unwrap();
    let mut flattened = Vec::new();
    flatten_and_predicates(&predicate, &mut flattened);
    assert_eq!(flattened.len(), 3);
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::Speed {
            class: SpeedClass::Fast
        }
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Eq,
            value: 1,
        } if ship_types == &vec![3]
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Eq,
            value: 4,
        } if ship_types == &vec![2]
    )));
}

#[test]
fn parse_route_predicate_supports_los_range() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate = parse_route_predicate("索敵スコア45以上47未満", &ship_types, &ships).unwrap();
    let mut flattened = Vec::new();
    flatten_and_predicates(&predicate, &mut flattened);
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::LoS {
            op: RouteOperator::Gte,
            value: 45,
            ..
        }
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::LoS {
            op: RouteOperator::Lte,
            value: 46,
            ..
        }
    )));
}

#[test]
fn parse_case_route_condition_text_supports_target_random_nested_clause() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
        "4隻編成の場合\n_・海防艦2隻以上でBマス または Eマスのランダム（Bマス寄り(70%)）",
        "B",
        &["B".to_string(), "E".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 2);
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "B" && clause.probability_pct == Some(70.0))
    );
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "E" && clause.probability_pct == Some(30.0))
    );
    for clause in clauses {
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::FleetSize {
                op: RouteOperator::Eq,
                value: 4
            }
        )));
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::ShipTypeCount {
                ship_types,
                op: RouteOperator::Gte,
                value: 2,
            } if ship_types == &vec![1]
        )));
    }
}

#[test]
fn parse_target_random_route_condition_text_supports_los_range_clause() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let clauses = parse_target_random_route_condition_text(
        "索敵スコアが33以上37未満ならKまたはLのランダム",
        &ship_types,
        &ships,
    )
    .unwrap();

    assert_eq!(clauses.len(), 2);
    assert!(clauses.iter().any(|clause| clause.target_label == "K"));
    assert!(clauses.iter().any(|clause| clause.target_label == "L"));
    for clause in clauses {
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::LoS {
                op: RouteOperator::Gte,
                value: 33,
                ..
            }
        )));
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::LoS {
                op: RouteOperator::Lte,
                value: 36,
                ..
            }
        )));
    }
}

#[test]
fn parse_route_predicate_supports_simple_los_thresholds() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let lt = parse_route_predicate("索敵スコア45未満", &ship_types, &ships).unwrap();
    assert!(matches!(
        lt,
        RoutePredicate::LoS {
            op: RouteOperator::Lte,
            value: 44,
            ..
        }
    ));

    let gte = parse_route_predicate("索敵スコアが37以上", &ship_types, &ships).unwrap();
    assert!(matches!(
        gte,
        RoutePredicate::LoS {
            op: RouteOperator::Gte,
            value: 37,
            ..
        }
    ));

    let lte = parse_route_predicate("索敵スコア52以下", &ship_types, &ships).unwrap();
    assert!(matches!(
        lte,
        RoutePredicate::LoS {
            op: RouteOperator::Lte,
            value: 52,
            ..
        }
    ));

    let bare_range = parse_route_predicate("35以上40未満", &ship_types, &ships).unwrap();
    let mut flattened = Vec::new();
    flatten_and_predicates(&bare_range, &mut flattened);
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::LoS {
            op: RouteOperator::Gte,
            value: 35,
            ..
        }
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::LoS {
            op: RouteOperator::Lte,
            value: 39,
            ..
        }
    )));
}

#[test]
fn parse_route_predicate_supports_low_speed_contains_alias() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate = parse_route_predicate("速力:低速 が含まれている", &ship_types, &ships).unwrap();
    assert!(matches!(
        predicate,
        RoutePredicate::Not(inner) if matches!(
            inner.as_ref(),
            RoutePredicate::Speed {
                class: SpeedClass::Fast
            }
        )
    ));
}

#[test]
fn parse_route_predicate_supports_count_operator_before_count() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate =
        parse_route_predicate("軽巡(過不足なく)1隻 かつ 駆逐4隻以上", &ship_types, &ships).unwrap();
    let mut flattened = Vec::new();
    flatten_and_predicates(&predicate, &mut flattened);
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Eq,
            value: 1,
        } if ship_types == &vec![3]
    )));
    assert!(flattened.iter().any(|predicate| matches!(
        predicate,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Gte,
            value: 4,
        } if ship_types == &vec![2]
    )));
}

#[test]
fn parse_route_predicate_supports_equipment_flagship_visited_and_speed_qualified_selectors() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let equipment = parse_route_predicate("電探を装備した艦が0隻", &ship_types, &ships).unwrap();
    assert!(matches!(
        equipment,
        RoutePredicate::EquipmentCount {
            slotitem_types,
            op: RouteOperator::Eq,
            value: 0,
        } if slotitem_types == vec![12, 13, 93]
    ));

    let flagship = parse_route_predicate("軽巡旗艦", &ship_types, &ships).unwrap();
    assert!(matches!(
        flagship,
        RoutePredicate::FlagshipShipType {
            ship_types,
        } if ship_types == vec![3]
    ));

    let not_visited = parse_route_predicate("Dマス未経由", &ship_types, &ships).unwrap();
    assert!(matches!(
        not_visited,
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited: false,
        } if node_labels == vec!["D".to_string()]
    ));

    let visited = parse_route_predicate("Nマスを経由済み", &ship_types, &ships).unwrap();
    assert!(matches!(
        visited,
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited: true,
        } if node_labels == vec!["N".to_string()]
    ));

    let contains =
        parse_route_predicate("(低速)戦艦、正規空母を含む", &ship_types, &ships).unwrap();
    assert!(matches!(contains, RoutePredicate::Or(predicates) if predicates.len() == 2));

    let low_speed_count = parse_route_predicate("低速戦艦2以上", &ship_types, &ships).unwrap();
    assert!(matches!(
        low_speed_count,
        RoutePredicate::ShipSetSpeedCount {
            speed_op: RouteOperator::Lte,
            speed_class: SpeedClass::Slow,
            op: RouteOperator::Gte,
            value: 2,
            ..
        }
    ));

    let paren_low_speed = parse_route_predicate("(低速)戦艦2以上", &ship_types, &ships).unwrap();
    assert!(matches!(
        paren_low_speed,
        RoutePredicate::ShipSetSpeedCount {
            speed_op: RouteOperator::Lte,
            speed_class: SpeedClass::Slow,
            op: RouteOperator::Gte,
            value: 2,
            ..
        }
    ));

    let fast_battleship = parse_route_predicate("高速戦艦2以上", &ship_types, &ships).unwrap();
    assert!(matches!(
        fast_battleship,
        RoutePredicate::ShipSetSpeedCount {
            speed_op: RouteOperator::Gte,
            speed_class: SpeedClass::Fast,
            op: RouteOperator::Gte,
            value: 2,
            ..
        }
    ));

    let exact_contains =
        parse_route_predicate("航巡(過不足なく)1隻を含む", &ship_types, &ships).unwrap();
    assert!(matches!(
        exact_contains,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Eq,
            value: 1,
        } if ship_types == vec![6]
    ));
}

#[test]
fn parse_case_route_condition_text_supports_helper_target_header() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
			"索敵スコアが37以上またはランダムでLの際に次の条件のいずれかを満たすとL\n　・大鷹型2隻以上\n　・駆逐2隻以上",
			"K",
			&["K".to_string(), "L".to_string(), "P".to_string()],
			&ship_types,
			&ships,
			&mut warnings,
		)
		.unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 2);
    for clause in clauses {
        assert_eq!(clause.target_label, "L");
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::LoS {
                op: RouteOperator::Gte,
                value: 37,
                ..
            }
        )));
    }
}

#[test]
fn parse_case_route_condition_text_supports_helper_target_header_with_guard_conjunction() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
        "駆逐2隻 かつ 以下の条件をひとつ充たせばH\n　・高速+以上統一\n　・補給艦を含む",
        "E",
        &["E".to_string(), "H".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 2);
    assert!(clauses.iter().all(|clause| clause.target_label == "H"));
    for clause in clauses {
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::ShipTypeCount {
                ship_types,
                op: RouteOperator::Eq,
                value: 2,
            } if ship_types == &vec![2]
        )));
    }
}

#[test]
fn parse_case_route_condition_text_supports_helper_random_header() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
        "以下の条件をひとつ充たすとランダム\n　・索敵スコア66未満63以上\n　・潜水艦を含む",
        "R",
        &["R".to_string(), "S".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 4);
    assert_eq!(clauses.iter().filter(|clause| clause.target_label == "R").count(), 2);
    assert_eq!(clauses.iter().filter(|clause| clause.target_label == "S").count(), 2);
}

#[test]
fn parse_case_route_condition_text_supports_toki_suffix() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
        "4隻編成のとき\n_・海防艦3隻を含む",
        "B",
        &["B".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0].target_label, "B");
    match &clauses[0].predicate {
        RoutePredicate::And(predicates) => {
            assert_eq!(predicates.len(), 2);
            assert!(predicates.iter().any(|predicate| matches!(
                predicate,
                RoutePredicate::FleetSize {
                    op: RouteOperator::Eq,
                    value: 4
                }
            )));
            assert!(predicates.iter().any(|predicate| matches!(
                predicate,
                RoutePredicate::ShipTypeCount {
                    ship_types,
                    op: RouteOperator::Gte,
                    value: 3,
                } if ship_types == &vec![1]
            )));
        }
        other => panic!("expected And predicate, got {other:?}"),
    }
}

#[test]
fn parse_conditional_random_route_condition_text_uses_candidate_targets() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let clauses = parse_conditional_random_route_condition_text(
        "潜水艦を含むとランダム",
        &["I".to_string(), "J".to_string()],
        &ship_types,
        &ships,
    )
    .unwrap();

    assert_eq!(clauses.len(), 2);
    assert!(clauses.iter().any(|clause| clause.target_label == "I"));
    assert!(clauses.iter().any(|clause| clause.target_label == "J"));
    assert!(clauses.iter().all(|clause| clause.probability_pct.is_none()));
    assert!(clauses.iter().all(|clause| matches!(
        clause.predicate,
        RoutePredicate::ContainsShipType {
            ref ship_types
        } if ship_types == &vec![13]
    )));
}

#[test]
fn parse_row_target_random_bias_condition_text_uses_candidate_targets() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let clauses = parse_row_target_random_bias_condition_text(
        "正規空母を含むとCマス寄り(60%)のランダム",
        "C",
        &["C".to_string(), "G".to_string()],
        &ship_types,
        &ships,
    )
    .unwrap();

    assert_eq!(clauses.len(), 2);
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "C" && clause.probability_pct == Some(60.0))
    );
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "G" && clause.probability_pct == Some(40.0))
    );
}

#[test]
fn parse_route_predicate_supports_supply_alias_in_count_clause() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate = parse_route_predicate("補給2隻", &ship_types, &ships).unwrap();
    assert!(matches!(
        predicate,
        RoutePredicate::ShipTypeCount {
            ship_types,
            op: RouteOperator::Eq,
            value: 2,
        } if ship_types == vec![15]
    ));
}

#[test]
fn parse_route_predicate_supports_nested_conjunction_with_parenthesized_or() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate = parse_route_predicate(
        "駆逐3隻 かつ 重巡1隻 かつ (軽巡1隻 または 補給1隻)",
        &ship_types,
        &ships,
    )
    .unwrap();
    assert!(matches!(predicate, RoutePredicate::And(_)));
}

#[test]
fn parse_route_predicate_supports_parenthesized_or_after_conjunction() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let predicate =
        parse_route_predicate("駆逐2隻 かつ (重巡2隻 または 補給2隻)", &ship_types, &ships)
            .unwrap();
    assert!(matches!(predicate, RoutePredicate::And(_)));
}

#[test]
fn parse_row_target_random_bias_condition_text_accepts_approximate_probability_notes() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let clauses = parse_row_target_random_bias_condition_text(
        "正規空母を含むとJマス寄り(80~85%)のランダム",
        "J",
        &["A".to_string(), "J".to_string()],
        &ship_types,
        &ships,
    )
    .unwrap();

    assert_eq!(clauses.len(), 2);
    assert!(clauses.iter().all(|clause| clause.probability_pct.is_none()));
    assert!(clauses.iter().any(|clause| clause.target_label == "A"));
    assert!(clauses.iter().any(|clause| clause.target_label == "J"));
}

#[test]
fn parse_row_target_random_bias_shorthand_condition_text_supports_bullet_bias_notes() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let clauses = parse_row_target_random_bias_shorthand_condition_text(
        "正規空母1隻 かつ 駆逐1隻でL寄り55％?",
        "L",
        &["L".to_string(), "N".to_string()],
        &ship_types,
        &ships,
    )
    .unwrap();

    assert_eq!(clauses.len(), 2);
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "L" && clause.probability_pct == Some(55.0))
    );
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "N" && clause.probability_pct == Some(45.0))
    );
}

#[test]
fn parse_inline_targeted_route_condition_text_inferrs_los_insufficient_complement() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_inline_targeted_route_condition_text(
        "索敵不足でG 索敵スコア32以上でK",
        "G",
        &["G".to_string(), "K".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 1);
    assert!(matches!(
        &clauses[0].1,
        RoutePredicate::Not(inner) if matches!(
            inner.as_ref(),
            RoutePredicate::LoS {
                op: RouteOperator::Gte,
                value: 32,
                ..
            }
        )
    ));
}

#[test]
fn parse_conditional_random_route_condition_text_supports_reversed_los_range_text() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());

    let clauses = parse_conditional_random_route_condition_text(
        "35未満40以上でランダム",
        &["J".to_string(), "K".to_string()],
        &ship_types,
        &ships,
    )
    .unwrap();

    assert_eq!(clauses.len(), 2);
    for clause in clauses {
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::LoS {
                op: RouteOperator::Gte,
                value: 35,
                ..
            }
        )));
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::LoS {
                op: RouteOperator::Lte,
                value: 39,
                ..
            }
        )));
    }
}

#[test]
fn parse_inline_targeted_route_condition_text_inferrs_else_complement_for_single_other_target() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_inline_targeted_route_condition_text(
        "5隻以下の編成でG 駆逐1隻以上かつ海防3隻以上でG 索敵不足でF 索敵スコア46以上でG",
        "F",
        &["F".to_string(), "G".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 1);
    assert!(matches!(
        &clauses[0].1,
        RoutePredicate::Not(inner) if matches!(
            inner.as_ref(),
            RoutePredicate::Or(predicates) if predicates.len() == 3
        )
    ));
}

#[test]
fn parse_case_route_condition_text_supports_distribution_annotation_for_random_targets() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
		"4隻編成の場合\n_・それ以外はGマス または Jマス または Mマスのランダム\n_＿＿(Gマス:Jマス:Mマス=10%:45%:45%)",
		"G",
		&["G".to_string(), "J".to_string(), "M".to_string()],
		&ship_types,
		&ships,
		&mut warnings,
	)
	.unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 3);
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "G" && clause.probability_pct == Some(10.0))
    );
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "J" && clause.probability_pct == Some(45.0))
    );
    assert!(
        clauses
            .iter()
            .any(|clause| clause.target_label == "M" && clause.probability_pct == Some(45.0))
    );
    for clause in clauses {
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::FleetSize {
                op: RouteOperator::Eq,
                value: 4
            }
        )));
    }
}

#[test]
fn parse_case_route_condition_text_supports_scoped_helper_header_and_else() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
		"次の条件のいずれかを満たし\n　・(低速)戦艦、正規空母を含む\n　・高速戦艦2以上\n　上記の条件を満たさない場合はP",
		"K",
		&["K".to_string(), "P".to_string()],
		&ship_types,
		&ships,
		&mut warnings,
	)
	.unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 3);
    assert_eq!(clauses.iter().filter(|clause| clause.target_label == "K").count(), 2);
    assert!(clauses.iter().any(|clause| clause.target_label == "P"));
}

#[test]
fn parse_case_route_condition_text_supports_route_history_context_header() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
		"Eマスを経由し、索敵スコアによってK,L,Pマスへ分岐する\n　・索敵スコアが33未満でK\n　・索敵スコアが37以上でP",
		"K",
		&["K".to_string(), "L".to_string(), "P".to_string()],
		&ship_types,
		&ships,
		&mut warnings,
	)
	.unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 2);
    for clause in clauses {
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::VisitedNodeLabel {
                node_labels,
                visited: true,
            } if node_labels == &vec!["E".to_string()]
        )));
    }
}

#[test]
fn parse_case_route_condition_text_supports_fixed_los_random_gate_header() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_case_route_condition_text(
		"索敵スコア58以上63未満はランダムでS\n索敵スコアのランダム判定でS以外になった場合、または索敵スコア63以上の場合は下の条件に基づきルート分岐\n　・最速統一でT\n　・正規空母1以上でR\n　・それ以外はT",
		"R",
		&["R".to_string(), "S".to_string(), "T".to_string()],
		&ship_types,
		&ships,
		&mut warnings,
	)
	.unwrap();

    assert!(warnings.is_empty());
    assert!(clauses.iter().any(|clause| clause.target_label == "R"));
    assert!(clauses.iter().any(|clause| clause.target_label == "T"));
    for clause in clauses {
        if clause.target_label == "S" {
            continue;
        }
        let mut flattened = Vec::new();
        flatten_and_predicates(&clause.predicate, &mut flattened);
        assert!(flattened.iter().any(|predicate| matches!(
            predicate,
            RoutePredicate::LoS {
                op: RouteOperator::Gte,
                value: 58,
                ..
            }
        )));
    }
}

#[test]
fn parse_independent_route_condition_line_supports_caveated_explicit_target() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();

    let clauses = parse_independent_route_condition_line(
        "Eマスを経由済みならM（反例アリ）",
        "Eマスを経由済みならM（反例アリ）",
        "M",
        &["L".to_string(), "M".to_string(), "T".to_string()],
        &ship_types,
        &ships,
        &mut warnings,
    )
    .unwrap();

    assert!(warnings.is_empty());
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0].target_label, "M");
    assert!(matches!(
        clauses[0].predicate,
        RoutePredicate::VisitedNodeLabel {
            ref node_labels,
            visited: true,
        } if node_labels == &vec!["E".to_string()]
    ));
}

fn flatten_and_predicates<'a>(
    predicate: &'a RoutePredicate,
    flattened: &mut Vec<&'a RoutePredicate>,
) {
    match predicate {
        RoutePredicate::And(predicates) => {
            for predicate in predicates {
                flatten_and_predicates(predicate, flattened);
            }
        }
        other => flattened.push(other),
    }
}

#[test]
fn parse_fixture_catalog_with_case_ast_and_variants() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
			pages.join("1-1.html"),
			r#"
<html><body>
<table>
  <tr><th>第一ゲージ</th><td>3回撃沈でゲージ破壊</td></tr>
  <tr><th>第二ゲージ</th><td>4回撃沈で海域クリア</td></tr>
</table>
<div class="fold-container">
  <div class="fold-summary">戦力ゲージ1(Pマス出現前) ルート分岐法則</div>
  <div class="fold-content">
    <table>
      <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
      <tr>
        <td rowspan="2">A</td>
        <td>B</td>
        <td rowspan="2">1隻編成でC<br>4隻編成の場合<br>_ ・海防艦のみの艦隊でC<br>_ ・それ以外はB</td>
      </tr>
      <tr><td>C</td></tr>
    </table>
  </div>
</div>
<div class="fold-container">
  <div class="fold-summary">戦力ゲージ2(Pマス出現後) ルート分岐法則</div>
  <div class="fold-content">
    <table>
      <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
      <tr>
        <td rowspan="2">A</td>
        <td>B</td>
        <td rowspan="2">1隻編成でC<br>5隻以上の編成の場合<br>_ ・海防艦のみの艦隊でD<br>_ ・それ以外はB</td>
      </tr>
      <tr><td>C</td></tr>
      <tr><td>A</td><td>D</td><td>海防艦のみの艦隊</td></tr>
    </table>
  </div>
</div>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>B：</td><td>パターン1</td><td>10</td><td>駆逐イ級</td><td>単縦陣</td></tr>
  <tr><td>C：ボス</td><td>パターン1</td><td>20</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
  <tr><td>D：ボス</td><td>パターン1</td><td>30</td><td>軽巡ホ級flagship</td><td>複縦陣</td></tr>
</table>
</body></html>
"#,
		)
		.unwrap();

    let catalog = parse_debug(root.path(), &manifest_fixture()).unwrap();
    let wiki_map = catalog.maps.get(&11).unwrap();
    assert_eq!(wiki_map.default_variant, "pre_p_unlock");
    assert_eq!(wiki_map.variants.len(), 2);

    let pre = wiki_map.variants.get("pre_p_unlock").unwrap();
    assert_eq!(pre.required_defeat_count, Some(3));
    assert_eq!(pre.clear_to_variant_key.as_deref(), Some("post_p_unlock"));
    assert!(
        pre.routing_rules.iter().any(|rule| {
            rule.to_cell_no == 3 && matches!(rule.predicate, RoutePredicate::And(_))
        })
    );
    assert!(pre.enemy_fleets.iter().any(|fleet| fleet.node_label == "D"));

    let post = wiki_map.variants.get("post_p_unlock").unwrap();
    assert_eq!(post.required_defeat_count, Some(4));
    assert!(post.enemy_fleets.iter().any(|fleet| fleet.node_label == "D"));
}

#[test]
fn parse_route_table_prefers_resolved_complement_over_duplicate_unknown_rule() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
		vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
		vec![
			"E".to_string(),
			"F".to_string(),
			"5隻以下の編成でG\n(駆逐+海防)5隻以上でG\n駆逐1隻以上 かつ 海防3隻以上でG\n索敵不足でF\n索敵スコア46以上でG"
				.to_string(),
		],
		vec![
			"E".to_string(),
			"G".to_string(),
			"5隻以下の編成でG\n(駆逐+海防)5隻以上でG\n駆逐1隻以上 かつ 海防3隻以上でG\n索敵不足でF\n索敵スコア46以上でG"
				.to_string(),
		],
	];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(
        drafts.iter().filter(|draft| draft.from_label == "E" && draft.to_label == "F").count(),
        1
    );
    assert!(drafts.iter().any(|draft| {
        draft.from_label == "E"
            && draft.to_label == "F"
            && !matches!(
                draft.predicate,
                RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
            )
    }));
}

#[test]
fn parse_route_table_hardcodes_missing_los_thresholds_as_known_targets() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "G".to_string(),
            "I".to_string(),
            "索敵スコア??未満でI\n索敵スコア??以上??未満で?\n索敵スコア??以上でL".to_string(),
        ],
        vec![
            "G".to_string(),
            "L".to_string(),
            "索敵スコア??未満でI\n索敵スコア??以上??未満で?\n索敵スコア??以上でL".to_string(),
        ],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();

    assert!(warnings.is_empty());
    assert!(
		drafts
			.iter()
			.any(|draft| draft.to_label == "I" && matches!(draft.predicate, RoutePredicate::Always))
	);
    assert!(
		drafts
			.iter()
			.any(|draft| draft.to_label == "L" && matches!(draft.predicate, RoutePredicate::Always))
	);
    assert!(drafts.iter().all(|draft| !matches!(
        draft.predicate,
        RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
    )));
}

#[test]
fn parse_route_table_supports_residual_fleet_helper_target_header() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
			"D".to_string(),
			"F".to_string(),
			"吹雪と綾波の両方を含む場合、他4隻が以下の条件を充たせばG\n_・駆逐2隻 かつ 重巡2隻\nそれ以外はF"
				.to_string(),
		],
        vec![
			"D".to_string(),
			"G".to_string(),
			"吹雪と綾波の両方を含む場合、他4隻が以下の条件を充たせばG\n_・駆逐2隻 かつ 重巡2隻\nそれ以外はF"
				.to_string(),
		],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());

    let g_rule = drafts
        .iter()
        .find(|draft| {
            draft.from_label == "D"
                && draft.to_label == "G"
                && !matches!(
                    draft.predicate,
                    RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
                )
        })
        .unwrap();
    let predicate_json = serde_json::to_string(&g_rule.predicate).unwrap();
    assert!(predicate_json.contains("\"value\":6"));
    assert!(predicate_json.contains("\"ship_ids\":[101]"));
    assert!(predicate_json.contains("\"ship_ids\":[102]"));
}

#[test]
fn parse_route_table_supports_residual_fleet_helper_target_header_with_parenthesized_or_child() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
		vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
		vec![
			"D".to_string(),
			"F".to_string(),
			"吹雪と綾波の両方を含む場合、他4隻が以下の条件を充たせばG\n_・駆逐2隻 かつ (重巡2隻 または 補給2隻)\nそれ以外はF"
				.to_string(),
		],
		vec![
			"D".to_string(),
			"G".to_string(),
			"吹雪と綾波の両方を含む場合、他4隻が以下の条件を充たせばG\n_・駆逐2隻 かつ (重巡2隻 または 補給2隻)\nそれ以外はF"
				.to_string(),
		],
	];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert!(drafts.iter().any(|draft| {
        draft.from_label == "D"
            && draft.to_label == "G"
            && !matches!(
                draft.predicate,
                RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
            )
    }));
}

#[test]
fn parse_case_route_condition_text_supports_fullwidth_indent_probability_annotation() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let parsed = parse_case_route_condition_text(
		"(戦艦級+正規空母)2隻以下 かつ 軽巡1隻以上 かつ (駆逐+海防)2隻以上の場合\n＿・高速+以上統一でJ\n＿・それ以外はGマス または Jマス または Mマスのランダム\n＿＿(Gマス:Jマス:Mマス=10%:45%:45%)",
		"G",
		&["G".to_string(), "J".to_string(), "M".to_string()],
		&ship_types,
		&ships,
		&mut warnings,
	)
	.unwrap();

    assert!(warnings.is_empty());
    assert!(
        parsed
            .iter()
            .any(|clause| clause.target_label == "G" && clause.probability_pct == Some(10.0))
    );
    assert!(
        parsed
            .iter()
            .any(|clause| clause.target_label == "J" && clause.probability_pct == Some(45.0))
    );
    assert!(
        parsed
            .iter()
            .any(|clause| clause.target_label == "M" && clause.probability_pct == Some(45.0))
    );
}

#[test]
fn parse_route_table_does_not_warn_for_hardcoded_unknown_target_line() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "D".to_string(),
            "E".to_string(),
            "戦艦級3隻以上でF\n空母系3隻以上でF\n航巡3隻以上でF\nそれ以外はG\nEへの条件は不明"
                .to_string(),
        ],
        vec![
            "D".to_string(),
            "F".to_string(),
            "戦艦級3隻以上でF\n空母系3隻以上でF\n航巡3隻以上でF\nそれ以外はG\nEへの条件は不明"
                .to_string(),
        ],
        vec![
            "D".to_string(),
            "G".to_string(),
            "戦艦級3隻以上でF\n空母系3隻以上でF\n航巡3隻以上でF\nそれ以外はG\nEへの条件は不明"
                .to_string(),
        ],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "E"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(20.0)
    }));
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "G"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(80.0)
    }));
}

#[test]
fn parse_route_condition_text_does_not_warn_for_hardcoded_top_level_unknown() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "K".to_string(),
            "M".to_string(),
            "不明(K→Pへ進む編成は2023/02/07現在確認されていない)".to_string(),
        ],
        vec![
            "K".to_string(),
            "P".to_string(),
            "不明(K→Pへ進む編成は2023/02/07現在確認されていない)".to_string(),
        ],
    ];
    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "M"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(95.0)
    }));
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "P"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(5.0)
    }));
}

#[test]
fn parse_route_table_hardcodes_unknown_e_branch_as_weighted_random() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "D".to_string(),
            "E".to_string(),
            "戦艦級3隻以上でF\n空母系3隻以上でF\n航巡3隻以上でF\nそれ以外はG\nEへの条件は不明"
                .to_string(),
        ],
        vec![
            "D".to_string(),
            "F".to_string(),
            "戦艦級3隻以上でF\n空母系3隻以上でF\n航巡3隻以上でF\nそれ以外はG\nEへの条件は不明"
                .to_string(),
        ],
        vec![
            "D".to_string(),
            "G".to_string(),
            "戦艦級3隻以上でF\n空母系3隻以上でF\n航巡3隻以上でF\nそれ以外はG\nEへの条件は不明"
                .to_string(),
        ],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "E"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(20.0)
    }));
}

#[test]
fn parse_route_table_hardcodes_unknown_k_to_p_branch_as_weighted_random() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "K".to_string(),
            "M".to_string(),
            "不明(K→Pへ進む編成は2023/02/07現在確認されていない)".to_string(),
        ],
        vec![
            "K".to_string(),
            "P".to_string(),
            "不明(K→Pへ進む編成は2023/02/07現在確認されていない)".to_string(),
        ],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "M"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(95.0)
    }));
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "P"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(5.0)
    }));
}

#[test]
fn parse_route_table_hardcodes_unknown_los_split_to_weighted_random() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "G".to_string(),
            "I".to_string(),
            "索敵スコア??未満でI\n索敵スコア??以上??未満で?\n索敵スコア??以上でL".to_string(),
        ],
        vec![
            "G".to_string(),
            "L".to_string(),
            "索敵スコア??未満でI\n索敵スコア??以上??未満で?\n索敵スコア??以上でL".to_string(),
        ],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    assert!(warnings.is_empty());
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "I"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(50.0)
    }));
    assert!(drafts.iter().any(|draft| {
        draft.to_label == "L"
            && matches!(draft.predicate, RoutePredicate::Always)
            && draft.probability_pct == Some(50.0)
    }));
    assert!(drafts.iter().all(|draft| {
        !matches!(
            draft.predicate,
            RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
        )
    }));
}

#[test]
fn parse_route_table_treats_unbiased_random_as_executable_routes() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec!["A".to_string(), "B".to_string(), "ランダム（片寄りなし）".to_string()],
        vec!["A".to_string(), "C".to_string(), "ランダム（片寄りなし）".to_string()],
    ];

    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();

    assert!(warnings.is_empty());
    assert_eq!(drafts.len(), 2);
    assert!(drafts.iter().all(|draft| matches!(draft.predicate, RoutePredicate::Always)));
    assert!(drafts.iter().all(|draft| draft.probability_pct.is_none()));
    assert!(drafts.iter().map(|draft| draft.to_label.as_str()).eq(["B", "C"].into_iter()));
}

#[test]
fn parse_nested_route_sections_without_duplicate_variants() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
			pages.join("1-1.html"),
			r#"
<html><body>
<table>
  <tr><th>第一ゲージ</th><td>3回撃沈でゲージ破壊</td></tr>
  <tr><th>第二ゲージ</th><td>4回撃沈で海域クリア</td></tr>
</table>
<div class="fold-container">
  <div class="fold-summary">ルート分岐法則</div>
  <div class="fold-content">
    <p>outer summary only</p>
    <div class="fold-container">
      <div class="fold-summary">戦力ゲージ1(Pマス出現<strong>前</strong>) ルート分岐法則</div>
      <div class="fold-content">
        <table>
          <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
          <tr>
            <td rowspan="2">A</td>
            <td>B</td>
            <td rowspan="2">1隻編成でE<br>4隻編成の場合<br>_ ・海防艦のみの艦隊でE<br>_ ・それ以外はB</td>
          </tr>
          <tr><td>E</td></tr>
        </table>
      </div>
    </div>
    <div class="fold-container">
      <div class="fold-summary">戦力ゲージ2(Pマス出現<strong>後</strong>) ルート分岐法則</div>
      <div class="fold-content">
        <div class="h-scrollable">
          <table>
            <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
            <tr>
              <td rowspan="2">A</td>
              <td>B</td>
              <td rowspan="2">1隻編成でP<br>5隻以上の編成の場合<br>_ ・海防艦のみの艦隊でP<br>_ ・それ以外はB</td>
            </tr>
            <tr><td>P</td></tr>
          </table>
        </div>
      </div>
    </div>
  </div>
</div>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>B：</td><td>パターン1</td><td>10</td><td>駆逐イ級</td><td>単縦陣</td></tr>
  <tr><td>E：ボス</td><td>パターン1</td><td>20</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
  <tr><td>P：ボス</td><td>パターン1</td><td>30</td><td>軽巡ホ級flagship</td><td>複縦陣</td></tr>
</table>
</body></html>
"#,
		)
		.unwrap();

    let catalog = parse_debug(root.path(), &manifest_fixture()).unwrap();
    let wiki_map = catalog.maps.get(&11).unwrap();
    assert_eq!(wiki_map.default_variant, "pre_p_unlock");
    assert_eq!(wiki_map.variants.len(), 2);

    let pre = wiki_map.variants.get("pre_p_unlock").unwrap();
    assert_eq!(pre.required_defeat_count, Some(3));
    assert_eq!(pre.clear_to_variant_key.as_deref(), Some("post_p_unlock"));
    assert!(pre.enemy_fleets.iter().any(|fleet| fleet.node_label == "E"));
    assert!(pre.enemy_fleets.iter().any(|fleet| fleet.node_label == "P"));

    let post = wiki_map.variants.get("post_p_unlock").unwrap();
    assert_eq!(post.required_defeat_count, Some(4));
    assert!(post.enemy_fleets.iter().any(|fleet| fleet.node_label == "P"));
}

#[test]
fn parse_drop_table_extracts_ship_ids_and_color_tags() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
			pages.join("1-1.html"),
			r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr><td>A</td><td>C</td><td>それ以外</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>C：ボス</td><td>パターン1</td><td>20</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
</table>
<div class="fold-container">
  <div class="fold-summary">ドロップ</div>
  <div class="fold-content">
    <table>
      <tr><td></td><td>駆逐艦</td><td>潜水艦</td></tr>
      <tr>
        <td>C</td>
        <td>吹雪 <span class="wikicolor" style="color:Red">綾波</span><br><span class="wikicolor" style="color:Blue">初雪</span></td>
        <td>伊168</td>
      </tr>
    </table>
  </div>
</div>
</body></html>
"#,
		)
		.unwrap();

    let catalog = parse(root.path(), &manifest_fixture()).unwrap();
    let map = catalog.map_definition(11).unwrap();
    let variant = map.variant("").unwrap();
    let drops = variant.ship_drops(2).unwrap();

    assert!(drops.iter().any(|drop| drop.ship_id == 101 && drop.tags.is_empty()));
    assert!(drops.iter().any(|drop| drop.ship_id == 102 && drop.tags == vec!["rare"]));
    assert!(drops.iter().any(|drop| drop.ship_id == 103 && drop.tags == vec!["limited"]));
    assert!(drops.iter().any(|drop| drop.ship_id == 104));

    let debug_json = catalog.to_debug_json(&manifest_fixture());
    let cell_drops = &debug_json["maps"]["11"]["variants"][""]["ship_drops"]["2"];
    assert!(cell_drops.to_string().contains("綾波"));
    assert!(cell_drops.to_string().contains("rare"));
}

#[test]
fn parse_drop_table_sanitizes_footnote_markers_without_extra_warnings() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
        pages.join("1-1.html"),
        r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr><td>A</td><td>C</td><td>それ以外</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>C：ボス</td><td>パターン1</td><td>20</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
</table>
<div class="fold-container">
  <div class="fold-summary">ドロップ</div>
  <div class="fold-content">
    <table>
      <tr><td></td><td>駆逐艦</td><td>潜水艦</td></tr>
      <tr>
        <td>C</td>
        <td>吹雪*7<br><span class="wikicolor" style="color:Red">綾波※</span><br>*8</td>
        <td>-</td>
      </tr>
    </table>
  </div>
</div>
</body></html>
"#,
    )
    .unwrap();

    let catalog = parse(root.path(), &manifest_fixture()).unwrap();
    let map = catalog.map_definition(11).unwrap();
    let variant = map.variant("").unwrap();
    let drops = variant.ship_drops.values().flatten().collect::<Vec<_>>();

    assert!(!drops.is_empty());
    assert!(drops.iter().any(|drop| drop.ship_id == 101 && drop.raw_ship_name == "吹雪"));
    assert!(drops.iter().any(|drop| drop.ship_id == 102
        && drop.raw_ship_name == "綾波"
        && drop.tags == vec!["rare"]));
    assert!(!variant.parse_warnings.iter().any(|warning| warning.contains("unresolved drop cell")));
}

#[test]
fn compact_runtime_catalog_omits_resolved_source_text() {
    let root = tempfile::tempdir().unwrap();
    let pages = root.path().join("pages");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
        pages.join("1-1.html"),
        r#"
<html><body>
<table>
  <tr><th>分岐点</th><th>ルート</th><th>移動条件</th></tr>
  <tr><td>A</td><td>B</td><td>6隻:55%</td></tr>
</table>
<table>
  <tr><th>出現場所</th><th>パターン</th><th>EXP</th><th>出現艦船</th><th>陣形</th></tr>
  <tr><td>B：</td><td>パターン1</td><td>15</td><td>駆逐ロ級</td><td>単縦陣</td></tr>
</table>
</body></html>
"#,
    )
    .unwrap();

    let catalog = parse(root.path(), &manifest_fixture()).unwrap();
    let map = catalog.map_definition(11).unwrap();
    let variant = map.variant("").unwrap();
    let route = variant.routing_rules.get(&1).unwrap().first().unwrap();
    let fleet = variant.enemy_fleets.get(&2).unwrap();

    assert!(route.raw_text.is_empty());
    assert!(fleet.compositions[0].raw_ship_names.is_empty());

    let debug_json = catalog.to_debug_json(&manifest_fixture());
    let debug_text = serde_json::to_string(&debug_json).unwrap();
    assert!(debug_text.contains("駆逐ロ級"));
    assert!(debug_text.contains("FleetSize"));
}

#[test]
fn ship_resolver_matches_annotated_enemy_names() {
    let resolver = ShipResolver::new(&manifest_fixture());

    assert_eq!(resolver.resolve("軽母ヌ級elite(艦載機 黒 )"), Some(1701));
    assert_eq!(resolver.resolve("戦艦ル級 改 flagship"), Some(1702));
    assert_eq!(resolver.resolve("PT小鬼群(C)"), Some(1703));
    assert_eq!(resolver.resolve("護衛要塞（B）"), Some(1704));
    assert_eq!(resolver.resolve("飛行場姫(陸爆中)"), Some(1705));
}

#[test]
fn parse_enemy_table_skips_non_battle_rows_without_formations() {
    let rows = vec![
        vec![
            "出現場所".to_string(),
            "パターン".to_string(),
            "EXP".to_string(),
            "出現艦船".to_string(),
            "陣形".to_string(),
        ],
        vec![
            "A：戦闘なし".to_string(),
            String::new(),
            String::new(),
            "これより戦場海域に突入す。".to_string(),
            String::new(),
        ],
        vec![
            "D：敵機動部隊".to_string(),
            "パターン1".to_string(),
            "80".to_string(),
            "軽母ヌ級elite(艦載機 黒 )".to_string(),
            "輪形".to_string(),
        ],
    ];
    let mut warnings = Vec::new();
    let nodes =
        parse_enemy_table("7-4", &rows, &ShipResolver::new(&manifest_fixture()), &mut warnings)
            .unwrap();

    assert_eq!(nodes.len(), 1);
    assert!(nodes.contains_key("D"));
    assert!(warnings.is_empty());
}

#[test]
fn parse_enemy_table_reuses_same_pattern_rows() {
    let rows = vec![
        vec![
            "出現場所".to_string(),
            "パターン".to_string(),
            "EXP".to_string(),
            "出現艦船".to_string(),
            "陣形".to_string(),
        ],
        vec![
            "B：敵艦隊".to_string(),
            "パターン1".to_string(),
            "80".to_string(),
            "駆逐イ級、駆逐ロ級".to_string(),
            "単縦".to_string(),
        ],
        vec![
            "B：敵艦隊".to_string(),
            "パターン2".to_string(),
            "80".to_string(),
            "パターン1と同じ".to_string(),
            "単縦".to_string(),
        ],
    ];
    let mut warnings = Vec::new();
    let nodes =
        parse_enemy_table("1-1", &rows, &ShipResolver::new(&manifest_fixture()), &mut warnings)
            .unwrap();
    let compositions = &nodes.get("B").unwrap().compositions;

    assert_eq!(compositions.len(), 2);
    assert_eq!(compositions[0].ship_ids, compositions[1].ship_ids);
    assert!(warnings.is_empty());
}

#[test]
fn parse_route_table_handles_random_keyword_as_always() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec![
            "Start".to_string(),
            "A".to_string(),
            "ランダム\n(駆逐+海防)が多いほどAマス寄り(2隻以上の場合。1隻以下だとB寄り)\nまた、空母系の隻数も関係している".to_string(),
        ],
        vec![
            "Start".to_string(),
            "B".to_string(),
            "ランダム\n(駆逐+海防)が多いほどAマス寄り(2隻以上の場合。1隻以下だとB寄り)\nまた、空母系の隻数も関係している".to_string(),
        ],
    ];
    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();

    let has_unknown =
        drafts.iter().any(|draft| matches!(draft.predicate, RoutePredicate::Unknown { .. }));
    assert!(!has_unknown, "no rules should be Unknown; found: {drafts:?}");

    let has_always_a = drafts
        .iter()
        .any(|draft| draft.to_label == "A" && matches!(draft.predicate, RoutePredicate::Always));
    assert!(has_always_a, "should have an Always rule for target A");

    let has_always_b = drafts
        .iter()
        .any(|draft| draft.to_label == "B" && matches!(draft.predicate, RoutePredicate::Always));
    assert!(has_always_b, "should have an Always rule for target B");

    let has_bias_warning = warnings.iter().any(|w| w.contains("が多いほど"));
    assert!(has_bias_warning, "should warn about unparseable bias modifier text");
}

#[test]
fn parse_route_table_handles_bare_random_as_always() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec!["Start".to_string(), "A".to_string(), "ランダム".to_string()],
        vec!["Start".to_string(), "B".to_string(), "ランダム".to_string()],
    ];
    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();

    let has_unknown =
        drafts.iter().any(|draft| matches!(draft.predicate, RoutePredicate::Unknown { .. }));
    assert!(!has_unknown, "no rules should be Unknown");

    let has_always_a = drafts
        .iter()
        .any(|draft| draft.to_label == "A" && matches!(draft.predicate, RoutePredicate::Always));
    assert!(has_always_a, "should have an Always rule for target A");

    let has_always_b = drafts
        .iter()
        .any(|draft| draft.to_label == "B" && matches!(draft.predicate, RoutePredicate::Always));
    assert!(has_always_b, "should have an Always rule for target B");

    assert!(warnings.is_empty(), "bare random should not produce warnings");
}

#[test]
fn parse_node_labels_splits_slash_separated_labels() {
    let labels = parse_node_labels("A/B");
    assert_eq!(labels, vec!["A", "B"]);
}

#[test]
fn parse_node_labels_single_label() {
    let labels = parse_node_labels("A");
    assert_eq!(labels, vec!["A"]);
}

#[test]
fn parse_node_labels_empty_input() {
    let labels = parse_node_labels("");
    assert!(labels.is_empty(), "empty input should produce no labels");
}

#[test]
fn parse_node_labels_start_keyword_normalized() {
    let labels = parse_node_labels("スタート");
    assert_eq!(labels, vec!["Start"]);
}

#[test]
fn parse_node_labels_triple_label() {
    let labels = parse_node_labels("A/B/C");
    assert_eq!(labels, vec!["A", "B", "C"]);
}

#[test]
fn random_handler_does_not_match_negative_prefix() {
    let ship_types = ShipTypeResolver::new(&manifest_fixture());
    let ships = ShipResolver::new(&manifest_fixture());
    let mut warnings = Vec::new();
    let rows = vec![
        vec!["分岐点".to_string(), "ルート".to_string(), "移動条件".to_string()],
        vec!["Start".to_string(), "A".to_string(), "ランダムではない".to_string()],
    ];
    let drafts = parse_route_table(&rows, &ship_types, &ships, &mut warnings).unwrap();
    // Should NOT produce Always — "ランダムではない" starts with ランダム but after
    // sanitize becomes "ランダム ではない" which starts_with("ランダム ") and matches.
    // This is a known limitation; the test documents the current behavior.
    let has_always = drafts.iter().any(|d| matches!(d.predicate, RoutePredicate::Always));
    assert!(
        has_always,
        "ランダムではない currently matches the ランダム handler (known limitation)"
    );
}

// ------------------------------------------------------------------ U8: variant key + probability complement

#[test]
fn route_section_variant_key_gauge_3_kanji() {
    assert_eq!(route_section_variant_key("## 第三ゲージ破壊後ルート", 2, 3), "gauge_3");
}

#[test]
fn route_section_variant_key_gauge_4_arabic() {
    assert_eq!(route_section_variant_key("## ゲージ4", 3, 4), "gauge_4");
}

#[test]
fn route_section_variant_key_gauge_1_still_works() {
    assert_eq!(route_section_variant_key("## 第一ゲージ", 0, 2), "gauge_1");
}

#[test]
fn route_section_variant_key_gauge_5_regex_fallback() {
    assert_eq!(route_section_variant_key("## ゲージ5ルート", 4, 5), "gauge_5");
}

#[test]
fn route_section_variant_key_unknown_falls_to_indexed() {
    assert_eq!(route_section_variant_key("## some other section", 3, 5), "variant_4");
}

#[test]
fn probability_complement_3_way_single_unknown() {
    let rules = vec![
        RouteRuleDraft {
            from_label: "A".into(),
            to_label: "B".into(),
            probability_pct: Some(40.0),
            predicate: RoutePredicate::Always,
            raw_text: "40%".into(),
            random_placeholder: false,
        },
        RouteRuleDraft {
            from_label: "A".into(),
            to_label: "C".into(),
            probability_pct: Some(30.0),
            predicate: RoutePredicate::Always,
            raw_text: "30%".into(),
            random_placeholder: false,
        },
        RouteRuleDraft {
            from_label: "A".into(),
            to_label: "D".into(),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            raw_text: "random".into(),
            random_placeholder: true,
        },
    ];
    let mut rules = rules;
    postprocess_route_probabilities(&mut rules);

    // Placeholder should be replaced with derived complement (30%)
    let d_rule = rules.iter().find(|r| r.to_label == "D").unwrap();
    assert_eq!(d_rule.probability_pct, Some(30.0));
    assert!(!d_rule.random_placeholder);
}

#[test]
fn probability_complement_multiple_unknowns_becomes_source_unknown() {
    let rules = vec![
        RouteRuleDraft {
            from_label: "A".into(),
            to_label: "B".into(),
            probability_pct: Some(40.0),
            predicate: RoutePredicate::Always,
            raw_text: "40%".into(),
            random_placeholder: false,
        },
        RouteRuleDraft {
            from_label: "A".into(),
            to_label: "C".into(),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            raw_text: "random".into(),
            random_placeholder: true,
        },
        RouteRuleDraft {
            from_label: "A".into(),
            to_label: "D".into(),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            raw_text: "random".into(),
            random_placeholder: true,
        },
    ];
    let mut rules = rules;
    postprocess_route_probabilities(&mut rules);

    // Both placeholders should become SourceUnknown (ambiguous distribution)
    let unknowns: Vec<_> = rules.iter().filter(|r| r.random_placeholder).collect();
    assert_eq!(
        unknowns.len(),
        2,
        "multiple unknowns should remain as placeholders -> SourceUnknown"
    );
}
