use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    sync::Arc,
};

use futures::{StreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};

use emukc_cache::{IntoVersion, Kache, KacheError};
use emukc_model::codex::Codex;

use errors::CacheListMakingError;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

pub mod config;
pub mod errors;
pub mod holes_report;
pub mod manifest;
pub mod progress;

mod source;

pub(crate) use source::kcs2::resources::slot::has_btxt_flat_coverage;

/// Strategy for making a cache list.
#[derive(Clone, Debug, PartialEq)]
pub enum CacheListMakeStrategy {
    /// Default strategy
    Default,
    /// Minimal strategy
    Minimal,
    /// Greedy strategy with configuration
    Greedy(config::GreedyConfig),
    /// Manifest strategy — uses `resource_manifest.json`
    Manifest,
    /// Rules strategy — uses `cache_rules.json`
    Rules,
}

/// A single cache list entry
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CacheListItem {
    /// resource path
    pub path: String,

    /// Resource version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl Ord for CacheListItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.cmp(&other.path).then_with(|| self.version.cmp(&other.version))
    }
}

impl PartialOrd for CacheListItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CacheListAuthorityStage {
    FallbackAuthored,
    RuleAuthored,
}

#[derive(Debug, Clone, Default)]
struct CacheListAuthorityTracker {
    path_stages: BTreeMap<String, CacheListAuthorityStage>,
    unresolved_rule_blockers: BTreeSet<String>,
    repo_fallback_bundle_assets: BTreeSet<String>,
    missing_bundle_assets: BTreeSet<String>,
}

/// Sideband diagnostics describing how a cache list candidate was produced.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheListBuildDiagnostics {
    /// Number of unique candidate paths authored directly by decoder rules.
    pub rule_authored_count: usize,
    /// Full sorted rule-authored candidate paths.
    pub rule_authored_paths: Vec<String>,
    /// Number of unique candidate paths that required legacy fallback generation.
    pub fallback_authored_count: usize,
    /// Full sorted fallback-authored candidate paths.
    pub fallback_authored_paths: Vec<String>,
    /// Fallback-authored counts grouped by normalized path prefix.
    pub fallback_authored_prefixes: Vec<CacheListPathPrefixCount>,
    /// Template-backed fallback residual reasons grouped by family.
    pub template_fallback_residual_reasons: Vec<CacheListTemplateResidualReason>,
    /// Decoder rule keys still unresolved for this candidate build.
    pub unresolved_rule_blockers: Vec<String>,
    /// Decoder bundle assets satisfied only through repo fallback instead of sibling outputs.
    pub repo_fallback_bundle_assets: Vec<String>,
    /// Decoder bundle assets unavailable from both sibling and repo locations.
    pub missing_bundle_assets: Vec<String>,
}

/// In-memory path-set build result plus decoder-first diagnostics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheListPathBuildOutput {
    /// Unique generated resource paths.
    pub paths: BTreeSet<String>,
    /// Sideband decoder-first diagnostics for the generated path set.
    pub diagnostics: CacheListBuildDiagnostics,
}

#[derive(Debug, Clone)]
pub(crate) struct CacheList {
    pub items: BTreeSet<CacheListItem>,

    current_authority_stage: Option<CacheListAuthorityStage>,
    authority_tracker: CacheListAuthorityTracker,
}

impl CacheList {
    pub fn new() -> Self {
        Self {
            items: BTreeSet::new(),
            current_authority_stage: None,
            authority_tracker: CacheListAuthorityTracker::default(),
        }
    }

    pub fn add(&mut self, path: String, version: impl IntoVersion) -> &mut Self {
        let version = version.into_version();
        self.record_path_authority(&path);
        let item = CacheListItem {
            path,
            version,
        };
        self.items.insert(item);

        self
    }

    pub fn add_unversioned(&mut self, path: String) -> &mut Self {
        self.record_path_authority(&path);
        let item = CacheListItem {
            path,
            version: None,
        };
        self.items.insert(item);

        self
    }

    pub(crate) fn set_authority_stage(
        &mut self,
        stage: Option<CacheListAuthorityStage>,
    ) -> Option<CacheListAuthorityStage> {
        std::mem::replace(&mut self.current_authority_stage, stage)
    }

    pub(crate) fn record_unresolved_rule_blockers(
        &mut self,
        blockers: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.authority_tracker
            .unresolved_rule_blockers
            .extend(blockers.into_iter().map(Into::into));
    }

    pub(crate) fn record_repo_fallback_bundle_assets(
        &mut self,
        assets: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.authority_tracker
            .repo_fallback_bundle_assets
            .extend(assets.into_iter().map(Into::into));
    }

    pub(crate) fn record_missing_bundle_assets(
        &mut self,
        assets: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.authority_tracker.missing_bundle_assets.extend(assets.into_iter().map(Into::into));
    }

    fn into_items(self) -> Vec<CacheListItem> {
        self.items.into_iter().collect()
    }

    fn into_path_set(self) -> BTreeSet<String> {
        self.items.into_iter().map(|item| item.path).collect()
    }

    fn into_path_build_output(self) -> CacheListPathBuildOutput {
        let CacheList {
            items,
            current_authority_stage: _,
            authority_tracker,
        } = self;
        let paths = items.into_iter().map(|item| item.path).collect::<BTreeSet<_>>();
        let rule_authored_paths = authority_tracker
            .path_stages
            .iter()
            .filter(|(_, stage)| **stage == CacheListAuthorityStage::RuleAuthored)
            .map(|(path, _)| path.clone())
            .collect::<BTreeSet<_>>();
        let fallback_authored_paths = authority_tracker
            .path_stages
            .iter()
            .filter(|(_, stage)| **stage == CacheListAuthorityStage::FallbackAuthored)
            .map(|(path, _)| path.clone())
            .collect::<BTreeSet<_>>();

        CacheListPathBuildOutput {
            paths,
            diagnostics: CacheListBuildDiagnostics {
                rule_authored_count: rule_authored_paths.len(),
                rule_authored_paths: rule_authored_paths.iter().cloned().collect(),
                fallback_authored_count: fallback_authored_paths.len(),
                fallback_authored_paths: fallback_authored_paths.iter().cloned().collect(),
                fallback_authored_prefixes: group_paths_by_prefix(&fallback_authored_paths),
                template_fallback_residual_reasons: template_residual_reasons(
                    &fallback_authored_paths,
                ),
                unresolved_rule_blockers: authority_tracker
                    .unresolved_rule_blockers
                    .iter()
                    .cloned()
                    .collect(),
                repo_fallback_bundle_assets: authority_tracker
                    .repo_fallback_bundle_assets
                    .iter()
                    .cloned()
                    .collect(),
                missing_bundle_assets: authority_tracker
                    .missing_bundle_assets
                    .iter()
                    .cloned()
                    .collect(),
            },
        }
    }

