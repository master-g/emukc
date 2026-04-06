use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use emukc::bootstrap::prelude::{
	BattleIncidentReport, BattleValidationReport, analyze_day_battle_incident,
	load_repo_battle_knowledge_assets, validate_day_battle_response,
};
use emukc_internal::prelude::Codex;
use serde_json::Value;

use crate::cfg::AppConfig;

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

pub(super) async fn exec(args: &BattleArgs, config: &AppConfig) -> Result<()> {
	match &args.command {
		Command::Validate(args) => validate_exec(args, config).await,
		Command::AnalyzeIncident(args) => analyze_incident_exec(args, config).await,
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

fn print_validation_report(input: &PathBuf, report: &BattleValidationReport) {
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

fn print_incident_report(input: &PathBuf, report: &BattleIncidentReport) {
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
}
