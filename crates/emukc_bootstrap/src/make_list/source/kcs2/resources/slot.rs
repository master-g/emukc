use std::sync::{Arc, LazyLock};

use emukc_cache::Kache;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
	make_list::{CacheList, batch_check_exists},
	prelude::{CacheListMakeStrategy, CacheListMakingError},
};

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	make_default(mst, list);

	match strategy {
		CacheListMakeStrategy::Default => {
			make_enemy_plane(list);
			make_btxt_flat(mst, list);
			make_character(mst, list);
		}
		CacheListMakeStrategy::Greedy(concurrent) => {
			make_enemy_plane_greedy(cache, concurrent, list).await?;
			make_btxt_flat_greedy(mst, cache, concurrent, list).await?;
			make_character_greedy(mst, cache, concurrent, list).await?;
		}
	};

	Ok(())
}

fn make_default(mst: &ApiManifest, list: &mut CacheList) {
	let baga_categories = vec!["card", "card_t", "item_on", "remodel", "statustop_item"];
	let default_categories =
		vec!["card", "card_t", "item_on", "item_up", "remodel", "statustop_item"];

	for slot in mst.api_mst_slotitem.iter() {
		let item_id = format!("{0:04}", slot.api_id);

		// ally
		if slot.api_sortno > 0 {
			let categories = if slot.api_type[0] != 27 {
				&default_categories
			} else {
				&baga_categories
			};
			for category in categories {
				let key = SuffixUtils::create(&item_id, format!("slot_{}", category).as_str());
				list.add(
					format!("kcs2/resources/slot/{category}/{item_id}_{key}.png"),
					slot.api_version,
				);
			}
		}

		// plane
		if slot.api_type[4] != 0 {
			list.add(
				format!("kcs2/resources/plane/{0:03}.png", slot.api_type[4]),
				slot.api_version,
			)
			.add(format!("kcs2/resources/plane/r{0:03}.png", slot.api_type[4]), slot.api_version);

			for category in ["airunit_banner", "airunit_fairy", "airunit_name"] {
				let key = SuffixUtils::create(&item_id, format!("slot_{}", category).as_str());
				list.add(
					format!("kcs2/resources/slot/{category}/{item_id}_{key}.png"),
					slot.api_version,
				);
			}
		}
	}
}

#[allow(unused)]
async fn make_enemy_plane_greedy(
	cache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let checks: Vec<(String, String)> = (1..=50)
		.map(|v| (format!("kcs2/resources/plane/e{0:03}.png", v), "".to_string()))
		.collect();

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, _), exists) in check_result {
		if exists {
			println!("{}", p);
			list.add_unversioned(p);
		}
	}

	Ok(())
}

const ENEMY_PLANE_MAX_ID: usize = 25;

fn make_enemy_plane(list: &mut CacheList) {
	for i in 1..=ENEMY_PLANE_MAX_ID {
		let p = format!("kcs2/resources/plane/e{0:03}.png", i);
		list.add_unversioned(p);
	}
}

async fn make_btxt_flat_greedy(
	mst: &ApiManifest,
	cache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_slotitem
		.iter()
		.map(|v| {
			let item_id = format!("{0:04}", v.api_id);
			let key = SuffixUtils::create(&item_id, "slot_btxt_flat");
			let ver = v.api_version.unwrap_or(1);
			(format!("kcs2/resources/slot/btxt_flat/{item_id}_{key}.png"), format!("{}", ver))
		})
		.collect();

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, _), exists) in check_result {
		if exists {
			println!("{}", p);
			list.add_unversioned(p);
		}
	}

	Ok(())
}

