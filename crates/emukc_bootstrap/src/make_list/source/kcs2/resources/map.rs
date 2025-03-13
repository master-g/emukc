use std::{collections::BTreeSet, sync::LazyLock};

use emukc_cache::{GetOption, Kache, KacheError, NoVersion};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

static DEFAULT_MAPS: LazyLock<&[&str]> = LazyLock::new(|| {
	&[
		"001/01.png",
		"001/01_image.json",
		"001/01_image.png",
		"001/01_info.json",
		"001/02.png",
		"001/02_image.json",
		"001/02_image.png",
		"001/02_info.json",
		"001/03.png",
		"001/03_image.json",
		"001/03_image.png",
		"001/03_info.json",
		"001/04.png",
		"001/04_image.json",
		"001/04_image.png",
		"001/04_info.json",
		"001/05.png",
		"001/05_image.json",
		"001/05_image.png",
		"001/05_info.json",
		"001/06.png",
		"001/06_image.json",
		"001/06_image.png",
		"001/06_info.json",
		"002/01.png",
		"002/01_image.json",
		"002/01_image.png",
		"002/01_info.json",
		"002/02.png",
		"002/02_image.json",
		"002/02_image.png",
		"002/02_info.json",
		"002/03.png",
		"002/03_image.json",
		"002/03_image.png",
		"002/03_info.json",
		"002/04.png",
		"002/04_image.json",
		"002/04_image.png",
		"002/04_info.json",
		"002/05.png",
		"002/05_image.json",
		"002/05_image.png",
		"002/05_info.json",
		"003/01.png",
		"003/01_image.json",
		"003/01_image.png",
		"003/01_info.json",
		"003/02.png",
		"003/02_image.json",
		"003/02_image.png",
		"003/02_info.json",
		"003/03.png",
		"003/03_image.json",
		"003/03_image.png",
		"003/03_info.json",
		"003/04.png",
		"003/04_image.json",
		"003/04_image.png",
		"003/04_info.json",
		"003/05.png",
		"003/05_image.json",
		"003/05_image.png",
		"003/05_info.json",
		"004/01.png",
		"004/01_image.json",
		"004/01_image.png",
		"004/01_info.json",
		"004/02.png",
		"004/02_image.json",
		"004/02_image.png",
		"004/02_info.json",
		"004/03.png",
		"004/03_image.json",
		"004/03_image.png",
		"004/03_info.json",
		"004/04.png",
		"004/04_image.json",
		"004/04_image.png",
		"004/04_info.json",
		"004/05.png",
		"004/05_image.json",
		"004/05_image.png",
		"004/05_info.json",
		"005/01.png",
		"005/01_image.json",
		"005/01_image.png",
		"005/01_info.json",
		"005/02.png",
		"005/02_image.json",
		"005/02_image.png",
		"005/02_info.json",
		"005/03.png",
		"005/03_image.json",
		"005/03_image.png",
		"005/03_info.json",
		"005/04.png",
		"005/04_image.json",
		"005/04_image.png",
		"005/04_info.json",
		"005/05.png",
		"005/05_image.json",
		"005/05_image.png",
		"005/05_info.json",
		"006/01.png",
		"006/01_image.json",
		"006/01_image.png",
		"006/01_info.json",
		"006/02.png",
		"006/02_image.json",
		"006/02_image.png",
		"006/02_info.json",
		"006/03.png",
		"006/03_image.json",
		"006/03_image.png",
		"006/03_info.json",
		"006/04.png",
		"006/04_image.json",
		"006/04_image.png",
		"006/04_info.json",
		"006/05.png",
		"006/05_image.json",
		"006/05_image.png",
		"006/05_info.json",
		"007/01.png",
		"007/01_image.json",
		"007/01_image.png",
		"007/01_info.json",
		"007/02.png",
		"007/02_image.json",
		"007/02_image.png",
		"007/02_image10.json",
		"007/02_image10.png",
		"007/02_info.json",
		"007/02_info10.json",
		"007/03.png",
		"007/03_image.json",
		"007/03_image.png",
		"007/03_image9.json",
		"007/03_image9.png",
		"007/03_info.json",
		"007/03_info9.json",
		"007/04.png",
		"007/04_image.json",
		"007/04_image.png",
		"007/04_info.json",
		"007/05.png",
		"007/05_image.json",
		"007/05_image.png",
		"007/05_image14.json",
		"007/05_image14.png",
		"007/05_image22.json",
		"007/05_image22.png",
		"007/05_info.json",
		"007/05_info14.json",
		"007/05_info22.json",
	]
});

