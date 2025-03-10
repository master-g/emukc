use anyhow::Result;
use clap::Args;

use crate::{cfg::AppConfig, state::State};
use emukc_internal::prelude::{AccountOps, ProfileOps};

/// Bootstrap command arguments
#[derive(Args, Debug)]
pub struct NewSessionArgs {
	#[arg(help = "user name")]
	#[arg(long)]
	name: String,

	#[arg(help = "password")]
	#[arg(long)]
	pass: String,

	#[arg(help = "do not start the server")]
	#[arg(long)]
	pub no_start: bool,
}

pub async fn exec(args: &NewSessionArgs, cfg: &AppConfig, state: &State) -> Result<()> {
	let info = state.sign_in(&args.name, &args.pass).await?;
	let session = state.start_game(&info.access_token.token, 1).await?;
	let port = cfg.bind.port();

	let url = format!("http://localhost:{port}/emukc?api_token={}", session.session.token);
	println!("{}", url);

	// open the url in the default browser
	open::that(url).unwrap();

	Ok(())
}
