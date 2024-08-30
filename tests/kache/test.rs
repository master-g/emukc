//! An example of downloading bootstrap files

use emukc::prelude::*;

fn main() {
	// initialize logger
	let _guard = new_log_builder()
		.with_log_level("trace")
		.with_source_file()
		.with_line_number()
		.with_file_appender(std::path::PathBuf::from(".data/.emukc.log"))
		.build()
		.unwrap();

	// run the async block
	with_enough_stack(async {
		// download all bootstrap files
		let dir = std::path::PathBuf::from(".data");
		let db_path = dir.join("emukc.db");
		// prepare the database
		let db = prepare(&db_path, false).await.unwrap();

		let kache = Kache::builder()
			.with_cache_root(dir.join("cache"))
			.with_db(std::sync::Arc::new(db))
			.with_proxy("http://127.0.0.1:1086".to_string())
			.with_gadgets_cdn("203.104.209.7".to_string())
			.with_content_cdn("203.104.209.71".to_string())
			.build()
			.unwrap();

		kache.get("kcs/sound/kcojkgkujsenly/168525.mp3", Some("27")).await.unwrap();
		kache.get("kcs2/resources/ship/character_full/0404_3736.png", None).await.unwrap();
		kache.get("gadget_html5/js/kcs_const.js", None).await.unwrap();
	});
}
