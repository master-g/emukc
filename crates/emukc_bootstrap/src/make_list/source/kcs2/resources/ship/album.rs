use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::make_list::CacheList;

pub(super) fn make(mst: &ApiManifest, list: &mut CacheList) {
	for ship in mst.api_mst_ship.iter() {
		if ship.api_aftershipid.is_none() {
			continue;
		}

		let ship_id = format!("{0:04}", ship.api_id);

		let graph = mst.api_mst_shipgraph.iter().find(|v| v.api_id == ship.api_id);
		let version = graph.map(|v| v.api_version.first()).flatten();

		for category in [
			"album_status",
			"banner",
			"banner_dmg",
			"card",
			"card_dmg",
			"character_full",
			"character_full_dmg",
			"character_up",
			"character_up_dmg",
			"supply_character_dmg",
		] {
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
