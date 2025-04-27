#![allow(unused)]
use std::{
	collections::HashMap,
	sync::{Arc, LazyLock},
};

use emukc_cache::Kache;
use emukc_model::kc2::start2::{ApiManifest, ApiMstShipgraph};

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

	make_preset(mst, list);
	match strategy {
		CacheListMakeStrategy::Default => {
			make_special_preset(mst, list);
		}

		CacheListMakeStrategy::Greedy(concurrent) => {
			make_special_greedy(mst, cache, concurrent, list).await?;
		}
		_ => {}
	};

	Ok(())
}

// https://github.com/Tibowl/KCCacheProxy/blob/33d826c46e1969c69cd83e784bd9b0addb44230e/src/proxy/preload.js#L583

static SPECIAL_CG: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		541, 571, 572, 573, 576, 577, 591, 592, 593, 954, 694, 601, 1496, 913, 918, 184, 634, 635,
		639, 640, 944, 949, 911, 916, 546, 392, 969, 724, 364, 733,
	]
});

static REPAIR_VOICE_SHIPS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		// These ships got special voice line (6, aka. Repair) implemented (some used by akashi remodel),
		// tested by trying and succeeding to http fetch mp3 from kc server
		56, 160, 224, // Naka
		65, 194, 268, // Haguro
		69,  // Choukai
		89,  // Houshou (Poke dupe)
		114, 200, 290, // Abukuma
		116, // Zuihou (Poke dupe)
		123, 142, 295, // Kinukasa
		126, 398, // I-168
		127, 399, // I-58
		135, 304, 543, // Naganami
		136, // Yamato Kai (Poke dupe)
		145, 961, // Shigure Kai Ni(San) (Event/Equip2 reuse)
		321, // Ooyodo Kai (Friend50 cut)
		412, // Yamashiro Kai Ni (Poke dupe)
		418, // Satsuki Kai Ni
		449, // Pola (Equip1 cut)
		496, // Zara due (Event reuse)
		515, // Ark Royal (Poke dupe)
		522, 884, // Yawatamaru (K2 Equip3 dupe), Unyou
		549, // Intrepid (Poke dupe)
		568, // Kuroshio Kai Ni (Poke dupe)
		573, // Mutsu Kai Ni (Poke dupe)
		578, // Asashimo Kai Ni
		580, // Maestrale Kai (Base Poke1 dupe)
		591, // Kongou K2C (Attack dupe)
		662, // Noshiro Kai Ni (Poke dupe)
		694, // Kirishima K2C
		951, // Amatsukaze Kai Ni
		955, 960, // Kiyoshimo K2(D)
		975, // Harusame Kai Ni
	]
});

static VOICE_DIFF: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		2475, 6547, 1471, 8691, 7847, 3595, 1767, 3311, 2507, 9651, 5321, 4473, 7117, 5947, 9489,
		2669, 8741, 6149, 1301, 7297, 2975, 6413, 8391, 9705, 2243, 2091, 4231, 3107, 9499, 4205,
		6013, 3393, 6401, 6985, 3683, 9447, 3287, 5181, 7587, 9353, 2135, 4947, 5405, 5223, 9457,
		5767, 9265, 8191, 3927, 3061, 2805, 3273, 7331,
	]
});

fn calc_voice_id(ship_id: i64, voice_id: i64) -> i64 {
	if voice_id <= 53 {
		100000 + 17 * (ship_id + 7) * (VOICE_DIFF[(voice_id - 1) as usize]) % 99173
	} else {
		voice_id
	}
}

fn get_voice_version(graph: &ApiMstShipgraph, voice_id: i64) -> i64 {
	let idx = if voice_id == 2 || voice_id == 3 {
		2
	} else {
		1
	};

	if graph.api_version.len() > idx {
		graph.api_version[idx].parse().unwrap_or(0)
	} else {
		0
	}
}

