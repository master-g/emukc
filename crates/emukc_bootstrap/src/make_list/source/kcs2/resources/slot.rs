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

static BTXT_FLAT_IDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		1, 2, 3, 4, 5, 6, 7, 8, 9, 8, 9, 10, 11, 12, 13, 23, 28, 29, 24, 25, 26, 29, 30, 31, 38,
		39, 32, 33, 48, 49, 40, 41, 58, 51, 52, 53, 54, 55, 57, 61, 62, 63, 78, 84, 85, 88, 89, 90,
		91, 92, 95, 67, 68, 69, 70, 76, 78, 79, 119, 80, 81, 82, 83, 84, 85, 87, 128, 129, 88, 89,
		91, 92, 93, 95, 139, 97, 98, 103, 108, 112, 113, 114, 122, 123, 124, 179, 183, 190, 191,
		192, 136, 137, 139, 140, 144, 229, 153, 154, 156, 157, 158, 160, 162, 163, 164, 165, 166,
		167, 172, 173, 175, 182, 183, 188, 189, 190, 278, 279, 280, 281, 282, 283, 284, 285, 286,
		289, 290, 293, 294, 295, 296, 297, 298, 299, 192, 193, 195, 199, 308, 309, 200, 203, 204,
		205, 207, 318, 208, 328, 329, 216, 217, 218, 338, 339, 224, 225, 228, 229, 238, 239, 358,
		359, 240, 241, 242, 243, 244, 245, 246, 251, 252, 254, 379, 380, 381, 382, 383, 384, 385,
		386, 387, 389, 390, 393, 394, 397, 398, 399, 256, 263, 264, 265, 266, 278, 279, 428, 429,
		280, 288, 289, 290, 291, 296, 301, 302, 303, 458, 304, 305, 307, 308, 309, 311, 468, 312,
		315, 316, 483, 490, 322, 323, 325, 326, 327, 508, 509, 329, 330, 335, 518, 519, 336, 340,
		343, 528, 529, 344, 347, 348, 349, 350, 351, 357, 362, 363, 365, 366, 1501, 1502, 1503,
		1504, 1505, 1506, 1507, 1508, 1509, 1510, 1511, 1512, 1513, 1514, 1515, 1516, 1527, 1528,
		1529, 1530, 1531, 1532, 1535, 1536, 1550, 1551, 1552, 1553, 1563, 1565, 1567, 1568, 1570,
		1576, 1577, 1578, 1579, 1580, 1584, 1585, 1587, 1588, 1589, 1590, 1591, 1592, 1593, 1596,
		1599, 1600, 1601, 1602, 1603, 1604, 1605, 1606, 1607, 1608, 1609, 1612, 1613, 1614, 1615,
		1616, 1622, 1623, 1624, 1627, 1637, 1638, 1639, 1641, 1642, 1643, 1644, 1645, 1647, 1649,
		1653, 1654,
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

static CHARACTER_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| vec![42]);

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
