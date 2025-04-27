use std::sync::LazyLock;

use emukc_cache::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{
	make_list::CacheList,
	prelude::{CacheListMakeStrategy, CacheListMakingError},
};

static MAP_ID_LIST: LazyLock<&[&str]> = LazyLock::new(|| {
	&[
		"00105", "00106", "00205", "00305", "00404", "00405", "00502", "00503", "00504", "00505",
		"00602", "00603", "00604", "00605", "00701", "00702", "00702_2", "00703", "00703_2",
		"00704", "00705", "00705_2", "00705_3",
	]
});

static EVENT_MAP_ID_LIST: LazyLock<&[&str]> = LazyLock::new(|| {
	&[
		"03801", "03802", "03803", "03804", "03805", "03901", "04001", "04101", "04201", "04301",
		"04301_2", "04302", "04302_2", "04303", "04303_2", "04401", "04402", "04402_2", "04403",
		"04403_2", "04404", "04405", "04405_2", "04501", "04502", "04502_2", "04503", "04503_2",
		"04601", "04601_2", "04602", "04701", "04701_2", "04701_3", "04801", "04801_2", "04802",
		"04802_2", "04803", "04804", "04804_2", "04804_3", "04805", "04805_2", "04806", "04806_2",
		"04807", "04807_2", "04807_3", "04901", "04901_2", "04902", "04902_2", "04903", "04903_2",
		"04903_3", "04904", "04904_2", "04904_3", "05001", "05001_2", "05001_3", "05002",
		"05002_2", "05002_3", "05003", "05003_2", "05003_3", "05004", "05004_2", "05004_3",
		"05004_4", "05005", "05005_2", "05005_3", "05101", "05101_2", "05101_3", "05102",
		"05102_2", "05102_3", "05103", "05103_2", "05103_3", "05103_4", "05201", "05201_2",
		"05202", "05202_2", "05203", "05203_2", "05203_3", "05301", "05302", "05302_2", "05303",
		"05303_2", "05303_3", "05304", "05304_2", "05304_3", "05305", "05305_2", "05305_3",
		"05401", "05402", "05402_2", "05402_3", "05403", "05403_2", "05403_3", "05404", "05404_2",
		"05404_3", "05405", "05405_2", "05405_3", "05405_4", "05501", "05502", "05502_2", "05503",
		"05503_2", "05504", "05504_2", "05505", "05505_2", "05505_3", "05505_4", "05506",
		"05506_2", "05506_3", "05506_4", "05601", "05601_2", "05602", "05602_2", "05602_3",
		"05603", "05603_2", "05603_3", "05604", "05604_2", "05605", "05605_2", "05605_3", "05606",
		"05606_2", "05606_3", "05606_4", "05701", "05701_2", "05702", "05702_2", "05703",
		"05703_2", "05703_3", "05704", "05704_2", "05704_3", "05705", "05705_2", "05705_3",
		"05706", "05706_2", "05706_3", "05707", "05707_2", "05707_3", "05707_4", "05707_5",
		"05801", "05801_2", "05802", "05802_2", "05803", "05803_2", "05803_3", "05804", "05804_2",
		"05804_3", "05804_4", "05901", "05902", "05902_2", "05902_3", "05903", "05903_2",
		"05903_3", "05903_4", "05904", "05904_2", "05904_3", "05905", "05905_2", "05905_3",
		"05905_4", "06001", "06002", "06002_2", "06003", "06003_2", "06003_3", "06004", "06004_2",
		"06004_3", "06005_2", "06005_3", "06006", "06006_2", "06006_3",
	]
});

#[derive(Debug, Serialize, Deserialize)]
struct GaugeConfig {
	img: String,
	vertical: VerticalConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerticalConfig {
	img: String,
}

pub(super) async fn make(
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	if strategy == CacheListMakeStrategy::Minimal {
		return Ok(());
	}

	for id in *MAP_ID_LIST {
		make_gauge_by_id(cache, id, list).await?;
	}

	for id in *EVENT_MAP_ID_LIST {
		make_gauge_by_id(cache, id, list).await?;

		// try to find more
		// for i in 2..=9 {
		// 	let new_id = format!("{id}_{i}");
		// 	if crawl_gauge_by_id(cache, &new_id).await.is_err() {
		// 		break;
		// 	}
		// }
	}

	// for id in ["06007", "06008"].into_iter() {
	// 	for i in 2..=9 {
	// 		let new_id = format!("{id}_{i}");
	// 		if make_gauge_by_id(cache, &new_id, list).await.is_err() {
	// 			break;
	// 		}
	// 	}
	// }

	// try to find more
	// for area_id in 60..=61 {
	// 	for map_id in 1..=10 {
	// 		let magic = format!("{area_id:03}{map_id:02}");
	// 		if crawl_gauge_by_id(cache, &magic).await.is_err() {
	// 			break;
	// 		}
	// 	}
	// }

	Ok(())
}

async fn make_gauge_by_id(
	cache: &Kache,
	id: &str,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let p = format!("kcs2/resources/gauge/{id}.json");
	let mut json_file = GetOption::new_non_mod().get(cache, &p, NoVersion).await?;
	list.add_unversioned(p);
	let mut raw = String::new();
	json_file.read_to_string(&mut raw).await.map_err(|e| KacheError::InvalidFile(e.to_string()))?;

	let config: GaugeConfig =
		serde_json::from_str(&raw).map_err(|e| KacheError::InvalidFile(e.to_string()))?;

	for img in [config.img, config.vertical.img] {
		list.add_unversioned(format!("kcs2/resources/gauge/{img}.png"));
		list.add_unversioned(format!("kcs2/resources/gauge/{img}_light.png"));
	}

	Ok(())
}
