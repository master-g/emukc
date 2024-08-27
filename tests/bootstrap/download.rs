//! An example of downloading bootstrap files

use emukc::prelude::*;

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
		dir.push("temp");
		download_all(dir, false, Some("http://127.0.0.1:1086")).await.unwrap();
	});
}
