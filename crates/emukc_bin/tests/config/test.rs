//! Test the configuration loading.

use emukc_bin::cfg::AppConfig;
use emukc_internal::app::with_enough_stack;

fn main() {
	with_enough_stack(async {
		load_config().await;
	})
}

async fn load_config() {
	let config = AppConfig::load("../../emukc.config").await.unwrap();

	println!("{:?}", config);
}
