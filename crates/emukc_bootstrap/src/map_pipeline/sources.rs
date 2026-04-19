use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use emukc_model::{
    codex::map::{MapCatalog, MapCellDefinition, MapDefinition, MapVariantDefinition},
    kc2::start2::ApiManifest,
};

use crate::{
    parser::error::ParseError,
    wikiwiki_map_asset::{
        RepoWikiwikiMapCatalogSource, load_repo_wikiwiki_map_catalog_asset,
        repo_wikiwiki_map_catalog_path,
    },
};

use super::report::MapCatalogWikiwikiSource;

const STAT_JSON_URL: &str =
    "https://raw.githubusercontent.com/KagamiChan/kcs2-mapdata/master/maps/stat.json";
const STAT_JSON_FILENAME: &str = "stat.json";

pub(super) struct ResolvedMapSources {
    pub(super) wikiwiki_source: MapCatalogWikiwikiSource,
    pub(super) wikiwiki_map_count: usize,
    pub(super) wikiwiki_catalog: Option<MapCatalog>,
    pub(super) public_overlay_map_count: usize,
    pub(super) public_overlay_catalog: MapCatalog,
    pub(super) stat_map_count: usize,
    pub(super) stat_catalog: Option<MapCatalog>,
    pub(super) stat_from_cache: bool,
}

pub(super) fn load_explicit_source_set(
    data_root: &Path,
    manifest: &ApiManifest,
    wikiwiki_catalog: Option<MapCatalog>,
) -> Result<ResolvedMapSources, ParseError> {
    let wikiwiki_map_count =
        wikiwiki_catalog.as_ref().map(|catalog| catalog.maps.len()).unwrap_or(0);
    let public_overlay_catalog = load_public_map_catalog_overlays()?;
    let public_overlay_map_count = public_overlay_catalog.maps.len();
    let (stat_catalog, stat_map_count, stat_from_cache) = load_stat_catalog(data_root);

    Ok(ResolvedMapSources {
        wikiwiki_source: if wikiwiki_catalog.is_some() {
            MapCatalogWikiwikiSource::Provided
        } else {
            MapCatalogWikiwikiSource::None
        },
        wikiwiki_map_count,
        wikiwiki_catalog,
        public_overlay_map_count,
        public_overlay_catalog,
        stat_map_count,
        stat_catalog,
        stat_from_cache,
    })
}

pub(super) fn load_repo_source_set(
    data_root: &Path,
    manifest: &ApiManifest,
) -> Result<ResolvedMapSources, ParseError> {
    let (wikiwiki_source, wikiwiki_catalog) = load_repo_wikiwiki_map_catalog()?;
    let wikiwiki_map_count =
        wikiwiki_catalog.as_ref().map(|catalog| catalog.maps.len()).unwrap_or(0);
    let public_overlay_catalog = load_public_map_catalog_overlays()?;
    let public_overlay_map_count = public_overlay_catalog.maps.len();
    let (stat_catalog, stat_map_count, stat_from_cache) = load_stat_catalog(data_root);

    Ok(ResolvedMapSources {
        wikiwiki_source,
        wikiwiki_map_count,
        wikiwiki_catalog,
        public_overlay_map_count,
        public_overlay_catalog,
        stat_map_count,
        stat_catalog,
        stat_from_cache,
    })
}

fn load_public_map_catalog_overlays() -> Result<MapCatalog, ParseError> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/public_map_catalog_overlays.json");
    serde_json::from_str::<MapCatalog>(include_str!(
        "../../assets/public_map_catalog_overlays.json"
    ))
    .map_err(|source| ParseError::json_at(&path, source))
}

fn load_repo_wikiwiki_map_catalog()
-> Result<(MapCatalogWikiwikiSource, Option<MapCatalog>), ParseError> {
    let path = repo_wikiwiki_map_catalog_path();
    let asset = load_repo_wikiwiki_map_catalog_asset()
        .map_err(|source| ParseError::io_at(&path, source))?;
    let source_name = match &asset.source {
        RepoWikiwikiMapCatalogSource::Filesystem(asset_path) => asset_path.display().to_string(),
        RepoWikiwikiMapCatalogSource::Embedded => {
            info!(
                "repo wikiwiki map catalog not found at {}; using embedded catalog asset",
                path.display()
            );
            "embedded wikiwiki map catalog".to_string()
        }
    };
    let source_kind = match asset.source {
        RepoWikiwikiMapCatalogSource::Filesystem(_) => MapCatalogWikiwikiSource::Filesystem,
        RepoWikiwikiMapCatalogSource::Embedded => MapCatalogWikiwikiSource::Embedded,
    };

    match serde_json::from_str::<MapCatalog>(asset.raw_json()) {
        Ok(catalog) => Ok((source_kind, Some(catalog))),
        Err(source) => {
            warn!("failed to parse {}: {}. Using overlay-only map catalog", source_name, source);
            Ok((source_kind, None))
        }
    }
}

