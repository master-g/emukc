use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::net;
use crate::state::State;
use crate::{cfg::AppConfig, state};
use emukc_internal::app::cst::LOGO;
use emukc_internal::prelude::{AccountOps, ProfileOps};

use super::bootstrap;

pub(super) async fn exec(config: &AppConfig) -> Result<()> {
	let state = prepare_state(config).await?;
	start(config, state).await?;

	Ok(())
}

async fn prepare_state(cfg: &AppConfig) -> Result<state::State> {
	let result = state::State::new(cfg).await;
	if result.is_ok() {
		return result;
	}

	let original_err = result.unwrap_err();
	warn!("Failed to load app state: {}", original_err);

	let should_create =
		inquire::Confirm::new("Cannot load app state, do you want to create a new one?")
			.with_default(true)
			.prompt()
			.unwrap_or(false);

	if should_create {
		// new state
		bootstrap::exec(
			cfg,
			&bootstrap::BootstrapArgs {
				overwrite: true,
				force_update: true,
				proxy: None,
				output: None,
			},
		)
		.await
		.map_err(|e| anyhow::anyhow!("Failed to create app state: {}", e))?;

		let state = state::State::new(cfg)
			.await
			.map_err(|e| anyhow::anyhow!("Failed to load app state after creation: {}", e))?;

		// prepare cache list and download resources
		prepare_resources(cfg, &state).await?;

		Ok(state)
	} else {
		Err(anyhow::anyhow!("User declined to create a new app state"))
	}
}

const OPT_0: &str = "Yes, but do a minimal download";
const OPT_1: &str = "Yes, do a full download";
const OPT_2: &str = "No, thanks";

async fn prepare_resources(cfg: &AppConfig, state: &State) -> Result<()> {
	let select = inquire::Select::new(
		"Would you like to pre-download game resources?",
		vec![OPT_0, OPT_1, OPT_2],
	)
	.with_help_message("A full download may take a long time, around 6GB of data.")
	.prompt()
	.map_err(|e| anyhow::anyhow!("Failed to ask user about downloading resources: {}", e))?;
	let strategy = match select {
		OPT_0 => emukc::bootstrap::prelude::CacheListMakeStrategy::Minimal,
		OPT_1 => emukc::bootstrap::prelude::CacheListMakeStrategy::Default,
		_ => return Ok(()),
	};

	// prepare cache list
	let cache_list_path =
		cfg.cache_root.join("cache_resources.nedb").to_string_lossy().into_owned();
	emukc_internal::bootstrap::prelude::make_cache_list(
		&state.codex.manifest,
		&state.kache,
		&cache_list_path,
		strategy,
		true,
	)
	.await
	.map_err(|e| anyhow::anyhow!("Failed to create cache resources list: {}", e))?;

	let help_msg = if strategy == emukc::bootstrap::prelude::CacheListMakeStrategy::Minimal {
		"This might take a while, please wait. The download will be minimal."
	} else {
		"WARNNING! This may take a long time, around 6GB of data."
	};

	// download resources now
	let download_now = inquire::Confirm::new(
		"Cache resources list created successfully, do you want to download them now?",
	)
	.with_help_message(help_msg)
	.with_default(false)
	.prompt()
	.map_err(|e| anyhow::anyhow!("Failed to ask user about downloading resources: {}", e))?;

	if download_now {
		let kache = state.kache.clone();
		emukc_internal::bootstrap::prelude::populate(kache, &cache_list_path, 16)
			.await
			.map_err(|e| anyhow::anyhow!("Failed to download cache resources: {}", e))?;
	}

	Ok(())
}

const DEFAULT_NAME: &str = "admin";
const DEFAULT_PASS: &str = "1234567";

async fn start(cfg: &AppConfig, state: state::State) -> Result<()> {
	let info = match state.sign_in(DEFAULT_NAME, DEFAULT_PASS).await {
		Ok(info) => info,
		Err(_) => {
			let info = state.sign_up(DEFAULT_NAME, DEFAULT_PASS).await?;
			state.new_profile(&info.access_token.token, DEFAULT_NAME).await?;
			info
		}
	};

	let session = state.start_game(&info.access_token.token, 1).await?;
	let port = cfg.bind.port();

	let url = format!("http://localhost:{port}/emukc?api_token={}", session.session.token);
	println!("{}", url);

	// start server in another thread
	let server_cfg = cfg.clone();
	let server_task = tokio::spawn(async move {
		println!("{LOGO}");
		let ct = CancellationToken::new();
		if let Err(e) = net::run(ct, &server_cfg, &state).await {
			eprintln!("Server error: {}", e);
		}
	});

	// delay for a bit to allow the server to start
	tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

	// open the url in the default browser
	open::that(url).unwrap();

	// wait for the server to finish
	server_task.await?;

	Ok(())
}
