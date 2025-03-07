use std::{collections::BTreeSet, sync::LazyLock};

use emukc_cache::prelude::*;
use emukc_model::kc2::start2::{ApiManifest, ApiMstFurniture};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::crawl::kcs2::fetch_res_impl;

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

pub(super) async fn crawl(mst: &ApiManifest, cache: &Kache) -> Result<(), KacheError> {
	for entry in mst.api_mst_furniture.iter() {
		if entry.api_active_flag == 1 {
			fetch_scripts(entry, cache).await?;
			fetch_movable(entry, cache).await?;
			fetch_impl(cache, entry.api_id, "thumbnail", "png", entry.api_version).await?;
		} else {
			fetch_normal(entry, cache).await?;
		}
	}

	fetch_outside(mst, cache).await?;
	fetch_reward_predefined(mst, cache).await?;

	Ok(())
}

async fn fetch_scripts(entry: &ApiMstFurniture, cache: &Kache) -> Result<(), KacheError> {
	let mut script_file =
		fetch_impl(cache, entry.api_id, "scripts", "json", entry.api_version).await?;

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
			fetch_impl(cache, id, "picture", "png", entry.api_version).await?;
		}
	}

	Ok(())
}

async fn fetch_movable(entry: &ApiMstFurniture, cache: &Kache) -> Result<(), KacheError> {
	for ext in ["json", "png"] {
		fetch_impl(cache, entry.api_id, "movable", ext, entry.api_version).await?;
	}

	Ok(())
}

static NORMAL_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![008, 043, 062, 121, 131, 134, 150, 153, 163, 167, 169, 173, 177, 190, 191]
});

async fn fetch_normal(entry: &ApiMstFurniture, cache: &Kache) -> Result<(), KacheError> {
	if NORMAL_HOLES.contains(&entry.api_id) {
		return Ok(());
	}

	fetch_impl(cache, entry.api_id, "normal", "png", entry.api_version).await?;

	Ok(())
}

async fn fetch_outside(mst: &ApiManifest, cache: &Kache) -> Result<(), KacheError> {
	let id_set: BTreeSet<i64> =
		mst.api_mst_furniture.iter().map(|entry| entry.api_outside_id).collect();

	for id in id_set {
		for i in 1..=5 {
			let _ = cache
				.get(
					format!("kcs2/resources/furniture/outside/window_bg_{id}-{i}.png").as_str(),
					NoVersion,
				)
				.await;
		}
	}

	Ok(())
}

#[allow(dead_code)]
async fn fetch_reward_greedy(mst: &ApiMstFurniture, cache: &Kache) -> Result<(), KacheError> {
	let _ = fetch_impl(cache, mst.api_id, "reward", "png", mst.api_version).await?;
	Ok(())
}

static REWARD_PREDEFINED: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		408, 479, 291, 520, 381, 578, 380, 433, 280, 558, 011, 426, 393, 268, 517, 261, 328, 607,
		292, 277, 458, 286, 382, 533, 588, 612, 628, 416, 557, 589, 501, 325, 293, 518, 581, 395,
		453, 459, 634, 183, 301, 516, 446, 618, 490, 505, 030, 510, 361, 324, 529, 474, 314, 569,
		407, 639, 478, 412, 632, 322, 555, 497, 600,
	]
});

async fn fetch_reward_predefined(mst: &ApiManifest, cache: &Kache) -> Result<(), KacheError> {
	for id in REWARD_PREDEFINED.iter() {
		if let Some(v) = mst.api_mst_furniture.iter().find(|v| v.api_id == *id) {
			let _ = fetch_impl(cache, *id, "reward", "png", v.api_version).await?;
		}
	}
	Ok(())
}

async fn fetch_impl<V>(
	cache: &Kache,
	id: i64,
	category: &str,
	extension: &str,
	ver: V,
) -> Result<tokio::fs::File, KacheError>
where
	V: IntoVersion + std::fmt::Debug,
{
	fetch_res_impl(cache, id, 3, "furniture", category, extension, ver).await
}
