use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use clap::Args;
use emukc_internal::{gameplay::prelude::missing_masters, model::kc2::live::LiveSnapshot};
use serde_json::Value;

use crate::state::State;

/// Import account command arguments
#[derive(Args, Debug)]
pub struct ImportAccountArgs {
    #[arg(help = "directory written by scripts/fetch_live_api.py, e.g. z/snapshot/2026-10-27")]
    #[arg(long)]
    snapshot: PathBuf,

    #[arg(help = "user name of the new account")]
    #[arg(short = 'u', long)]
    name: String,

    #[arg(help = "password of the new account")]
    #[arg(short = 'p', long)]
    pass: String,
}

/// The `api_data` of one captured response. The `svdata=` prefix is optional; anything
/// but `api_result: 1` is refused, since it is the error page and not the state.
fn read_api_data(dir: &Path, file: &str) -> Result<Value> {
    let path = dir.join(file);
    let raw =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let raw = raw.trim_start();
    let response: Value = serde_json::from_str(raw.strip_prefix("svdata=").unwrap_or(raw))
        .with_context(|| format!("{} is not JSON", path.display()))?;
    let result = response.get("api_result").and_then(Value::as_i64);
    if result != Some(1) {
        bail!("{}: api_result is {result:?}, not 1", path.display());
    }
    response.get("api_data").cloned().ok_or_else(|| anyhow!("{} has no api_data", path.display()))
}

/// Read the two responses the import needs from a capture directory.
fn load_snapshot(dir: &Path) -> Result<LiveSnapshot> {
    let port = read_api_data(dir, "api_port_port.json")?;
    let require_info = read_api_data(dir, "api_get_member_require_info.json")?;
    Ok(LiveSnapshot::from_api_data(&port, &require_info)?)
}

pub async fn exec(args: &ImportAccountArgs, state: &State) -> Result<()> {
    let snapshot = load_snapshot(&args.snapshot)?;
    let missing = missing_masters(&state.codex, &snapshot);
    if !missing.is_empty() {
        bail!(
            "the codex lacks ships {:?} and equipment {:?}; rebuild it with `bootstrap` first",
            missing.ships,
            missing.slot_items
        );
    }

    let account = state.sign_up(&args.name, &args.pass).await?;
    let imported = async {
        let profile = state.new_profile(&account.access_token.token, &args.name).await?;
        anyhow::Ok(state.import_account(profile.profile.id, &snapshot).await?)
    }
    .await;
    let report = match imported {
        Ok(report) => report,
        Err(e) => {
            // Leave nothing behind, so the same name can be tried again.
            state.delete_account(&args.name, &args.pass).await?;
            return Err(e);
        }
    };

    print!("{report}");
    println!("log in with: emukcd new-session --name {} --pass <password>", args.name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, file: &str, body: &str) {
        fs::write(dir.join(file), body).unwrap();
    }

    #[test]
    fn a_response_is_read_with_or_without_the_svdata_prefix() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "plain.json", r#"{"api_result": 1, "api_data": {"a": 1}}"#);
        write(dir.path(), "prefixed.json", r#"svdata={"api_result": 1, "api_data": {"a": 2}}"#);
        assert_eq!(read_api_data(dir.path(), "plain.json").unwrap()["a"], 1);
        assert_eq!(read_api_data(dir.path(), "prefixed.json").unwrap()["a"], 2);
    }

    #[test]
    fn an_error_response_or_a_missing_endpoint_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "api_port_port.json", r#"{"api_result": 100, "api_result_msg": "x"}"#);
        let error = load_snapshot(dir.path()).unwrap_err().to_string();
        assert!(error.contains("api_result is Some(100)"), "{error}");

        write(dir.path(), "api_port_port.json", r#"{"api_result": 1, "api_data": {}}"#);
        let error = load_snapshot(dir.path()).unwrap_err().to_string();
        assert!(error.contains("cannot read") && error.contains("require_info"), "{error}");
    }
}
