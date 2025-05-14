use std::{
	collections::BTreeMap,
	sync::{Arc, LazyLock},
};

use emukc_cache::Kache;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::{ApiManifest, ApiMstSlotitem};

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
	if strategy == CacheListMakeStrategy::Minimal {
		return Ok(());
	}

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
		_ => {}
	};

	Ok(())
}

fn make_default(mst: &ApiManifest, list: &mut CacheList) {
	let baga_categories = vec!["card", "card_t", "item_on", "remodel", "statustop_item"];
	let default_categories =
		vec!["card", "card_t", "item_on", "item_up", "remodel", "statustop_item"];

	let mut plane_slots: BTreeMap<i64, &ApiMstSlotitem> = BTreeMap::new();

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
		if let Some(key) = (slot.api_type[4] != 0).then_some(slot.api_type[4]) {
			plane_slots
				.entry(key)
				.and_modify(|entry| {
					if entry.api_version.is_none() && slot.api_version.is_some() {
						*entry = slot;
					}
				})
				.or_insert(slot);
		}
	}

	for (id, slot) in plane_slots.iter() {
		list.add(format!("kcs2/resources/plane/{0:03}.png", id), slot.api_version)
			.add(format!("kcs2/resources/plane/r{0:03}.png", id), slot.api_version);

		let item_id = format!("{0:04}", slot.api_id);

		for category in ["airunit_banner", "airunit_fairy", "airunit_name"] {
			let key = SuffixUtils::create(&item_id, format!("slot_{}", category).as_str());
			list.add(
				format!("kcs2/resources/slot/{category}/{item_id}_{key}.png"),
				slot.api_version,
			);
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
		1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 27, 28, 29, 30, 31, 32, 35, 36, 37, 38,
		39, 40, 41, 48, 49, 50, 51, 58, 63, 64, 65, 66, 67, 71, 75, 76, 77, 78, 84, 85, 88, 89, 90,
		91, 92, 95, 103, 104, 105, 106, 114, 116, 117, 119, 120, 121, 122, 123, 124, 125, 127, 128,
		129, 130, 131, 133, 134, 135, 137, 139, 141, 142, 147, 154, 160, 161, 162, 172, 173, 174,
		179, 183, 190, 191, 192, 210, 211, 213, 214, 220, 229, 231, 232, 234, 235, 236, 240, 242,
		243, 244, 245, 246, 247, 254, 255, 257, 266, 267, 274, 275, 276, 278, 279, 280, 281, 282,
		283, 284, 285, 286, 289, 290, 293, 294, 295, 296, 297, 298, 299, 300, 301, 303, 307, 308,
		309, 310, 313, 314, 315, 317, 318, 320, 328, 329, 330, 331, 332, 338, 339, 340, 341, 344,
		345, 356, 357, 358, 359, 360, 361, 362, 363, 364, 365, 366, 373, 374, 376, 379, 380, 381,
		382, 383, 384, 385, 386, 387, 389, 390, 393, 394, 397, 398, 399, 400, 407, 410, 411, 412,
		426, 427, 428, 429, 430, 440, 441, 442, 443, 450, 455, 456, 457, 458, 460, 461, 463, 464,
		465, 467, 468, 470, 473, 474, 483, 490, 502, 503, 505, 506, 507, 508, 509, 511, 512, 517,
		518, 519, 520, 524, 527, 528, 529, 530, 533, 534, 535, 536, 537, 545, 552, 553, 555, 556,
		557, 558, 1501, 1502, 1503, 1504, 1505, 1506, 1507, 1508, 1509, 1510, 1511, 1512, 1513,
		1514, 1515, 1516, 1527, 1528, 1529, 1530, 1531, 1532, 1535, 1536, 1550, 1551, 1552, 1553,
		1563, 1565, 1567, 1568, 1570, 1576, 1577, 1578, 1579, 1580, 1584, 1585, 1587, 1588, 1589,
		1590, 1591, 1592, 1593, 1596, 1599, 1600, 1601, 1602, 1603, 1604, 1605, 1606, 1607, 1608,
		1609, 1612, 1613, 1614, 1615, 1616, 1622, 1623, 1624, 1627, 1637, 1638, 1639, 1641, 1642,
		1643, 1644, 1645, 1647, 1649, 1653, 1654,
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