    fn record_path_authority(&mut self, path: &str) {
        let Some(stage) = self.current_authority_stage else {
            return;
        };
        match self.authority_tracker.path_stages.get(path).copied() {
            Some(existing) if existing >= stage => {}
            _ => {
                self.authority_tracker.path_stages.insert(path.to_string(), stage);
            }
        }
    }
}

/// A grouped delta count by normalized path prefix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheListPathPrefixCount {
    /// Path prefix bucket (for example `kcs2/resources/ship/full`).
    pub prefix: String,
    /// Number of paths in the bucket.
    pub count: usize,
}

/// Stable reason category for template-backed fallback residuals.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CacheListTemplateResidualReasonKind {
    MissingDescriptorEvidence,
    PartialCoverage,
    UnavailableRuntimeInput,
    UncoveredResidualMembership,
}

impl std::fmt::Display for CacheListTemplateResidualReasonKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(template_residual_kind_label(*self))
    }
}

/// Template-backed fallback residual reason grouped by family.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheListTemplateResidualReason {
    /// Stable template family label.
    pub family: String,
    /// Number of fallback-authored paths in the family.
    pub count: usize,
    /// Machine-readable residual category.
    pub kind: CacheListTemplateResidualReasonKind,
    /// Human-readable reason.
    pub reason: String,
}

/// Domain-level overlap metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheListDomainCoverage {
    pub domain: String,
    pub baseline_count: usize,
    pub candidate_count: usize,
    pub intersection_count: usize,
    pub baseline_coverage_pct: f64,
    pub candidate_overlap_pct: f64,
}

/// Path-based comparison report between a baseline and a candidate cache list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheListComparisonReport {
    /// Unique baseline path count.
    pub baseline_unique_count: usize,
    /// Unique candidate path count.
    pub candidate_unique_count: usize,
    /// Unique intersection path count.
    pub intersection_count: usize,
    /// Paths only present in the baseline.
    pub baseline_only_count: usize,
    /// Paths only present in the candidate.
    pub candidate_only_count: usize,
    /// Intersection / baseline unique count, rounded to two decimals.
    pub baseline_coverage_pct: f64,
    /// Intersection / candidate unique count, rounded to two decimals.
    pub candidate_overlap_pct: f64,
    /// Full sorted baseline-only paths.
    pub baseline_only_paths: Vec<String>,
    /// Full sorted candidate-only paths.
    pub candidate_only_paths: Vec<String>,
    /// Baseline-only counts grouped by normalized path prefix.
    pub baseline_only_prefixes: Vec<CacheListPathPrefixCount>,
    /// Candidate-only counts grouped by normalized path prefix.
    pub candidate_only_prefixes: Vec<CacheListPathPrefixCount>,
    /// Domain-level coverage metrics for major cache-list domains.
    pub domain_coverages: Vec<CacheListDomainCoverage>,
    /// Number of candidate paths authored directly by decoder rules.
    pub rule_authored_candidate_count: usize,
    /// Number of candidate paths authored through legacy fallback stages.
    pub fallback_authored_candidate_count: usize,
    /// Fallback-authored candidate counts grouped by normalized path prefix.
    pub fallback_authored_candidate_prefixes: Vec<CacheListPathPrefixCount>,
    /// Number of `kcs/sound/*` candidate paths authored directly by decoder rules.
    pub sound_rule_authored_candidate_count: usize,
    /// Number of `kcs/sound/*` candidate paths still authored through fallback.
    pub sound_fallback_authored_candidate_count: usize,
    /// Fallback-authored `kcs/sound/*` counts grouped by normalized path prefix.
    pub sound_fallback_authored_candidate_prefixes: Vec<CacheListPathPrefixCount>,
    /// Template-backed rule-authored candidate counts grouped by family.
    pub template_rule_authored_candidate_families: Vec<CacheListPathPrefixCount>,
    /// Template-backed fallback-authored candidate counts grouped by family.
    pub template_fallback_authored_candidate_families: Vec<CacheListPathPrefixCount>,
    /// Template-backed fallback residual reasons grouped by family.
    pub template_fallback_residual_reasons: Vec<CacheListTemplateResidualReason>,
    /// Decoder rule keys or bundle conditions still blocking migration.
    pub unresolved_rule_blockers: Vec<String>,
    /// Optional decoder bundle assets satisfied only through repo fallback.
    pub repo_fallback_bundle_assets: Vec<String>,
    /// Optional decoder bundle assets unavailable from both sibling and repo locations.
    pub missing_bundle_assets: Vec<String>,
    /// Explicit migration blocker summary for decoder-first default-switch planning.
    pub migration_blockers: Vec<String>,
    /// Whether the candidate is migration-ready for measured domains.
    pub migration_ready: Option<bool>,
}

