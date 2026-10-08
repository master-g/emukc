use anyhow::{Result, bail};
use clap::Args;
use emukc_internal::gameplay::prelude::{PRESETS, Preset, apply_scenario};

use crate::{cfg::AppConfig, state::State};

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

    #[arg(help = "do not open the browser")]
    #[arg(long)]
    no_open: bool,

    #[arg(
        help = "scenario preset applied to a newly created profile, which also skips the tutorial"
    )]
    #[arg(long)]
    scenario: Option<String>,
}

/// `api_tutorial_progress` once every tutorial step has been shown.
const TUTORIAL_DONE: i64 = 100;

pub async fn exec(args: &NewSessionArgs, cfg: &AppConfig, state: &State) -> Result<()> {
    let info = match state.sign_in(&args.name, &args.pass).await {
        Ok(info) => info,
        Err(_) => {
            let info = state.sign_up(&args.name, &args.pass).await?;
            let profile = state.new_profile(&info.access_token.token, &args.name).await?;
            if let Some(name) = &args.scenario {
                let Some(preset) = Preset::lookup(name) else {
                    let known = PRESETS.iter().map(|preset| preset.name).collect::<Vec<_>>();
                    bail!("unknown scenario preset '{name}' (known presets: {})", known.join(", "));
                };
                let profile_id = profile.profile.id;
                apply_scenario(state, profile_id, &(preset.build)()).await?;
                state.update_user_first_flag(profile_id, 1).await?;
                state.update_tutorial_progress(profile_id, TUTORIAL_DONE).await?;
            }
            info
        }
    };

    let session = state.start_game(&info.access_token.token, 1).await?;
    let port = cfg.bind.port();

    let url = format!("http://localhost:{port}/emukc?api_token={}", session.session.token);
    println!("{url}");

    if !args.no_open {
        // open the url in the default browser
        open::that(url).unwrap();
    }

    Ok(())
}
