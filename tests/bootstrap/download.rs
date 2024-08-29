//! An example of downloading bootstrap files

use emukc::{
	model::{kc2::navy::KcNavy, profile::material::MaterialConfig},
	prelude::*,
};
use memory_stats::memory_stats;

fn print_memory_usage() {
	if let Some(usage) = memory_stats() {
		let physical_memory_in_mb = usage.physical_mem as f64 / 1024.0 / 1024.0;
		println!("Physical memory usage: {:.2} MB", physical_memory_in_mb);
	} else {
		println!("Failed to get memory usage");
	}
}

fn main() {
	// initialize logger
	let _guard = new_log_builder()
		.with_log_level("error")
		.with_source_file()
		.with_line_number()
		.with_file_appender(std::path::PathBuf::from(".data/.emukc.log"))
		.build()
		.unwrap();

	// run the async block
	with_enough_stack(async {
		// download all bootstrap files
		let mut dir = std::path::PathBuf::from(".data");
		let save_codex_to = dir.join("codex");
		let db_path = dir.join("emukc.db");
		dir.push("temp");
		download_all(&dir, false, Some("http://127.0.0.1:1086")).await.unwrap();

		print_memory_usage();

		// parse the partial codex
		let partial_codex = parse_partial_codex(dir).unwrap();

		// create the codex
		let codex = Codex {
			manifest: partial_codex.manifest,
			ship_basic: partial_codex.ship_basic,
			ship_class_name: partial_codex.ship_class_name,
			ship_extra_info: partial_codex.ship_extra_info,
			slotitem_extra_info: partial_codex.slotitem_extra_info,
			ship_remodel_info: partial_codex.ship_remodel_info,
			quest: partial_codex.quest,
			ship_extra_voice: Kc3rdShipVoiceMap::new(),
			navy: KcNavy::default(),
			material_cfg: MaterialConfig::default(),
		};

		// save
		codex.save(&save_codex_to, true).unwrap();

		// test load
		let _codex = Codex::load(save_codex_to).unwrap();

		// prepare the database
		let _db = prepare(&db_path, false).await.unwrap();

		print_memory_usage();
	});
}
