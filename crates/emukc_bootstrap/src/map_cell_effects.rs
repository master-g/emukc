//! What the resource cells give and the maelstroms take.
//!
//! No machine-readable source has these, so `map_cell_effects.json` is written by hand
//! from wikiwiki's area pages, by map name and node label. Assembling the map catalog
//! pins it onto cell numbers.

use std::collections::BTreeMap;

use emukc_model::codex::map::{CellEffect, CellEffects, MapCatalog};
use serde::Deserialize;

use crate::parser::error::ParseError;

/// `map_cell_effects.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapCellEffectsAsset {
    /// Where the file comes from.
    #[expect(dead_code, reason = "for the reader of the file")]
    pub note: String,
    /// Equipment id to name: what counts as a landing craft at a resource cell.
    pub landing_craft: BTreeMap<i64, String>,
    /// Equipment id to name: what counts as a drum.
    pub drums: BTreeMap<i64, String>,
    /// Map name such as `2-2`, then node label.
    pub maps: BTreeMap<String, BTreeMap<String, CellEffect>>,
}

/// The asset checked into the repository.
pub fn load_repo_map_cell_effects() -> Result<MapCellEffectsAsset, ParseError> {
    let asset = crate::assets::MAP_CELL_EFFECTS;
    let path = asset.path();
    let (_, raw) = asset.load().map_err(|source| ParseError::io_at(&path, source))?;
    serde_json::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))
}

/// The kind of cell an effect belongs on: 2 for a resource cell, 3 for a maelstrom.
fn event_id_of(effect: &CellEffect) -> i64 {
    if effect.maelstrom.is_some() {
        3
    } else {
        2
    }
}

/// Pin the effects onto the catalog's cells.
///
/// The asset and the regular maps must agree both ways: an entry for a cell that is not a
/// resource cell or a maelstrom is a mistake in the asset, and such a cell without an entry
/// would give or take nothing.
pub fn apply_cell_effects(
    catalog: &mut MapCatalog,
    asset: &MapCellEffectsAsset,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut cells: BTreeMap<i64, BTreeMap<i64, CellEffect>> = BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for map in catalog.maps.values().filter(|map| !map.is_event) {
        let name = format!("{}-{}", map.maparea_id, map.mapinfo_no);
        let effects = asset.maps.get(&name);
        for cell in map.variants.values().flat_map(|variant| &variant.cells) {
            if !matches!(cell.event_id, 2 | 3) {
                continue;
            }
            let label = cell.node_label.as_deref().unwrap_or_default();
            match effects.and_then(|effects| effects.get(label)) {
                Some(effect) if event_id_of(effect) == cell.event_id => {
                    used.insert((name.clone(), label.to_string()));
                    cells.entry(map.map_id).or_default().insert(cell.cell_no, effect.clone());
                }
                Some(_) => errors.push(format!(
                    "{name} {label}: the entry is not for a cell of event {}",
                    cell.event_id
                )),
                None => errors.push(format!(
                    "{name} {label} (event {}) has no entry in map_cell_effects.json",
                    cell.event_id
                )),
            }
        }
    }
    for (name, effects) in &asset.maps {
        for label in effects.keys() {
            if !used.contains(&(name.clone(), label.clone())) {
                errors.push(format!("{name} {label} is not a resource cell or a maelstrom"));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    catalog.cell_effects = CellEffects {
        landing_craft: asset.landing_craft.keys().copied().collect(),
        drums: asset.drums.keys().copied().collect(),
        cells,
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::map::{
        CellGain, CellItem, MapCellDefinition, MapDefinition, MapVariantDefinition,
    };

    use super::*;

    fn catalog(cells: &[(i64, &str, i64, i64)]) -> MapCatalog {
        let mut map = MapDefinition::minimal(12);
        map.variants.insert(
            String::new(),
            MapVariantDefinition {
                cells: cells
                    .iter()
                    .map(|&(cell_no, label, event_id, event_kind)| MapCellDefinition {
                        cell_no,
                        event_id,
                        event_kind,
                        node_label: Some(label.to_string()),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
        );
        let mut catalog = MapCatalog::default();
        catalog.maps.insert(12, map);
        catalog
    }

    fn asset(entries: &[&str]) -> MapCellEffectsAsset {
        let gain = CellEffect {
            gains: vec![CellGain {
                item: CellItem::Ammo,
                min: 10,
                max: 20,
                per_drum: 2.0,
                per_craft: 3.0,
                cap: None,
                guess: None,
            }],
            maelstrom: None,
        };
        MapCellEffectsAsset {
            maps: BTreeMap::from([(
                "1-2".to_string(),
                entries.iter().map(|label| ((*label).to_string(), gain.clone())).collect(),
            )]),
            ..Default::default()
        }
    }

    #[test]
    fn an_entry_lands_on_the_cell_with_its_label() {
        let mut catalog = catalog(&[(1, "A", 4, 1), (2, "B", 2, 0)]);
        apply_cell_effects(&mut catalog, &asset(&["B"])).unwrap();
        assert_eq!(catalog.cell_effect(12, 2).unwrap().gains[0].item, CellItem::Ammo);
        assert!(catalog.cell_effect(12, 1).is_none());
    }

    #[test]
    fn a_resource_cell_without_an_entry_is_an_error() {
        let mut catalog = catalog(&[(2, "B", 2, 0)]);
        let errors = apply_cell_effects(&mut catalog, &asset(&[])).unwrap_err();
        assert!(errors[0].contains("1-2 B (event 2) has no entry"), "{errors:?}");
    }

    #[test]
    fn an_entry_for_another_kind_of_cell_is_an_error() {
        let mut catalog = catalog(&[(1, "A", 4, 1), (2, "B", 3, 0)]);
        let errors = apply_cell_effects(&mut catalog, &asset(&["A", "B"])).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("1-2 B: the entry is not for")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("1-2 A is not a resource cell")), "{errors:?}");
    }

    #[test]
    fn the_repo_asset_names_its_equipment_as_the_master_data_does() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.data/codex/start2.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{}: {err}; run `bootstrap` first", path.display()));
        let manifest: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let names: BTreeMap<i64, &str> = manifest["api_mst_slotitem"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| (item["api_id"].as_i64().unwrap(), item["api_name"].as_str().unwrap()))
            .collect();
        let asset = load_repo_map_cell_effects().unwrap();
        for (id, name) in asset.landing_craft.iter().chain(&asset.drums) {
            assert_eq!(names.get(id), Some(&name.as_str()), "equipment {id}");
        }
    }
}
