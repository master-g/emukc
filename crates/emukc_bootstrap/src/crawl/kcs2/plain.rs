use std::sync::LazyLock;

use emukc_cache::kache;
use tokio::io::{AsyncBufReadExt, BufReader};

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

pub(super) async fn crawl_kcs2_plain(cache: &kache::Kache) -> Result<(), kache::Error> {
	for res in PLAIN_RES.iter() {
		cache.get(format!("kcs2/{}", res).as_str(), None).await?;
	}

	let mainjs_ver = parse_main_js_version(cache).await?;
	cache.get("kcs2/js/main.js", Some(&mainjs_ver)).await?;

	Ok(())
}

async fn parse_main_js_version(cache: &kache::Kache) -> Result<String, kache::Error> {
	let mainjs = cache.get("gadget_html5/js/kcs_const.js", None).await?;

	let reader = BufReader::new(mainjs);
	let mut lines = reader.lines();

	while let Some(line) = lines.next_line().await? {
		// check if contains version info
		if let Some(captures) = VERSION_REGEX.captures(&line) {
			let version = captures.get(1).unwrap().as_str();
			return Ok(version.to_string());
		}
	}

	Err(kache::Error::InvalidFile("kcs_const.js has no version info".to_string()))
}
