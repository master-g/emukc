//! Compare a decoder-produced manifest cache list against the current bootstrap baseline.

use std::{
    collections::BTreeSet,
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use config::{Config, FileFormat};
use emukc::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExampleConfig {
    workspace_root: PathBuf,
    cache_root: PathBuf,
    mods_root: Option<PathBuf>,
    proxy: Option<String>,
    gadgets_cdn: Vec<String>,
    game_cdn: Vec<String>,
}

impl ExampleConfig {
    fn load(path: impl AsRef<str>) -> anyhow::Result<Self> {
        let source = config::File::new(path.as_ref(), FileFormat::Toml);
        let cfg = Config::builder().add_source(source).build()?;
        let mut cfg = cfg.try_deserialize::<ExampleConfig>()?;

        if cfg.workspace_root.is_relative() {
            let cfg_dir = Path::new(path.as_ref()).parent().unwrap_or(Path::new("."));
            cfg.workspace_root = cfg_dir.join(&cfg.workspace_root).canonicalize()?;
            cfg.cache_root = cfg_dir.join(&cfg.cache_root).canonicalize()?;
            cfg.mods_root = cfg
                .mods_root
                .as_ref()
                .map(|mods_root| cfg_dir.join(mods_root).canonicalize())
                .transpose()?;
        }

        Ok(cfg)
    }

    fn codex_root(&self) -> PathBuf {
        self.workspace_root.join("codex")
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BaselineStrategy {
    Default,
    Manifest,
    Greedy,
}

impl BaselineStrategy {
    fn into_make_strategy(self, concurrent: usize) -> CacheListMakeStrategy {
        match self {
            Self::Default => CacheListMakeStrategy::Default,
            Self::Manifest => CacheListMakeStrategy::Manifest,
            Self::Greedy => CacheListMakeStrategy::Greedy(GreedyConfig {
                concurrent,
            }),
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    about = "Compare decoder-produced cache list coverage against the current bootstrap baseline"
)]
struct CompareArgs {
    #[arg(long, default_value = "emukc.config.toml")]
    config: String,

    #[arg(long)]
    manifest: Option<PathBuf>,

    #[arg(long)]
    rules: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = BaselineStrategy::Default)]
    baseline: BaselineStrategy,

    #[arg(long, default_value_t = 16)]
    concurrent: usize,

    #[arg(long)]
    report_json: Option<PathBuf>,

    #[arg(long)]
    baseline_paths: Option<PathBuf>,

    #[arg(long)]
    candidate_paths: Option<PathBuf>,

    #[arg(long, default_value_t = 10)]
    sample_limit: usize,
}

fn write_path_set(path: &Path, items: &BTreeSet<String>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    std::fs::write(path, items.iter().cloned().collect::<Vec<_>>().join("\n") + "\n")?;
    Ok(())
}

fn print_prefix_section(title: &str, rows: &[CacheListPathPrefixCount], limit: usize) {
    println!("{title}");
    for row in rows.iter().take(limit) {
        println!("  {:>6}  {}", row.count, row.prefix);
    }
}

fn print_path_samples(title: &str, rows: &[String], limit: usize) {
    println!("{title}");
    for row in rows.iter().take(limit) {
        println!("  {row}");
    }
}

fn print_domain_section(report: &CacheListComparisonReport) {
    println!("Domain coverage");
    for row in &report.domain_coverages {
        println!(
            "  {:<10} baseline {:>6} candidate {:>6} overlap {:>6} baseline_cov {:>6.2}% candidate_overlap {:>6.2}%",
            row.domain,
            row.baseline_count,
            row.candidate_count,
            row.intersection_count,
            row.baseline_coverage_pct,
            row.candidate_overlap_pct,
        );
    }
}

fn print_authority_section(report: &CacheListComparisonReport) {
    if report.migration_ready.is_none() {
        return;
    }

    println!("Decoder-first authority");
    println!("  rule-authored candidate paths: {}", report.rule_authored_candidate_count);
    println!("  fallback-authored candidate paths: {}", report.fallback_authored_candidate_count);
    println!(
        "  sound rule-authored candidate paths: {}",
        report.sound_rule_authored_candidate_count
    );
    println!(
        "  sound fallback-authored candidate paths: {}",
        report.sound_fallback_authored_candidate_count
    );
    if !report.sound_fallback_authored_candidate_prefixes.is_empty() {
        println!("  sound fallback residual prefixes:");
        for row in &report.sound_fallback_authored_candidate_prefixes {
            println!("    {:>6}  {}", row.count, row.prefix);
        }
    }
    if !report.template_rule_authored_candidate_families.is_empty() {
        println!("  template-backed rule-authored families:");
        for row in &report.template_rule_authored_candidate_families {
            println!("    {:>6}  {}", row.count, row.prefix);
        }
    }
    if !report.template_fallback_authored_candidate_families.is_empty() {
        println!("  template-backed fallback residual families:");
        for row in &report.template_fallback_authored_candidate_families {
            println!("    {:>6}  {}", row.count, row.prefix);
        }
    }
    if !report.template_fallback_residual_reasons.is_empty() {
        println!("  template-backed fallback residual reasons:");
        for row in &report.template_fallback_residual_reasons {
            println!("    {:>6}  {}  {}: {}", row.count, row.family, row.kind, row.reason);
        }
    }
    if !report.fallback_authored_candidate_prefixes.is_empty() {
        println!("  fallback residual prefixes:");
        for row in &report.fallback_authored_candidate_prefixes {
            println!("    {:>6}  {}", row.count, row.prefix);
        }
    }
    if !report.repo_fallback_bundle_assets.is_empty() {
        println!(
            "  repo fallback bundle assets: {}",
            report.repo_fallback_bundle_assets.join(", ")
        );
    }
    if !report.missing_bundle_assets.is_empty() {
        println!("  missing bundle assets: {}", report.missing_bundle_assets.join(", "));
    }
    if !report.unresolved_rule_blockers.is_empty() {
        println!("  unresolved rule blockers: {}", report.unresolved_rule_blockers.join(", "));
    }
    println!(
        "  migration ready: {}",
        if report.migration_ready == Some(true) {
            "yes"
        } else {
            "no"
        }
    );
    if !report.migration_blockers.is_empty() {
        println!("  migration blockers:");
        for blocker in &report.migration_blockers {
            println!("    {blocker}");
        }
    }
}

fn build_kache(cfg: &ExampleConfig) -> anyhow::Result<Kache> {
    Kache::builder()
        .with_cache_root(cfg.cache_root.clone())
        .with_mods_root(cfg.mods_root.clone())
        .with_gadgets_cdns(cfg.gadgets_cdn.clone())
        .with_content_cdns(cfg.game_cdn.clone())
        .with_proxy(cfg.proxy.clone())
        .build()
        .map_err(anyhow::Error::from)
}

fn main() -> anyhow::Result<()> {
    let _guard = new_log_builder()
        .with_log_level("info")
        .with_source_file()
        .with_line_number()
        .with_file_appender(std::path::PathBuf::from(".data/.emukc.log"))
        .build()
        .unwrap();

    let args = CompareArgs::parse();

    with_enough_stack(async move {
        let cfg = ExampleConfig::load(&args.config)?;
        let codex = Codex::load(cfg.codex_root(), true)?;
        let kache = build_kache(&cfg)?;

        let baseline_strategy = args.baseline.into_make_strategy(args.concurrent);
        let baseline_paths = build_cache_list_paths(&codex, &kache, baseline_strategy).await?;
        let (candidate_paths, candidate_diagnostics) = if let Some(rules_path) = &args.rules {
            let output =
                build_cache_list_path_output_with_rules_path(&codex, &kache, rules_path).await?;
            (output.paths, Some(output.diagnostics))
        } else {
            let manifest_path = args.manifest.as_ref().ok_or_else(|| {
                anyhow::anyhow!("--manifest is required when --rules is not provided")
            })?;
            (
                build_cache_list_paths_with_manifest_path(
                    &codex,
                    &kache,
                    CacheListMakeStrategy::Manifest,
                    manifest_path,
                )
                .await?,
                None,
            )
        };
        let mut report = compare_cache_list_path_sets(&baseline_paths, &candidate_paths);
        if let Some(diagnostics) = &candidate_diagnostics {
            apply_candidate_build_diagnostics(&mut report, diagnostics);
        }

        println!("Baseline strategy: {:?}", args.baseline);
        if let Some(manifest_path) = &args.manifest {
            println!("Candidate manifest: {}", manifest_path.display());
        }
        if let Some(rules_path) = &args.rules {
            println!("Candidate rules: {}", rules_path.display());
        }
        println!("Baseline unique paths: {}", report.baseline_unique_count);
        println!("Candidate unique paths: {}", report.candidate_unique_count);
        println!("Intersection: {}", report.intersection_count);
        println!("Candidate over baseline: {:.2}%", report.baseline_coverage_pct);
        println!("Baseline overlap within candidate: {:.2}%", report.candidate_overlap_pct);
        println!("Only baseline: {}", report.baseline_only_count);
        println!("Only candidate: {}", report.candidate_only_count);
        println!();

        print_prefix_section(
            "Top baseline-only prefixes",
            &report.baseline_only_prefixes,
            args.sample_limit,
        );
        println!();
        print_prefix_section(
            "Top candidate-only prefixes",
            &report.candidate_only_prefixes,
            args.sample_limit,
        );
        println!();
        print_domain_section(&report);
        println!();
        print_authority_section(&report);
        println!();
        print_path_samples(
            "Sample baseline-only paths",
            &report.baseline_only_paths,
            args.sample_limit,
        );
        println!();
        print_path_samples(
            "Sample candidate-only paths",
            &report.candidate_only_paths,
            args.sample_limit,
        );

        if let Some(path) = &args.report_json {
            if let Some(parent) = path.parent() {
                create_dir_all(parent)?;
            }
            std::fs::write(path, serde_json::to_string_pretty(&report)?)?;
            println!("\nReport JSON written to {}", path.display());
        }

        if let Some(path) = &args.baseline_paths {
            write_path_set(path, &baseline_paths)?;
            println!("Baseline path list written to {}", path.display());
        }

        if let Some(path) = &args.candidate_paths {
            write_path_set(path, &candidate_paths)?;
            println!("Candidate path list written to {}", path.display());
        }

        anyhow::Ok(())
    })?;

    Ok(())
}
