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

use emukc_model::codex::map::MapCatalog;
use emukc_network::{client::new_reqwest_client, download::Request, reqwest};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::download::BootstrapDownloadError;

const KCNAV_API_ROOT: &str = "https://tsunkit.net/api/routing";
const KCNAV_ROOT: &str = "kcnav";
/// Says who is asking; the requests are never dressed up as a browser.
const KCNAV_USER_AGENT: &str =
    concat!("emukc-bootstrap/", env!("CARGO_PKG_VERSION"), " (+https://github.com/master-g/emukc)");
/// Paging parameters: `drops` returns every entry without them and would be cut short with them.
const KCNAV_QUERY_SKIPPED: [&str; 2] = ["page", "perPage"];

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
/// `drops` times out without it. Unset parameters are left out; the rest go in key order.
pub fn kcnav_query(meta: &Value) -> Result<String, String> {
    let defaults = meta
        .pointer("/result/paramDefaults")
        .and_then(Value::as_object)
        .ok_or("the KCNav meta document has no result.paramDefaults")?;
    let mut pairs = defaults
        .iter()
        .filter(|(key, _)| !KCNAV_QUERY_SKIPPED.contains(&key.as_str()))
        .filter_map(|(key, value)| match value {
            Value::String(text) if !text.is_empty() => Some((key, text.clone())),
            Value::Number(_) | Value::Bool(_) => Some((key, value.to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
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
    let result = Request::builder()
        .url(url)
        .save_as(save_as)
        .overwrite(true)
        .skip_header_check(true)
        .build()?
        .execute(Some(client.clone()))
        .await;
    tokio::time::sleep(interval).await;
    result?;
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
        for (rank, min) in ranks {
            if min.is_some() && !seen.ranks.contains(rank) {
                seen.ranks.push_str(rank);
            }
        }
    }
    Ok(())
}

fn add_fleets(node: &mut KcnavNode, value: &Value) -> Result<(), String> {
    for entry in entries::<RawFleet>(value)? {
        // Combined enemy fleets stay on disk until something can field them.
        if !entry.escort_fleet.is_empty() {
            continue;
        }
        // The same fleet comes back once per equipment numbering (500 and 1500 based).
        let ship_ids = entry.main_fleet.iter().map(|ship| ship.id).collect::<Vec<_>>();
        match node
            .fleets
            .iter_mut()
            .find(|fleet| fleet.ship_ids == ship_ids && fleet.formation == entry.formation)
        {
            Some(fleet) => fleet.weight += entry.count,
            None => node.fleets.push(KcnavFleet {
                levels: entry.main_fleet.iter().map(|ship| ship.lvl).collect(),
                ship_ids,
                formation: entry.formation,
                weight: entry.count,
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
    for map_dir in list(dir)?.into_iter().filter(|path| path.is_dir()) {
        let map = map_dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let route = read_json(&map_dir.join("map.json"))?;
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
    })
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
            "end=2026-10-07&minGauge=1&retreats=true&scale=1.0"
        );
        assert!(kcnav_query(&serde_json::json!({"result": {}})).is_err());
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
}
