//! Test the configuration loading.

use emukc_bin::cfg::AppConfig;
use emukc_bin::state::State;
use emukc_internal::app::with_enough_stack;
use emukc_internal::prelude::new_log_builder;

fn main() {
	// initialize logger
	let _guard =
		new_log_builder().with_log_level("trace").with_source_file().with_line_number().build();

	with_enough_stack(async {
		load_config().await;
	})
}

async fn load_config() {
	let config = AppConfig::load("../../emukc.config.toml").unwrap();
	println!("{:?}", config);

	let _state = State::new(&config).await.unwrap();
}