pub(super) async fn make(_cache: &Kache, list: &mut CacheList) -> Result<(), CacheListMakingError> {
	// get default area maps
	get_default_areas(list);

	// get event area maps from preset
	// get_event_area_preset(list);

	let start = std::time::Instant::now();
	get_event_area_greedy(_cache, list).await?;
	warn!("Time taken to make map list: {:?}", start.elapsed());

	Ok(())
}

fn get_default_areas(list: &mut CacheList) {
	for p in DEFAULT_MAPS.iter() {
		list.add_unversioned(format!("kcs2/resources/map/{p}"));
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct MapInfoJson {
	spots: Vec<serde_json::Value>,
}

async fn find_in_local_then_remote(
	cache: &Kache,
	p: &str,
) -> Result<Option<tokio::fs::File>, KacheError> {
	let file = match GetOption::new().disable_mod().disable_remote().get(cache, p, NoVersion).await
	{
		Ok(f) => f,
		Err(_) => {
			// check if exist
			if !cache.exists_on_remote(p, NoVersion).await? {
				// not exist
				return Ok(None);
			}
			// fetch from CDN
			GetOption::new().disable_mod().disable_local().get(cache, p, NoVersion).await?
		}
	};

	Ok(Some(file))
}

type EventMapInfo = (i64, i64, Option<Vec<i64>>);

#[allow(unused)]
async fn get_event_area_greedy(
	cache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let mut map_info_set: BTreeSet<EventMapInfo> = BTreeSet::new();

	for event_id in 42..=60 {
		let area_id = format!("{event_id:03}");

		for map in 1..=9 {
			let map_id = format!("{map:02}");
			let mut spots = 0;
			let cover = format!("kcs2/resources/map/{area_id}/{map_id}.png");
			if cache.exists_on_remote(&cover, NoVersion).await? {
				list.add_unversioned(cover);
			} else {
				break;
			}

			let mut info_set: EventMapInfo = (event_id, map, None);
			let mut spot_vec: Vec<i64> = Vec::new();

			loop {
				let suffix = if spots == 0 {
					"".to_string()
				} else {
					format!("{spots}")
				};

				let json_path = format!("kcs2/resources/map/{area_id}/{map_id}_info{suffix}.json");

				let mut file = match find_in_local_then_remote(cache, &json_path).await? {
					Some(f) => f,
					None => break,
				};

				let image_png_path =
					format!("kcs2/resources/map/{area_id}/{map_id}_image{suffix}.png");
				let image_json_path =
					format!("kcs2/resources/map/{area_id}/{map_id}_image{suffix}.json");

				list.add_unversioned(json_path.clone());
				list.add_unversioned(image_png_path);
				list.add_unversioned(image_json_path);

				if spots != 0 {
					spot_vec.push(spots as i64);
				}

				// find suffix
				let mut content = String::new();
				file.read_to_string(&mut content).await?;
				let map_info: MapInfoJson = serde_json::from_str(&content)?;
				spots += map_info.spots.len();
			}

			if !spot_vec.is_empty() {
				info_set.2 = Some(spot_vec);
			}

			map_info_set.insert(info_set);

			if spots == 0 {
				break;
			}
		}
	}

	println!("{:?}", map_info_set);
	Ok(())
}

static EVENT_PRESET: LazyLock<Vec<EventMapInfo>> = LazyLock::new(|| {
	vec![
		(42, 1, None),
		(42, 2, Some(vec![11])),
		(42, 3, Some(vec![22])),
		(42, 4, Some(vec![19, 27, 31])),
		(42, 5, Some(vec![14, 32, 37])),
		(42, 6, None),
		(43, 1, Some(vec![11])),
		(43, 2, Some(vec![18, 23, 36, 37])),
		(43, 3, Some(vec![27, 37])),
		(43, 4, None),
		(44, 1, Some(vec![26])),
		(44, 2, Some(vec![19])),
		(44, 3, Some(vec![22, 28])),
		(44, 4, Some(vec![39])),
		(44, 5, Some(vec![25, 41])),
		(44, 6, None),
		(45, 1, Some(vec![14])),
		(45, 2, Some(vec![21])),
		(45, 3, Some(vec![15, 21, 29])),
		(45, 4, None),
		(46, 1, Some(vec![13])),
		(46, 2, Some(vec![25])),
		(46, 3, Some(vec![25])),
		(46, 4, Some(vec![26])),
		(46, 5, Some(vec![16])),
		(46, 6, Some(vec![25, 33])),
		(46, 7, None),
		(47, 1, Some(vec![23, 24, 29])),
		(47, 2, None),
		(48, 1, Some(vec![16])),
		(48, 2, Some(vec![19])),
		(48, 3, Some(vec![17, 21])),
		(48, 4, Some(vec![18, 26])),
		(48, 5, Some(vec![32, 42])),
		(48, 6, Some(vec![35, 38])),
		(48, 7, Some(vec![37, 50, 60])),
		(48, 8, None),
		(49, 1, Some(vec![13])),
		(49, 2, Some(vec![22, 27])),
		(49, 3, Some(vec![24, 39, 41])),
		(49, 4, Some(vec![26, 34, 42])),
		(49, 5, None),
		(50, 1, Some(vec![15, 16, 24])),
		(50, 2, Some(vec![24, 29])),
		(50, 3, Some(vec![17, 25, 37])),
		(50, 4, Some(vec![17, 41, 50, 59])),
		(50, 5, Some(vec![16, 44, 51])),
		(50, 6, None),
		(51, 1, Some(vec![11, 12, 28])),
		(51, 2, Some(vec![16, 32, 38])),
		(51, 3, Some(vec![21, 33, 50, 54])),
		(51, 4, None),
		(52, 1, Some(vec![18])),
		(52, 2, Some(vec![16, 21])),
		(52, 3, Some(vec![13, 36, 38])),
		(52, 4, None),
		(53, 1, Some(vec![18, 27, 38])),
		(53, 2, Some(vec![20, 38])),
		(53, 3, Some(vec![17, 26, 31])),
		(53, 4, Some(vec![22, 43, 54])),
		(53, 5, Some(vec![13, 28, 43, 49])),
		(53, 6, None),
		(54, 1, Some(vec![16, 18])),
		(54, 2, Some(vec![22, 35])),
		(54, 3, Some(vec![16, 30, 34])),
		(54, 4, Some(vec![13, 20, 34, 42])),
		(54, 5, Some(vec![18, 21, 36, 49, 73])),
		(54, 6, None),
		(55, 1, Some(vec![15])),
		(55, 2, Some(vec![14, 20])),
		(55, 3, Some(vec![13, 28, 36])),
		(55, 4, Some(vec![18, 22, 39])),
		(55, 5, Some(vec![15, 16, 24, 25])),
		(55, 6, Some(vec![8, 16, 32, 38, 44])),
		(55, 7, None),
		(56, 1, Some(vec![21, 25])),
		(56, 2, Some(vec![18, 25, 42])),
		(56, 3, Some(vec![20, 35, 43])),
		(56, 4, Some(vec![18, 24, 30])),
		(56, 5, Some(vec![29, 33, 47, 50])),
		(56, 6, Some(vec![11, 26, 35, 39, 50])),
		(56, 7, None),
		(57, 1, Some(vec![18, 23])),
		(57, 2, Some(vec![23, 36])),
		(57, 3, Some(vec![14, 25])),
		(57, 4, Some(vec![14, 36, 45, 48])),
		(57, 5, Some(vec![16, 25, 33])),
		(57, 6, Some(vec![10, 17, 27, 39])),
		(57, 7, Some(vec![19, 25, 26, 36, 43, 52, 56])),
		(57, 8, None),
		(58, 1, Some(vec![9, 24, 32])),
		(58, 2, Some(vec![15, 28])),
		(58, 3, Some(vec![23, 27, 36])),
		(58, 4, Some(vec![10, 14, 20, 38, 46])),
		(58, 5, None),
		(59, 1, Some(vec![14, 23])),
		(59, 2, Some(vec![9, 26, 29])),
		(59, 3, Some(vec![9, 14, 26, 37, 49])),
		(59, 4, Some(vec![21, 29, 38])),
		(59, 5, Some(vec![10, 22, 41, 50, 55])),
		(59, 6, None),
		(60, 1, Some(vec![15, 22])),
		(60, 2, Some(vec![12, 18, 35])),
		(60, 3, Some(vec![18, 25, 44, 59, 63, 66])),
		(60, 4, None),
	]
});

#[allow(unused)]
fn get_event_area_preset(list: &mut CacheList) {
	for info in EVENT_PRESET.iter() {
		let p = format!("kcs2/resources/map/{0:03}/{1:02}", info.0, info.1);
		list.add_unversioned(format!("{p}.png"))
			.add_unversioned(format!("{p}_image.json"))
			.add_unversioned(format!("{p}_image.png"))
			.add_unversioned(format!("{p}_info.json"));

		if let Some(subs) = &info.2 {
			for sub in subs {
				list.add_unversioned(format!("{p}_image{sub}.json"))
					.add_unversioned(format!("{p}_image{sub}.png"))
					.add_unversioned(format!("{p}_info{sub}.json"));
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::make_list::CacheList;

	use super::get_event_area_preset;

	#[test]
	fn test_preset() {
		let mut list = CacheList::new();
		get_event_area_preset(&mut list);

		println!("{:?}", list);
	}
}