fn round_pct(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn normalize_path_prefix(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    if path.starts_with("kcs2/resources/") && parts.len() >= 4 {
        parts[..4].join("/")
    } else if parts.len() >= 3 {
        parts[..3].join("/")
    } else {
        path.to_string()
    }
}

fn group_paths_by_prefix(paths: &BTreeSet<String>) -> Vec<CacheListPathPrefixCount> {
    let mut counts = HashMap::<String, usize>::new();
    for path in paths {
        *counts.entry(normalize_path_prefix(path)).or_default() += 1;
    }

    let mut grouped = counts
        .into_iter()
        .map(|(prefix, count)| CacheListPathPrefixCount {
            prefix,
            count,
        })
        .collect::<Vec<_>>();

    grouped.sort_by(|left, right| {
        right.count.cmp(&left.count).then_with(|| left.prefix.cmp(&right.prefix))
    });

    grouped
}

fn classify_domain(path: &str) -> &'static str {
    if path.starts_with("kcs2/resources/ship/") {
        "ship"
    } else if path.starts_with("kcs2/resources/slot/") {
        "slot"
    } else if path.starts_with("kcs/sound/") || path.starts_with("kcs2/resources/se/") {
        "sound"
    } else if path.starts_with("kcs2/resources/map/") {
        "map"
    } else if path.starts_with("kcs2/resources/furniture/") {
        "furniture"
    } else if path.starts_with("kcs2/resources/bgm/") {
        "bgm"
    } else if path.starts_with("kcs2/resources/useitem/") {
        "useitem"
    } else if path.starts_with("kcs2/resources/voice/") {
        "voice"
    } else if path.starts_with("kcs2/resources/plane/") {
        "plane"
    } else {
        "other"
    }
}

fn classify_template_family(path: &str) -> Option<&'static str> {
    if path.starts_with("kcs2/resources/map/") {
        Some("map")
    } else if path.starts_with("kcs2/resources/gauge/") {
        Some("gauge")
    } else if path.starts_with("kcs2/resources/furniture/") {
        Some("furniture")
    } else if path.starts_with("kcs2/resources/bgm/") {
        Some("bgm")
    } else if path.starts_with("kcs/sound/kc9998/") {
        Some("sound.kc9998")
    } else if path.starts_with("kcs2/resources/voice/titlecall_") {
        Some("voice.titlecall")
    } else if path.starts_with("kcs2/resources/useitem/card/") {
        Some("useitem.card")
    } else if path.starts_with("kcs2/resources/useitem/card_/") {
        Some("useitem.card_")
    } else if path.starts_with("kcs2/resources/area/") {
        Some("area")
    } else if path.starts_with("kcs2/resources/worldselect/") {
        Some("worldselect")
    } else {
        None
    }
}

fn group_template_paths_by_family(
    paths: impl IntoIterator<Item = impl AsRef<str>>,
) -> Vec<CacheListPathPrefixCount> {
    let mut counts = HashMap::<String, usize>::new();
    for path in paths {
        if let Some(family) = classify_template_family(path.as_ref()) {
            *counts.entry(family.to_string()).or_default() += 1;
        }
    }
    let mut grouped = counts
        .into_iter()
        .map(|(prefix, count)| CacheListPathPrefixCount {
            prefix,
            count,
        })
        .collect::<Vec<_>>();
    grouped.sort_by(|left, right| {
        right.count.cmp(&left.count).then_with(|| left.prefix.cmp(&right.prefix))
    });
    grouped
}

fn template_residual_reason_for_family(
    family: &str,
) -> Option<(CacheListTemplateResidualReasonKind, &'static str)> {
    match family {
        "map" => Some((
            CacheListTemplateResidualReasonKind::PartialCoverage,
            "decoder templates cover manifest map base and sidecar subsets, but fallback still owns unproven default/event map variants",
        )),
        "gauge" => Some((
            CacheListTemplateResidualReasonKind::PartialCoverage,
            "decoder templates cover manifest gauge JSON paths, but gauge image sidecars and event variants require additional runtime evidence",
        )),
        "sound.kc9998" => Some((
            CacheListTemplateResidualReasonKind::UnavailableRuntimeInput,
            "kc9998 path shape is decoder-backed, but full membership requires a validated cache-source sound bucket",
        )),
        "bgm" => Some((
            CacheListTemplateResidualReasonKind::UncoveredResidualMembership,
            "manifest.bgm and manifest.mapbgm cover decoder-owned BGM paths, but legacy static battle BGM ids remain outside those inputs",
        )),
        "useitem.card" | "useitem.card_" => Some((
            CacheListTemplateResidualReasonKind::PartialCoverage,
            "decoder UI evidence covers observed useitem ids, but fallback still owns residual card ids outside the observed subset",
        )),
        "area" => Some((
            CacheListTemplateResidualReasonKind::PartialCoverage,
            "decoder and manifest evidence cover observed area paths, but fallback still owns residual area variants",
        )),
        "furniture" => Some((
            CacheListTemplateResidualReasonKind::UncoveredResidualMembership,
            "decoder furniture templates cover observed categories, but fallback still owns residual furniture members outside those descriptors",
        )),
        "voice.titlecall" | "worldselect" => Some((
            CacheListTemplateResidualReasonKind::PartialCoverage,
            "decoder template range covers the known generated subset, but fallback residuals indicate uncovered members",
        )),
        _ => None,
    }
}

fn template_residual_kind_label(kind: CacheListTemplateResidualReasonKind) -> &'static str {
    match kind {
        CacheListTemplateResidualReasonKind::MissingDescriptorEvidence => {
            "missing-descriptor-evidence"
        }
        CacheListTemplateResidualReasonKind::PartialCoverage => "partial-coverage",
        CacheListTemplateResidualReasonKind::UnavailableRuntimeInput => "unavailable-runtime-input",
        CacheListTemplateResidualReasonKind::UncoveredResidualMembership => {
            "uncovered-residual-membership"
        }
    }
}

fn template_residual_reasons(paths: &BTreeSet<String>) -> Vec<CacheListTemplateResidualReason> {
    group_template_paths_by_family(paths.iter())
        .into_iter()
        .filter_map(|row| {
            let (kind, reason) = template_residual_reason_for_family(row.prefix.as_str())?;
            Some(CacheListTemplateResidualReason {
                family: row.prefix,
                count: row.count,
                kind,
                reason: reason.to_string(),
            })
        })
        .collect()
}

