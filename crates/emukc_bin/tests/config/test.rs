//! Test the configuration loading.

use emukc_bin::cfg::AppConfig;
use emukc_bin::state::State;
use emukc_internal::app::with_enough_stack;
use emukc_internal::prelude::{new_log_builder, AccountOps, ProfileOps};

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

	let state = State::new(&config).await.unwrap();

	let sign = state.sign_in("admin", "1234567").await.unwrap();
	let profile = state.start_game(&sign.access_token.token, 1).await.unwrap();

	println!("{:?}", profile);
}