fn make_preset(mst: &ApiManifest, list: &mut CacheList) {
	for graph in mst.api_mst_shipgraph.iter() {
		match graph.api_sortno {
			Some(id) => {
				if id == 0 {
					continue;
				}
			}
			None => continue,
		}

		let Some(ship_mst) = mst.find_ship(graph.api_id) else {
			continue;
		};

		if graph.api_battle_n.is_none() {
			continue;
		}
		if graph.api_boko_d.is_none() {
			continue;
		}

		let mut vnums = vec![
			1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
			26, 27, 28,
		];

		let voicef = ship_mst.api_voicef.unwrap_or(0);
		if voicef & 0b0001 != 0 {
			vnums.push(29);
		}
		if voicef & 0b0010 != 0 {
			vnums.extend_from_slice(&[
				30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
				51, 52, 53,
			]);
		}
		if voicef & 0b0100 != 0 {
			vnums.push(129);
		}

		if SPECIAL_CG.contains(&graph.api_id) {
			vnums.push(900);
			// 	vnums.extend_from_slice(&[901, 902, 903]);
		}

		// fix zeppelin, see: https://github.com/KC3Kai/KC3Kai/blob/da2a3d60ee21335af886b0bd10ef12f6d9cdd287/src/library/modules/Meta.js#L80
		if [432, 353].contains(&graph.api_id) {
			vnums.extend_from_slice(&[917, 918]);
		}

		if REPAIR_VOICE_SHIPS.contains(&graph.api_id) {
			vnums.push(6);
		}

		for voice_id in vnums {
			let path = format!(
				"kcs/sound/kc{}/{}.mp3",
				graph.api_filename,
				calc_voice_id(graph.api_id, voice_id)
			);
			let ver = get_voice_version(graph, voice_id);

			list.add(path, ver);
		}
	}
}

async fn make_special_greedy(
	mst: &ApiManifest,
	cache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let mut checks: Vec<(String, String)> = Vec::new();
	let mut lookups: HashMap<String, (i64, i64)> = HashMap::new();

	for graph in mst.api_mst_shipgraph.iter() {
		match graph.api_sortno {
			Some(id) => {
				if id == 0 {
					continue;
				}
			}
			None => continue,
		}

		let Some(ship_mst) = mst.find_ship(graph.api_id) else {
			continue;
		};

		if graph.api_battle_n.is_none() {
			continue;
		}
		if graph.api_boko_d.is_none() {
			continue;
		}

		if SPECIAL_CG.contains(&graph.api_id) {
			for voice_id in [901, 902, 903, 904, 905] {
				let path = format!(
					"kcs/sound/kc{}/{}.mp3",
					graph.api_filename,
					calc_voice_id(graph.api_id, voice_id)
				);
				let ver = get_voice_version(graph, voice_id);

				lookups.insert(path.clone(), (graph.api_id, voice_id));
				checks.push((path, format!("{}", ver)));
			}
		}
	}

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;
	for ((p, v), exists) in check_result {
		if exists {
			if let Some((ship_id, voice_id)) = lookups.get(&p) {
				println!("{}: {}", ship_id, voice_id);
			}

			list.add(p, v);
		}
	}

	Ok(())
}

type SpecialVoice = (i64, Vec<i64>);

static SPECIAL: LazyLock<Vec<SpecialVoice>> = LazyLock::new(|| {
	vec![
		(184, vec![901]),
		(541, vec![901, 902, 903]),
		(573, vec![901, 902]),
		(634, vec![901]),
		(635, vec![901]),
		(639, vec![901]),
		(640, vec![901]),
		(911, vec![901]),
		(916, vec![901]),
		(944, vec![901]),
		(949, vec![901]),
	]
});

fn make_special_preset(mst: &ApiManifest, list: &mut CacheList) {
	for (ship_id, voice_ids) in SPECIAL.iter() {
		let Some(graph) = mst.find_shipgraph(*ship_id) else {
			continue;
		};

		for voice_id in voice_ids {
			let path = format!(
				"kcs/sound/kc{}/{}.mp3",
				graph.api_filename,
				calc_voice_id(*ship_id, *voice_id)
			);
			list.add(path, get_voice_version(graph, *voice_id));
		}
	}
}