fn compute_domain_coverages(
    baseline: &BTreeSet<String>,
    candidate: &BTreeSet<String>,
) -> Vec<CacheListDomainCoverage> {
    let mut domains = std::collections::BTreeSet::new();
    for path in baseline.iter().chain(candidate.iter()) {
        domains.insert(classify_domain(path));
    }

    domains
        .into_iter()
        .map(|domain| {
            let baseline_count =
                baseline.iter().filter(|path| classify_domain(path) == domain).count();
            let candidate_count =
                candidate.iter().filter(|path| classify_domain(path) == domain).count();
            let intersection_count = baseline
                .iter()
                .filter(|path| classify_domain(path) == domain && candidate.contains(*path))
                .count();

            CacheListDomainCoverage {
                domain: domain.to_string(),
                baseline_count,
                candidate_count,
                intersection_count,
                baseline_coverage_pct: if baseline_count == 0 {
                    0.0
                } else {
                    round_pct(intersection_count as f64 / baseline_count as f64 * 100.0)
                },
                candidate_overlap_pct: if candidate_count == 0 {
                    0.0
                } else {
                    round_pct(intersection_count as f64 / candidate_count as f64 * 100.0)
                },
            }
        })
        .collect()
}

/// Compare two cache-list path sets and return a structured report.
pub fn compare_cache_list_path_sets(
    baseline: &BTreeSet<String>,
    candidate: &BTreeSet<String>,
) -> CacheListComparisonReport {
    let intersection = baseline.intersection(candidate).count();
    let baseline_only = baseline.difference(candidate).cloned().collect::<BTreeSet<_>>();
    let candidate_only = candidate.difference(baseline).cloned().collect::<BTreeSet<_>>();

    CacheListComparisonReport {
        baseline_unique_count: baseline.len(),
        candidate_unique_count: candidate.len(),
        intersection_count: intersection,
        baseline_only_count: baseline_only.len(),
        candidate_only_count: candidate_only.len(),
        baseline_coverage_pct: if baseline.is_empty() {
            0.0
        } else {
            round_pct(intersection as f64 / baseline.len() as f64 * 100.0)
        },
        candidate_overlap_pct: if candidate.is_empty() {
            0.0
        } else {
            round_pct(intersection as f64 / candidate.len() as f64 * 100.0)
        },
        baseline_only_paths: baseline_only.iter().cloned().collect(),
        candidate_only_paths: candidate_only.iter().cloned().collect(),
        baseline_only_prefixes: group_paths_by_prefix(&baseline_only),
        candidate_only_prefixes: group_paths_by_prefix(&candidate_only),
        domain_coverages: compute_domain_coverages(baseline, candidate),
        rule_authored_candidate_count: 0,
        fallback_authored_candidate_count: 0,
        fallback_authored_candidate_prefixes: Vec::new(),
        sound_rule_authored_candidate_count: 0,
        sound_fallback_authored_candidate_count: 0,
        sound_fallback_authored_candidate_prefixes: Vec::new(),
        template_rule_authored_candidate_families: Vec::new(),
        template_fallback_authored_candidate_families: Vec::new(),
        template_fallback_residual_reasons: Vec::new(),
        unresolved_rule_blockers: Vec::new(),
        repo_fallback_bundle_assets: Vec::new(),
        missing_bundle_assets: Vec::new(),
        migration_blockers: Vec::new(),
        migration_ready: None,
    }
}

