use std::sync::LazyLock;

use emukc_cache::{Kache, NoVersion};
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

pub(super) async fn make(cache: &Kache, list: &mut CacheList) -> Result<(), CacheListMakingError> {
	// get default area maps
	get_default_areas(list);

	// measure time
	let start = std::time::Instant::now();
	get_event_area(cache, list).await?;
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

async fn get_event_area(cache: &Kache, list: &mut CacheList) -> Result<(), CacheListMakingError> {
	for event_id in 42..=90 {
		let area_id = format!("{event_id:03}");

		for map_id in 1..=9 {
			let mut spots = 0;
			loop {
				let suffix = if spots == 0 {
					"".to_string()
				} else {
					format!("{spots}")
				};

				let json_path =
					format!("kcs2/resources/map/{area_id}/{map_id:02}_info{suffix}.json");
				if !cache.exists_on_remote(&json_path, NoVersion).await? {
					break;
				}
				let image_png_path =
					format!("kcs2/resources/map/{area_id}/{map_id:02}_image{suffix}.png");
				let image_json_path =
					format!("kcs2/resources/map/{area_id}/{map_id:02}_image{suffix}.json");

				list.add_unversioned(json_path.clone());
				list.add_unversioned(image_png_path);
				list.add_unversioned(image_json_path);

				// find suffix
				let mut file = cache.get(&json_path, NoVersion).await?;
				let mut content = String::new();
				file.read_to_string(&mut content).await?;
				let map_info: MapInfoJson = serde_json::from_str(&content)?;
				spots += map_info.spots.len();
			}
		}
	}
	Ok(())
}
