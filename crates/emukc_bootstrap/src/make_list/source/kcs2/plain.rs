use std::sync::LazyLock;

use emukc_cache::prelude::*;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{make_list::CacheList, prelude::CacheListMakingError};

static PLAIN_RES: LazyLock<&[&str]> = LazyLock::new(|| {
	&[
		"css/fonts.css",
		"js/world.js",
		"hc.html",
		"world.html",
		"resources/font/A-OTF-UDShinGoPro-Light.woff2",
		"resources/font/A-OTF-UDShinGoPro-Regular.woff2",
	]
});

static VERSION_REGEX: LazyLock<regex::Regex> =
	LazyLock::new(|| regex::Regex::new(r#"VersionInfo\.scriptVesion\s*=\s*"([^"]+)";"#).unwrap());

pub(super) async fn make(cache: &Kache, list: &mut CacheList) -> Result<(), CacheListMakingError> {
	for res in PLAIN_RES.iter() {
		list.add_unversioned(format!("kcs2/{}", res));
	}

	let mainjs_ver = parse_main_js_version(cache).await?;
	list.add(format!("kcs2/main.js"), &mainjs_ver);

	Ok(())
}

async fn parse_main_js_version(cache: &Kache) -> Result<String, KacheError> {
	let mainjs = cache.get("gadget_html5/js/kcs_const.js", "").await?;

	let reader = BufReader::new(mainjs);
	let mut lines = reader.lines();

	while let Some(line) = lines.next_line().await? {
		// check if contains version info
		if let Some(captures) = VERSION_REGEX.captures(&line) {
			let version = captures.get(1).unwrap().as_str();
			return Ok(version.to_string());
		}
	}

	Err(KacheError::InvalidFile("kcs_const.js has no version info".to_string()))
}
