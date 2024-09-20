//! Test the configuration loading.

use emukc_bin::cfg::AppConfig;
use emukc_bin::state::State;
use emukc_internal::app::with_enough_stack;

fn main() {
	with_enough_stack(async {
		load_config().await;
	})
}

async fn load_config() {
	let config = AppConfig::load("../../emukc.config").unwrap();
	println!("{:?}", config);

	let state = State::new(&config).await.unwrap();
	println!("{:?}", state);
}
