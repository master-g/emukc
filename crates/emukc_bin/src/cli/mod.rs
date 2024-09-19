use std::process::ExitCode;

use clap::{Parser, Subcommand};

use emukc_internal::prelude::*;

mod version;

const INFO: &str = r#"
Yet Another Kantai Collection Emulator
"#;

#[derive(Parser, Debug)]
#[command(name = "EmuKC command-line interface and server", bin_name = "emukcd")]
#[command(author, version, about = INFO, before_help = LOGO)]
#[command(disable_version_flag = true, arg_required_else_help = true)]
struct Cli {
	#[arg(help = "Configuration file to use")]
	#[arg(env = "EMUKC_CONFIG", short, long)]
	#[arg(default_value = "emukc.config.toml")]
	#[arg(global = true)]
	config: String,

	#[arg(help = "The logging level")]
	#[arg(env = "EMUKC_LOG_LEVEL", short = 'l', long = "log")]
	#[arg(default_value = "info")]
	#[arg(global = true)]
	log: String,

	#[command(subcommand)]
	command: Option<Commands>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Commands {
	#[command(about = "Print version information")]
	Version,
}

pub async fn init() -> ExitCode {
	let args = Cli::parse();

	// version command is special
	if let Some(Commands::Version) = args.command {
		version::init().await.unwrap();
		return ExitCode::SUCCESS;
	}

	// if let Err(e) = output {
	// 	error!("{}", e);
	// 	ExitCode::FAILURE
	// } else {
	// 	ExitCode::SUCCESS
	// }

	ExitCode::SUCCESS
}
