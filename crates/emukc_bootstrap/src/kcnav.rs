//! `KCNav` (`tsunkit.net/nav`) as a source of drops and enemy fleets.
//!
//! `KCNav` is the query front end of `TsunDB`'s crowd-sourced sortie records. Two steps, with the
//! raw responses on disk in between: [`sync_kcnav`] downloads, [`normalize_kcnav`] turns a
//! directory of raw responses into one document without touching the network, so the same
//! directory always normalizes to the same bytes.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Duration,
};

use emukc_model::codex::map::{EnemyComposition, MapCatalog, ShipDropDefinition};
use emukc_network::{client::new_reqwest_client, download::Request, reqwest};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{download::BootstrapDownloadError, parser::wikiwiki_map::EnemyNodeRows};

const KCNAV_API_ROOT: &str = "https://tsunkit.net/api/routing";
const KCNAV_ROOT: &str = "kcnav";
/// Says who is asking; the requests are never dressed up as a browser.
const KCNAV_USER_AGENT: &str =
    concat!("emukc-bootstrap/", env!("CARGO_PKG_VERSION"), " (+https://github.com/master-g/emukc)");
/// Paging parameters: `drops` returns every entry without them and would be cut short with them.
const KCNAV_QUERY_SKIPPED: [&str; 2] = ["page", "perPage"];

/// The server gives up on a query after 60 seconds itself; this only catches a dead connection.
const KCNAV_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

// ponytail: 1 keeps everything. Set real thresholds once the first full sync shows the
// distribution, and say so in the asset note.
const MIN_DROP_COUNT: i64 = 1;
const MIN_FLEET_COUNT: i64 = 1;

/// Where the raw responses live under `data_root`.
pub fn kcnav_dir(data_root: impl AsRef<Path>) -> PathBuf {
    data_root.as_ref().join(KCNAV_ROOT)
}

/// What a sync should fetch and how gently.
#[derive(Debug, Clone)]
pub struct KcnavSyncOptions {
    /// Limit the sync to these maps, such as `1-1`. Empty means every regular map.
    pub maps: BTreeSet<String>,
    /// Seconds to wait after every request. Values below 1 are raised to 1.
    pub interval_secs: u64,
}

/// What a sync did.
#[derive(Debug, Default, Clone, Copy)]
pub struct KcnavSyncStats {
    /// Responses downloaded.
    pub downloaded: usize,
    /// Files that were already there.
    pub skipped: usize,
    /// Requests that failed or answered with an error document.
    pub failed: usize,
}

/// The battle edges of every regular map, as map name to edge ids.
///
/// `KCNav` numbers edges the way the game numbers cells, so a cell number is an edge id.
/// Which edges end in a battle is read off our own catalog rather than `KCNav`'s event codes.
pub fn kcnav_battle_edges(catalog: &MapCatalog) -> BTreeMap<String, BTreeSet<i64>> {
    let mut edges = BTreeMap::<String, BTreeSet<i64>>::new();
    for map in catalog.maps.values() {
        if map.is_event || !(1..=7).contains(&map.maparea_id) {
            continue;
        }
        let name = format!("{}-{}", map.maparea_id, map.mapinfo_no);
        for variant in map.variants.values() {
            edges.entry(name.clone()).or_default().extend(
                variant
                    .cells
                    .iter()
                    // 4 battle, 5 boss, 7 aerial, 10 long-range air raid. An aerial
                    // reconnaissance cell is also 7; asking about it only returns nothing.
                    .filter(|cell| matches!(cell.event_id, 4 | 5 | 7 | 10))
                    .map(|cell| cell.cell_no),
            );
        }
    }
    edges
}

