//! Manual Tsunkit map download helpers for offline data generation workflows.

use std::{
	collections::{BTreeMap, BTreeSet},
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use emukc_network::{client::new_reqwest_client, download::Request, reqwest};
use futures::StreamExt;
use serde::Deserialize;
use tokio::time::timeout;

use crate::download::BootstrapDownloadError;

const TSUNKIT_NAV_ROOT: &str = "tsunkit_nav";
const TSUNKIT_MAP_META_URL: &str = "https://tsunkit.net/api/routing/maps/all/meta";
const TSUNKIT_MAPS_URL_ROOT: &str = "https://tsunkit.net/api/routing/maps";
const TSUNKIT_META_TIMEOUT_SECS: u64 = 15;
const TSUNKIT_MAP_TIMEOUT_SECS: u64 = 20;
const TSUNKIT_NODESUMMARY_TIMEOUT_SECS: u64 = 20;
const TSUNKIT_ENEMYCOMP_TIMEOUT_SECS: u64 = 15;
const TSUNKIT_META_RETRIES: usize = 2;
const TSUNKIT_MAP_RETRIES: usize = 2;
const TSUNKIT_NODESUMMARY_RETRIES: usize = 1;
const TSUNKIT_ENEMYCOMP_RETRIES: usize = 1;

#[derive(Debug, Default, Clone, Copy)]
/// Download counts collected during a manual Tsunkit sync.
pub struct TsunkitNavDownloadStats {
	/// Number of map payloads downloaded successfully.
	pub maps: usize,
	/// Number of node summary payloads downloaded successfully.
	pub nodesummaries: usize,
	/// Number of enemy composition payloads downloaded successfully.
	pub enemycomps: usize,
	/// Number of requests skipped due to timeout or other fetch errors.
	pub failures: usize,
}

#[derive(Debug, Clone)]
/// Controls which Tsunkit artifact groups are fetched during a manual sync.
pub struct TsunkitNavDownloadOptions {
	/// Maximum parallel request count used for the selected artifact groups.
	pub concurrent: Option<usize>,
	/// Optional map name allow-list, such as `1-1` or `57-3`.
	pub map_filter: Option<BTreeSet<String>>,
	/// Whether to fetch `/nodesummary` payloads.
	pub fetch_nodesummaries: bool,
	/// Whether to fetch `/enemycomps` payloads.
	pub fetch_enemycomps: bool,
}

impl Default for TsunkitNavDownloadOptions {
	fn default() -> Self {
		Self {
			concurrent: Some(2),
			map_filter: None,
			fetch_nodesummaries: true,
			fetch_enemycomps: true,
		}
	}
}

#[derive(Debug, Deserialize)]
struct TsunkitMapMetaResponse {
	result: TsunkitMapMetaResult,
}

#[derive(Debug, Deserialize)]
struct TsunkitMapMetaResult {
	maps: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapResponse {
	result: TsunkitMapResult,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapResult {
	#[serde(default)]
	spots: BTreeMap<String, (i64, i64, Option<String>)>,
	#[serde(default, rename = "mapSet")]
	map_set: TsunkitMapSet,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapSet {
	#[serde(default)]
	map: TsunkitMapLayer,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapLayer {
	#[serde(default)]
	spots: Vec<TsunkitRenderedSpot>,
	#[serde(default)]
	enemies: Vec<TsunkitRenderedEnemyMarker>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitRenderedSpot {
	x: i64,
	y: i64,
	#[serde(default)]
	no: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitRenderedEnemyMarker {
	no: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitNodeSummaryResponse {
	#[serde(default)]
	result: BTreeMap<String, TsunkitNodeSummaryEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitNodeSummaryEntry {
	#[serde(default)]
	battles: i64,
}

/// Download Tsunkit map artifacts into `<dir>/tsunkit_nav`.
///
/// This is intended for manual tooling and examples, not the default bootstrap flow.
pub async fn download_tsunkit_nav(
	dir: impl AsRef<Path>,
	overwrite: bool,
	proxy: Option<&str>,
	concurrent: Option<usize>,
) -> Result<TsunkitNavDownloadStats, BootstrapDownloadError> {
	download_tsunkit_nav_with_options(
		dir,
		overwrite,
		proxy,
		TsunkitNavDownloadOptions {
			concurrent,
			..TsunkitNavDownloadOptions::default()
		},
	)
	.await
}

/// Download Tsunkit artifacts into `<dir>/tsunkit_nav` using explicit fetch options.
pub async fn download_tsunkit_nav_with_options(
	dir: impl AsRef<Path>,
	overwrite: bool,
	proxy: Option<&str>,
	options: TsunkitNavDownloadOptions,
) -> Result<TsunkitNavDownloadStats, BootstrapDownloadError> {
	let root = dir.as_ref().join(TSUNKIT_NAV_ROOT);
	let maps_dir = root.join("maps");
	let nodesummary_dir = root.join("nodesummary");
	let enemycomps_dir = root.join("enemycomps");

	for path in [&root, &maps_dir, &nodesummary_dir, &enemycomps_dir] {
		if !path.exists() {
			std::fs::create_dir_all(path)?;
		}
	}

	let client = Arc::new(new_reqwest_client(proxy, None).map_err(|source| {
		BootstrapDownloadError::ReqwestClient {
			proxy: proxy.map(ToOwned::to_owned),
			source,
		}
	})?);

	let meta_path = root.join("meta.json");
	download_tsunkit_file(
		client.clone(),
		TSUNKIT_MAP_META_URL,
		&meta_path,
		overwrite,
		TSUNKIT_META_TIMEOUT_SECS,
		TSUNKIT_META_RETRIES,
	)
	.await?;

	let meta = read_json::<TsunkitMapMetaResponse>(&meta_path)?;
	let mut map_names = meta.result.maps.keys().cloned().collect::<Vec<_>>();
	map_names.sort();
	if let Some(map_filter) = &options.map_filter {
		map_names.retain(|map_name| map_filter.contains(map_name));
	}

	let enemycomp_concurrency = options.concurrent.unwrap_or(2).clamp(1, 2);
	let mut stats = TsunkitNavDownloadStats::default();

	for map_name in map_names {
		let map_path = maps_dir.join(format!("{map_name}.json"));
		if let Err(error) = download_tsunkit_file(
			client.clone(),
			&format!("{TSUNKIT_MAPS_URL_ROOT}/{map_name}"),
			&map_path,
			overwrite,
			TSUNKIT_MAP_TIMEOUT_SECS,
			TSUNKIT_MAP_RETRIES,
		)
		.await
		{
			stats.failures += 1;
			warn!("skipping tsunkit map {}: {}", map_name, error);
			continue;
		}
		stats.maps += 1;

		let map_data = match read_json::<TsunkitMapResponse>(&map_path) {
			Ok(map_data) => map_data,
			Err(error) => {
				stats.failures += 1;
				warn!("failed to parse tsunkit map {}: {}", map_name, error);
				continue;
			}
		};

		let nodesummary = if options.fetch_nodesummaries {
			let nodesummary_path = nodesummary_dir.join(format!("{map_name}.json"));
			match download_tsunkit_file(
				client.clone(),
				&format!("{TSUNKIT_MAPS_URL_ROOT}/{map_name}/nodesummary"),
				&nodesummary_path,
				overwrite,
				TSUNKIT_NODESUMMARY_TIMEOUT_SECS,
				TSUNKIT_NODESUMMARY_RETRIES,
			)
			.await
			{
				Ok(()) => {
					stats.nodesummaries += 1;
					read_json::<TsunkitNodeSummaryResponse>(&nodesummary_path).ok()
				}
				Err(error) => {
					stats.failures += 1;
					warn!("skipping tsunkit nodesummary {}: {}", map_name, error);
					None
				}
			}
		} else {
			None
		};

		let node_keys = collect_enemycomp_nodes(&map_data, nodesummary.as_ref());
		if node_keys.is_empty() || !options.fetch_enemycomps {
			continue;
		}

		let per_map_enemy_dir = enemycomps_dir.join(&map_name);
		if !per_map_enemy_dir.exists() {
			std::fs::create_dir_all(&per_map_enemy_dir)?;
		}

		let mut pending = futures::stream::iter(node_keys.into_iter().map(|node_key| {
			let client = client.clone();
			let map_name = map_name.clone();
			let path = per_map_enemy_dir.join(format!("{node_key}.json"));
			async move {
				let url = format!("{TSUNKIT_MAPS_URL_ROOT}/{map_name}/nodes/{node_key}/enemycomps");
				download_tsunkit_file(
					client,
					&url,
					&path,
					overwrite,
					TSUNKIT_ENEMYCOMP_TIMEOUT_SECS,
					TSUNKIT_ENEMYCOMP_RETRIES,
				)
				.await
				.map(|_| node_key)
			}
		}))
		.buffer_unordered(enemycomp_concurrency);

		while let Some(result) = futures::StreamExt::next(&mut pending).await {
			match result {
				Ok(_) => stats.enemycomps += 1,
				Err(error) => {
					stats.failures += 1;
					warn!("skipping tsunkit enemy comp for {}: {}", map_name, error);
				}
			}
		}
	}

	Ok(stats)
}

async fn download_tsunkit_file(
	client: Arc<reqwest::Client>,
	url: &str,
	path: &Path,
	overwrite: bool,
	timeout_secs: u64,
	max_attempts: usize,
) -> Result<(), BootstrapDownloadError> {
	if path.exists() && !overwrite {
		return Ok(());
	}

	let mut last_error = None;

	for attempt in 1..=max_attempts.max(1) {
		let request = Request::builder()
			.url(url)
			.save_as(path)
			.overwrite(overwrite)
			.skip_header_check(true)
			.build()?;

		match timeout(Duration::from_secs(timeout_secs), request.execute(Some((*client).clone())))
			.await
		{
			Ok(Ok(())) => return Ok(()),
			Ok(Err(source)) => {
				last_error = Some(BootstrapDownloadError::Download(source));
			}
			Err(_) => {
				last_error = Some(BootstrapDownloadError::Timeout {
					url: url.to_string(),
					save_as: path.display().to_string(),
					action: "downloading tsunkit resource",
					timeout_secs,
				});
			}
		}

		if attempt < max_attempts.max(1) {
			tokio::time::sleep(Duration::from_millis(250 * attempt as u64)).await;
		}
	}

	Err(last_error.unwrap_or_else(|| {
		BootstrapDownloadError::Download(emukc_network::download::DownloadError::FileNotFound {
			url: url.to_string(),
			save_as: PathBuf::from(path),
			final_url: url.to_string(),
		})
	}))
}

fn read_json<T>(path: &Path) -> Result<T, BootstrapDownloadError>
where
	T: serde::de::DeserializeOwned,
{
	let raw = std::fs::read_to_string(path)?;
	serde_json::from_str(&raw).map_err(|source| BootstrapDownloadError::Json {
		path: path.to_path_buf(),
		source,
	})
}

fn collect_enemycomp_nodes(
	map: &TsunkitMapResponse,
	node_summary: Option<&TsunkitNodeSummaryResponse>,
) -> Vec<String> {
	let coord_to_cell_no = map
		.result
		.map_set
		.map
		.spots
		.iter()
		.filter_map(|spot| spot.no.map(|no| ((spot.x, spot.y), no)))
		.collect::<BTreeMap<_, _>>();

	let mut battle_cell_nos = map
		.result
		.map_set
		.map
		.enemies
		.iter()
		.filter_map(|enemy| (enemy.no > 0).then_some(enemy.no))
		.collect::<BTreeSet<_>>();

	if let Some(node_summary) = node_summary {
		for (cell_no, entry) in &node_summary.result {
			if entry.battles > 0
				&& let Ok(cell_no) = cell_no.parse::<i64>()
			{
				battle_cell_nos.insert(cell_no);
			}
		}
	}

	let mut nodes = map
		.result
		.spots
		.iter()
		.filter_map(|(node_key, (x, y, label))| {
			if label.as_deref() == Some("Start") {
				return None;
			}
			let cell_no = coord_to_cell_no.get(&(*x, *y))?;
			battle_cell_nos.contains(cell_no).then(|| node_key.clone())
		})
		.collect::<Vec<_>>();
	nodes.sort();
	nodes.dedup();
	nodes
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn collect_enemycomp_nodes_prefers_battle_cells() {
		let map = serde_json::from_str::<TsunkitMapResponse>(
			r#"{
  "result": {
    "spots": {
      "1": [260, 246, "Start"],
      "A": [597, 328, null],
      "B": [840, 204, null],
      "C": [858, 486, null]
    },
    "mapSet": {
      "map": {
        "spots": [
          { "x": 260, "y": 246 },
          { "no": 1, "x": 597, "y": 328 },
          { "no": 2, "x": 840, "y": 204 },
          { "no": 3, "x": 858, "y": 486 }
        ],
        "enemies": [{ "no": 3 }]
      }
    }
  }
}"#,
		)
		.unwrap();
		let node_summary = serde_json::from_str::<TsunkitNodeSummaryResponse>(
			r#"{
  "result": {
    "2": { "battles": 12 },
    "3": { "battles": 7 }
  }
}"#,
		)
		.unwrap();

		let nodes = collect_enemycomp_nodes(&map, Some(&node_summary));
		assert_eq!(nodes, vec!["B".to_string(), "C".to_string()]);
	}
}