/// Load stat catalog from cache or download. Returns (catalog, map_count, from_cache).
fn load_stat_catalog(data_root: &Path) -> (Option<MapCatalog>, usize, bool) {
    let cache_path = data_root.join(STAT_JSON_FILENAME);

    // Try loading from cache first
    if cache_path.exists() {
        match std::fs::read_to_string(&cache_path) {
            Ok(raw) => match parse_stat_json(&raw) {
                Ok(catalog) => {
                    let count = catalog.maps.len();
                    info!("stat.json: loaded from cache, {count} maps");
                    return (Some(catalog), count, true);
                }
                Err(e) => {
                    warn!("stat.json: failed to parse cache: {e}");
                }
            },
            Err(e) => {
                warn!("stat.json: failed to read cache: {e}");
            }
        }
    }

    // Try downloading
    match download_stat_json(&cache_path) {
        Ok(raw) => match parse_stat_json(&raw) {
            Ok(catalog) => {
                let count = catalog.maps.len();
                info!("stat.json: downloaded and cached, {count} maps");
                if let Err(e) = std::fs::write(&cache_path, &raw) {
                    warn!("stat.json: failed to cache: {e}");
                }
                (Some(catalog), count, false)
            }
            Err(e) => {
                warn!("stat.json: failed to parse downloaded data: {e}");
                (None, 0, false)
            }
        },
        Err(e) => {
            info!("stat.json: download failed ({e}), no cache available");
            (None, 0, false)
        }
    }
}

fn download_stat_json(_cache_path: &Path) -> Result<String, String> {
    std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime create: {e}"))?;
            rt.block_on(async {
                let client = emukc_network::client::new_reqwest_client(None, None)
                    .map_err(|e| format!("client create: {e}"))?;
                let resp =
                    client.get(STAT_JSON_URL).send().await.map_err(|e| format!("download: {e}"))?;
                if !resp.status().is_success() {
                    return Err(format!("http {}", resp.status()));
                }
                resp.text().await.map_err(|e| format!("read body: {e}"))
            })
        })
        .join()
        .unwrap()
    })
}

/// Parse stat.json into a MapCatalog.
///
/// stat.json format: `{ "map_id": { "label": { "cell_id": "label", "event_id": N, "event_kind": N } } }`
///
/// Each map becomes a MapDefinition with a single "" variant. Each cell gets
/// `node_label = Some(label)` and `event_id`/`event_kind` from stat data.
/// Cell numbers are assigned sequentially starting from 1 (label-based matching
/// happens during assembly merge via `semantic_cell_no_map`).
fn parse_stat_json(raw: &str) -> Result<MapCatalog, String> {
    #[derive(serde::Deserialize)]
    struct StatCell {
        #[allow(dead_code)]
        cell_id: String,
        event_id: i64,
        event_kind: i64,
    }

    let stat: BTreeMap<String, BTreeMap<String, StatCell>> =
        serde_json::from_str(raw).map_err(|e| format!("parse: {e}"))?;

    let mut catalog = MapCatalog::default();

    for (map_id_str, cells) in stat {
        let map_id: i64 = map_id_str.parse().map_err(|e| format!("map_id {map_id_str}: {e}"))?;

        let mut variant_cells = Vec::new();
        for (i, (label, cell_data)) in cells.into_iter().enumerate() {
            variant_cells.push(MapCellDefinition {
                cell_no: (i as i64) + 1,
                color_no: 0,
                event_id: cell_data.event_id,
                event_kind: cell_data.event_kind,
                next_cells: Vec::new(),
                node_label: Some(label),
                master_cell_id: None,
                distance: None,
            });
        }

        let maparea_id = map_id / 10;
        let mapinfo_no = map_id % 10;
        catalog.maps.insert(
            map_id,
            MapDefinition {
                map_id,
                maparea_id,
                mapinfo_no,
                name: String::new(),
                level: 0,
                sally_flag: Vec::new(),
                is_event: false,
                reset_policy: Default::default(),
                airbase_count: None,
                gauge_type: None,
                gauge_count: None,
                required_defeat_count: None,
                max_hp: None,
                default_variant: String::new(),
                rank_stage_ids: Default::default(),
                variants: [(
                    String::new(),
                    MapVariantDefinition {
                        variant_key: String::new(),
                        boss_cell_no: 0,
                        cells: variant_cells,
                        routing_rules: Default::default(),
                        enemy_fleets: Default::default(),
                        ship_drops: Default::default(),
                        required_defeat_count: None,
                        clear_to_variant_key: None,
                        parse_warnings: Vec::new(),
                    },
                )]
                .into_iter()
                .collect(),
            },
        );
    }

    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stat_json_basic() {
        let raw = r#"{
			"13": {
				"A": { "cell_id": "A", "event_id": 6, "event_kind": 6 },
				"B": { "cell_id": "B", "event_id": 4, "event_kind": 1 },
				"J": { "cell_id": "J", "event_id": 5, "event_kind": 1 }
			},
			"11": {
				"C": { "cell_id": "C", "event_id": 2, "event_kind": 0 }
			}
		}"#;

        let catalog = parse_stat_json(raw).unwrap();
        assert_eq!(catalog.maps.len(), 2);

        let map_13 = catalog.maps.get(&13).unwrap();
        let variant = map_13.variants.get("").unwrap();
        assert_eq!(variant.cells.len(), 3);

        // Check that cells have node_label set
        let cell_a = variant.cells.iter().find(|c| c.node_label.as_deref() == Some("A")).unwrap();
        assert_eq!(cell_a.event_id, 6);
        assert_eq!(cell_a.event_kind, 6);

        let cell_j = variant.cells.iter().find(|c| c.node_label.as_deref() == Some("J")).unwrap();
        assert_eq!(cell_j.event_id, 5);
        assert_eq!(cell_j.event_kind, 1);

        let map_11 = catalog.maps.get(&11).unwrap();
        let variant_11 = map_11.variants.get("").unwrap();
        assert_eq!(variant_11.cells.len(), 1);
        assert_eq!(variant_11.cells[0].event_id, 2);
        assert_eq!(variant_11.cells[0].node_label.as_deref(), Some("C"));
    }

    #[test]
    fn parse_stat_json_empty() {
        let catalog = parse_stat_json("{}").unwrap();
        assert!(catalog.maps.is_empty());
    }
}