const BTXT_FLAT_IDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		0001, 0002, 0003, 0004, 0005, 0006, 0007, 0008, 0009, 0010, 0011, 0012, 0013, 0014, 0015,
		0027, 0028, 0029, 0030, 0031, 0032, 0035, 0036, 0037, 0038, 0039, 0040, 0041, 0048, 0049,
		0050, 0051, 0058, 0063, 0064, 0065, 0066, 0067, 0071, 0075, 0076, 0077, 0078, 0084, 0085,
		0088, 0089, 0090, 0091, 0092, 0095, 0103, 0104, 0105, 0106, 0114, 0116, 0117, 0119, 0120,
		0121, 0122, 0123, 0124, 0125, 0127, 0128, 0129, 0130, 0131, 0133, 0134, 0135, 0137, 0139,
		0141, 0142, 0147, 0154, 0160, 0161, 0162, 0172, 0173, 0174, 0179, 0183, 0190, 0191, 0192,
		0210, 0211, 0213, 0214, 0220, 0229, 0231, 0232, 0234, 0235, 0236, 0240, 0242, 0243, 0244,
		0245, 0246, 0247, 0254, 0255, 0257, 0266, 0267, 0274, 0275, 0276, 0278, 0279, 0280, 0281,
		0282, 0283, 0284, 0285, 0286, 0289, 0290, 0293, 0294, 0295, 0296, 0297, 0298, 0299, 0300,
		0301, 0303, 0307, 0308, 0309, 0310, 0313, 0314, 0315, 0317, 0318, 0320, 0328, 0329, 0330,
		0331, 0332, 0338, 0339, 0340, 0341, 0344, 0345, 0356, 0357, 0358, 0359, 0360, 0361, 0362,
		0363, 0364, 0365, 0366, 0373, 0374, 0376, 0379, 0380, 0381, 0382, 0383, 0384, 0385, 0386,
		0387, 0389, 0390, 0393, 0394, 0397, 0398, 0399, 0400, 0407, 0410, 0411, 0412, 0426, 0427,
		0428, 0429, 0430, 0440, 0441, 0442, 0443, 0450, 0455, 0456, 0457, 0458, 0460, 0461, 0463,
		0464, 0465, 0467, 0468, 0470, 0473, 0474, 0483, 0490, 0502, 0503, 0505, 0506, 0507, 0508,
		0509, 0511, 0512, 0517, 0518, 0519, 0520, 0524, 0527, 0528, 0529, 0530, 0533, 0534, 0535,
		0536, 0537, 0545, 0552, 0553, 0555, 0556, 1501, 1502, 1503, 1504, 1505, 1506, 1507, 1508,
		1509, 1510, 1511, 1512, 1513, 1514, 1515, 1516, 1527, 1528, 1529, 1530, 1531, 1532, 1535,
		1536, 1550, 1551, 1552, 1553, 1563, 1565, 1567, 1568, 1570, 1576, 1577, 1578, 1579, 1580,
		1584, 1585, 1587, 1588, 1589, 1590, 1591, 1592, 1593, 1596, 1599, 1600, 1601, 1602, 1603,
		1604, 1605, 1606, 1607, 1608, 1609, 1612, 1613, 1614, 1615, 1616, 1622, 1623, 1624, 1627,
		1637, 1638, 1639, 1641, 1642, 1643, 1644, 1645, 1647, 1649, 1653, 1654,
	]
});

fn make_btxt_flat(api: &ApiManifest, list: &mut CacheList) {
	for id in BTXT_FLAT_IDS.iter() {
		let item_id = format!("{0:04}", id);
		let key = SuffixUtils::create(&item_id, "slot_btxt_flat");
		let ver =
			api.api_mst_slotitem.iter().find(|v| v.api_id == *id).unwrap().api_version.unwrap_or(1);
		list.add(format!("kcs2/resources/slot/btxt_flat/{item_id}_{key}.png"), format!("{}", ver));
	}
}

const CHARACTER_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| vec![042]);

fn make_character(mst: &ApiManifest, list: &mut CacheList) {
	for m in mst.api_mst_slotitem.iter() {
		if m.api_sortno == 0 || CHARACTER_HOLES.contains(&m.api_id) {
			continue;
		}

		let item_id = format!("{0:04}", m.api_id);
		let key = SuffixUtils::create(&item_id, "slot_item_character");
		list.add(
			format!("kcs2/resources/slot/item_character/{}_{}.png", item_id, key),
			m.api_version,
		);
	}
}

async fn make_character_greedy(
	mst: &ApiManifest,
	cache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_slotitem
		.iter()
		.filter(|v| v.api_sortno > 0)
		.map(|v| {
			let item_id = format!("{0:04}", v.api_id);
			let key = SuffixUtils::create(&item_id, "slot_item_character");
			let ver = v.api_version.unwrap_or(1);
			(format!("kcs2/resources/slot/item_character/{item_id}_{key}.png"), format!("{}", ver))
		})
		.collect();

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, _), exists) in check_result {
		if exists {
			println!("{}", p);
			list.add_unversioned(p);
		}
	}

	Ok(())
}
