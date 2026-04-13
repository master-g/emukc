use crate::kc2::start2::ApiManifest;

use super::{MapCatalog, RouteOperator, RoutePredicate, SpeedClass};

pub(super) fn to_debug_json(catalog: &MapCatalog, manifest: &ApiManifest) -> serde_json::Value {
    let maps = catalog
        .maps
        .iter()
        .map(|(map_id, definition)| {
            let variants = definition
                .variants
                .iter()
                .map(|(variant_key, variant)| {
                    let routes = variant
                        .routing_rules
                        .iter()
                        .map(|(from_cell_no, rules)| {
                            let debug_rules = rules
								.iter()
								.map(|rule| {
									serde_json::json!({
										"to_cell_no": rule.to_cell_no,
										"priority": rule.priority,
										"weight": rule.weight,
										"probability_pct": rule.probability_pct,
										"predicate": route_predicate_debug_json(&rule.predicate, manifest),
										"raw_text": (!rule.raw_text.is_empty()).then_some(rule.raw_text.clone()),
									})
								})
								.collect::<Vec<_>>();
                            (from_cell_no.to_string(), serde_json::Value::Array(debug_rules))
                        })
                        .collect::<serde_json::Map<String, serde_json::Value>>();

                    let enemy_fleets = variant
                        .enemy_fleets
                        .iter()
                        .map(|(cell_no, fleet)| {
                            let compositions = fleet
                                .compositions
                                .iter()
                                .map(|composition| {
                                    let ships = composition
                                        .ship_ids
                                        .iter()
                                        .map(|ship_id| {
                                            let name = manifest
                                                .find_ship(*ship_id)
                                                .map(|ship| ship.api_name.clone())
                                                .unwrap_or_else(|| format!("unknown:{ship_id}"));
                                            serde_json::json!({
                                                "id": ship_id,
                                                "name": name,
                                            })
                                        })
                                        .collect::<Vec<_>>();
                                    serde_json::json!({
                                        "comp_id": composition.comp_id,
                                        "weight": composition.weight,
                                        "formation": composition.formation,
                                        "ships": ships,
                                        "raw_ship_names": (!composition.raw_ship_names.is_empty())
                                            .then_some(composition.raw_ship_names.clone()),
                                    })
                                })
                                .collect::<Vec<_>>();
                            (
                                cell_no.to_string(),
                                serde_json::json!({
                                    "battle_kind": fleet.battle_kind,
                                    "formations": fleet.formations,
                                    "compositions": compositions,
                                }),
                            )
                        })
                        .collect::<serde_json::Map<String, serde_json::Value>>();

                    let ship_drops = variant
                        .ship_drops
                        .iter()
                        .map(|(cell_no, drops)| {
                            let drops = drops
								.iter()
								.map(|drop| {
									let name = manifest
										.find_ship(drop.ship_id)
										.map(|ship| ship.api_name.clone())
										.unwrap_or_else(|| format!("unknown:{}", drop.ship_id));
									serde_json::json!({
										"ship_id": drop.ship_id,
										"ship_name": name,
										"raw_ship_name": (!drop.raw_ship_name.is_empty()).then_some(drop.raw_ship_name.clone()),
										"tags": (!drop.tags.is_empty()).then_some(drop.tags.clone()),
									})
								})
								.collect::<Vec<_>>();
                            (cell_no.to_string(), serde_json::Value::Array(drops))
                        })
                        .collect::<serde_json::Map<String, serde_json::Value>>();

                    let cells = variant
                        .cells
                        .iter()
                        .map(|cell| {
                            serde_json::json!({
                                "cell_no": cell.cell_no,
                                "color_no": cell.color_no,
                                "event_id": cell.event_id,
                                "event_kind": cell.event_kind,
                                "next_cells": cell.next_cells,
                                "master_cell_id": cell.master_cell_id,
                                "distance": cell.distance,
                            })
                        })
                        .collect::<Vec<_>>();

                    (
                        variant_key.clone(),
                        serde_json::json!({
                            "boss_cell_no": variant.boss_cell_no,
                            "required_defeat_count": variant.required_defeat_count,
                            "clear_to_variant_key": variant.clear_to_variant_key,
                            "cells": cells,
                            "routing_rules": routes,
                            "enemy_fleets": enemy_fleets,
                            "ship_drops": ship_drops,
                            "parse_warnings": variant.parse_warnings,
                        }),
                    )
                })
                .collect::<serde_json::Map<String, serde_json::Value>>();

            (
                map_id.to_string(),
                serde_json::json!({
                    "name": definition.name,
                    "maparea_id": definition.maparea_id,
                    "mapinfo_no": definition.mapinfo_no,
                    "is_event": definition.is_event,
                    "variants": variants,
                }),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();

    serde_json::json!({ "maps": maps })
}

fn route_predicate_debug_json(
    predicate: &RoutePredicate,
    manifest: &ApiManifest,
) -> serde_json::Value {
    match predicate {
        RoutePredicate::Always => serde_json::json!({
            "kind": "Always",
            "text": "always",
        }),
        RoutePredicate::VisitedNode {
            cell_nos,
            visited,
        } => serde_json::json!({
            "kind": "VisitedNode",
            "cell_nos": cell_nos,
            "visited": visited,
            "text": if *visited {
                format!("visited {:?}", cell_nos)
            } else {
                format!("not visited {:?}", cell_nos)
            },
        }),
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => serde_json::json!({
            "kind": "VisitedNodeLabel",
            "node_labels": node_labels,
            "visited": visited,
            "text": if *visited {
                format!("visited {}", node_labels.join("/"))
            } else {
                format!("not visited {}", node_labels.join("/"))
            },
        }),
        RoutePredicate::FleetSize {
            op,
            value,
        } => serde_json::json!({
            "kind": "FleetSize",
            "op": format!("{op:?}"),
            "value": value,
            "text": format!("fleet size {} {value}", route_operator_text(*op)),
        }),
        RoutePredicate::EquipmentCount {
            slotitem_types,
            op,
            value,
        } => serde_json::json!({
            "kind": "EquipmentCount",
            "slotitem_types": slotitem_types,
            "op": format!("{op:?}"),
            "value": value,
            "text": format!("equipment ship count {} {value}", route_operator_text(*op)),
        }),
        RoutePredicate::ShipTypeCount {
            ship_types,
            op,
            value,
        } => {
            let names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "ShipTypeCount",
                "ship_types": ship_types,
                "ship_type_names": names,
                "op": format!("{op:?}"),
                "value": value,
                "text": format!("{} count {} {value}", names.join("/"), route_operator_text(*op)),
            })
        }
        RoutePredicate::ContainsShipType {
            ship_types,
        } => {
            let names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "ContainsShipType",
                "ship_types": ship_types,
                "ship_type_names": names,
                "text": format!("contains {}", names.join("/")),
            })
        }
        RoutePredicate::FlagshipShipType {
            ship_types,
        } => {
            let names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "FlagshipShipType",
                "ship_types": ship_types,
                "ship_type_names": names,
                "text": format!("flagship is {}", names.join("/")),
            })
        }
        RoutePredicate::FlagshipShipId {
            ship_ids,
        } => {
            let names = ship_ids
                .iter()
                .map(|ship_id| {
                    manifest
                        .find_ship(*ship_id)
                        .map(|ship| ship.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_id}"))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "FlagshipShipId",
                "ship_ids": ship_ids,
                "ship_names": names,
                "text": format!("flagship is {}", names.join("/")),
            })
        }
        RoutePredicate::ContainsShipId {
            ship_ids,
        } => {
            let names = ship_ids
                .iter()
                .map(|ship_id| {
                    manifest
                        .find_ship(*ship_id)
                        .map(|ship| ship.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_id}"))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "ContainsShipId",
                "ship_ids": ship_ids,
                "ship_names": names,
                "text": format!("contains {}", names.join("/")),
            })
        }
        RoutePredicate::ContainsShipSet {
            ship_types,
            ship_ids,
        } => {
            let type_names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            let ship_names = ship_ids
                .iter()
                .map(|ship_id| {
                    manifest
                        .find_ship(*ship_id)
                        .map(|ship| ship.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_id}"))
                })
                .collect::<Vec<_>>();
            let mut labels = type_names.clone();
            labels.extend(ship_names.clone());
            serde_json::json!({
                "kind": "ContainsShipSet",
                "ship_types": ship_types,
                "ship_type_names": type_names,
                "ship_ids": ship_ids,
                "ship_names": ship_names,
                "text": format!("contains {}", labels.join("/")),
            })
        }
        RoutePredicate::OnlyShipTypes {
            ship_types,
        } => {
            let names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "kind": "OnlyShipTypes",
                "ship_types": ship_types,
                "ship_type_names": names,
                "text": format!("only {}", names.join("/")),
            })
        }
        RoutePredicate::OnlyShipSet {
            ship_types,
            ship_ids,
        } => {
            let type_names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            let ship_names = ship_ids
                .iter()
                .map(|ship_id| {
                    manifest
                        .find_ship(*ship_id)
                        .map(|ship| ship.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_id}"))
                })
                .collect::<Vec<_>>();
            let mut labels = type_names.clone();
            labels.extend(ship_names.clone());
            serde_json::json!({
                "kind": "OnlyShipSet",
                "ship_types": ship_types,
                "ship_type_names": type_names,
                "ship_ids": ship_ids,
                "ship_names": ship_names,
                "text": format!("only {}", labels.join("/")),
            })
        }
        RoutePredicate::ShipSetCount {
            ship_types,
            ship_ids,
            op,
            value,
        } => {
            let type_names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            let ship_names = ship_ids
                .iter()
                .map(|ship_id| {
                    manifest
                        .find_ship(*ship_id)
                        .map(|ship| ship.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_id}"))
                })
                .collect::<Vec<_>>();
            let mut labels = type_names.clone();
            labels.extend(ship_names.clone());
            serde_json::json!({
                "kind": "ShipSetCount",
                "ship_types": ship_types,
                "ship_type_names": type_names,
                "ship_ids": ship_ids,
                "ship_names": ship_names,
                "op": format!("{op:?}"),
                "value": value,
                "text": format!("{} count {} {value}", labels.join("/"), route_operator_text(*op)),
            })
        }
        RoutePredicate::ShipSetSpeedCount {
            ship_types,
            ship_ids,
            speed_op,
            speed_class,
            op,
            value,
        } => {
            let type_names = ship_types
                .iter()
                .map(|ship_type| {
                    manifest
                        .find_ship_type(*ship_type)
                        .map(|ship_type| ship_type.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_type}"))
                })
                .collect::<Vec<_>>();
            let ship_names = ship_ids
                .iter()
                .map(|ship_id| {
                    manifest
                        .find_ship(*ship_id)
                        .map(|ship| ship.api_name.clone())
                        .unwrap_or_else(|| format!("unknown:{ship_id}"))
                })
                .collect::<Vec<_>>();
            let mut labels = type_names.clone();
            labels.extend(ship_names.clone());
            serde_json::json!({
                "kind": "ShipSetSpeedCount",
                "ship_types": ship_types,
                "ship_type_names": type_names,
                "ship_ids": ship_ids,
                "ship_names": ship_names,
                "speed_op": format!("{speed_op:?}"),
                "speed_class": format!("{speed_class:?}"),
                "op": format!("{op:?}"),
                "value": value,
                "text": format!(
                    "{} ships with speed {} {} count {} {value}",
                    labels.join("/"),
                    route_operator_text(*speed_op),
                    speed_class_text(*speed_class),
                    route_operator_text(*op),
                ),
            })
        }
        RoutePredicate::Speed {
            class,
        } => serde_json::json!({
            "kind": "Speed",
            "class": format!("{class:?}"),
            "text": format!("speed {}", speed_class_text(*class)),
        }),
        RoutePredicate::LoS {
            formula,
            op,
            value,
        } => serde_json::json!({
            "kind": "LoS",
            "formula": formula,
            "op": format!("{op:?}"),
            "value": value,
            "text": format!(
                "LoS{} {} {value}",
                formula.as_ref().map(|formula| format!(" ({formula})")).unwrap_or_default(),
                route_operator_text(*op)
            ),
        }),
        RoutePredicate::DrumCanisterCount {
            op,
            value,
        } => serde_json::json!({
            "kind": "DrumCanisterCount",
            "op": format!("{op:?}"),
            "value": value,
            "text": format!("drum canister count {} {value}", route_operator_text(*op)),
        }),
        RoutePredicate::And(predicates) => serde_json::json!({
            "kind": "And",
            "items": predicates.iter().map(|predicate| route_predicate_debug_json(predicate, manifest)).collect::<Vec<_>>(),
        }),
        RoutePredicate::Or(predicates) => serde_json::json!({
            "kind": "Or",
            "items": predicates.iter().map(|predicate| route_predicate_debug_json(predicate, manifest)).collect::<Vec<_>>(),
        }),
        RoutePredicate::Not(predicate) => serde_json::json!({
            "kind": "Not",
            "item": route_predicate_debug_json(predicate, manifest),
        }),
        RoutePredicate::FleetSizeWeightedRandom {
            weights,
        } => serde_json::json!({
            "kind": "FleetSizeWeightedRandom",
            "weights": weights.iter().map(|w| serde_json::json!({
                "fleet_size": w.fleet_size,
                "probability_pct": w.probability_pct,
            })).collect::<Vec<_>>(),
        }),
        RoutePredicate::Unknown {
            raw_text,
        } => serde_json::json!({
            "kind": "Unknown",
            "raw_text": raw_text,
        }),
        RoutePredicate::SourceUnknown {
            raw_text,
        } => serde_json::json!({
            "kind": "SourceUnknown",
            "raw_text": raw_text,
        }),
    }
}

fn route_operator_text(op: RouteOperator) -> &'static str {
    match op {
        RouteOperator::Eq => "==",
        RouteOperator::Gte => ">=",
        RouteOperator::Lte => "<=",
    }
}

fn speed_class_text(class: SpeedClass) -> &'static str {
    match class {
        SpeedClass::Slow => "slow+",
        SpeedClass::Fast => "fast",
        SpeedClass::FastPlus => "fast+",
        SpeedClass::Fastest => "fastest",
    }
}
