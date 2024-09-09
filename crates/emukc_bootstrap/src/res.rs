//! Resource list for the bootstrap.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ResourceCategory {
	Start2,
	MainJS,
	KccpQuests,
	KcData,
	KcWikiSlotItem,
	ShipsNedb,
	TsunKitQuests,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resource<'a> {
	pub category: ResourceCategory,
	pub url: &'a str,
	pub save_as: &'a str,
	pub unzip_to: Option<&'a str>,
}

pub static RES_LIST: LazyLock<Vec<Resource<'static>>> = LazyLock::new(|| {
	vec![
		Resource {
			category: ResourceCategory::Start2,
			url: "http://api.kcwiki.moe/start2",
			save_as: "start2.json",
			unzip_to: None,
		},
		Resource {
			category: ResourceCategory::MainJS,
			url: "https://github.com/kcwiki/kancolle-main/raw/master/dist/main.js",
			save_as: "main.js",
			unzip_to: None,
		},
		Resource {
			category: ResourceCategory::KccpQuests,
			url: "https://raw.githubusercontent.com/Oradimi/KanColle-English-Patch-KCCP/master/EN-patch/kcs2/js/main.js/ignore-raw_text_translations/ignore-_quests.json",
			save_as: "kccp_quests.json",
			unzip_to: None,
		},
		Resource {
			category: ResourceCategory::KcData,
			url: "https://github.com/kcwikizh/kcdata/archive/refs/heads/gh-pages.zip",
			save_as: "kc_data.zip",
			unzip_to: Some("kc_data"),
		},
		Resource {
			category: ResourceCategory::KcWikiSlotItem,
			url: "https://github.com/kcwiki/kancolle-data/raw/master/db/equipment.json",
			save_as: "kcwiki_slotitem.json",
			unzip_to: None,
		},
		Resource {
			category: ResourceCategory::ShipsNedb,
			url: "https://raw.githubusercontent.com/kcwikizh/WhoCallsTheFleet-DB/master/db/ships.nedb",
			save_as: "ships.nedb",
			unzip_to: None,
		},
		Resource {
			category: ResourceCategory::TsunKitQuests,
			url: "https://raw.githubusercontent.com/planetarian/TsunKitQuests/main/quests.json",
			save_as: "tsunkit_quests.json",
			unzip_to: None,
		}
	]
});

#[cfg(test)]
mod test {
	#[test]
	fn test_res_list() {
		super::RES_LIST.iter().for_each(|res| {
			println!("{:?}", res);
		});
	}
}