/// The query string the front end sends, built from the `paramDefaults` of the meta document.
///
/// Unset parameters are left out; the rest go in key order. `start` is unset by default, and
/// a busy edge then takes the server over a minute and ends in a 504 (1-1 A, measured
/// 2026-10-06; one year answers in 17 seconds). So the window is the year before `end` —
/// which also keeps drop tables that were changed long ago out of the counts.
pub fn kcnav_query(meta: &Value) -> Result<String, String> {
    let defaults = meta
        .pointer("/result/paramDefaults")
        .and_then(Value::as_object)
        .ok_or("the KCNav meta document has no result.paramDefaults")?;
    let mut pairs = defaults
        .iter()
        .filter(|(key, _)| !KCNAV_QUERY_SKIPPED.contains(&key.as_str()))
        .filter_map(|(key, value)| match value {
            Value::String(text) if !text.is_empty() => Some((key.as_str(), text.clone())),
            Value::Number(_) | Value::Bool(_) => Some((key.as_str(), value.to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !pairs.iter().any(|(key, _)| *key == "start") {
        let end = defaults.get("end").and_then(Value::as_str).unwrap_or_default();
        let start = end
            .split_once('-')
            .and_then(|(year, rest)| Some(format!("{}-{rest}", year.parse::<i64>().ok()? - 1)))
            .ok_or_else(|| format!("the KCNav meta document has no usable end date: `{end}`"))?;
        pairs.push(("start", start));
    }
    pairs.sort();
    Ok(pairs.into_iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join("&"))
}

/// The requests for one map: its route document, then two per battle edge.
fn map_jobs(dir: &Path, map: &str, edges: &BTreeSet<i64>, query: &str) -> Vec<(String, PathBuf)> {
    let base = format!("{KCNAV_API_ROOT}/maps/{map}");
    let dir = dir.join(map);
    let mut jobs = vec![(base.clone(), dir.join("map.json"))];
    for edge in edges {
        for kind in ["enemycomps", "drops"] {
            jobs.push((
                format!("{base}/edges/{edge}/{kind}?{query}"),
                dir.join(format!("edge_{edge}_{kind}.json")),
            ));
        }
    }
    jobs
}

/// `KCNav` answers some failures with `200` and `{"error": ...}`; such a body must not be kept
/// as if it were data.
fn check_response(raw: &str) -> Result<Value, String> {
    let value = serde_json::from_str::<Value>(raw).map_err(|err| err.to_string())?;
    if value.get("result").is_none() {
        let error = value.get("error").map_or_else(|| "no result".to_owned(), Value::to_string);
        return Err(error);
    }
    Ok(value)
}

/// Download one response unless it is already there. Returns its checked content and whether
/// a request was made.
#[expect(clippy::result_large_err)]
async fn fetch(
    client: &reqwest::Client,
    url: &str,
    save_as: &Path,
    interval: Duration,
) -> Result<(Value, bool), BootstrapDownloadError> {
    let bad = |err: String| BootstrapDownloadError::Generic(format!("{url}: {err}"));
    if save_as.exists() {
        let value = check_response(&std::fs::read_to_string(save_as)?).map_err(bad)?;
        return Ok((value, false));
    }
    let request = Request::builder()
        .url(url)
        .save_as(save_as)
        .overwrite(true)
        .skip_header_check(true)
        .build()?
        .execute(Some(client.clone()));
    // The download layer has no timeout of its own, and this server can sit on a query.
    let result = tokio::time::timeout(KCNAV_REQUEST_TIMEOUT, request).await;
    tokio::time::sleep(interval).await;
    result.map_err(|_| bad(format!("no answer within {KCNAV_REQUEST_TIMEOUT:?}")))??;
    match check_response(&std::fs::read_to_string(save_as)?) {
        Ok(value) => Ok((value, true)),
        Err(err) => {
            std::fs::remove_file(save_as)?;
            Err(bad(err))
        }
    }
}

/// Download the raw responses for `edges` into [`kcnav_dir`], one request at a time.
///
/// Files already on disk are kept, so an interrupted sync resumes where it stopped. A failed
/// request is counted and the sync moves on; the caller decides what a non-zero count means.
#[expect(clippy::result_large_err)]
pub async fn sync_kcnav(
    data_root: impl AsRef<Path>,
    edges: &BTreeMap<String, BTreeSet<i64>>,
    options: &KcnavSyncOptions,
    proxy: Option<&str>,
) -> Result<KcnavSyncStats, BootstrapDownloadError> {
    let dir = kcnav_dir(data_root);
    let interval = Duration::from_secs(options.interval_secs.max(1));
    let client = new_reqwest_client(proxy, Some(KCNAV_USER_AGENT)).map_err(|source| {
        BootstrapDownloadError::ReqwestClient {
            proxy: proxy.map(ToOwned::to_owned),
            source,
        }
    })?;

    // The meta document carries today's date as a default, so it is fetched once and reused:
    // a resumed sync asks the same questions as the one it continues.
    std::fs::create_dir_all(&dir)?;
    let meta_url = format!("{KCNAV_API_ROOT}/maps/all/meta");
    let (meta, _) = fetch(&client, &meta_url, &dir.join("meta.json"), interval).await?;
    let query = kcnav_query(&meta).map_err(BootstrapDownloadError::Generic)?;

    let mut stats = KcnavSyncStats::default();
    for (map, edges) in edges {
        if !options.maps.is_empty() && !options.maps.contains(map) {
            continue;
        }
        std::fs::create_dir_all(dir.join(map))?;
        for (url, save_as) in map_jobs(&dir, map, edges, &query) {
            match fetch(&client, &url, &save_as, interval).await {
                Ok((_, true)) => stats.downloaded += 1,
                Ok((_, false)) => stats.skipped += 1,
                Err(err) => {
                    warn!("kcnav: {err}");
                    stats.failed += 1;
                }
            }
        }
        info!("kcnav: {map} done ({stats:?})");
    }
    Ok(stats)
}

/// The normalized `KCNav` data, keyed by map name and node label.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KcnavCatalog {
    /// Where the data comes from and how it was filtered.
    pub note: String,
    /// Map name such as `1-1` to node label to what was observed there.
    pub maps: BTreeMap<String, BTreeMap<String, KcnavNode>>,
    /// Map name to edge id (our `cell_no`) to what kind of cell the edge ends in.
    #[serde(default)]
    pub cells: BTreeMap<String, BTreeMap<i64, KcnavCell>>,
}

/// What kind of cell an edge ends in, as recorded from the game's own responses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KcnavCell {
    /// Label of the node the edge ends in.
    pub label: String,
    /// `KCNav`'s colour code: `api_color_no`, except 90 (nothing happens) and 91 (the player
    /// picks the next node), which are its own.
    pub color_no: i64,
    /// `api_event_id`.
    pub event_id: i64,
}

/// What was observed at one node, summed over every edge that enters it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KcnavNode {
    /// Battles with a recorded drop outcome.
    pub drop_samples: i64,
    /// How many of them dropped nothing.
    pub no_drop: i64,
    /// The ships dropped, most frequent first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drops: Vec<KcnavDrop>,
    /// The enemy fleets met, most frequent first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fleets: Vec<KcnavFleet>,
}

/// One dropped ship.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KcnavDrop {
    /// Master id.
    pub ship_id: i64,
    /// Japanese name, for reading the asset.
    pub name: String,
    /// Times it dropped.
    pub weight: i64,
    /// The ranks it was seen dropping at, out of `S`, `A`, `B`.
    pub ranks: String,
}

