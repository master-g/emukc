//! The phases of the multi-gauge regular maps.
//!
//! A map such as 7-5 is played in phases: each has its own boss and its own part of the
//! map, and emptying one gauge opens the next. Where a phase ends comes from `KCNav`'s
//! map list; what moves a map on to the next phase is written down by hand, because no
//! machine-readable source has it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use emukc_model::codex::map::{MapCatalog, MapResetPolicy, MapVariantDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::parser::error::ParseError;

/// One phase of a map.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GaugePhase {
    /// Variant name; `phase<N>` when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The phase holds the cells numbered below this; the last phase holds them all.
    /// Filled in from `KCNav`, never by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells_below: Option<i64>,
    /// Boss kills that empty the phase's gauge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defeats: Option<i64>,
    /// Node whose arrival opens the next phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reach: Option<String>,
    /// Node that must also have been won with an S rank before the next phase opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s_rank_at: Option<String>,
    /// Free text for the reader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Map name such as `7-5` to its phases in order. Both the hand-maintained rules and the
/// generated asset have this shape; only the latter carries `cells_below`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MapGaugePhasesAsset {
    /// Where the file comes from.
    pub note: String,
    /// The phases of each map.
    pub maps: BTreeMap<String, Vec<GaugePhase>>,
}

/// Where the generated phase asset lives.
pub fn repo_map_gauge_phases_path() -> PathBuf {
    crate::assets::MAP_GAUGE_PHASES.path()
}

fn load(asset: crate::assets::RepoAsset) -> Result<MapGaugePhasesAsset, ParseError> {
    let path = asset.path();
    let (_, raw) = asset.load().map_err(|source| ParseError::io_at(&path, source))?;
    serde_json::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))
}

/// The hand-maintained rules checked into the repository.
pub fn load_repo_map_gauge_rules() -> Result<MapGaugePhasesAsset, ParseError> {
    load(crate::assets::MAP_GAUGE_RULES)
}

/// The generated phase asset checked into the repository.
pub fn load_repo_map_gauge_phases() -> Result<MapGaugePhasesAsset, ParseError> {
    load(crate::assets::MAP_GAUGE_PHASES)
}

/// Join the hand-maintained rules with the phase boundaries in `KCNav`'s map list
/// (`maps/all/meta`).
///
/// A map the two disagree about is an error both ways: a multi-phase map without rules
/// would silently stay one phase, and rules for a map `KCNav` gives one phase have nothing
/// to cut it by.
pub fn kcnav_gauge_phases(
    meta: &Value,
    rules: &MapGaugePhasesAsset,
) -> Result<MapGaugePhasesAsset, String> {
    let listed = meta
        .pointer("/result/maps")
        .or_else(|| meta.pointer("/result"))
        .and_then(Value::as_object)
        .ok_or("the map list has no maps")?;
    let mut maps = BTreeMap::new();
    for (name, map) in listed {
        // Event maps are out of scope; the regular worlds are numbered below 10.
        if map.get("world").and_then(Value::as_i64).is_none_or(|world| world >= 10) {
            continue;
        }
        let breakpoints = map
            .get("breakpoints")
            .and_then(Value::as_array)
            .map(|edges| edges.iter().filter_map(Value::as_i64).collect::<Vec<_>>())
            .unwrap_or_default();
        let Some(phases) = rules.maps.get(name) else {
            if breakpoints.is_empty() {
                continue;
            }
            return Err(format!(
                "{name} has {} phases but no rules in map_gauge_rules.json",
                breakpoints.len() + 1
            ));
        };
        if phases.len() != breakpoints.len() + 1 {
            return Err(format!(
                "{name}: {} phases in map_gauge_rules.json, {} in the map list",
                phases.len(),
                breakpoints.len() + 1
            ));
        }
        let phases = phases
            .iter()
            .enumerate()
            .map(|(index, phase)| GaugePhase {
                cells_below: breakpoints.get(index).copied(),
                note: None,
                ..phase.clone()
            })
            .collect();
        maps.insert(name.clone(), phases);
    }
    if let Some(name) = rules.maps.keys().find(|name| !maps.contains_key(*name)) {
        return Err(format!("{name} has rules but is not a multi-phase map in the map list"));
    }
    Ok(MapGaugePhasesAsset {
        note: "The phases of the multi-gauge regular maps: map_gauge_rules.json joined with \
               the phase boundaries of KCNav's map list (a boundary is the first edge of the \
               next phase, and an edge id is our cell_no). Generated by `kcnav normalize`; \
               edit map_gauge_rules.json instead."
            .to_owned(),
        maps,
    })
}

fn phase_key(index: usize, phase: &GaugePhase) -> String {
    phase.key.clone().unwrap_or_else(|| format!("phase{}", index + 1))
}

