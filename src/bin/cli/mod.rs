use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

use emukc_internal::prelude::*;
use serve::ServeArgs;

use crate::{cfg::AppConfig, state::State};

mod auto;
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
#[command(disable_version_flag = true)] // , arg_required_else_help = true)]
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

	#[command(about = "Start a game session with given username and password")]
	NewSession(dev::NewSessionArgs),

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
	State::new(cfg, false)
		.await
		.inspect_err(|e| {
			error!("Failed to prepare application state: {}", e);
		})
		.ok()
}

fn find_config_file(arg: &str) -> anyhow::Result<PathBuf> {
	let default_path = PathBuf::from(arg);
	if default_path.exists() {
		return Ok(default_path);
	}

	let pwd = std::env::current_dir()?;
	let config_path = pwd.join(default_path);
	if config_path.exists() {
		return Ok(config_path);
	}

	let exe_path = std::env::current_exe()?;
	let exe_dir = exe_path.parent().unwrap();
	let config_path = exe_dir.join(arg);
	if config_path.exists() {
		return Ok(config_path);
	}

	let env_path = std::env::var("EMUKC_CONFIG")?;
	let env_path = PathBuf::from(env_path);
	if env_path.exists() {
		return Ok(env_path);
	}

	Err(anyhow::anyhow!(
		"Configuration file not found at '{}', '{}', or '{}'",
		arg,
		pwd.display(),
		exe_dir.display()
	))
}

pub async fn init() -> ExitCode {
	let args = Cli::parse();

	// version command is special
	if let Some(Commands::Version) = args.command {
		version::init().await.unwrap();
		return ExitCode::SUCCESS;
	}

	// load configuration
	let cfg_path = match find_config_file(&args.config) {
		Ok(cfg_path) => {
			debug!("Using configuration file at '{}'", cfg_path.display());
			cfg_path
		}
		Err(e) => {
			eprintln!("Configuration file not found, err'{e}'");
			return ExitCode::FAILURE;
		}
	};

	let cfg = match AppConfig::load(cfg_path.to_string_lossy().as_ref()) {
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
			eprintln!("Failed to get log directory: {e}");
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
		Some(Commands::Nuke) => dev::nuke::exec(&cfg).await,
		Some(Commands::NewSession(args)) => {
			let Some(state) = prepare_state(&cfg).await else {
				eprintln!("Failed to prepare application state");
				return ExitCode::FAILURE;
			};
			if let Err(e) = dev::new_session::exec(&args, &cfg, &state).await {
				eprintln!("Failed to start new session: {e}");
				return ExitCode::FAILURE;
			}
			if !args.no_start {
				serve::exec(
					&ServeArgs {
						no_banner: true,
					},
					&cfg,
					&state,
				)
				.await
			} else {
				Ok(())
			}
		}
		Some(Commands::Bootstrap(args)) => bootstrap::exec(&cfg, &args).await,
		Some(Commands::Cache(args)) => cache::exec(&args, &cfg).await,
		Some(Commands::Serve(args)) => {
			let Some(state) = prepare_state(&cfg).await else {
				return ExitCode::FAILURE;
			};
			serve::exec(&args, &cfg, &state).await
		}
		None => auto::exec(&cfg).await,
		_ => Ok(()),
	};

	if let Err(e) = output {
		error!("{}", e);
		ExitCode::FAILURE
	} else {
		ExitCode::SUCCESS
	}
}
