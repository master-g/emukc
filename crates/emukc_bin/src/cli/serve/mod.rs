use anyhow::Result;
use clap::Args;
use tokio_util::sync::CancellationToken;

use emukc_internal::app::cst::LOGO;

use crate::{cfg::AppConfig, net, state::State};

#[derive(Args, Debug)]
pub(super) struct ServeArgs {
	#[arg(help = "Whether to hide the startup banner")]
	#[arg(env = "EMUKC_NO_BANNER", long)]
	#[arg(default_value_t = false)]
	no_banner: bool,
}

pub(super) async fn exec(args: &ServeArgs, cfg: &AppConfig, state: &State) -> Result<()> {
	if !args.no_banner {
		println!("{LOGO}");
	}

	let ct = CancellationToken::new();
	net::run(ct, cfg, state).await?;

	Ok(())
}
