use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{cfg::AppConfig, state};

#[derive(Args, Debug)]
pub(super) struct DumpArguments {
	#[arg(help = "Output file path")]
	#[arg(long)]
	pub output: Option<String>,

	#[arg(help = "Overwrite existing file")]
	#[arg(long)]
	pub overwrite: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Entry {
	path: String,
	version: Option<String>,
}

/// Dump cache resources file list
pub(super) async fn exec(args: &DumpArguments, config: &AppConfig) -> Result<()> {
	let state = state::State::new(config, false).await?;
	let kache = &state.kache;
	let output = match &args.output {
		Some(path) => PathBuf::from(path),
		None => config.cache_root.join("cache_resources.nedb"),
	};

	if output.exists() && !args.overwrite {
		return Err(anyhow::anyhow!("File already exists"));
	}

	let mut file =
		tokio::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&output).await?;

	let list = kache.export().await?;
	for (path, version) in list {
		let entry = Entry {
			path,
			version,
		};
		let line = serde_json::to_string(&entry)?;

		// append line to file
		file.write_all(line.as_bytes()).await?;
		file.write_all(b"\n").await?;
	}

	file.flush().await?;
	file.shutdown().await?;

	Ok(())
}