/// Apply decoder-first build diagnostics to a comparison report and derive migration readiness.
pub fn apply_candidate_build_diagnostics(
    report: &mut CacheListComparisonReport,
    diagnostics: &CacheListBuildDiagnostics,
) {
    let sound_rule_authored_paths = diagnostics
        .rule_authored_paths
        .iter()
        .filter(|path| path.starts_with("kcs/sound/"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let sound_fallback_authored_paths = diagnostics
        .fallback_authored_paths
        .iter()
        .filter(|path| path.starts_with("kcs/sound/"))
        .cloned()
        .collect::<BTreeSet<_>>();

    report.rule_authored_candidate_count = diagnostics.rule_authored_count;
    report.fallback_authored_candidate_count = diagnostics.fallback_authored_count;
    report.fallback_authored_candidate_prefixes = diagnostics.fallback_authored_prefixes.clone();
    report.sound_rule_authored_candidate_count = sound_rule_authored_paths.len();
    report.sound_fallback_authored_candidate_count = sound_fallback_authored_paths.len();
    report.sound_fallback_authored_candidate_prefixes =
        group_paths_by_prefix(&sound_fallback_authored_paths);
    report.template_rule_authored_candidate_families =
        group_template_paths_by_family(diagnostics.rule_authored_paths.iter());
    report.template_fallback_authored_candidate_families =
        group_template_paths_by_family(diagnostics.fallback_authored_paths.iter());
    report.template_fallback_residual_reasons =
        if diagnostics.template_fallback_residual_reasons.is_empty() {
            let fallback_paths =
                diagnostics.fallback_authored_paths.iter().cloned().collect::<BTreeSet<_>>();
            template_residual_reasons(&fallback_paths)
        } else {
            diagnostics.template_fallback_residual_reasons.clone()
        };
    report.unresolved_rule_blockers = diagnostics.unresolved_rule_blockers.clone();
    report.repo_fallback_bundle_assets = diagnostics.repo_fallback_bundle_assets.clone();
    report.missing_bundle_assets = diagnostics.missing_bundle_assets.clone();
    report.migration_blockers = collect_migration_blockers(report);
    report.migration_ready = Some(report.migration_blockers.is_empty());
}

fn collect_migration_blockers(report: &CacheListComparisonReport) -> Vec<String> {
    let mut blockers = Vec::new();

    if report.baseline_only_count > 0 {
        blockers.push(format!("baseline-only paths: {}", report.baseline_only_count));
    }
    if report.fallback_authored_candidate_count > 0 {
        blockers.push(format!(
            "fallback-authored candidate paths: {}",
            report.fallback_authored_candidate_count
        ));
    }
    if !report.unresolved_rule_blockers.is_empty() {
        blockers.push(format!(
            "unresolved rule blockers: {}",
            report.unresolved_rule_blockers.join(", ")
        ));
    }
    if !report.template_fallback_residual_reasons.is_empty() {
        let families = report
            .template_fallback_residual_reasons
            .iter()
            .map(|reason| {
                format!(
                    "{} ({}): {}: {}",
                    reason.family,
                    reason.count,
                    template_residual_kind_label(reason.kind),
                    reason.reason
                )
            })
            .collect::<Vec<_>>();
        blockers.push(format!("template-backed fallback residuals: {}", families.join("; ")));
    } else if !report.template_fallback_authored_candidate_families.is_empty() {
        let families = report
            .template_fallback_authored_candidate_families
            .iter()
            .map(|family| format!("{} ({})", family.prefix, family.count))
            .collect::<Vec<_>>();
        blockers.push(format!("template-backed fallback residuals: {}", families.join(", ")));
    }
    if !report.repo_fallback_bundle_assets.is_empty() {
        blockers.push(format!(
            "repo fallback bundle assets: {}",
            report.repo_fallback_bundle_assets.join(", ")
        ));
    }
    if !report.missing_bundle_assets.is_empty() {
        blockers
            .push(format!("missing bundle assets: {}", report.missing_bundle_assets.join(", ")));
    }

    blockers
}

async fn build_list(
    codex: &Codex,
    kache: &Kache,
    strategy: CacheListMakeStrategy,
    manifest_override: Option<&manifest::ResourceManifest>,
    decoder_assets_override: Option<&manifest::DecoderCoverageAssets>,
    rules_bundle_override: Option<&manifest::DecoderRulesBundle>,
) -> Result<CacheList, CacheListMakingError> {
    let mut list = CacheList::new();
    source::make(
        codex,
        kache,
        strategy,
        manifest_override,
        decoder_assets_override,
        rules_bundle_override,
        &mut list,
    )
    .await?;
    Ok(list)
}

/// Build a cache-list path set in memory.
pub async fn build_cache_list_paths(
    codex: &Codex,
    kache: &Kache,
    strategy: CacheListMakeStrategy,
) -> Result<BTreeSet<String>, CacheListMakingError> {
    Ok(build_list(codex, kache, strategy, None, None, None).await?.into_path_set())
}

/// Build a cache-list item list in memory.
pub async fn build_cache_list_items(
    codex: &Codex,
    kache: &Kache,
    strategy: CacheListMakeStrategy,
) -> Result<Vec<CacheListItem>, CacheListMakingError> {
    Ok(build_list(codex, kache, strategy, None, None, None).await?.into_items())
}

/// Build a cache-list path set in memory with an explicit manifest override.
pub async fn build_cache_list_paths_with_manifest_path(
    codex: &Codex,
    kache: &Kache,
    strategy: CacheListMakeStrategy,
    manifest_path: impl AsRef<Path>,
) -> Result<BTreeSet<String>, CacheListMakingError> {
    let manifest_data = manifest::load_resource_manifest_from_path(&manifest_path)?;
    let decoder_assets = manifest::load_decoder_coverage_assets_from_manifest_path(&manifest_path)?;
    Ok(build_list(codex, kache, strategy, Some(&manifest_data), Some(&decoder_assets), None)
        .await?
        .into_path_set())
}

/// Build a cache-list item list in memory with an explicit manifest override.
pub async fn build_cache_list_items_with_manifest_path(
    codex: &Codex,
    kache: &Kache,
    strategy: CacheListMakeStrategy,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<CacheListItem>, CacheListMakingError> {
    let manifest_data = manifest::load_resource_manifest_from_path(&manifest_path)?;
    let decoder_assets = manifest::load_decoder_coverage_assets_from_manifest_path(&manifest_path)?;
    Ok(build_list(codex, kache, strategy, Some(&manifest_data), Some(&decoder_assets), None)
        .await?
        .into_items())
}

/// Build a cache-list path set in memory with an explicit cache-rules override.
pub async fn build_cache_list_paths_with_rules_path(
    codex: &Codex,
    kache: &Kache,
    rules_path: impl AsRef<Path>,
) -> Result<BTreeSet<String>, CacheListMakingError> {
    Ok(build_cache_list_path_output_with_rules_path(codex, kache, rules_path).await?.paths)
}

/// Build a cache-list item list in memory with an explicit cache-rules override.
pub async fn build_cache_list_items_with_rules_path(
    codex: &Codex,
    kache: &Kache,
    rules_path: impl AsRef<Path>,
) -> Result<Vec<CacheListItem>, CacheListMakingError> {
    let rules_bundle = manifest::load_cache_rules_bundle_from_path(rules_path)?;
    Ok(build_list(codex, kache, CacheListMakeStrategy::Rules, None, None, Some(&rules_bundle))
        .await?
        .into_items())
}

/// Build a cache-list path set plus decoder-first diagnostics in memory with an explicit cache-rules override.
pub async fn build_cache_list_path_output_with_rules_path(
    codex: &Codex,
    kache: &Kache,
    rules_path: impl AsRef<Path>,
) -> Result<CacheListPathBuildOutput, CacheListMakingError> {
    let rules_bundle = manifest::load_cache_rules_bundle_from_path(rules_path)?;
    Ok(build_list(codex, kache, CacheListMakeStrategy::Rules, None, None, Some(&rules_bundle))
        .await?
        .into_path_build_output())
}

/// Make a cache list.
///
/// # Arguments
///
/// * `mst` - The API manifest.
/// * `kache` - The kache instance.
/// * `outpath` - The output path.
/// * `overwrite` - Whether to overwrite the output file if it already exists.
///
/// # Returns
///
/// A `Result` containing either `Ok(())` if the cache list was successfully made, or an error if it failed.
pub async fn make(
    codex: &Codex,
    kache: &Kache,
    outpath: impl AsRef<std::path::Path>,
    strategy: CacheListMakeStrategy,
    overwrite: bool,
) -> Result<(), CacheListMakingError> {
    let out = outpath.as_ref().to_owned();
    if !overwrite && out.exists() {
        return Err(CacheListMakingError::FileExists(out));
    }

    info!("making cache list to {:?}", out);

    let list = build_list(codex, kache, strategy.clone(), None, None, None).await?;

    for item in list.items.iter() {
        let line = serde_json::to_string(item)?;
        debug!("{}", line);
    }

    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&out).await?;
    for item in list.items.iter() {
        let line = serde_json::to_string(item)?;
        file.write_all(line.as_bytes()).await?;
        file.write_u8(b'\n').await?;
    }

    file.sync_all().await?;

    info!("cache list made to {:?}", out);

    // Generate holes report for greedy mode
    if matches!(strategy, CacheListMakeStrategy::Greedy(_)) {
        let holes_path = out.parent().unwrap_or(std::path::Path::new(".")).join("holes_report.txt");
        let holes = source::kcs2::resources::ship::get_holes_report();
        if !holes.is_empty() {
            let mut holes_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&holes_path)
                .await?;
            holes_file.write_all(b"// Generated holes report - copy to source files\n\n").await?;
            for hole in holes {
                holes_file.write_all(hole.as_bytes()).await?;
                holes_file.write_all(b"\n\n").await?;
            }
            holes_file.sync_all().await?;
            info!("holes report saved to {:?}", holes_path);
        }
        source::kcs2::resources::ship::clear_holes_report();
    }

    Ok(())
}

const MAX_CHECK_SIZE: usize = 32;

/// Check if a list of URLs exist on the remote cache.
///
/// # Arguments
///
/// * `cache` - The remote cache to check against.
/// * `urls` - The list of URLs to check.
/// * `concurrent` - The maximum number of concurrent checks.
/// * `tracker` - Optional progress tracker.
///
/// # Returns
///
/// A `HashMap` mapping each URL to a boolean indicating whether it exists on the remote cache.
pub async fn batch_check_exists(
    cache: Arc<Kache>,
    mut urls: Vec<(String, String)>,
    concurrent: usize,
    tracker: Option<Arc<progress::ProgressTracker>>,
) -> Result<HashMap<(String, String), bool>, KacheError> {
    let q = concurrent.clamp(1, MAX_CHECK_SIZE);
    let mut result: HashMap<(String, String), bool> = HashMap::new();
    let mut tasks = FuturesUnordered::new();
    let mut check_count = 0;

    loop {
        while tasks.len() < q {
            match urls.pop() {
                Some((url, ver)) => {
                    let c = cache.clone();
                    let key = url.clone();
                    let t = tracker.clone();
                    tasks.push(async move {
                        let exists = c.exists_on_remote(&key, &ver).await?;
                        if let Some(tracker) = t {
                            tracker.increment_checked();
                            if exists {
                                tracker.increment_found();
                            }
                        }
                        Ok::<((String, String), bool), KacheError>(((key, ver), exists))
                    });
                }
                None => {
                    break;
                }
            }
        }

        if tasks.is_empty() {
            break;
        }

        match tasks.next().await {
            Some(Ok(((key, ver), exists))) => {
                result.insert((key, ver), exists);
                check_count += 1;
                if let Some(ref t) = tracker
                    && check_count % 100 == 0
                {
                    t.report();
                }
            }
            Some(Err(err)) => {
                return Err(err);
            }
            None => {
                break;
            }
        }
    }

    if let Some(ref t) = tracker {
        t.report();
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn codex_root() -> PathBuf {
        repo_root().join(".data/codex")
    }

    fn manifest_path() -> PathBuf {
        repo_root().join("crates/emukc_bootstrap/assets/resource_manifest.json")
    }

    fn make_kache() -> Kache {
        let cache_root = repo_root().join("z/cache");
        let db_path = repo_root().join(".data/tmp").join(format!(
            "kache-make-list-test-{}.redb",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        Kache::builder()
            .with_cache_root(cache_root)
            .with_mods_root(Some(repo_root().join("z/mods")))
            .with_db_path(db_path.to_string_lossy().into_owned())
            .with_proxy(Some("socks5://127.0.0.1:1086".to_string()))
            .with_gadgets_cdn("w00g.kancolle-server.com".to_string())
            .with_content_cdn("w01y.kancolle-server.com".to_string())
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn build_cache_list_paths_with_manifest_path_matches_repo_manifest_strategy() {
        let codex = Codex::load(codex_root(), true).unwrap();
        let kache = make_kache();

        let baseline =
            build_cache_list_paths(&codex, &kache, CacheListMakeStrategy::Manifest).await.unwrap();
        let overridden = build_cache_list_paths_with_manifest_path(
            &codex,
            &kache,
            CacheListMakeStrategy::Manifest,
            manifest_path(),
        )
        .await
        .unwrap();

        assert_eq!(baseline, overridden);
    }

    #[tokio::test]
    async fn build_cache_list_paths_with_rules_path_loads_explicit_rule_bundle() {
        let codex = Codex::load(codex_root(), true).unwrap();
        let kache = make_kache();
        let temp_dir = repo_root().join(".data/tmp");
        fs::create_dir_all(&temp_dir).unwrap();
        let rules_path = temp_dir.join("cache_rules.integration.json");

        let payload = json!({
            "version": 1,
            "generatedAt": "2026-04-23T00:00:00Z",
            "scriptVersion": "6.2.8.0",
            "summary": {
                "shipRuleCount": 1,
                "slotRuleCount": 2,
                "observedCompleteRuleCount": 3,
                "partialRuleCount": 0,
                "unresolvedRuleCount": 0
            },
            "resourceManifest": {
                "version": 2,
                "generatedAt": "2026-04-23T00:00:00Z",
                "summary": {
                    "totalEntries": 2,
                    "shipEntryCount": 1,
                    "slotitemEntryCount": 1,
                    "textureProviderEntryCount": 0,
                    "explicitPathEntryCount": 0,
                    "totalExplicitPaths": 0,
                    "modulesCovered": 1
                },
                "pathRules": null,
                "entries": [
                    {
                        "kind": "ship",
                        "source": "test",
                        "targetType": "special",
                        "shipMstIdSource": "this._mst_id",
                        "damagedSource": "false",
                        "moduleIds": [],
                        "moduleNames": []
                    },
                    {
                        "kind": "slotitem",
                        "source": "test",
                        "targetType": "item_up",
                        "slotMstIdSources": ["this._slot1.mstID"],
                        "moduleIds": [],
                        "moduleNames": []
                    }
                ]
            },
            "resourceCategories": {
                "version": 1,
                "generatedAt": "2026-04-23T00:00:00Z",
                "scriptVersion": "6.2.8.0",
                "summary": {
                    "shipTargetTypeCount": 0,
                    "slotTargetTypeCount": 0,
                    "spRemodelSubcategoryCount": 0,
                    "shipGenerationGroupCount": 0,
                    "slotGenerationGroupCount": 0
                },
                "shipTargetTypes": [],
                "slotTargetTypes": [],
                "shipGenerationGroups": {
                    "defaultFriendly": [],
                    "defaultAbyssal": [],
                    "friendGraph": [],
                    "enemyGraph": []
                },
                "slotGenerationGroups": {
                    "default": [],
                    "baga": [],
                    "airunit": []
                },
                "spRemodelSubcategories": []
            },
            "shipRules": {
                "special": {
                    "coverageMode": "observed-complete",
                    "kind": "special_cases",
                    "cases": [{ "damaged": false, "shipIds": [1] }],
                    "moduleIds": ["m1"],
                    "moduleNames": ["special-module"]
                }
            },
            "slotRules": {
                "itemUp": {
                    "coverageMode": "observed-complete",
                    "kind": "item_up_normalization",
                    "replaceMap": { "1501": 1 },
                    "enemySlotBorder": 1500,
                    "exclude": [],
                    "moduleIds": ["m2"],
                    "moduleNames": ["slot-loader"]
                },
                "btxtFlat": {
                    "coverageMode": "observed-complete",
                    "kind": "btxt_flat_non_enemy_runtime_slots",
                    "excludeEnemyItems": true,
                    "moduleIds": ["m3"],
                    "moduleNames": ["btxt-module"]
                }
            },
            "unresolvedRules": []
        });
        fs::write(&rules_path, serde_json::to_string_pretty(&payload).unwrap()).unwrap();

        let paths =
            build_cache_list_paths_with_rules_path(&codex, &kache, &rules_path).await.unwrap();

        assert!(paths.iter().any(|path| path.contains("kcs2/resources/ship/special/0001_")));
        assert!(paths.iter().any(|path| path.contains("kcs2/resources/slot/item_up/0001_")));
        assert!(!paths.iter().any(|path| path.contains("kcs2/resources/slot/item_up/1501_")));
    }

    #[test]
    fn cache_list_path_build_output_reports_authority_diagnostics() {
        let mut list = CacheList::new();
        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        list.add_unversioned("kcs2/resources/se/999.mp3".to_string());
        list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
        list.add_unversioned("gadget_html5/js/kcs_const.js".to_string());
        list.record_unresolved_rule_blockers(["ship.targetSemantics"]);
        list.record_repo_fallback_bundle_assets(["resource_categories.json"]);
        list.set_authority_stage(previous);

        let output = list.into_path_build_output();

        assert!(output.paths.contains("kcs2/resources/se/999.mp3"));
        assert_eq!(output.diagnostics.rule_authored_count, 1);
        assert_eq!(output.diagnostics.fallback_authored_count, 1);
        assert_eq!(
            output.diagnostics.unresolved_rule_blockers,
            vec!["ship.targetSemantics".to_string()]
        );
        assert_eq!(
            output.diagnostics.repo_fallback_bundle_assets,
            vec!["resource_categories.json".to_string()]
        );
    }

    #[test]
    fn compare_cache_list_report_counts_overlap_and_prefix_deltas() {
        let baseline = BTreeSet::from([
            "gadget_html5/js/kcs_const.js".to_string(),
            "kcs2/resources/ship/full/0001_0000_1.png".to_string(),
            "kcs2/resources/slot/card/0001_0000.png".to_string(),
        ]);
        let candidate = BTreeSet::from([
            "gadget_html5/js/kcs_const.js".to_string(),
            "kcs2/resources/ship/full/0001_0000_1.png".to_string(),
            "kcs2/resources/ship/banner/0001_0000.png".to_string(),
        ]);

        let report = compare_cache_list_path_sets(&baseline, &candidate);

        assert_eq!(report.baseline_unique_count, 3);
        assert_eq!(report.candidate_unique_count, 3);
        assert_eq!(report.intersection_count, 2);
        assert_eq!(report.baseline_only_count, 1);
        assert_eq!(report.candidate_only_count, 1);
        assert_eq!(report.baseline_coverage_pct, 66.67);
        assert_eq!(report.candidate_overlap_pct, 66.67);
        assert!(
            report
                .baseline_only_paths
                .contains(&"kcs2/resources/slot/card/0001_0000.png".to_string())
        );
        assert!(
            report
                .candidate_only_paths
                .contains(&"kcs2/resources/ship/banner/0001_0000.png".to_string())
        );
        assert_eq!(report.baseline_only_prefixes[0].prefix, "kcs2/resources/slot/card");
        assert_eq!(report.candidate_only_prefixes[0].prefix, "kcs2/resources/ship/banner");
    }

    #[test]
    fn apply_candidate_build_diagnostics_marks_migration_blockers() {
        let baseline = BTreeSet::from(["kcs2/resources/ship/full/0001.png".to_string()]);
        let candidate = BTreeSet::from(["kcs2/resources/ship/banner/0001.png".to_string()]);
        let mut report = compare_cache_list_path_sets(&baseline, &candidate);
        let diagnostics = CacheListBuildDiagnostics {
            rule_authored_count: 10,
            rule_authored_paths: Vec::new(),
            fallback_authored_count: 3,
            fallback_authored_paths: vec![
                "gadget_html5/js/kcs_const.js".to_string(),
                "kcs2/resources/stype/etext/001.png".to_string(),
                "kcs2/resources/stype/etext/002.png".to_string(),
            ],
            fallback_authored_prefixes: vec![CacheListPathPrefixCount {
                prefix: "kcs2/resources/stype/etext".to_string(),
                count: 2,
            }],
            template_fallback_residual_reasons: Vec::new(),
            unresolved_rule_blockers: vec!["ship.targetSemantics".to_string()],
            repo_fallback_bundle_assets: vec!["ui_resources.json".to_string()],
            missing_bundle_assets: vec!["audio_resources.json".to_string()],
        };

        apply_candidate_build_diagnostics(&mut report, &diagnostics);

        assert_eq!(report.rule_authored_candidate_count, 10);
        assert_eq!(report.fallback_authored_candidate_count, 3);
        assert_eq!(report.sound_rule_authored_candidate_count, 0);
        assert_eq!(report.sound_fallback_authored_candidate_count, 0);
        assert_eq!(
            report.fallback_authored_candidate_prefixes,
            diagnostics.fallback_authored_prefixes
        );
        assert_eq!(report.migration_ready, Some(false));
        assert!(report.migration_blockers.contains(&"baseline-only paths: 1".to_string()));
        assert!(
            report.migration_blockers.contains(&"fallback-authored candidate paths: 3".to_string())
        );
        assert!(
            report
                .migration_blockers
                .contains(&"unresolved rule blockers: ship.targetSemantics".to_string())
        );
        assert!(
            report
                .migration_blockers
                .contains(&"repo fallback bundle assets: ui_resources.json".to_string())
        );
        assert!(
            report
                .migration_blockers
                .contains(&"missing bundle assets: audio_resources.json".to_string())
        );
    }

    #[test]
    fn apply_candidate_build_diagnostics_marks_migration_ready_when_clear() {
        let baseline = BTreeSet::from(["kcs2/resources/ship/full/0001.png".to_string()]);
        let candidate = BTreeSet::from(["kcs2/resources/ship/full/0001.png".to_string()]);
        let mut report = compare_cache_list_path_sets(&baseline, &candidate);
        let diagnostics = CacheListBuildDiagnostics {
            rule_authored_count: 1,
            ..Default::default()
        };

        apply_candidate_build_diagnostics(&mut report, &diagnostics);

        assert_eq!(report.rule_authored_candidate_count, 1);
        assert_eq!(report.fallback_authored_candidate_count, 0);
        assert_eq!(report.sound_rule_authored_candidate_count, 0);
        assert_eq!(report.sound_fallback_authored_candidate_count, 0);
        assert_eq!(report.migration_ready, Some(true));
        assert!(report.migration_blockers.is_empty());
    }

    #[test]
    fn apply_candidate_build_diagnostics_tracks_sound_counts() {
        let baseline = BTreeSet::new();
        let candidate = BTreeSet::from(["kcs/sound/kc9999/11.mp3".to_string()]);
        let mut report = compare_cache_list_path_sets(&baseline, &candidate);
        let diagnostics = CacheListBuildDiagnostics {
            rule_authored_count: 2,
            rule_authored_paths: vec![
                "kcs/sound/kc9999/11.mp3".to_string(),
                "kcs/sound/kc9998/22.mp3".to_string(),
            ],
            fallback_authored_count: 2,
            fallback_authored_paths: vec![
                "kcs/sound/kc9997/33.mp3".to_string(),
                "gadget_html5/js/kcs_const.js".to_string(),
            ],
            fallback_authored_prefixes: vec![CacheListPathPrefixCount {
                prefix: "kcs/sound/kc9997".to_string(),
                count: 1,
            }],
            template_fallback_residual_reasons: Vec::new(),
            unresolved_rule_blockers: Vec::new(),
            repo_fallback_bundle_assets: Vec::new(),
            missing_bundle_assets: Vec::new(),
        };

        apply_candidate_build_diagnostics(&mut report, &diagnostics);

        assert_eq!(report.sound_rule_authored_candidate_count, 2);
        assert_eq!(report.sound_fallback_authored_candidate_count, 1);
        assert_eq!(
            report.sound_fallback_authored_candidate_prefixes,
            vec![CacheListPathPrefixCount {
                prefix: "kcs/sound/kc9997".to_string(),
                count: 1,
            }]
        );
    }

    #[test]
    fn apply_candidate_build_diagnostics_tracks_template_families() {
        let baseline = BTreeSet::from(["kcs2/resources/map/001/01.png".to_string()]);
        let candidate = baseline.clone();
        let mut report = compare_cache_list_path_sets(&baseline, &candidate);
        let diagnostics = CacheListBuildDiagnostics {
            rule_authored_count: 2,
            rule_authored_paths: vec![
                "kcs2/resources/map/001/01.png".to_string(),
                "kcs2/resources/voice/titlecall_1/001.mp3".to_string(),
            ],
            fallback_authored_count: 1,
            fallback_authored_paths: vec![
                "kcs2/resources/furniture/normal/001_0001.png".to_string(),
            ],
            fallback_authored_prefixes: vec![CacheListPathPrefixCount {
                prefix: "kcs2/resources/furniture/normal".to_string(),
                count: 1,
            }],
            ..Default::default()
        };

        apply_candidate_build_diagnostics(&mut report, &diagnostics);

        assert_eq!(
            report.template_rule_authored_candidate_families,
            vec![
                CacheListPathPrefixCount {
                    prefix: "map".to_string(),
                    count: 1,
                },
                CacheListPathPrefixCount {
                    prefix: "voice.titlecall".to_string(),
                    count: 1,
                },
            ]
        );
        assert_eq!(
            report.template_fallback_authored_candidate_families,
            vec![CacheListPathPrefixCount {
                prefix: "furniture".to_string(),
                count: 1,
            }]
        );
        assert_eq!(
            report.template_fallback_residual_reasons,
            vec![CacheListTemplateResidualReason {
                family: "furniture".to_string(),
                count: 1,
                kind: CacheListTemplateResidualReasonKind::UncoveredResidualMembership,
                reason: "decoder furniture templates cover observed categories, but fallback still owns residual furniture members outside those descriptors".to_string(),
            }]
        );
        assert!(report.migration_blockers.iter().any(|blocker| {
            blocker.contains(
                "template-backed fallback residuals: furniture (1): uncovered-residual-membership",
            )
        }));
    }
}
