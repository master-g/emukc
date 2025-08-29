use std::{
	collections::BTreeSet,
	sync::{Arc, LazyLock},
};

use emukc_cache::prelude::*;
use emukc_model::kc2::start2::{ApiManifest, ApiMstFurniture};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{
	make_list::{CacheList, batch_check_exists, source::kcs2::gen_path},
	prelude::{CacheListMakeStrategy, CacheListMakingError},
};

#[derive(Debug, Serialize, Deserialize)]
struct FurniturePictureScript {
	#[serde(default)]
	action1: Option<Action1>,

	#[serde(flatten)]
	other: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Action1 {
	data: Vec<Vec<ActionObj>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActionObj {
	filename: String,
	popup: Option<Popup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Popup {
	src: String,
	se_open: Option<String>,
	se_close: Option<String>,
}

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	if strategy == CacheListMakeStrategy::Minimal {
		return Ok(());
	}

	for entry in mst.api_mst_furniture.iter() {
		if entry.api_active_flag == 1 {
			make_scripts(entry, cache, list).await?;
			make_movable(entry, list);
			list.add(gen_furniture_path(entry.api_id, "thumbnail", "png"), entry.api_version);
		} else {
			make_normal(entry, list).await;
		}
	}

	make_outside(mst, list);

	match strategy {
		CacheListMakeStrategy::Minimal => {}
		CacheListMakeStrategy::Default => {
			make_reward_predefined(mst, list);
			// make_extra_greedy("card", mst, cache, list, 16).await?;
		}
		CacheListMakeStrategy::Greedy(concurrent) => {
			make_extra_greedy("reward", mst, cache, list, concurrent).await?;
			make_extra_greedy("card", mst, cache, list, concurrent).await?;
		}
	}

	Ok(())
}

async fn make_scripts(
	entry: &ApiMstFurniture,
	kache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let p = gen_furniture_path(entry.api_id, "scripts", "json");
	let mut script_file = GetOption::new_non_mod().get(kache, &p, entry.api_version).await?;
	list.add(p, entry.api_version);

	let mut raw = String::new();
	script_file.read_to_string(&mut raw).await?;
	// remove BOM from raw
	raw = raw.trim_start_matches('\u{feff}').to_string();

	let script: FurniturePictureScript =
		serde_json::from_str(&raw).map_err(|e| KacheError::InvalidFile(e.to_string()))?;

	if let Some(action1) = script.action1 {
		let pictures: Vec<&str> = action1
			.data
			.iter()
			.flatten()
			.filter_map(|d| d.popup.as_ref().map(|i| i.src.as_str()))
			.collect();
		for id in pictures {
			let id: i64 = id.parse().unwrap();
			list.add(gen_furniture_path(id, "picture", "png"), entry.api_version);
		}
	}

	Ok(())
}

fn make_movable(entry: &ApiMstFurniture, list: &mut CacheList) {
	for ext in ["json", "png"] {
		list.add(gen_furniture_path(entry.api_id, "movable", ext), entry.api_version);
	}
}

static NORMAL_HOLES: LazyLock<Vec<i64>> =
	LazyLock::new(|| vec![8, 43, 62, 121, 131, 134, 150, 153, 163, 167, 169, 173, 177, 190, 191]);

async fn make_normal(entry: &ApiMstFurniture, list: &mut CacheList) {
	if !NORMAL_HOLES.contains(&entry.api_id) {
		list.add(gen_furniture_path(entry.api_id, "normal", "png"), entry.api_version);
	}
}

fn make_outside(mst: &ApiManifest, list: &mut CacheList) {
	let id_set: BTreeSet<i64> =
		mst.api_mst_furniture.iter().map(|entry| entry.api_outside_id).collect();

	for id in id_set {
		for i in 1..=5 {
			list.add_unversioned(format!(
				"kcs2/resources/furniture/outside/window_bg_{id}-{i}.png"
			));
		}
	}
}

static REWARD_PREDEFINED: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		11, 30, 183, 261, 268, 277, 280, 286, 291, 292, 293, 301, 314, 322, 324, 325, 328, 361,
		380, 381, 382, 393, 395, 407, 408, 412, 416, 426, 433, 446, 453, 458, 459, 474, 478, 479,
		490, 497, 501, 505, 510, 516, 517, 518, 520, 529, 533, 555, 557, 558, 569, 578, 581, 588,
		589, 600, 607, 612, 618, 628, 632, 634, 639, 640, 650, 656,
	]
});

static CARD_PREDEFINED: LazyLock<Vec<i64>> = LazyLock::new(|| vec![311, 334, 486, 487]);

fn make_reward_predefined(mst: &ApiManifest, list: &mut CacheList) {
	for id in REWARD_PREDEFINED.iter() {
		if let Some(v) = mst.api_mst_furniture.iter().find(|v| v.api_id == *id) {
			list.add(gen_furniture_path(*id, "reward", "png"), v.api_version);
		}
	}

	for id in CARD_PREDEFINED.iter() {
		if let Some(v) = mst.api_mst_furniture.iter().find(|v| v.api_id == *id) {
			list.add(gen_furniture_path(*id, "card", "png"), v.api_version);
		}
	}
}

async fn make_extra_greedy(
	key: &str,
	mst: &ApiManifest,
	cache: &Kache,
	list: &mut CacheList,
	concurrent: usize,
) -> Result<(), KacheError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_furniture
		.iter()
		.map(|v| (gen_furniture_path(v.api_id, key, "png"), v.api_version.to_string()))
		.collect();

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{p}, {v}");
			list.add(p, v);
		}
	}

	Ok(())
}

fn gen_furniture_path(id: i64, category: &str, extension: &str) -> String {
	gen_path(id, 3, "furniture", category, extension)
}
