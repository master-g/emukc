use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use emukc::bootstrap::prelude::{
    BattleIncidentReport, BattleValidationReport, analyze_day_battle_incident,
    load_repo_battle_knowledge_assets, validate_day_battle_response,
};
use emukc_internal::{
    crypto::rng,
    db::sea_orm::DbConn,
    prelude::{
        AccountOps, BattleSimulation, Codex, HasContext, PRESETS, PracticeStore, Preset,
        ProfileOps, Scenario, ShipOps, SortieOps, SortieRepository, SortieStore, apply_scenario,
        new_mem_db, render_day_battle,
    },
};
use serde_json::Value;

use crate::cfg::AppConfig;

#[path = "drift_check.rs"]
mod drift_check;

#[derive(Debug, Args)]
pub(super) struct BattleArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Validate a battle payload against client-derived battle rules")]
    Validate(ValidateArgs),
    #[command(about = "Analyze a missing battle resource incident")]
    AnalyzeIncident(AnalyzeIncidentArgs),
    #[command(about = "Run a seeded, scenario-driven sortie and print the battle transcript")]
    Sim(SimArgs),
    #[command(about = "Detect that the decoded client moved (the sync-loop trigger)")]
    DriftCheck(drift_check::DriftCheckArgs),
}

#[derive(Debug, Args)]
struct InputArgs {
    #[arg(help = "Input battle JSON file")]
    #[arg(long, value_name = "FILE")]
    input: PathBuf,

    #[arg(help = "Print structured JSON output")]
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ValidateArgs {
    #[command(flatten)]
    input: InputArgs,
}

#[derive(Debug, Args)]
struct AnalyzeIncidentArgs {
    #[command(flatten)]
    input: InputArgs,

    #[arg(help = "Missing resource URL reported by the client/cache layer")]
    #[arg(long, value_name = "URL")]
    missing_url: String,
}

#[derive(Debug, Args)]
struct SimArgs {
    #[arg(help = "Named preset scenario: fresh_1_1 or leveled_for_mid_boss")]
    #[arg(long, value_name = "NAME")]
    scenario: String,

    #[arg(help = "RNG seed (same seed + scenario reproduces the whole sortie)")]
    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(help = "Override the preset's target map area id")]
    #[arg(long, value_name = "AREA")]
    area: Option<i64>,

    #[arg(help = "Override the preset's target map info no")]
    #[arg(long, value_name = "NO")]
    map: Option<i64>,

    #[arg(help = "Friendly formation id")]
    #[arg(long, default_value_t = 1)]
    formation: i64,

    #[arg(help = "Search seeds for a branch instead of running one: night or cutin")]
    #[arg(long, value_name = "PREDICATE")]
    find: Option<String>,

    #[arg(help = "Max seeds to try during a --find search")]
    #[arg(long, default_value_t = 1000)]
    max_seeds: u64,

    #[arg(help = "Print structured JSON output")]
    #[arg(long)]
    json: bool,
}

pub(super) async fn exec(args: &BattleArgs, config: &AppConfig) -> Result<()> {
    match &args.command {
        Command::Validate(args) => validate_exec(args, config).await,
        Command::AnalyzeIncident(args) => analyze_incident_exec(args, config).await,
        Command::Sim(args) => sim_exec(args, config),
        Command::DriftCheck(args) => drift_check::exec(args),
    }
}

async fn validate_exec(args: &ValidateArgs, config: &AppConfig) -> Result<()> {
    let codex = load_codex(config)?;
    let payload = load_battle_payload(&args.input.input)?;
    let assets = load_repo_battle_knowledge_assets()?;
    let report = validate_day_battle_response(&codex.manifest, &payload, &assets)?;

    if args.input.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_validation_report(&args.input.input, &report);
    }

    if report.has_errors() {
        bail!("battle validation failed with {} error(s)", count_errors(&report));
    }

    Ok(())
}

