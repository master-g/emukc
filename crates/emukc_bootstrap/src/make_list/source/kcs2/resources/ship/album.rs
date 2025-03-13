use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::make_list::CacheList;

pub(super) fn make(mst: &ApiManifest, list: &mut CacheList) {
	for ship in mst.api_mst_ship.iter() {
		if ship.api_aftershipid.is_none() {
			continue;
		}

		let ship_id = format!("{0:04}", ship.api_id);
		let p = format!(
			"kcs2/resources/ship/album_status/{ship_id}_{}.png",
			SuffixUtils::create(&ship_id, "ship_album_status")
		);

		let graph = mst.api_mst_shipgraph.iter().find(|v| v.api_id == ship.api_id);
		let version = graph.map(|v| v.api_version.first()).flatten();
		list.add(p, version);
	}
}
