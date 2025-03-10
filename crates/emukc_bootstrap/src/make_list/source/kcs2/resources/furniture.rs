use std::{collections::BTreeSet, sync::LazyLock};

use emukc_cache::prelude::*;
use emukc_model::kc2::start2::{ApiManifest, ApiMstFurniture};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{
	make_list::{CacheList, source::kcs2::gen_path},
	prelude::CacheListMakingError,
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
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
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
	make_reward_predefined(mst, list);

	Ok(())
}

async fn make_scripts(
	entry: &ApiMstFurniture,
	kache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let p = gen_furniture_path(entry.api_id, "scripts", "json");
	let mut script_file = kache.get(&p, entry.api_version).await?;
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

static NORMAL_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![008, 043, 062, 121, 131, 134, 150, 153, 163, 167, 169, 173, 177, 190, 191]
});

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
		408, 479, 291, 520, 381, 578, 380, 433, 280, 558, 011, 426, 393, 268, 517, 261, 328, 607,
		292, 277, 458, 286, 382, 533, 588, 612, 628, 416, 557, 589, 501, 325, 293, 518, 581, 395,
		453, 459, 634, 183, 301, 516, 446, 618, 490, 505, 030, 510, 361, 324, 529, 474, 314, 569,
		407, 639, 478, 412, 632, 322, 555, 497, 600,
	]
});

fn make_reward_predefined(mst: &ApiManifest, list: &mut CacheList) {
	for id in REWARD_PREDEFINED.iter() {
		if let Some(v) = mst.api_mst_furniture.iter().find(|v| v.api_id == *id) {
			list.add(gen_furniture_path(*id, "reward", "png"), v.api_version);
		}
	}
}

fn gen_furniture_path(id: i64, category: &str, extension: &str) -> String {
	gen_path(id, 3, "furniture", category, extension)
}
