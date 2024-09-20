use clap::Args;
use emukc_internal::prelude::{download_all, parse_partial_codex};

use crate::cfg::AppConfig;

/// Bootstrap command arguments
#[derive(Args, Debug)]
pub(super) struct BootstrapArgs {
	#[arg(help = "Overwrite existing files")]
	#[arg(short, long)]
	overwrite: bool,

	#[arg(help = "use another proxy")]
	#[arg(long)]
	proxy: Option<String>,

	#[arg(help = "specify output directory")]
	#[arg(long)]
	output: Option<String>,
}

/// Execute the bootstrap command
pub(super) async fn exec(
	cfg: &AppConfig,
	args: &BootstrapArgs,
) -> Result<(), Box<dyn std::error::Error>> {
	let proxy = cfg.proxy.as_deref().or(args.proxy.as_deref());
	let output = if let Some(output) = &args.output {
		std::path::PathBuf::from(output)
	} else {
		cfg.temp_root()?
	};

	// download files needed for constructing the codex
	download_all(&output, args.overwrite, proxy).await?;

	// parse the codex
	let codex = parse_partial_codex(&output)?;

	// save the codex
	let codex_root = cfg.codex_root()?;
	codex.save(&codex_root, args.overwrite)?;

	Ok(())
}
