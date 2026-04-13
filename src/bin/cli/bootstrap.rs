use anyhow::Result;
use clap::Args;
use emukc_internal::prelude::{download_all, parse_partial_codex};

use crate::cfg::AppConfig;

/// Bootstrap command arguments
#[derive(Args, Debug)]
pub(super) struct BootstrapArgs {
    #[arg(help = "Overwrite existing files")]
    #[arg(short, long)]
    pub(super) overwrite: bool,

    #[arg(help = "Remove version files from cache folder")]
    #[arg(long)]
    pub(super) force_update: bool,

    #[arg(help = "use another proxy")]
    #[arg(long)]
    pub(super) proxy: Option<String>,

    #[arg(help = "specify output directory")]
    #[arg(long)]
    pub(super) output: Option<String>,
}

/// Execute the bootstrap command
pub(super) async fn exec(cfg: &AppConfig, args: &BootstrapArgs) -> Result<()> {
    let proxy = resolve_proxy(cfg, args);
    let output = if let Some(output) = &args.output {
        std::path::PathBuf::from(output)
    } else {
        cfg.temp_root()?
    };

    // download files needed for constructing the codex
    download_all(&output, args.overwrite, proxy, Some(16)).await?;

    // parse the codex
    let codex = parse_partial_codex(&output)?;

    // save the codex
    let codex_root = cfg.codex_root()?;
    codex.save(&codex_root, args.overwrite)?;

    if args.force_update {
        let p = cfg.cache_root.join("gadget_html5").join("js").join("kcs_const.js");
        if p.exists() {
            std::fs::remove_file(p)?;
        } else {
            warn!("{:?} not found.", p);
        }
        let p = cfg.cache_root.join("kcs2").join("version.json");
        if p.exists() {
            std::fs::remove_file(p)?;
        } else {
            warn!("{:?} not found.", p);
        }
        info!("version files in kcs cache removed.");
    }

    info!("Bootstrap completed successfully.");

    Ok(())
}

fn resolve_proxy<'a>(cfg: &'a AppConfig, args: &'a BootstrapArgs) -> Option<&'a str> {
    args.proxy.as_deref().or(cfg.proxy.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config(proxy: Option<&str>) -> AppConfig {
        AppConfig {
            workspace_root: std::path::PathBuf::from(".data"),
            cache_root: std::path::PathBuf::from("./z/cache"),
            mods_root: Some(std::path::PathBuf::from("./z/mods")),
            bind: "127.0.0.1:8443".parse().unwrap(),
            tls_cert: Some(std::path::PathBuf::from(".data/cert.pem")),
            tls_key: Some(std::path::PathBuf::from(".data/key.pem")),
            proxy: proxy.map(ToOwned::to_owned),
            gadgets_cdn: vec![],
            game_cdn: vec![],
        }
    }

    fn sample_args(proxy: Option<&str>) -> BootstrapArgs {
        BootstrapArgs {
            overwrite: false,
            force_update: false,
            proxy: proxy.map(ToOwned::to_owned),
            output: None,
        }
    }

    #[test]
    fn test_resolve_proxy_prefers_cli_override() {
        let cfg = sample_config(Some("socks5://127.0.0.1:1086"));
        let args = sample_args(Some("http://127.0.0.1:1086"));

        assert_eq!(resolve_proxy(&cfg, &args), Some("http://127.0.0.1:1086"));
    }

    #[test]
    fn test_resolve_proxy_falls_back_to_config() {
        let cfg = sample_config(Some("socks5://127.0.0.1:1086"));
        let args = sample_args(None);

        assert_eq!(resolve_proxy(&cfg, &args), Some("socks5://127.0.0.1:1086"));
    }
}
