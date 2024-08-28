//! An example of downloading bootstrap files

use emukc::{
	model::{kc2::navy::KcNavy, profile::material::MaterialConfig},
	prelude::*,
};

fn main() {
	let _guard = new_log_builder()
		.with_log_level("debug")
		.with_source_file()
		.with_line_number()
		.with_file_appender(std::path::PathBuf::from(".data/.emukc.log"))
		.build()
		.unwrap();

	with_enough_stack(async {
		let mut dir = std::path::PathBuf::from(".data");
		let save_codex_to = dir.join("codex");
		dir.push("temp");
		download_all(&dir, false, Some("http://127.0.0.1:1086")).await.unwrap();

		let partial_codex = parse_partial_codex(dir).unwrap();
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

		codex.save(save_codex_to, true).unwrap();
	});
}