async fn analyze_incident_exec(args: &AnalyzeIncidentArgs, config: &AppConfig) -> Result<()> {
    let codex = load_codex(config)?;
    let payload = load_battle_payload(&args.input.input)?;
    let assets = load_repo_battle_knowledge_assets()?;
    let report =
        analyze_day_battle_incident(&codex.manifest, &payload, &assets, Some(&args.missing_url))?;

    if args.input.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_incident_report(&args.input.input, &report);
    }

    if report.validation.has_errors()
        || !report.protocol_suspicions.is_empty()
        || !report.bootstrap_gaps.is_empty()
    {
        bail!(
            "battle incident analysis found {} protocol suspicion(s), {} bootstrap gap(s), {} validation error(s)",
            report.protocol_suspicions.len(),
            report.bootstrap_gaps.len(),
            count_errors(&report.validation),
        );
    }

    Ok(())
}

fn sim_exec(args: &SimArgs, config: &AppConfig) -> Result<()> {
    let codex = load_codex(config)?;
    let (scenario, default_area, default_no) = resolve_scenario(&args.scenario)?;
    let area = args.area.unwrap_or(default_area);
    let map_no = args.map.unwrap_or(default_no);
    let target = SortieTarget {
        area,
        map_no,
        formation: args.formation,
    };

    if let Some(find) = &args.find {
        let predicate = FindPredicate::parse(find)?;
        let found =
            seed_search(codex, args.seed, args.max_seeds, scenario, target, move |outcome| {
                predicate.matches(outcome)
            })?;
        match found {
            Some((seed, outcome)) => {
                if args.json {
                    let out = serde_json::json!({
                        "find": find,
                        "found_seed": seed,
                        "transcript": outcome.transcript,
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    println!("found seed {seed} matching '{find}'");
                    print!("{}", outcome.transcript);
                }
                Ok(())
            }
            None => bail!(
                "no seed matching '{find}' found within {} seeds (from {})",
                args.max_seeds,
                args.seed
            ),
        }
    } else {
        let transcript = render_sortie_once(codex, args.seed, scenario, target)?;
        if args.json {
            let out = serde_json::json!({
                "scenario": args.scenario,
                "seed": args.seed,
                "area": area,
                "map": map_no,
                "transcript": transcript,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            print!("{transcript}");
        }
        Ok(())
    }
}

/// Resolve a named preset to its scenario and default sortie target (area, no),
/// reading from the shared [`PRESETS`] registry so the CLI and the sim→validate
/// gate stay in sync by construction.
fn resolve_scenario(name: &str) -> Result<(Scenario, i64, i64)> {
    let Some(preset) = Preset::lookup(name) else {
        let known = PRESETS.iter().map(|preset| preset.name).collect::<Vec<_>>().join(", ");
        bail!("unknown scenario preset '{name}' (known presets: {known})");
    };
    Ok(((preset.build)(), preset.maparea, preset.mapinfo))
}

/// Minimal in-memory gameplay context for the sim, mirroring the integration
/// `TestContext` shape (codex + in-mem DB + isolated sortie/practice stores).
struct SimContext {
    db: DbConn,
    codex: Codex,
    sortie_store: SortieStore,
    practice_store: PracticeStore,
}

impl HasContext for SimContext {
    fn db(&self) -> &DbConn {
        &self.db
    }
    fn codex(&self) -> &Codex {
        &self.codex
    }
    fn sortie_store(&self) -> &SortieStore {
        &self.sortie_store
    }
    fn practice_store(&self) -> &PracticeStore {
        &self.practice_store
    }
}

/// Where a sim run sorties: map area + info no, and the friendly formation.
#[derive(Clone, Copy)]
struct SortieTarget {
    area: i64,
    map_no: i64,
    formation: i64,
}

/// What a single seeded sortie run produced — shared by the printer and the
/// seed-search predicate.
struct RunOutcome {
    transcript: String,
    /// Midnight became available (both fleets survived the day battle).
    midnight_available: bool,
    /// A day-shelling cut-in / special attack occurred.
    saw_cutin: bool,
}

/// A branch predicate for the `--find` seed search.
#[derive(Clone, Copy)]
enum FindPredicate {
    Night,
    Cutin,
}

impl FindPredicate {
    fn parse(name: &str) -> Result<Self> {
        match name {
            "night" => Ok(Self::Night),
            "cutin" => Ok(Self::Cutin),
            other => bail!("unknown --find predicate '{other}' (known: night, cutin)"),
        }
    }

    fn matches(self, outcome: &RunOutcome) -> bool {
        match self {
            Self::Night => outcome.midnight_available,
            Self::Cutin => outcome.saw_cutin,
        }
    }
}

/// Search seeds from `start_seed` (inclusive) for up to `max_attempts`, returning
/// the first seed whose run satisfies `predicate` plus that run's outcome.
///
/// Runs on a dedicated current-thread runtime: the thread-local seed only holds
/// on a current-thread executor, so the sim must NOT inherit the CLI's
/// multi-thread runtime — a task migration at an `.await` would land post-seed
/// RNG draws on an unseeded worker thread and break reproducibility. One
/// context/profile/scenario is built once and reused across seeds; stale sortie
/// state is cleared between attempts.
fn seed_search<P>(
    codex: Codex,
    start_seed: u64,
    max_attempts: u64,
    scenario: Scenario,
    target: SortieTarget,
    predicate: P,
) -> Result<Option<(u64, RunOutcome)>>
where
    P: Fn(&RunOutcome) -> bool + Send + 'static,
{
    let handle = std::thread::spawn(move || -> Result<Option<(u64, RunOutcome)>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to build current-thread runtime for battle sim")?;
        rt.block_on(async move {
            let db = new_mem_db().await.context("failed to create in-memory database")?;
            let ctx = SimContext {
                db,
                codex,
                sortie_store: SortieStore::new(),
                practice_store: PracticeStore::new(),
            };
            let profile_id = create_sim_profile(&ctx).await?;
            apply_scenario(&ctx, profile_id, &scenario).await.context("apply scenario")?;

            // Snapshot the post-scenario fleet so every seed starts from the same
            // state. Without this, accumulated sortie damage across iterations
            // would make a found seed depend on the search history rather than on
            // (scenario, seed) alone, breaking reproduction on a plain run.
            let baseline = ctx.get_ships(profile_id).await.context("snapshot fleet")?;

            for offset in 0..max_attempts {
                let seed = start_seed.wrapping_add(offset);
                for ship in &baseline {
                    ctx.update_ship(ship).await.context("restore fleet baseline")?;
                }
                ctx.clear_sortie_state_if_any(profile_id).await;
                rng::seed(seed);
                let outcome = run_first_battle_outcome(&ctx, profile_id, target).await?;
                if predicate(&outcome) {
                    return Ok(Some((seed, outcome)));
                }
            }
            Ok(None)
        })
    });

    handle.join().map_err(|_| anyhow::anyhow!("battle sim worker thread panicked"))?
}

/// Drive one seeded sortie and return its transcript (single-seed convenience).
fn render_sortie_once(
    codex: Codex,
    seed: u64,
    scenario: Scenario,
    target: SortieTarget,
) -> Result<String> {
    let found = seed_search(codex, seed, 1, scenario, target, |_| true)?;
    Ok(found.expect("the always-true predicate matches the first seed").1.transcript)
}

async fn create_sim_profile(ctx: &SimContext) -> Result<i64> {
    let account = ctx.sign_up("battle-sim", "1234567").await.context("sign up")?;
    let profile =
        ctx.new_profile(&account.access_token.token, "battle-sim").await.context("new profile")?;
    let session = ctx
        .start_game(&account.access_token.token, profile.profile.id)
        .await
        .context("start game")?;
    Ok(session.profile.id)
}

async fn run_first_battle_outcome(
    ctx: &SimContext,
    profile_id: i64,
    target: SortieTarget,
) -> Result<RunOutcome> {
    ctx.start_sortie(profile_id, 1, target.area, target.map_no).await.context("start_sortie")?;
    ctx.sortie_battle(profile_id, target.formation).await.context("sortie_battle")?;

    let session = ctx
        .sortie_store()
        .get_pending_battle(profile_id)
        .context("no pending battle session after sortie_battle")?;

    let midnight_available = session.packet.midnight_flag != 0;
    let saw_cutin = [
        session.packet.hougeki1.as_ref(),
        session.packet.hougeki2.as_ref(),
        session.packet.hougeki3.as_ref(),
    ]
    .into_iter()
    .flatten()
    .any(|h| h.api_at_type.iter().any(|&t| t != 0));

    let simulation = BattleSimulation {
        friendly: session.friendly,
        enemy: session.enemy,
        packet: session.packet,
        outcome: session.outcome,
    };
    let transcript = render_day_battle(&simulation);

    ctx.sortie_battle_result(profile_id).await.context("sortie_battle_result")?;
    Ok(RunOutcome {
        transcript,
        midnight_available,
        saw_cutin,
    })
}

fn load_codex(config: &AppConfig) -> Result<Codex> {
    let codex_root = config.codex_root()?;
    Codex::load_without_cache_source(&codex_root)
        .with_context(|| format!("failed to load codex from {}", codex_root.display()))
}

fn load_battle_payload(path: &PathBuf) -> Result<Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read battle payload from {}", path.display()))?;
    let trimmed = raw.trim();
    let json = trimmed.strip_prefix("svdata=").unwrap_or(trimmed);
    let value: Value = serde_json::from_str(json)
        .with_context(|| format!("failed to parse battle payload JSON from {}", path.display()))?;

    if let Some(api_data) = value.get("api_data") {
        return Ok(api_data.clone());
    }

    Ok(value)
}

fn count_errors(report: &BattleValidationReport) -> usize {
    report
        .findings
        .iter()
        .filter(|finding| {
            matches!(finding.severity, emukc::bootstrap::prelude::BattleValidationSeverity::Error)
        })
        .count()
}

fn print_validation_report(input: &std::path::Path, report: &BattleValidationReport) {
    let error_count = count_errors(report);
    let warning_count = report.findings.len().saturating_sub(error_count);

    println!("battle validate");
    println!("input: {}", input.display());
    println!("errors: {}", error_count);
    println!("warnings: {}", warning_count);
    println!("expected resources: {}", report.expected_resources.len());
    println!("candidate resources: {}", report.candidate_resources.len());

    if !report.findings.is_empty() {
        println!("findings:");
        for finding in &report.findings {
            let field = finding.field.as_deref().unwrap_or("<none>");
            println!("- {:?} {:?} {}: {}", finding.severity, finding.kind, field, finding.message);
        }
    }

    if !report.expected_resources.is_empty() {
        println!("expected resources:");
        for resource in report.expected_resources.iter().take(10) {
            println!("- [{}:{}] {}", resource.kind, resource.target_type, resource.path);
        }
    }

    if !report.candidate_resources.is_empty() {
        println!("candidate resources:");
        for resource in report.candidate_resources.iter().take(10) {
            println!("- [{}:{}] {}", resource.kind, resource.target_type, resource.path);
        }
    }
}

fn print_incident_report(input: &Path, report: &BattleIncidentReport) {
    print_validation_report(input, &report.validation);
    if let Some(path) = &report.missing_resource_path {
        println!("missing resource: {}", path);
    }

    if !report.trigger_matches.is_empty() {
        println!("trigger matches:");
        for trigger in &report.trigger_matches {
            println!(
                "- {} -> {} via {} ({})",
                trigger.protocol_source,
                trigger.resource_target,
                trigger.consumer_module,
                trigger.confidence,
            );
        }
    }

    if !report.protocol_suspicions.is_empty() {
        println!("protocol suspicions:");
        for finding in &report.protocol_suspicions {
            println!("- {}", finding.message);
        }
    }

    if !report.bootstrap_gaps.is_empty() {
        println!("bootstrap gaps:");
        for finding in &report.bootstrap_gaps {
            println!("- {}", finding.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_internal::prelude::ShipSpec;

    fn target_1_1() -> SortieTarget {
        SortieTarget {
            area: 1,
            map_no: 1,
            formation: 1,
        }
    }

    #[test]
    fn load_battle_payload_reads_api_data_envelope() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("battle.json");
        fs::write(&path, r#"{"api_result":1,"api_data":{"api_ship_ke":[1]}}"#).unwrap();

        let payload = load_battle_payload(&path).unwrap();
        assert_eq!(payload["api_ship_ke"][0], 1);
    }

    #[test]
    fn load_battle_payload_accepts_svdata_prefix() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("battle.txt");
        fs::write(&path, r#"svdata={"api_result":1,"api_data":{"api_ship_ke":[2]}}"#).unwrap();

        let payload = load_battle_payload(&path).unwrap();
        assert_eq!(payload["api_ship_ke"][0], 2);
    }

    #[test]
    fn unknown_preset_fails_with_clear_message() {
        let err = resolve_scenario("does-not-exist").unwrap_err();
        assert!(
            err.to_string().contains("unknown scenario preset"),
            "expected a clear unknown-preset error, got: {err}"
        );
    }

    #[test]
    fn sim_transcript_is_deterministic_in_process() {
        // Same scenario + seed must produce byte-identical transcripts within one
        // process. Asserting in-process is the point: two separate CLI invocations
        // each build a fresh runtime and would not catch thread-migration drift.
        let codex_a = Codex::load_without_cache_source(".data/codex").expect(
            "Codex load failed; run `cargo run -- bootstrap` first to populate .data/codex/",
        );
        let codex_b = Codex::load_without_cache_source(".data/codex").unwrap();

        let a = render_sortie_once(codex_a, 7, Scenario::fresh_1_1(), target_1_1()).unwrap();
        let b = render_sortie_once(codex_b, 7, Scenario::fresh_1_1(), target_1_1()).unwrap();

        assert_eq!(a, b, "same seed must yield byte-identical transcripts in-process");
        assert!(a.contains("result: rank"), "transcript should end in a result rank:\n{a}");
    }

    #[test]
    fn seed_search_reports_not_found_at_cap() {
        let codex = Codex::load_without_cache_source(".data/codex").expect(
            "Codex load failed; run `cargo run -- bootstrap` first to populate .data/codex/",
        );
        // An always-false predicate must exhaust the cap and report not-found
        // rather than looping forever.
        let found =
            seed_search(codex, 1, 4, Scenario::fresh_1_1(), target_1_1(), |_| false).unwrap();
        assert!(found.is_none(), "always-false predicate must hit the attempt cap");
    }

    #[test]
    fn seed_search_finds_night_and_reproduces() {
        // Covers AE2. A single heavily-damaged (taiha) flagship deals little
        // shelling damage, so it often fails to clear 1-1's enemy, leaving it
        // alive and making midnight available — a findable rare branch.
        let night_scenario = || Scenario {
            fleet: vec![ShipSpec::new(951, 1).with_hp(1)],
            ..Default::default()
        };

        let codex = Codex::load_without_cache_source(".data/codex").unwrap();
        let found =
            seed_search(codex, 1, 500, night_scenario(), target_1_1(), |o| o.midnight_available)
                .unwrap();
        let (seed, outcome) = found.expect("night branch should be findable within 500 seeds");
        assert!(outcome.midnight_available);

        // Re-running the reported seed reproduces the branch on a plain run.
        let codex2 = Codex::load_without_cache_source(".data/codex").unwrap();
        let rerun = seed_search(codex2, seed, 1, night_scenario(), target_1_1(), |_| true)
            .unwrap()
            .unwrap()
            .1;
        assert!(rerun.midnight_available, "reported seed must reproduce the night branch");
    }
}
