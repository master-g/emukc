use std::process::ExitCode;

use clap::{Parser, Subcommand};

use emukc_internal::prelude::*;

use crate::{cfg::AppConfig, state::State};

mod bootstrap;
mod cache;
mod dev;
mod serve;
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
	#[command(about = "Remove game database and create a new account and profile")]
	Nuke,

	#[command(about = "Prepare the bootstrap files")]
	Bootstrap(bootstrap::BootstrapArgs),

	#[command(about = "Cache management")]
	Cache(cache::CacheArgs),

	#[command(about = "Start the server")]
	Serve(serve::ServeArgs),

	#[command(about = "Print version information")]
	Version,
}

// prepare the application state
async fn prepare_state(cfg: &AppConfig) -> Option<State> {
	State::new(cfg)
		.await
		.inspect_err(|e| {
			error!("Failed to prepare application state: {}", e);
		})
		.ok()
}

pub async fn init() -> ExitCode {
	let args = Cli::parse();

	// version command is special
	if let Some(Commands::Version) = args.command {
		version::init().await.unwrap();
		return ExitCode::SUCCESS;
	}

	println!("{}", std::env::current_exe().unwrap().to_str().unwrap());

	// load configuration
	let cfg = match AppConfig::load(&args.config) {
		Ok(cfg) => cfg,
		Err(e) => {
			eprintln!("Failed to load configuration at '{}', err: {}", &args.config, e);
			return ExitCode::FAILURE;
		}
	};

	// initialize logging
	let log_dir = match cfg.log_root() {
		Ok(log_dir) => log_dir,
		Err(e) => {
			eprintln!("Failed to get log directory: {}", e);
			return ExitCode::FAILURE;
		}
	};
	let Some(_guard) = new_log_builder()
		.with_log_level(&args.log)
		.with_source_file()
		.with_line_number()
		.with_file_appender(log_dir)
		.build()
	else {
		eprintln!("Failed to initialize logging");
		return ExitCode::FAILURE;
	};

	let output = match args.command {
		Some(Commands::Nuke) => dev::exec(&cfg).await,
		Some(Commands::Bootstrap(args)) => bootstrap::exec(&cfg, &args).await,
		Some(Commands::Cache(args)) => cache::exec(&args, &cfg).await,
		Some(Commands::Serve(args)) => {
			let Some(state) = prepare_state(&cfg).await else {
				return ExitCode::FAILURE;
			};
			serve::exec(&args, &cfg, &state).await
		}
		_ => Ok(()),
	};

	if let Err(e) = output {
		error!("{}", e);
		ExitCode::FAILURE
	} else {
		ExitCode::SUCCESS
	}
}