/// One enemy fleet in one formation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KcnavFleet {
    /// Master ids, flagship first.
    pub ship_ids: Vec<i64>,
    /// Levels, in the same order.
    pub levels: Vec<i64>,
    /// `api_formation`.
    pub formation: i64,
    /// Times it was met.
    pub weight: i64,
    /// The escort fleet of an enemy combined fleet; empty for a single fleet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escort_ship_ids: Vec<i64>,
    /// Levels of the escort fleet, in the same order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escort_levels: Vec<i64>,
}

#[derive(Deserialize)]
struct RawDrop {
    id: i64,
    #[serde(default)]
    name: String,
    drops: i64,
    total: i64,
    min_s: Option<i64>,
    min_a: Option<i64>,
    min_b: Option<i64>,
}

#[derive(Deserialize)]
struct RawShip {
    id: i64,
    lvl: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFleet {
    main_fleet: Vec<RawShip>,
    #[serde(default)]
    escort_fleet: Vec<RawShip>,
    formation: i64,
    count: i64,
}

fn entries<T: serde::de::DeserializeOwned>(value: &Value) -> Result<Vec<T>, String> {
    let entries = value.pointer("/result/entries").cloned().ok_or("no result.entries")?;
    serde_json::from_value(entries).map_err(|err| err.to_string())
}

fn add_drops(node: &mut KcnavNode, value: &Value) -> Result<(), String> {
    let raw = entries::<RawDrop>(value)?;
    if let Some(count) = value.pointer("/result/count").and_then(Value::as_i64)
        && count != raw.len() as i64
    {
        return Err(format!("{count} drop entries announced, {} present", raw.len()));
    }
    node.drop_samples += raw.first().map_or(0, |entry| entry.total);
    for entry in raw {
        if entry.id < 0 {
            node.no_drop += entry.drops;
            continue;
        }
        let ranks = [("S", entry.min_s), ("A", entry.min_a), ("B", entry.min_b)];
        let seen = match node.drops.iter_mut().find(|drop| drop.ship_id == entry.id) {
            Some(seen) => seen,
            None => {
                node.drops.push(KcnavDrop {
                    ship_id: entry.id,
                    name: entry.name,
                    ..Default::default()
                });
                node.drops.last_mut().expect("just pushed")
            }
        };
        seen.weight += entry.drops;
        let ranks = ranks.map(|(rank, min)| (rank, min.is_some() || seen.ranks.contains(rank)));
        seen.ranks = ranks.iter().filter(|(_, seen)| *seen).map(|(rank, _)| *rank).collect();
    }
    Ok(())
}

fn add_fleets(node: &mut KcnavNode, value: &Value) -> Result<(), String> {
    for entry in entries::<RawFleet>(value)? {
        // The same fleet comes back once per equipment numbering (500 and 1500 based).
        let ids = |fleet: &[RawShip]| fleet.iter().map(|ship| ship.id).collect::<Vec<_>>();
        let levels = |fleet: &[RawShip]| fleet.iter().map(|ship| ship.lvl).collect::<Vec<_>>();
        let (ship_ids, escort_ship_ids) = (ids(&entry.main_fleet), ids(&entry.escort_fleet));
        match node.fleets.iter_mut().find(|fleet| {
            fleet.ship_ids == ship_ids
                && fleet.escort_ship_ids == escort_ship_ids
                && fleet.formation == entry.formation
        }) {
            Some(fleet) => fleet.weight += entry.count,
            None => node.fleets.push(KcnavFleet {
                levels: levels(&entry.main_fleet),
                ship_ids,
                formation: entry.formation,
                weight: entry.count,
                escort_levels: levels(&entry.escort_fleet),
                escort_ship_ids,
            }),
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    check_response(&raw).map_err(|err| format!("{}: {err}", path.display()))
}

/// Turn a directory written by [`sync_kcnav`] into one catalog.
///
/// Reads nothing but that directory. An edge file that names an edge the map's route does not
/// have, or a truncated drop list, is an error rather than a gap.
pub fn normalize_kcnav(dir: impl AsRef<Path>) -> Result<KcnavCatalog, String> {
    let dir = dir.as_ref();
    let list = |dir: &Path| -> Result<Vec<PathBuf>, String> {
        let mut paths = std::fs::read_dir(dir)
            .map_err(|err| format!("{}: {err}", dir.display()))?
            .map(|entry| entry.map(|entry| entry.path()).map_err(|err| err.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        Ok(paths)
    };

    let mut maps = BTreeMap::new();
    let mut cells = BTreeMap::new();
    for map_dir in list(dir)?.into_iter().filter(|path| path.is_dir()) {
        let map = map_dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let route = read_json(&map_dir.join("map.json"))?;
        cells.insert(map.clone(), route_cells(&route).map_err(|err| format!("{map}: {err}"))?);
        let mut nodes = BTreeMap::<String, KcnavNode>::new();
        for path in list(&map_dir)? {
            let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let Some((edge, kind)) =
                name.strip_prefix("edge_").and_then(|rest| rest.split_once('_'))
            else {
                continue;
            };
            let label = route
                .pointer(&format!("/result/route/{edge}/1"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                format!("{}: edge {edge} is not in the route", path.display())
            })?;
            let node = nodes.entry(label.to_owned()).or_default();
            let value = read_json(&path)?;
            match kind {
                "drops" => add_drops(node, &value),
                "enemycomps" => add_fleets(node, &value),
                _ => continue,
            }
            .map_err(|err| format!("{}: {err}", path.display()))?;
        }
        for node in nodes.values_mut() {
            node.drops.retain(|drop| drop.weight >= MIN_DROP_COUNT);
            node.fleets.retain(|fleet| fleet.weight >= MIN_FLEET_COUNT);
            node.drops.sort_by_key(|drop| (-drop.weight, drop.ship_id));
            node.fleets.sort_by(|a, b| {
                (-a.weight, &a.ship_ids, a.formation).cmp(&(-b.weight, &b.ship_ids, b.formation))
            });
        }
        nodes.retain(|_, node| node.drop_samples > 0 || !node.fleets.is_empty());
        maps.insert(map, nodes);
    }

    Ok(KcnavCatalog {
        note: format!(
            "Drops and enemy fleets observed by TsunDB contributors, queried through KCNav \
             (tsunkit.net/nav) and summed per node. Weights are observation counts. Drops seen \
             fewer than {MIN_DROP_COUNT} times and fleets met fewer than {MIN_FLEET_COUNT} times \
             are left out. Generated by `kcnav normalize`; do not edit."
        ),
        maps,
        cells,
    })
}

/// The cell kinds a route document records. Edges nobody has travelled (event −1) are left out.
fn route_cells(route: &Value) -> Result<BTreeMap<i64, KcnavCell>, String> {
    let edges = route.pointer("/result/route").and_then(Value::as_object).ok_or("no route")?;
    let mut cells = BTreeMap::new();
    for (edge, entry) in edges {
        let bad = || format!("edge {edge} is not [from, to, colour, event]");
        let edge_id = edge.parse::<i64>().map_err(|_| bad())?;
        let (Some(label), Some(color_no), Some(event_id)) = (
            entry.get(1).and_then(Value::as_str),
            entry.get(2).and_then(Value::as_i64),
            entry.get(3).and_then(Value::as_i64),
        ) else {
            return Err(bad());
        };
        if event_id < 0 {
            continue;
        }
        // An edge from nowhere leads into a start point, which our catalog labels `Start`.
        let label = if entry.get(0).is_some_and(Value::is_null) {
            "Start"
        } else {
            label
        };
        cells.insert(
            edge_id,
            KcnavCell {
                label: label.to_owned(),
                color_no,
                event_id,
            },
        );
    }
    Ok(cells)
}

/// The repo-tracked cell kind asset: map name, then edge id (our `cell_no`).
#[derive(Debug, Serialize, Deserialize)]
pub struct KcnavCellEventsAsset {
    /// Where the file comes from.
    pub note: String,
    /// The cells of each map.
    pub maps: BTreeMap<String, BTreeMap<i64, KcnavCell>>,
}

/// Where the repo-tracked cell kind asset lives.
pub fn repo_kcnav_cell_events_path() -> PathBuf {
    crate::assets::KCNAV_CELL_EVENTS.path()
}

/// Lay the recorded cell kinds out as the cell kind asset.
pub fn kcnav_cell_events(kcnav: &KcnavCatalog) -> KcnavCellEventsAsset {
    KcnavCellEventsAsset {
        note: "What each cell of the regular maps is, as recorded by TsunDB contributors and \
               read from KCNav's route documents: map name, then edge id, which is our cell_no. \
               color_no is api_color_no except 90 (nothing happens) and 91 (the player picks \
               the next node). Generated by `kcnav normalize`; do not edit."
            .to_owned(),
        maps: kcnav.cells.clone(),
    }
}

/// The repo-tracked cell kind asset.
pub fn load_repo_kcnav_cell_events()
-> Result<KcnavCellEventsAsset, crate::parser::error::ParseError> {
    use crate::parser::error::ParseError;
    let path = repo_kcnav_cell_events_path();
    let (_, raw) = crate::assets::KCNAV_CELL_EVENTS
        .load()
        .map_err(|source| ParseError::io_at(&path, source))?;
    serde_json::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))
}

/// Correct the cell kinds the topology source could only guess.
///
/// That source knows whether a node has a name, not what happens on it, so every named node
/// became a battle. Only the plain kinds are corrected — nothing, battle, boss, the "nothing
/// happened" message, a resource pick-up, a start point; the catalog's own codes for air
/// raids, night battles and reconnaissance are left alone, and so are the kinds nothing here
/// plays out (8, 9). Returns how many cells changed, or the cells whose label disagrees.
pub fn apply_cell_events(
    catalog: &mut MapCatalog,
    events: &KcnavCellEventsAsset,
) -> Result<usize, Vec<String>> {
    let mut changed = 0;
    let mut errors = Vec::new();
    for map in catalog.maps.values_mut() {
        let name = format!("{}-{}", map.maparea_id, map.mapinfo_no);
        let Some(recorded) = events.maps.get(&name) else {
            continue;
        };
        for variant in map.variants.values_mut() {
            for cell in &mut variant.cells {
                let Some(seen) = recorded.get(&cell.cell_no) else {
                    continue;
                };
                if cell.node_label.as_deref() != Some(seen.label.as_str()) {
                    errors.push(format!(
                        "{name} cell {}: labelled {:?} here, {} in the recorded route",
                        cell.cell_no, cell.node_label, seen.label
                    ));
                    continue;
                }
                if cell.event_id == seen.event_id || !matches!(cell.event_id, 1 | 4 | 5 | 6) {
                    continue;
                }
                let colour = |fallback| {
                    if seen.color_no < 90 {
                        seen.color_no
                    } else {
                        fallback
                    }
                };
                let kind = match seen.event_id {
                    // A start point keeps its colour: a real capture has 6-5's drawn as 4.
                    0 => (cell.color_no, 0, 0),
                    2 => (colour(2), 2, 0),
                    4 => (4, 4, 1),
                    5 => (5, 5, 1),
                    // The message kind: 2 where the player picks the next node, else 1.
                    6 => (
                        cell.color_no,
                        6,
                        if seen.color_no == 91 {
                            2
                        } else {
                            1
                        },
                    ),
                    _ => continue,
                };
                (cell.color_no, cell.event_id, cell.event_kind) = kind;
                changed += 1;
            }
            // The topology source flags a wrong boss on a few maps. Where the flagged cell
            // turned out not to be one, the boss is the last cell that is.
            let is_boss = |cell_no| {
                variant.cells.iter().any(|cell| cell.cell_no == cell_no && cell.event_id == 5)
            };
            if !is_boss(variant.boss_cell_no)
                && let Some(boss) = variant.cells.iter().rev().find(|cell| cell.event_id == 5)
            {
                variant.boss_cell_no = boss.cell_no;
            }
        }
    }
    if errors.is_empty() {
        Ok(changed)
    } else {
        Err(errors)
    }
}

/// The repo-tracked ship drop asset: map id, variant key, node label, then the outcomes.
#[derive(Debug, Serialize, Deserialize)]
pub struct MapShipDropsAsset {
    /// Why the file exists and where it comes from.
    pub note: String,
    /// The drops; `ship_id` 0 is the outcome "nothing drops".
    pub maps: BTreeMap<i64, BTreeMap<String, BTreeMap<String, Vec<ShipDropDefinition>>>>,
}

/// Where the repo-tracked ship drop asset lives.
pub fn repo_map_ship_drops_path() -> PathBuf {
    crate::assets::MAP_SHIP_DROPS.path()
}

/// The hand-maintained list of limited-time drops: map name, node label, then the ships.
#[derive(Debug, Default, Deserialize)]
pub struct LimitedDrops {
    maps: BTreeMap<String, BTreeMap<String, Vec<LimitedDrop>>>,
}

#[derive(Debug, Deserialize)]
struct LimitedDrop {
    ship_id: i64,
}

impl LimitedDrops {
    /// The list checked into the repository.
    pub fn load_repo() -> Result<Self, String> {
        let (_, raw) = crate::assets::MAP_LIMITED_DROPS.load().map_err(|err| err.to_string())?;
        serde_json::from_str(&raw).map_err(|err| err.to_string())
    }

    fn contains(&self, map: &str, label: &str, ship_id: i64) -> bool {
        self.maps
            .get(map)
            .and_then(|nodes| nodes.get(label))
            .is_some_and(|ships| ships.iter().any(|ship| ship.ship_id == ship_id))
    }
}

/// Lay the observed drops out as the ship drop asset.
///
/// The observations cannot tell a limited-time drop from a regular one — a campaign's drops
/// are simply counted while it runs. Entries on the `limited` list are tagged so, and a
/// tagged entry never drops.
pub fn kcnav_ship_drops(
    kcnav: &KcnavCatalog,
    catalog: &MapCatalog,
    limited: &LimitedDrops,
) -> MapShipDropsAsset {
    let mut maps = BTreeMap::new();
    for map in catalog.maps.values() {
        let name = format!("{}-{}", map.maparea_id, map.mapinfo_no);
        let Some(nodes) = kcnav.maps.get(&name) else {
            continue;
        };
        let mut variants = BTreeMap::new();
        for (key, variant) in &map.variants {
            let labels = variant
                .cells
                .iter()
                .filter_map(|cell| cell.node_label.as_deref())
                .collect::<BTreeSet<_>>();
            let drops = nodes
                .iter()
                .filter(|(label, node)| labels.contains(label.as_str()) && !node.drops.is_empty())
                .map(|(label, node)| {
                    let nothing = ShipDropDefinition {
                        weight: node.no_drop,
                        ..Default::default()
                    };
                    let ships = node.drops.iter().map(|drop| ShipDropDefinition {
                        ship_id: drop.ship_id,
                        raw_ship_name: drop.name.clone(),
                        weight: drop.weight,
                        ranks: drop.ranks.clone(),
                        tags: if limited.contains(&name, label, drop.ship_id) {
                            vec!["limited".to_owned()]
                        } else {
                            Vec::new()
                        },
                    });
                    let outcomes = (node.no_drop > 0).then_some(nothing).into_iter().chain(ships);
                    (label.clone(), outcomes.collect())
                })
                .collect::<BTreeMap<_, Vec<_>>>();
            variants.insert(key.clone(), drops);
        }
        maps.insert(map.map_id, variants);
    }
    MapShipDropsAsset {
        note: format!(
            "Ship drops of the regular maps, keyed by map id, variant key and node label. {} An              entry with ship_id 0 is the outcome \"nothing drops\"; ranks lists the win ranks an              outcome was seen at. Entries listed in map_limited_drops.json are tagged limited and \
             never drop.",
            kcnav.note
        ),
        maps,
    }
}

/// The repo-tracked enemy fleet asset: map id, variant key, node label, then the fleets met.
#[derive(Debug, Serialize, Deserialize)]
pub struct KcnavEnemyFleetsAsset {
    /// Where the file comes from.
    pub note: String,
    /// The fleets of each node.
    pub maps: BTreeMap<i64, BTreeMap<String, BTreeMap<String, EnemyNodeRows>>>,
}

/// Where the repo-tracked enemy fleet asset lives.
pub fn repo_kcnav_enemy_fleets_path() -> PathBuf {
    crate::assets::KCNAV_ENEMY_FLEETS.path()
}

/// Lay the observed enemy fleets out as the enemy fleet asset.
///
/// A fleet naming a ship `known_ship` rejects is left out: it could not be fielded.
pub fn kcnav_enemy_fleets(
    kcnav: &KcnavCatalog,
    catalog: &MapCatalog,
    known_ship: impl Fn(i64) -> bool,
) -> KcnavEnemyFleetsAsset {
    let mut maps = BTreeMap::new();
    for map in catalog.maps.values() {
        let Some(nodes) = kcnav.maps.get(&format!("{}-{}", map.maparea_id, map.mapinfo_no)) else {
            continue;
        };
        let mut variants = BTreeMap::new();
        for (key, variant) in &map.variants {
            let mut rows = BTreeMap::new();
            for (label, node) in nodes {
                let mut cells =
                    variant.cells.iter().filter(|cell| cell.node_label.as_deref() == Some(label));
                let compositions = node
                    .fleets
                    .iter()
                    .filter(|fleet| {
                        fleet
                            .ship_ids
                            .iter()
                            .chain(&fleet.escort_ship_ids)
                            .all(|id| known_ship(*id))
                    })
                    // ponytail: combined fleets are held back until `ec_battle` can field them
                    // (plan 2026-10-06-006 U3); drop this filter then.
                    .filter(|fleet| fleet.escort_ship_ids.is_empty())
                    .enumerate()
                    .map(|(index, fleet)| EnemyComposition {
                        comp_id: format!("kcnav:{index}"),
                        weight: fleet.weight,
                        ship_ids: fleet.ship_ids.clone(),
                        formation: Some(fleet.formation),
                        levels: fleet.levels.clone(),
                        escort_ship_ids: fleet.escort_ship_ids.clone(),
                        escort_levels: fleet.escort_levels.clone(),
                        ..Default::default()
                    })
                    .collect::<Vec<_>>();
                let Some(first) = cells.next() else {
                    continue;
                };
                if compositions.is_empty() {
                    continue;
                }
                // From the recorded route, not from our catalog: the catalog's own flag is
                // what `apply_cell_events` corrects.
                let name = format!("{}-{}", map.maparea_id, map.mapinfo_no);
                let recorded = kcnav.cells.get(&name);
                let is_boss = std::iter::once(first).chain(cells).any(|cell| {
                    recorded
                        .and_then(|cells| cells.get(&cell.cell_no))
                        .is_some_and(|seen| seen.event_id == 5)
                });
                rows.insert(
                    label.clone(),
                    EnemyNodeRows {
                        is_boss,
                        compositions,
                    },
                );
            }
            variants.insert(key.clone(), rows);
        }
        maps.insert(map.map_id, variants);
    }
    KcnavEnemyFleetsAsset {
        note: format!(
            "Enemy fleets of the regular maps, keyed by map id, variant key and node label. {}",
            kcnav.note
        ),
        maps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/kcnav");

    #[test]
    fn kcnav_query_keeps_set_parameters_in_key_order() {
        let meta = serde_json::json!({"result": {"paramDefaults": {
            "retreats": true, "minGauge": 1, "start": null, "mainComp": "",
            "end": "2026-10-07", "page": 0, "perPage": 20, "scale": 1.0
        }}});
        assert_eq!(
            kcnav_query(&meta).unwrap(),
            "end=2026-10-07&minGauge=1&retreats=true&scale=1.0&start=2025-10-07"
        );
        assert!(kcnav_query(&serde_json::json!({"result": {}})).is_err());
        assert!(kcnav_query(&serde_json::json!({"result": {"paramDefaults": {}}})).is_err());
    }

    #[test]
    fn kcnav_jobs_ask_for_the_route_then_both_kinds_per_edge() {
        let jobs = map_jobs(Path::new("raw"), "1-1", &BTreeSet::from([2, 3]), "a=1");
        let urls = jobs.iter().map(|(url, _)| url.as_str()).collect::<Vec<_>>();
        assert_eq!(urls.len(), 5);
        assert_eq!(urls[0], "https://tsunkit.net/api/routing/maps/1-1");
        assert_eq!(urls[2], "https://tsunkit.net/api/routing/maps/1-1/edges/2/drops?a=1");
        assert_eq!(jobs[2].1, Path::new("raw/1-1/edge_2_drops.json"));
    }

    #[test]
    fn kcnav_error_documents_are_not_data() {
        assert!(check_response(r#"{"result": {"entries": []}}"#).is_ok());
        assert!(check_response(r#"{"error": "Not found"}"#).unwrap_err().contains("Not found"));
        assert!(check_response("<html>").is_err());
    }

    /// A file already on disk is returned without a request: the client here has no route to
    /// anywhere, so reaching for the network would fail the test.
    #[tokio::test]
    async fn kcnav_fetch_skips_what_is_already_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.json");
        std::fs::write(&path, r#"{"result": {}}"#).unwrap();
        let client = new_reqwest_client(Some("http://127.0.0.1:1"), None).unwrap();
        let (_, requested) =
            fetch(&client, "http://127.0.0.1:1/x", &path, Duration::ZERO).await.unwrap();
        assert!(!requested);
    }

    #[test]
    fn kcnav_normalize_sums_drops_and_fleets_per_node() {
        let catalog = normalize_kcnav(FIXTURES).unwrap();
        let node = &catalog.maps["1-1"]["B"];

        // Edge 2 alone has 561585 samples; the fixture's edge 9 is a copy of it entering
        // the same node, so everything doubles.
        assert_eq!(node.drop_samples, 2 * 561_585);
        assert_eq!(node.no_drop, 2 * 181_202);
        assert!(node.drops.iter().all(|drop| drop.ship_id > 0));
        assert_eq!(node.drops[0].name, "敷波");
        assert_eq!(node.drops[0].weight, 2 * 17_373);
        assert_eq!(node.drops[0].ranks, "SA");
        assert!(node.drops.iter().all(|drop| ["S", "SA", "SAB", "A", "AB", "B", "SB"].contains(&drop.ranks.as_str())));
        assert_eq!(
            node.no_drop + node.drops.iter().map(|drop| drop.weight).sum::<i64>(),
            node.drop_samples
        );

        // Six records, two per fleet: one for each equipment numbering.
        let fleets = node
            .fleets
            .iter()
            .map(|fleet| (fleet.ship_ids.clone(), fleet.weight))
            .collect::<Vec<_>>();
        assert_eq!(
            fleets,
            vec![
                (vec![1501, 1501], 311_279 + 522_715),
                (vec![1502, 1502], 273_191 + 456_212),
                (vec![1503, 1503], 194_947 + 326_160),
            ]
        );
        assert!(node.fleets.iter().all(|fleet| fleet.levels == [1, 1] && fleet.formation == 1));
    }

    #[test]
    fn kcnav_normalize_is_deterministic() {
        let once = serde_json::to_string(&normalize_kcnav(FIXTURES).unwrap()).unwrap();
        let twice = serde_json::to_string(&normalize_kcnav(FIXTURES).unwrap()).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn kcnav_normalize_rejects_an_edge_outside_the_route() {
        let dir = tempfile::tempdir().unwrap();
        let map = dir.path().join("1-1");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::copy(format!("{FIXTURES}/1-1/map.json"), map.join("map.json")).unwrap();
        std::fs::copy(format!("{FIXTURES}/1-1/edge_2_drops.json"), map.join("edge_77_drops.json"))
            .unwrap();
        assert!(normalize_kcnav(dir.path()).unwrap_err().contains("edge 77"));
    }

    #[test]
    fn kcnav_ship_drops_tag_the_listed_limited_drops() {
        use emukc_model::codex::map::{MapCellDefinition, MapDefinition, MapVariantDefinition};

        let variant = MapVariantDefinition {
            cells: vec![MapCellDefinition {
                cell_no: 2,
                node_label: Some("B".to_owned()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut catalog = MapCatalog::default();
        catalog.maps.insert(
            11,
            MapDefinition {
                map_id: 11,
                maparea_id: 1,
                mapinfo_no: 1,
                variants: BTreeMap::from([(String::new(), variant)]),
                ..Default::default()
            },
        );
        let limited: LimitedDrops =
            serde_json::from_str(r#"{"maps": {"1-1": {"B": [{"ship_id": 14}]}}}"#).unwrap();

        let asset = kcnav_ship_drops(&normalize_kcnav(FIXTURES).unwrap(), &catalog, &limited);
        let drops = &asset.maps[&11][""]["B"];

        assert_eq!((drops[0].ship_id, drops[0].weight), (0, 2 * 181_202));
        let tagged = drops.iter().filter(|drop| !drop.tags.is_empty()).collect::<Vec<_>>();
        assert_eq!(tagged.len(), 1);
        assert_eq!((tagged[0].ship_id, tagged[0].tags[0].as_str()), (14, "limited"));
    }

    fn catalog_with(cells: Vec<(i64, &str, (i64, i64, i64))>, boss_cell_no: i64) -> MapCatalog {
        use emukc_model::codex::map::{MapCellDefinition, MapDefinition, MapVariantDefinition};

        let variant = MapVariantDefinition {
            boss_cell_no,
            cells: cells
                .into_iter()
                .map(|(cell_no, label, (color_no, event_id, event_kind))| MapCellDefinition {
                    cell_no,
                    color_no,
                    event_id,
                    event_kind,
                    node_label: Some(label.to_owned()),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let mut catalog = MapCatalog::default();
        catalog.maps.insert(
            11,
            MapDefinition {
                map_id: 11,
                maparea_id: 1,
                mapinfo_no: 1,
                variants: BTreeMap::from([(String::new(), variant)]),
                ..Default::default()
            },
        );
        catalog
    }

    fn events(cells: &[(i64, &str, i64, i64)]) -> KcnavCellEventsAsset {
        let cells = cells
            .iter()
            .map(|&(edge, label, color_no, event_id)| {
                (
                    edge,
                    KcnavCell {
                        label: label.to_owned(),
                        color_no,
                        event_id,
                    },
                )
            })
            .collect();
        KcnavCellEventsAsset {
            note: String::new(),
            maps: BTreeMap::from([("1-1".to_owned(), cells)]),
        }
    }

    #[test]
    fn kcnav_route_cells_come_from_the_route_document() {
        let route = read_json(Path::new(&format!("{FIXTURES}/1-1/map.json"))).unwrap();
        let cells = route_cells(&route).unwrap();
        let kinds = cells
            .iter()
            .map(|(edge, cell)| (*edge, cell.label.as_str(), cell.event_id))
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![(0, "Start", 0), (1, "A", 4), (2, "B", 4), (3, "C", 5), (9, "B", 4)]
        );
    }

    #[test]
    fn cell_events_correct_the_guessed_kinds() {
        let battle = (4, 4, 1);
        let mut catalog = catalog_with(
            vec![
                (1, "A", battle),      // nothing happens there
                (2, "B", battle),      // the player picks the next node
                (3, "C", battle),      // a boss the topology source missed
                (4, "D", (5, 5, 1)),   // flagged boss, really a resource pick-up
                (5, "E", (10, 10, 1)), // the catalog's own code for an air raid: left alone
                (6, "F", battle),      // nobody recorded this edge: left alone
                (7, "G", (6, 1, 0)),   // a kind nothing here plays out: left alone
            ],
            4,
        );
        let recorded = events(&[
            (1, "A", 90, 6),
            (2, "B", 91, 6),
            (3, "C", 5, 5),
            (4, "D", 2, 2),
            (5, "E", 10, 4),
            (7, "G", 8, 8),
        ]);

        assert_eq!(apply_cell_events(&mut catalog, &recorded), Ok(4));

        let variant = &catalog.maps[&11].variants[""];
        let kinds = variant
            .cells
            .iter()
            .map(|cell| (cell.color_no, cell.event_id, cell.event_kind))
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![(4, 6, 1), (4, 6, 2), (5, 5, 1), (2, 2, 0), (10, 10, 1), battle, (6, 1, 0)]
        );
        assert_eq!(
            variant.boss_cell_no, 3,
            "the flagged boss was not one; the real one takes over"
        );
    }

    #[test]
    fn cell_events_reject_a_route_that_names_another_node() {
        let mut catalog = catalog_with(vec![(1, "A", (4, 4, 1))], 1);
        let errors = apply_cell_events(&mut catalog, &events(&[(1, "Z", 90, 6)])).unwrap_err();
        assert!(errors[0].contains("1-1 cell 1"), "{errors:?}");
    }
}
