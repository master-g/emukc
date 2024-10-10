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
	with_enough_stack(async { check().await.unwrap() });
}

async fn get_kache() -> Result<Kache, Box<dyn std::error::Error>> {
	// download all bootstrap files
	let dir = std::path::PathBuf::from(".data");
	let db_path = dir.join("emukc.db");
	// prepare the database
	let db = prepare(&db_path, false).await?;

	let kache = Kache::builder()
		.with_cache_root(std::path::PathBuf::from("z").join("cache"))
		.with_db(std::sync::Arc::new(db))
		.with_proxy(Some("http://127.0.0.1:1086".to_string()))
		.with_gadgets_cdn("203.104.209.7".to_string())
		.with_content_cdn("203.104.209.71".to_string())
		.build()?;

	Ok(kache)
}

#[allow(dead_code)]
async fn test_get() -> Result<(), Box<dyn std::error::Error>> {
	let kache = get_kache().await?;
	kache.get("kcs/sound/kcojkgkujsenly/168525.mp3", Some("27")).await?;
	kache.get("kcs2/resources/ship/character_full/0404_3736.png", None).await?;
	kache.get("gadget_html5/js/kcs_const.js", None).await?;
	Ok(())
}

#[allow(dead_code)]
async fn import() -> Result<(), Box<dyn std::error::Error>> {
	let kache = get_kache().await?;
	import_kccp_cache(&kache, "./z/cache/cached.json", Some("./z/cache")).await?;
	Ok(())
}

#[allow(dead_code)]
async fn check() -> Result<(), Box<dyn std::error::Error>> {
	let kache = get_kache().await.unwrap();
	let (total, invalid, missing) = kache.check_all(true).await?;
	println!("total: {}, invalid: {}, missing: {}", total, invalid, missing);
	Ok(())
}
