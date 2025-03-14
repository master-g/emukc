use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

pub(super) async fn make(
	mst: &ApiManifest,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	make_non_graph(mst, list);
	make_graph(mst, list);

	Ok(())
}

fn make_non_graph(mst: &ApiManifest, list: &mut CacheList) {
	for ship in mst.api_mst_ship.iter() {
		let categories = if ship.api_aftershipid.is_none() {
			vec!["banner", "banner3", "banner3_g_dmg"]
		} else {
			vec![
				"album_status",
				"banner",
				"banner2",
				"banner2_dmg",
				"banner2_g_dmg",
				"banner_dmg",
				"banner_g_dmg",
				"card",
				"card_dmg",
				"character_full",
				"character_full_dmg",
				"character_up",
				"character_up_dmg",
				"power_up",
				"remodel",
				"remodel_dmg",
				"supply_character",
				"supply_character_dmg",
			]
		};

		let ship_id = format!("{0:04}", ship.api_id);

		let graph = mst.api_mst_shipgraph.iter().find(|v| v.api_id == ship.api_id);
		let version = graph.map(|v| v.api_version.first()).flatten();

		for category in categories {
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str())
				),
				version,
			);
		}
	}
}

fn make_graph(mst: &ApiManifest, list: &mut CacheList) {
	for graph in mst.api_mst_shipgraph.iter() {
		let ship_id = format!("{0:04}", graph.api_id);
		let version = graph.api_version.first();

		for category in ["full", "full_dmg"] {
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
					graph.api_filename
				),
				version,
			);
		}
	}
}
