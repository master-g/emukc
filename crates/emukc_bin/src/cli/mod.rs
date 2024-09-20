use std::process::ExitCode;

use clap::{Parser, Subcommand};

use emukc_internal::prelude::*;

use crate::{cfg::AppConfig, state::State};

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

	// load configuration
	let cfg = match AppConfig::load(&args.config) {
		Ok(cfg) => cfg,
		Err(e) => {
			error!("Failed to load configuration: {}", e);
			return ExitCode::FAILURE;
		}
	};

	// initialize logging
	let Some(_guard) = new_log_builder()
		.with_log_level(&args.log)
		.with_source_file()
		.with_line_number()
		.with_file_appender(cfg.workspace_root.join("logs"))
		.build()
	else {
		error!("Failed to initialize logging");
		return ExitCode::FAILURE;
	};

	// prepare the application state
	let _state = match State::new(&cfg).await {
		Ok(state) => state,
		Err(e) => {
			error!("Failed to initialize application state: {}", e);
			return ExitCode::FAILURE;
		}
	};

	// if let Err(e) = output {
	// 	error!("{}", e);
	// 	ExitCode::FAILURE
	// } else {
	// 	ExitCode::SUCCESS
	// }

	ExitCode::SUCCESS
}