/// Cut each listed map into one variant per phase and wire the phases together.
///
/// A map that already has every phase as a variant (7-3) is only checked against the
/// boundaries and wired. Runs after the cell kinds are known, since a phase's boss is the
/// boss cell that phase adds.
pub fn apply_gauge_phases(
    catalog: &mut MapCatalog,
    asset: &MapGaugePhasesAsset,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (name, phases) in &asset.maps {
        let Some(definition) = catalog
            .maps
            .values_mut()
            .find(|map| format!("{}-{}", map.maparea_id, map.mapinfo_no) == *name)
        else {
            continue;
        };
        let keys =
            phases.iter().enumerate().map(|(i, phase)| phase_key(i, phase)).collect::<Vec<_>>();
        let precut = keys.iter().all(|key| definition.variants.contains_key(key));
        let whole = if precut {
            None
        } else {
            match definition.variants.remove("") {
                Some(whole) => Some(whole),
                None => {
                    errors.push(format!("{name}: no whole-map variant to cut into phases"));
                    continue;
                }
            }
        };

        let mut lower = 0;
        for (index, phase) in phases.iter().enumerate() {
            let upper = phase.cells_below.unwrap_or(i64::MAX);
            let mut variant = match &whole {
                Some(whole) => cut(whole, &keys[index], upper),
                None => definition.variants.remove(&keys[index]).unwrap_or_default(),
            };
            if let Err(error) = wire(&mut variant, phase, keys.get(index + 1), lower, upper) {
                errors.push(format!("{name} {}: {error}", keys[index]));
            }
            definition.variants.insert(keys[index].clone(), variant);
            lower = upper;
        }
        definition.default_variant.clone_from(&keys[0]);
        definition.reset_policy = MapResetPolicy::Monthly;
        definition.gauge_type.get_or_insert(1);
        definition.gauge_count =
            Some(phases.iter().filter(|phase| phase.defeats.is_some()).count() as i64);
        definition.required_defeat_count = phases.iter().find_map(|phase| phase.defeats);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The part of the whole map a phase can be played on.
fn cut(whole: &MapVariantDefinition, key: &str, upper: i64) -> MapVariantDefinition {
    let mut variant = whole.clone();
    key.clone_into(&mut variant.variant_key);
    variant.cells.retain(|cell| cell.cell_no < upper);
    for cell in &mut variant.cells {
        cell.next_cells.retain(|next| *next < upper);
    }
    variant
}

fn wire(
    variant: &mut MapVariantDefinition,
    phase: &GaugePhase,
    next_key: Option<&String>,
    lower: i64,
    upper: i64,
) -> Result<(), String> {
    if let Some(stray) = variant.cells.iter().find(|cell| cell.cell_no >= upper) {
        return Err(format!("cell {} lies beyond the phase, which ends at {upper}", stray.cell_no));
    }
    if upper != i64::MAX && variant.cells.len() as i64 != upper {
        return Err(format!("{} cells, but the phase ends at {upper}", variant.cells.len()));
    }
    let labelled = |label: &Option<String>| -> Result<Vec<i64>, String> {
        let Some(label) = label else {
            return Ok(Vec::new());
        };
        let cells = variant
            .cells
            .iter()
            .filter(|cell| cell.node_label.as_deref() == Some(label.as_str()))
            .map(|cell| cell.cell_no)
            .collect::<Vec<_>>();
        if cells.is_empty() {
            return Err(format!("no cell is labelled {label}"));
        }
        Ok(cells)
    };
    let advance_on_reach = labelled(&phase.reach)?;
    let advance_needs_s_rank_at = labelled(&phase.s_rank_at)?;

    // The gauge's boss is the one this phase adds; earlier bosses stay on the map as
    // ordinary dead ends. A phase that adds none keeps showing the last one.
    let boss = variant
        .cells
        .iter()
        .find(|cell| cell.cell_no >= lower && cell.event_id == 5)
        .map(|cell| cell.cell_no);
    match (boss, phase.defeats) {
        (Some(boss), _) => variant.boss_cell_no = boss,
        (None, Some(_)) => return Err("a gauge, but the phase adds no boss cell".to_owned()),
        (None, None) => {
            if let Some(last) = variant.cells.iter().rev().find(|cell| cell.event_id == 5) {
                variant.boss_cell_no = last.cell_no;
            }
        }
    }
    if phase.defeats.is_none() && advance_on_reach.is_empty() && next_key.is_some() {
        return Err("nothing moves the map on from this phase".to_owned());
    }
    variant.required_defeat_count = phase.defeats;
    variant.clear_to_variant_key = next_key.cloned();
    variant.advance_on_reach = advance_on_reach;
    variant.advance_needs_s_rank_at = advance_needs_s_rank_at;
    Ok(())
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::map::{MapCellDefinition, MapDefinition};
    use serde_json::json;

    use super::*;

    fn cell(cell_no: i64, label: &str, event_id: i64, next_cells: &[i64]) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            event_id,
            next_cells: next_cells.to_vec(),
            node_label: Some(label.to_owned()),
            ..Default::default()
        }
    }

    fn rules(phases: Vec<GaugePhase>) -> MapGaugePhasesAsset {
        MapGaugePhasesAsset {
            note: String::new(),
            maps: BTreeMap::from([("7-5".to_owned(), phases)]),
        }
    }

    fn defeats(count: i64) -> GaugePhase {
        GaugePhase {
            defeats: Some(count),
            ..Default::default()
        }
    }

    fn catalog() -> MapCatalog {
        let mut definition = MapDefinition::minimal(75);
        definition.variants.insert(
            String::new(),
            MapVariantDefinition {
                boss_cell_no: 4,
                cells: vec![
                    cell(0, "Start", 0, &[1]),
                    cell(1, "A", 4, &[2, 3]),
                    cell(2, "K", 5, &[]),
                    cell(3, "M", 4, &[4]),
                    cell(4, "T", 5, &[]),
                ],
                ..Default::default()
            },
        );
        MapCatalog {
            maps: BTreeMap::from([(75, definition)]),
            ..Default::default()
        }
    }

    #[test]
    fn boundaries_come_from_the_map_list_and_must_match_the_rules() {
        let meta = json!({"result": {
            "7-5": {"world": 7, "breakpoints": [3]},
            "1-1": {"world": 1, "breakpoints": null},
            "61-1": {"world": 61, "breakpoints": [5]},
        }});
        let asset = kcnav_gauge_phases(&meta, &rules(vec![defeats(2), defeats(3)])).unwrap();
        assert_eq!(asset.maps["7-5"][0].cells_below, Some(3));
        assert_eq!(asset.maps["7-5"][1].cells_below, None);

        let err = kcnav_gauge_phases(&meta, &rules(vec![defeats(2)])).unwrap_err();
        assert!(err.contains("1 phases in map_gauge_rules.json, 2 in the map list"), "{err}");
    }

    #[test]
    fn a_map_is_cut_into_phases_that_lead_to_one_another() {
        let mut catalog = catalog();
        let phases = vec![
            GaugePhase {
                cells_below: Some(3),
                s_rank_at: Some("A".to_owned()),
                ..defeats(2)
            },
            defeats(3),
        ];
        apply_gauge_phases(&mut catalog, &rules(phases)).unwrap();

        let map = &catalog.maps[&75];
        assert_eq!(map.default_variant, "phase1");
        assert_eq!(map.reset_policy, MapResetPolicy::Monthly);
        assert_eq!(map.gauge_count, Some(2));
        assert!(!map.variants.contains_key(""));

        let first = &map.variants["phase1"];
        assert_eq!(first.cells.iter().map(|cell| cell.cell_no).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(first.cell(1).unwrap().next_cells, [2], "the edge to M is not open yet");
        assert_eq!(first.boss_cell_no, 2);
        assert_eq!(first.required_defeat_count, Some(2));
        assert_eq!(first.clear_to_variant_key.as_deref(), Some("phase2"));
        assert_eq!(first.advance_needs_s_rank_at, [1]);

        let second = &map.variants["phase2"];
        assert_eq!(second.cells.len(), 5);
        assert_eq!(second.boss_cell_no, 4, "the gauge's boss is the one the phase adds");
        assert_eq!(second.required_defeat_count, Some(3));
        assert_eq!(second.clear_to_variant_key, None);
        assert_eq!(map.chained_gauge("phase2"), Some((2, 3)));
    }

    #[test]
    fn a_phase_opened_by_arrival_has_no_gauge_and_shows_the_next_one() {
        let mut catalog = catalog();
        let phases = vec![
            GaugePhase {
                cells_below: Some(3),
                ..defeats(2)
            },
            GaugePhase {
                cells_below: Some(4),
                reach: Some("M".to_owned()),
                ..Default::default()
            },
            defeats(3),
        ];
        apply_gauge_phases(&mut catalog, &rules(phases)).unwrap();

        let map = &catalog.maps[&75];
        let second = &map.variants["phase2"];
        assert_eq!(second.advance_on_reach, [3]);
        assert_eq!(second.required_defeat_count, None);
        assert_eq!(second.boss_cell_no, 2);
        assert_eq!(map.gauge_count, Some(2));
        assert_eq!(map.chained_gauge("phase2"), Some((2, 3)));
    }

    #[test]
    fn rules_the_map_cannot_carry_are_refused() {
        let phases = vec![
            GaugePhase {
                cells_below: Some(2),
                ..defeats(2)
            },
            defeats(3),
        ];
        let errors = apply_gauge_phases(&mut catalog(), &rules(phases)).unwrap_err();
        assert!(errors[0].contains("the phase adds no boss cell"), "{errors:?}");
    }
}
