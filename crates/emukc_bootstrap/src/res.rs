//! Resource list for the bootstrap.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[allow(unused)]
#[derive(Serialize, Deserialize, Debug)]
pub enum ResourceCategory {
	Start2,
	KccpQuests,
	KcData,
	KcWikiSlotItem,
	KcWikiShip,
	KcWikiUseItem,
	ShipsNedb,
	TsunKitQuests,
}

#[derive(Debug)]
pub struct Resource<'a> {
	pub url: &'a str,
	pub save_as: &'a str,
	pub unzip_to: Option<&'a str>,
}

pub static RES_LIST: LazyLock<Vec<Resource<'static>>> = LazyLock::new(|| {
	vec![
		Resource {
			// category: ResourceCategory::Start2,
			url: "http://api.kcwiki.moe/start2",
			save_as: "start2.json",
			unzip_to: None,
		},
		Resource {
			// category: ResourceCategory::KccpQuests,
			url: "https://raw.githubusercontent.com/Oradimi/KanColle-English-Patch-KCCP/master/EN-patch/kcs2/js/main.js/ignore-raw_text_translations/ignore-_quests.json",
			save_as: "kccp_quests.json",
			unzip_to: None,
		},
		Resource {
			// category: ResourceCategory::KcData,
			url: "https://github.com/kcwikizh/kcdata/archive/refs/heads/gh-pages.zip",
			save_as: "kc_data.zip",
			unzip_to: Some("kc_data"),
		},
		Resource {
			// category: ResourceCategory::KcWikiSlotItem,
			url: "https://raw.githubusercontent.com/kcwiki/kancolle-data/refs/heads/master/wiki/equipment.json",
			save_as: "kcwiki_slotitem.json",
			unzip_to: None,
		},
		Resource {
			// category: ResourceCategory::KcWikiShip,
			url: "https://raw.githubusercontent.com/kcwiki/kancolle-data/refs/heads/master/wiki/ship.json",
			save_as: "kcwiki_ship.json",
			unzip_to: None,
		},
		Resource {
			// category: ResourceCategory::KcWikiUseItem,
			url: "https://raw.githubusercontent.com/kcwiki/kancolle-data/refs/heads/master/wiki/item.json",
			save_as: "kcwiki_useitem.json",
			unzip_to: None,
		},
		Resource {
			// category: ResourceCategory::ShipsNedb,
			url: "https://raw.githubusercontent.com/kcwikizh/WhoCallsTheFleet-DB/master/db/ships.nedb",
			save_as: "ships.nedb",
			unzip_to: None,
		},
		Resource {
			// category: ResourceCategory::TsunKitQuests,
			url: "https://raw.githubusercontent.com/planetarian/TsunKitQuests/main/quests.json",
			save_as: "tsunkit_quests.json",
			unzip_to: None,
		},
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
