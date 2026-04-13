#![allow(missing_docs)]

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use emukc_cache::IntoVersion;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;
use serde::{Deserialize, Serialize};

use crate::make_list::{CacheList, errors::CacheListMakingError, has_btxt_flat_coverage};

const EMBEDDED_BATTLE_PROTOCOL_FIELDS_JSON: &str =
    include_str!("../assets/battle_protocol_fields.json");
const EMBEDDED_BATTLE_RESOURCE_RULES_JSON: &str =
    include_str!("../assets/battle_resource_rules.json");
const EMBEDDED_BATTLE_MODULE_INDEX_JSON: &str = include_str!("../assets/battle_module_index.json");
const EMBEDDED_BATTLE_SLOT_RESOURCE_TRIGGERS_JSON: &str =
    include_str!("../assets/battle_slot_resource_triggers.json");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoBattleKnowledgeSource {
    Filesystem(PathBuf),
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleKnowledgeAssetSources {
    pub protocol_fields: RepoBattleKnowledgeSource,
    pub resource_rules: RepoBattleKnowledgeSource,
    pub module_index: RepoBattleKnowledgeSource,
    pub slot_resource_triggers: RepoBattleKnowledgeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleProtocolFieldRule {
    pub id: String,
    pub module_id: String,
    pub readable_name: String,
    pub field: String,
    pub access_kind: String,
    pub source_object: Option<String>,
    pub conditional: bool,
    pub phases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleResourceRule {
    pub id: String,
    pub module_id: String,
    pub readable_name: String,
    pub resource_kind: String,
    pub action: String,
    pub target_type: Option<String>,
    pub provider: Option<String>,
    pub texture_ids: Vec<i64>,
    pub ship_mst_id_source: Option<String>,
    pub damaged_source: Option<String>,
    pub slot_mst_id_sources: Vec<String>,
    pub explicit_paths: Vec<String>,
    pub trigger_hints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleKnowledgeModuleDependency {
    pub module_id: String,
    pub readable_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleModuleKnowledge {
    pub id: String,
    pub readable_name: String,
    pub file_name: String,
    pub module_kind: String,
    pub cleanup_tier: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<BattleKnowledgeModuleDependency>,
    pub protocol_fields: Vec<String>,
    pub resource_rule_ids: Vec<String>,
    pub explicit_resource_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleProtocolFieldsSummary {
    pub module_count: usize,
    pub protocol_field_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleResourceRulesSummary {
    pub module_count: usize,
    pub resource_rule_count: usize,
    pub explicit_resource_path_count: usize,
    pub ship_resource_rule_count: usize,
    pub slotitem_resource_rule_count: usize,
    pub texture_provider_rule_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleModuleIndexSummary {
    pub module_count: usize,
    pub protocol_field_count: usize,
    pub resource_rule_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleProtocolFieldsAsset {
    pub script_version: String,
    pub summary: BattleProtocolFieldsSummary,
    pub fields: Vec<BattleProtocolFieldRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleResourceRulesAsset {
    pub script_version: String,
    pub summary: BattleResourceRulesSummary,
    pub rules: Vec<BattleResourceRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleModuleIndexAsset {
    pub script_version: String,
    pub summary: BattleModuleIndexSummary,
    pub modules: Vec<BattleModuleKnowledge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleSlotResourceTrigger {
    pub id: String,
    pub consumer_module_id: String,
    pub consumer_readable_name: String,
    pub protocol_sources: Vec<String>,
    pub resource_target: String,
    pub confidence: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleSlotResourceTriggersSummary {
    pub module_count: usize,
    pub slot_resource_trigger_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleSlotResourceTriggersAsset {
    pub script_version: String,
    pub summary: BattleSlotResourceTriggersSummary,
    pub triggers: Vec<BattleSlotResourceTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleKnowledgeAssets {
    pub sources: BattleKnowledgeAssetSources,
    pub protocol_fields: BattleProtocolFieldsAsset,
    pub resource_rules: BattleResourceRulesAsset,
    pub module_index: BattleModuleIndexAsset,
    pub slot_resource_triggers: BattleSlotResourceTriggersAsset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleValidationFindingKind {
    MissingField,
    InvalidFieldShape,
    ArrayLengthMismatch,
    FlagPayloadMismatch,
    UnknownShipMstId,
    MissingShipGraph,
    UnknownSlotitemMstId,
    ProtocolSuspicion,
    BootstrapGap,
    UnexpectedBattleSlotResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleValidationFinding {
    pub severity: BattleValidationSeverity,
    pub kind: BattleValidationFindingKind,
    pub field: Option<String>,
    pub message: String,
    pub resource_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleIncidentTriggerMatch {
    pub protocol_source: String,
    pub consumer_module: String,
    pub resource_target: String,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedBattleResource {
    pub kind: String,
    pub entity_id: i64,
    pub target_type: String,
    pub path: String,
    pub note: String,
    pub protocol_source: Option<String>,
    pub consumer_module: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleValidationReport {
    pub findings: Vec<BattleValidationFinding>,
    pub expected_resources: Vec<ExpectedBattleResource>,
    pub candidate_resources: Vec<ExpectedBattleResource>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleIncidentReport {
    pub validation: BattleValidationReport,
    pub missing_resource_path: Option<String>,
    pub trigger_matches: Vec<BattleIncidentTriggerMatch>,
    pub protocol_suspicions: Vec<BattleValidationFinding>,
    pub bootstrap_gaps: Vec<BattleValidationFinding>,
}

impl BattleValidationReport {
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|finding| finding.severity == BattleValidationSeverity::Error)
    }
}

const DAY_BATTLE_ARRAY_FIELDS: &[&str] = &[
    "api_ship_ke",
    "api_ship_lv",
    "api_e_nowhps",
    "api_e_maxhps",
    "api_eSlot",
    "api_eParam",
    "api_f_nowhps",
    "api_f_maxhps",
    "api_fParam",
];

const DAY_BATTLE_SCALAR_FLAG_FIELDS: &[&str] = &["api_opening_flag", "api_opening_taisen_flag"];

const DAY_BATTLE_ARRAY_FLAG_FIELDS: &[&str] = &["api_stage_flag", "api_hourai_flag"];

const DAY_BATTLE_PROTOCOL_PAYLOAD_ALLOWLIST: &[&str] = &[
    "api_kouku",
    "api_hougeki1",
    "api_hougeki2",
    "api_hougeki3",
    "api_opening_atack",
    "api_opening_taisen",
    "api_raigeki",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DamageState {
    Healthy,
    Shouha,
    Chuuha,
    Taiha,
    Sunk,
}

#[derive(Debug, Default)]
struct SlotitemResourceTargets {
    expected: BTreeSet<String>,
    candidate: BTreeSet<String>,
}

pub fn repo_battle_protocol_fields_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/battle_protocol_fields.json")
}

pub fn repo_battle_resource_rules_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/battle_resource_rules.json")
}

pub fn repo_battle_module_index_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/battle_module_index.json")
}

pub fn repo_battle_slot_resource_triggers_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/battle_slot_resource_triggers.json")
}

fn load_text_asset_from(
    path: &Path,
    embedded: &'static str,
) -> io::Result<(RepoBattleKnowledgeSource, Cow<'static, str>)> {
    match fs::read_to_string(path) {
        Ok(raw_json) => {
            Ok((RepoBattleKnowledgeSource::Filesystem(path.to_path_buf()), Cow::Owned(raw_json)))
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            Ok((RepoBattleKnowledgeSource::Embedded, Cow::Borrowed(embedded)))
        }
        Err(source) => Err(source),
    }
}

pub fn load_repo_battle_knowledge_assets() -> io::Result<BattleKnowledgeAssets> {
    let protocol_path = repo_battle_protocol_fields_path();
    let resource_path = repo_battle_resource_rules_path();
    let module_index_path = repo_battle_module_index_path();
    let slot_resource_triggers_path = repo_battle_slot_resource_triggers_path();
    let (protocol_source, protocol_json) =
        load_text_asset_from(&protocol_path, EMBEDDED_BATTLE_PROTOCOL_FIELDS_JSON)?;
    let (resource_source, resource_json) =
        load_text_asset_from(&resource_path, EMBEDDED_BATTLE_RESOURCE_RULES_JSON)?;
    let (module_index_source, module_index_json) =
        load_text_asset_from(&module_index_path, EMBEDDED_BATTLE_MODULE_INDEX_JSON)?;
    let (slot_resource_triggers_source, slot_resource_triggers_json) = load_text_asset_from(
        &slot_resource_triggers_path,
        EMBEDDED_BATTLE_SLOT_RESOURCE_TRIGGERS_JSON,
    )?;

    Ok(BattleKnowledgeAssets {
        sources: BattleKnowledgeAssetSources {
            protocol_fields: protocol_source,
            resource_rules: resource_source,
            module_index: module_index_source,
            slot_resource_triggers: slot_resource_triggers_source,
        },
        protocol_fields: serde_json::from_str(&protocol_json)?,
        resource_rules: serde_json::from_str(&resource_json)?,
        module_index: serde_json::from_str(&module_index_json)?,
        slot_resource_triggers: serde_json::from_str(&slot_resource_triggers_json)?,
    })
}

fn texture_provider_paths(provider: &str) -> &'static [&'static str] {
    match provider {
        "BATTLE_AIRUNIT" => &["battle/battle_airunit.json", "battle/battle_airunit.png"],
        "BATTLE_CUTIN_ANTI_AIR" => {
            &["battle/battle_cutin_anti_air.json", "battle/battle_cutin_anti_air.png"]
        }
        "BATTLE_CUTIN_DAMAGE" => {
            &["battle/battle_cutin_damage.json", "battle/battle_cutin_damage.png"]
        }
        "BATTLE_CUTIN_GOUCHIN" => {
            &["battle/battle_cutin_gouchin.json", "battle/battle_cutin_gouchin.png"]
        }
        "BATTLE_MAIN" => &["battle/battle_main.json", "battle/battle_main.png"],
        "BATTLE_NIGHT" => &["battle/battle_night.json", "battle/battle_night.png"],
        "BATTLE_RESULT_EVENT_BASE" => &[
            "battle_result/battle_result_event_base.json",
            "battle_result/battle_result_event_base.png",
        ],
        "BATTLE_RESULT_MAIN" => {
            &["battle_result/battle_result_main.json", "battle_result/battle_result_main.png"]
        }
        "BATTLE_ZRK" => &["battle/battle_zrk.json", "battle/battle_zrk.png"],
        "COMMON_MISC" => &["common/common_misc.json", "common/common_misc.png"],
        "MAP_FLAGSHIP_DAMAGE" => &["map/map_flagship_damage.json", "map/map_flagship_damage.png"],
        _ => &[],
    }
}

fn normalize_missing_resource_path(raw: &str) -> String {
    let no_query = raw.split('?').next().unwrap_or(raw);
    if let Some(index) = no_query.find("/kcs2/") {
        no_query[index + 1..].to_string()
    } else {
        no_query.trim_start_matches('/').to_string()
    }
}

fn parse_slot_resource_path(path: &str) -> Option<(String, i64)> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 5 || parts[0] != "kcs2" || parts[1] != "resources" || parts[2] != "slot" {
        return None;
    }
    let target_type = parts[3].to_string();
    let file_name = parts[4];
    let slot_prefix = file_name.split('_').next()?;
    let slot_id = slot_prefix.parse::<i64>().ok()?;
    Some((target_type, slot_id))
}

fn collect_hougeki_slot_ids(
    object: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, BTreeSet<i64>> {
    let mut slot_ids_by_source: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();

    for field in ["api_hougeki1", "api_hougeki2", "api_hougeki3"] {
        let Some(hougeki) = object.get(field).and_then(serde_json::Value::as_object) else {
            continue;
        };
        let Some(si_list_rows) = hougeki.get("api_si_list").and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        let entry = slot_ids_by_source.entry(format!("{field}.api_si_list[*][*]")).or_default();

        for row in si_list_rows {
            let Some(slot_ids) = row.as_array() else {
                continue;
            };
            for slot_id in slot_ids.iter().filter_map(serde_json::Value::as_i64) {
                if slot_id > 0 {
                    entry.insert(slot_id);
                }
            }
        }
    }

    slot_ids_by_source
}

pub(crate) fn append_battle_rule_provider_assets(
    versions: &BTreeMap<String, String>,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    let assets = load_repo_battle_knowledge_assets()?;
    let providers: BTreeSet<&str> = assets
        .resource_rules
        .rules
        .iter()
        .filter_map(|rule| {
            (rule.resource_kind == "texture-provider").then_some(rule.provider.as_deref()).flatten()
        })
        .collect();

    for provider in providers {
        for relative_path in texture_provider_paths(provider) {
            let category = relative_path.split('/').next().unwrap_or_default();
            let version = versions.get(category);
            let full_path = format!("kcs2/img/{relative_path}");
            let version_value = version.cloned().into_version();
            let already_present = list
                .items
                .iter()
                .any(|item| item.path == full_path && item.version == version_value);
            if !already_present {
                list.add(full_path, version);
            }
        }
    }

    Ok(())
}

fn push_missing_field(
    report: &mut BattleValidationReport,
    field: &str,
    message: impl Into<String>,
) {
    report.findings.push(BattleValidationFinding {
        severity: BattleValidationSeverity::Error,
        kind: BattleValidationFindingKind::MissingField,
        field: Some(field.to_string()),
        message: message.into(),
        resource_path: None,
    });
}

fn push_invalid_shape(
    report: &mut BattleValidationReport,
    field: &str,
    message: impl Into<String>,
) {
    report.findings.push(BattleValidationFinding {
        severity: BattleValidationSeverity::Error,
        kind: BattleValidationFindingKind::InvalidFieldShape,
        field: Some(field.to_string()),
        message: message.into(),
        resource_path: None,
    });
}

fn push_warning(
    report: &mut BattleValidationReport,
    kind: BattleValidationFindingKind,
    field: Option<&str>,
    message: impl Into<String>,
    resource_path: Option<String>,
) {
    report.findings.push(BattleValidationFinding {
        severity: BattleValidationSeverity::Warning,
        kind,
        field: field.map(str::to_string),
        message: message.into(),
        resource_path,
    });
}

fn push_error(
    report: &mut BattleValidationReport,
    kind: BattleValidationFindingKind,
    field: Option<&str>,
    message: impl Into<String>,
    resource_path: Option<String>,
) {
    report.findings.push(BattleValidationFinding {
        severity: BattleValidationSeverity::Error,
        kind,
        field: field.map(str::to_string),
        message: message.into(),
        resource_path,
    });
}

fn as_object<'a>(
    value: &'a serde_json::Value,
) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    value.as_object()
}

fn read_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Option<&'a Vec<serde_json::Value>> {
    object.get(field)?.as_array()
}

fn read_i64_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    report: &mut BattleValidationReport,
) -> Option<Vec<i64>> {
    let values = match object.get(field) {
        Some(value) => value,
        None => {
            push_missing_field(report, field, format!("battle response is missing `{field}`"));
            return None;
        }
    };
    let array = match values.as_array() {
        Some(array) => array,
        None => {
            push_invalid_shape(report, field, format!("`{field}` is expected to be an array"));
            return None;
        }
    };

    let mut numbers = Vec::with_capacity(array.len());
    for value in array {
        match value.as_i64() {
            Some(number) => numbers.push(number),
            None => {
                push_invalid_shape(report, field, format!("`{field}` must contain only integers"));
                return None;
            }
        }
    }

    Some(numbers)
}

fn read_i64_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    report: &mut BattleValidationReport,
) -> Option<i64> {
    let value = match object.get(field) {
        Some(value) => value,
        None => {
            push_missing_field(report, field, format!("battle response is missing `{field}`"));
            return None;
        }
    };

    match value.as_i64() {
        Some(number) => Some(number),
        None => {
            push_invalid_shape(report, field, format!("`{field}` is expected to be an integer"));
            None
        }
    }
}

fn array_len(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    report: &mut BattleValidationReport,
) -> Option<usize> {
    let values = match object.get(field) {
        Some(value) => value,
        None => {
            push_missing_field(report, field, format!("battle response is missing `{field}`"));
            return None;
        }
    };
    match values.as_array() {
        Some(array) => Some(array.len()),
        None => {
            push_invalid_shape(report, field, format!("`{field}` is expected to be an array"));
            None
        }
    }
}

fn ensure_required_scalar_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    fields: &[&str],
) {
    for field in fields {
        let _ = read_i64_field(object, field, report);
    }
}

fn ensure_required_array_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    fields: &[&str],
) {
    for field in fields {
        let _ = array_len(object, field, report);
    }
}

fn check_equal_lengths(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    fields: &[&str],
) {
    let mut seen_lengths = BTreeMap::new();
    for field in fields {
        if let Some(length) = array_len(object, field, report) {
            seen_lengths.insert(*field, length);
        }
    }
    if seen_lengths.len() < 2 {
        return;
    }

    let unique_lengths: BTreeSet<usize> = seen_lengths.values().copied().collect();
    if unique_lengths.len() > 1 {
        let detail = seen_lengths
            .iter()
            .map(|(field, length)| format!("{field}={length}"))
            .collect::<Vec<_>>()
            .join(", ");
        push_error(
            report,
            BattleValidationFindingKind::ArrayLengthMismatch,
            None,
            format!("battle response arrays must align: {detail}"),
            None,
        );
    }
}

fn check_array_flag_payload(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    flag_field: &str,
    index: usize,
    payload_field: &str,
) {
    let Some(flag_values) = read_array(object, flag_field) else {
        return;
    };
    let Some(flag_value) = flag_values.get(index).and_then(serde_json::Value::as_i64) else {
        return;
    };
    let payload_present = object.get(payload_field).is_some_and(|value| !value.is_null());

    if flag_value == 0 && payload_present {
        push_warning(
            report,
            BattleValidationFindingKind::FlagPayloadMismatch,
            Some(payload_field),
            format!("`{payload_field}` is present even though `{flag_field}[{index}]` is 0"),
            None,
        );
    } else if flag_value == 1 && !payload_present {
        push_error(
            report,
            BattleValidationFindingKind::FlagPayloadMismatch,
            Some(payload_field),
            format!("`{payload_field}` is missing even though `{flag_field}[{index}]` is 1"),
            None,
        );
    }
}

fn check_scalar_flag_payload(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    flag_field: &str,
    payload_field: &str,
) {
    let Some(flag_value) = read_i64_field(object, flag_field, report) else {
        return;
    };
    let payload_present = object.get(payload_field).is_some_and(|value| !value.is_null());

    if flag_value == 0 && payload_present {
        push_warning(
            report,
            BattleValidationFindingKind::FlagPayloadMismatch,
            Some(payload_field),
            format!("`{payload_field}` is present even though `{flag_field}` is 0"),
            None,
        );
    } else if flag_value == 1 && !payload_present {
        push_error(
            report,
            BattleValidationFindingKind::FlagPayloadMismatch,
            Some(payload_field),
            format!("`{payload_field}` is missing even though `{flag_field}` is 1"),
            None,
        );
    }
}

fn collect_high_confidence_day_payload_fields(assets: &BattleKnowledgeAssets) -> BTreeSet<String> {
    assets
        .protocol_fields
        .fields
        .iter()
        .filter(|field| field.phases.iter().any(|phase| phase == "day"))
        .filter(|field| {
            matches!(field.access_kind.as_str(), "number" | "numArray" | "object" | "objectArray")
        })
        .filter(|field| DAY_BATTLE_PROTOCOL_PAYLOAD_ALLOWLIST.contains(&field.field.as_str()))
        .map(|field| field.field.clone())
        .collect()
}

fn collect_slotitem_target_types(assets: &BattleKnowledgeAssets) -> SlotitemResourceTargets {
    let mut targets = SlotitemResourceTargets::default();

    for target_type in assets
        .resource_rules
        .rules
        .iter()
        .filter_map(|rule| {
            (rule.resource_kind == "slotitem").then_some(rule.target_type.as_deref()).flatten()
        })
        .filter(|target_type| matches!(*target_type, "item_on" | "item_up" | "btxt_flat"))
    {
        match target_type {
            "item_up" => {
                targets.expected.insert(target_type.to_string());
            }
            "item_on" | "btxt_flat" => {
                targets.candidate.insert(target_type.to_string());
            }
            _ => {}
        }
    }

    targets
}

fn classify_damage_state(now_hp: i64, max_hp: i64) -> Option<DamageState> {
    if max_hp <= 0 {
        return None;
    }

    let ratio = 100 * now_hp / max_hp;
    Some(if ratio <= 0 {
        DamageState::Sunk
    } else if ratio <= 25 {
        DamageState::Taiha
    } else if ratio <= 50 {
        DamageState::Chuuha
    } else if ratio <= 75 {
        DamageState::Shouha
    } else {
        DamageState::Healthy
    })
}

fn uses_damaged_ship_resources(state: DamageState) -> bool {
    matches!(state, DamageState::Chuuha | DamageState::Taiha | DamageState::Sunk)
}

fn build_ship_resource_path(
    manifest: &ApiManifest,
    ship_id: i64,
    target_type: &str,
    damaged: bool,
) -> Option<String> {
    let _ = manifest.find_ship(ship_id)?;
    let graph = manifest.find_shipgraph(ship_id)?;
    let category = if target_type == "full" {
        if damaged {
            "full_dmg"
        } else {
            "full"
        }
    } else if damaged {
        "banner_dmg"
    } else {
        "banner"
    };

    let filename = (target_type == "full").then_some(graph.api_filename.as_str());
    Some(SuffixUtils::format_kc2_resource(ship_id as u64, "ship", category, "png", filename))
}

fn build_slotitem_resource_path(slot_id: i64, target_type: &str) -> String {
    let item_id = format!("{slot_id:04}");
    let key = SuffixUtils::create(&item_id, format!("slot_{target_type}").as_str());
    format!("kcs2/resources/slot/{target_type}/{item_id}_{key}.png")
}

fn dedupe_resources(resources: Vec<ExpectedBattleResource>) -> Vec<ExpectedBattleResource> {
    let mut seen = BTreeSet::new();
    resources
        .into_iter()
        .filter(|resource| {
            seen.insert((
                resource.kind.clone(),
                resource.entity_id,
                resource.target_type.clone(),
                resource.path.clone(),
            ))
        })
        .collect()
}

pub fn validate_day_battle_response<T: Serialize>(
    manifest: &ApiManifest,
    response: &T,
    assets: &BattleKnowledgeAssets,
) -> Result<BattleValidationReport, serde_json::Error> {
    let value = serde_json::to_value(response)?;
    let Some(object) = as_object(&value) else {
        let mut report = BattleValidationReport::default();
        push_invalid_shape(
            &mut report,
            "<root>",
            "battle response must serialize to a JSON object",
        );
        return Ok(report);
    };

    let mut report = BattleValidationReport::default();
    let day_payload_fields = collect_high_confidence_day_payload_fields(assets);

    ensure_required_array_fields(object, &mut report, DAY_BATTLE_ARRAY_FIELDS);
    ensure_required_array_fields(object, &mut report, DAY_BATTLE_ARRAY_FLAG_FIELDS);
    ensure_required_scalar_fields(object, &mut report, DAY_BATTLE_SCALAR_FLAG_FIELDS);

    check_equal_lengths(
        object,
        &mut report,
        &["api_ship_ke", "api_ship_lv", "api_e_nowhps", "api_e_maxhps", "api_eSlot", "api_eParam"],
    );
    check_equal_lengths(object, &mut report, &["api_f_nowhps", "api_f_maxhps", "api_fParam"]);

    if day_payload_fields.contains("api_kouku") {
        check_array_flag_payload(object, &mut report, "api_stage_flag", 0, "api_kouku");
    }
    if day_payload_fields.contains("api_opening_atack") {
        check_scalar_flag_payload(object, &mut report, "api_opening_flag", "api_opening_atack");
    }
    if day_payload_fields.contains("api_opening_taisen") {
        check_scalar_flag_payload(
            object,
            &mut report,
            "api_opening_taisen_flag",
            "api_opening_taisen",
        );
    }
    if day_payload_fields.contains("api_hougeki1") {
        check_array_flag_payload(object, &mut report, "api_hourai_flag", 0, "api_hougeki1");
    }
    if day_payload_fields.contains("api_hougeki2") {
        check_array_flag_payload(object, &mut report, "api_hourai_flag", 1, "api_hougeki2");
    }
    if day_payload_fields.contains("api_hougeki3") {
        check_array_flag_payload(object, &mut report, "api_hourai_flag", 2, "api_hougeki3");
    }
    if day_payload_fields.contains("api_raigeki") {
        check_array_flag_payload(object, &mut report, "api_hourai_flag", 3, "api_raigeki");
    }

    let enemy_ship_ids = read_i64_array(object, "api_ship_ke", &mut report).unwrap_or_default();
    let enemy_nowhps = read_i64_array(object, "api_e_nowhps", &mut report).unwrap_or_default();
    let enemy_maxhps = read_i64_array(object, "api_e_maxhps", &mut report).unwrap_or_default();
    let slot_rows = read_array(object, "api_eSlot").cloned().unwrap_or_default();
    let slot_target_types = collect_slotitem_target_types(assets);

    for (index, ship_id) in enemy_ship_ids.iter().copied().enumerate() {
        if ship_id <= 0 {
            continue;
        }

        let damage_state = enemy_nowhps
            .get(index)
            .zip(enemy_maxhps.get(index))
            .and_then(|(now, max)| classify_damage_state(*now, *max));
        let uses_damaged_resources = damage_state.is_some_and(uses_damaged_ship_resources);
        let banner_path =
            build_ship_resource_path(manifest, ship_id, "banner", uses_damaged_resources);
        let full_path = build_ship_resource_path(manifest, ship_id, "full", uses_damaged_resources);

        if manifest.find_ship(ship_id).is_none() {
            push_error(
                &mut report,
                BattleValidationFindingKind::UnknownShipMstId,
                Some("api_ship_ke"),
                format!("enemy ship mst id `{ship_id}` is not present in the manifest"),
                banner_path.clone(),
            );
            continue;
        }

        if manifest.find_shipgraph(ship_id).is_none() {
            push_error(
                &mut report,
                BattleValidationFindingKind::MissingShipGraph,
                Some("api_ship_ke"),
                format!("enemy ship mst id `{ship_id}` has no shipgraph entry"),
                banner_path.clone(),
            );
            continue;
        }

        if let Some(path) = banner_path {
            report.expected_resources.push(ExpectedBattleResource {
                kind: "ship".to_string(),
                entity_id: ship_id,
                target_type: if uses_damaged_resources {
                    "banner_dmg".to_string()
                } else {
                    "banner".to_string()
                },
                path,
                note: "battle banner image expected by ShipBanner".to_string(),
                protocol_source: None,
                consumer_module: Some("ShipBanner".to_string()),
            });
        }
        if let Some(path) = full_path {
            report.expected_resources.push(ExpectedBattleResource {
                kind: "ship".to_string(),
                entity_id: ship_id,
                target_type: if uses_damaged_resources {
                    "full_dmg".to_string()
                } else {
                    "full".to_string()
                },
                path,
                note: "potential cutin preload ship image".to_string(),
                protocol_source: None,
                consumer_module: Some("CutinResourcesPreloadTask".to_string()),
            });
        }
    }

    for slot_row in slot_rows {
        let Some(slot_ids) = slot_row.as_array() else {
            push_invalid_shape(
                &mut report,
                "api_eSlot",
                "`api_eSlot` rows must be arrays of slotitem ids",
            );
            continue;
        };

        for slot_id in slot_ids.iter().filter_map(serde_json::Value::as_i64) {
            if slot_id <= 0 {
                continue;
            }
            if manifest.find_slotitem(slot_id).is_none() {
                push_error(
                    &mut report,
                    BattleValidationFindingKind::UnknownSlotitemMstId,
                    Some("api_eSlot"),
                    format!("enemy slotitem mst id `{slot_id}` is not present in the manifest"),
                    None,
                );
                continue;
            }

            for target_type in slot_target_types.expected.iter() {
                report.expected_resources.push(ExpectedBattleResource {
                    kind: "slotitem".to_string(),
                    entity_id: slot_id,
                    target_type: target_type.to_string(),
                    path: build_slotitem_resource_path(slot_id, target_type),
                    note: "potential battle preload slotitem resource".to_string(),
                    protocol_source: None,
                    consumer_module: None,
                });
            }
            for target_type in slot_target_types.candidate.iter() {
                report.candidate_resources.push(ExpectedBattleResource {
                    kind: "slotitem".to_string(),
                    entity_id: slot_id,
                    target_type: target_type.to_string(),
                    path: build_slotitem_resource_path(slot_id, target_type),
                    note: "lower-confidence battle preload slotitem resource".to_string(),
                    protocol_source: None,
                    consumer_module: None,
                });
            }
        }
    }

    report.expected_resources = dedupe_resources(report.expected_resources);
    report.candidate_resources = dedupe_resources(report.candidate_resources);

    Ok(report)
}

pub fn analyze_day_battle_incident<T: Serialize>(
    manifest: &ApiManifest,
    response: &T,
    assets: &BattleKnowledgeAssets,
    missing_resource_url: Option<&str>,
) -> Result<BattleIncidentReport, serde_json::Error> {
    let value = serde_json::to_value(response)?;
    let validation = validate_day_battle_response(manifest, &value, assets)?;
    let Some(object) = as_object(&value) else {
        return Ok(BattleIncidentReport {
            validation,
            missing_resource_path: missing_resource_url.map(normalize_missing_resource_path),
            ..BattleIncidentReport::default()
        });
    };

    let missing_resource_path = missing_resource_url.map(normalize_missing_resource_path);
    let slot_ids_by_source = collect_hougeki_slot_ids(object);
    let mut report = BattleIncidentReport {
        validation,
        missing_resource_path: missing_resource_path.clone(),
        ..BattleIncidentReport::default()
    };

    let Some(missing_resource_path) = missing_resource_path else {
        return Ok(report);
    };

    let Some((target_type, slot_id)) = parse_slot_resource_path(&missing_resource_path) else {
        return Ok(report);
    };

    let resource_target = format!("slot/{target_type}");
    for trigger in assets
        .slot_resource_triggers
        .triggers
        .iter()
        .filter(|trigger| trigger.resource_target == resource_target)
    {
        for protocol_source in trigger.protocol_sources.iter() {
            if slot_ids_by_source
                .get(protocol_source)
                .is_some_and(|slot_ids| slot_ids.contains(&slot_id))
            {
                report.trigger_matches.push(BattleIncidentTriggerMatch {
                    protocol_source: protocol_source.clone(),
                    consumer_module: trigger.consumer_readable_name.clone(),
                    resource_target: trigger.resource_target.clone(),
                    confidence: trigger.confidence.clone(),
                });
            }
        }
    }

    if report.trigger_matches.is_empty() {
        report.protocol_suspicions.push(BattleValidationFinding {
			severity: BattleValidationSeverity::Warning,
			kind: BattleValidationFindingKind::UnexpectedBattleSlotResource,
			field: Some("api_hougeki*.api_si_list".to_string()),
			message: format!(
				"missing resource `{missing_resource_path}` could not be matched to any known battle slot-resource trigger"
			),
			resource_path: Some(missing_resource_path),
		});
        return Ok(report);
    }

    let slot_name = manifest
        .find_slotitem(slot_id)
        .map(|slotitem| slotitem.api_name.as_str())
        .unwrap_or("<unknown-slotitem>");

    let finding = if target_type == "btxt_flat" && !has_btxt_flat_coverage(slot_id) {
        BattleValidationFinding {
            severity: BattleValidationSeverity::Error,
            kind: BattleValidationFindingKind::ProtocolSuspicion,
            field: Some("api_hougeki*.api_si_list".to_string()),
            message: format!(
                "battle protocol exposed slotitem `{slot_id}` ({slot_name}) through api_hougeki*.api_si_list, which drives `{missing_resource_path}` but bootstrap does not treat this slot as a valid btxt_flat target"
            ),
            resource_path: Some(missing_resource_path),
        }
    } else {
        BattleValidationFinding {
            severity: BattleValidationSeverity::Warning,
            kind: BattleValidationFindingKind::BootstrapGap,
            field: Some("api_hougeki*.api_si_list".to_string()),
            message: format!(
                "battle protocol can drive `{missing_resource_path}` via slotitem `{slot_id}` ({slot_name}); investigate bootstrap coverage for this resource target"
            ),
            resource_path: Some(missing_resource_path),
        }
    };

    if finding.kind == BattleValidationFindingKind::ProtocolSuspicion {
        report.protocol_suspicions.push(finding);
    } else {
        report.bootstrap_gaps.push(finding);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_model::kc2::start2::{ApiMstShip, ApiMstShipgraph, ApiMstSlotitem};

    fn build_day_battle_assets() -> BattleKnowledgeAssets {
        BattleKnowledgeAssets {
            sources: BattleKnowledgeAssetSources {
                protocol_fields: RepoBattleKnowledgeSource::Embedded,
                resource_rules: RepoBattleKnowledgeSource::Embedded,
                module_index: RepoBattleKnowledgeSource::Embedded,
                slot_resource_triggers: RepoBattleKnowledgeSource::Embedded,
            },
            protocol_fields: BattleProtocolFieldsAsset {
                script_version: "test".to_string(),
                summary: BattleProtocolFieldsSummary {
                    module_count: 1,
                    protocol_field_count: 7,
                },
                fields: vec![
                    ("api_kouku", "object"),
                    ("api_opening_atack", "object"),
                    ("api_opening_taisen", "objectArray"),
                    ("api_hougeki1", "objectArray"),
                    ("api_hougeki2", "objectArray"),
                    ("api_hougeki3", "objectArray"),
                    ("api_raigeki", "object"),
                ]
                .into_iter()
                .map(|(field, access_kind)| BattleProtocolFieldRule {
                    id: format!("83034:{field}:{access_kind}:this._o"),
                    module_id: "83034".to_string(),
                    readable_name: "RawDayBattleData".to_string(),
                    field: field.to_string(),
                    access_kind: access_kind.to_string(),
                    source_object: Some("this._o".to_string()),
                    conditional: false,
                    phases: vec!["day".to_string()],
                })
                .collect(),
            },
            resource_rules: BattleResourceRulesAsset {
                script_version: "test".to_string(),
                summary: BattleResourceRulesSummary {
                    module_count: 1,
                    resource_rule_count: 3,
                    explicit_resource_path_count: 0,
                    ship_resource_rule_count: 1,
                    slotitem_resource_rule_count: 2,
                    texture_provider_rule_count: 0,
                },
                rules: vec![
                    BattleResourceRule {
                        id: "ship".to_string(),
                        module_id: "1".to_string(),
                        readable_name: "ShipBanner".to_string(),
                        resource_kind: "ship".to_string(),
                        action: "getShip".to_string(),
                        target_type: Some("banner".to_string()),
                        provider: None,
                        texture_ids: vec![],
                        ship_mst_id_source: Some("ship".to_string()),
                        damaged_source: Some("ship.isDamaged()".to_string()),
                        slot_mst_id_sources: vec![],
                        explicit_paths: vec![],
                        trigger_hints: vec!["battle".to_string()],
                    },
                    BattleResourceRule {
                        id: "slot-up".to_string(),
                        module_id: "1".to_string(),
                        readable_name: "CutinResourcesPreloadTask".to_string(),
                        resource_kind: "slotitem".to_string(),
                        action: "getSlotitem".to_string(),
                        target_type: Some("item_up".to_string()),
                        provider: None,
                        texture_ids: vec![],
                        ship_mst_id_source: None,
                        damaged_source: None,
                        slot_mst_id_sources: vec!["slot".to_string()],
                        explicit_paths: vec![],
                        trigger_hints: vec!["battle".to_string()],
                    },
                    BattleResourceRule {
                        id: "slot-btxt".to_string(),
                        module_id: "1".to_string(),
                        readable_name: "CutinResourcesPreloadTask".to_string(),
                        resource_kind: "slotitem".to_string(),
                        action: "getSlotitem".to_string(),
                        target_type: Some("btxt_flat".to_string()),
                        provider: None,
                        texture_ids: vec![],
                        ship_mst_id_source: None,
                        damaged_source: None,
                        slot_mst_id_sources: vec!["slot".to_string()],
                        explicit_paths: vec![],
                        trigger_hints: vec!["battle".to_string(), "cutin".to_string()],
                    },
                ],
            },
            module_index: BattleModuleIndexAsset {
                script_version: "test".to_string(),
                summary: BattleModuleIndexSummary {
                    module_count: 1,
                    protocol_field_count: 7,
                    resource_rule_count: 3,
                },
                modules: vec![],
            },
            slot_resource_triggers: BattleSlotResourceTriggersAsset {
                script_version: "test".to_string(),
                summary: BattleSlotResourceTriggersSummary {
                    module_count: 1,
                    slot_resource_trigger_count: 1,
                },
                triggers: vec![BattleSlotResourceTrigger {
                    id: "trigger".to_string(),
                    consumer_module_id: "69595".to_string(),
                    consumer_readable_name: "CutinCanvasSpRDJ".to_string(),
                    protocol_sources: vec!["api_hougeki1.api_si_list[*][*]".to_string()],
                    resource_target: "slot/btxt_flat".to_string(),
                    confidence: "high".to_string(),
                    notes: "Cutin text consumer".to_string(),
                }],
            },
        }
    }

    fn build_manifest_with_enemy() -> ApiManifest {
        let mut manifest = ApiManifest::default();
        manifest.api_mst_ship.push(ApiMstShip {
            api_id: 1501,
            ..ApiMstShip::default()
        });
        manifest.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 1501,
            api_filename: "enemy_test".to_string(),
            ..ApiMstShipgraph::default()
        });
        manifest.api_mst_slotitem.push(ApiMstSlotitem {
            api_id: 42,
            ..ApiMstSlotitem::default()
        });
        manifest.api_mst_slotitem.push(ApiMstSlotitem {
            api_id: 102,
            api_name: "航空特別増加食".to_string(),
            ..ApiMstSlotitem::default()
        });
        manifest
    }

    fn build_valid_day_battle_response() -> serde_json::Value {
        serde_json::json!({
            "api_ship_ke": [1501],
            "api_ship_lv": [1],
            "api_e_nowhps": [75],
            "api_e_maxhps": [100],
            "api_eSlot": [[42, -1, -1, -1, -1]],
            "api_eParam": [[1, 1, 1, 1]],
            "api_f_nowhps": [20],
            "api_f_maxhps": [20],
            "api_fParam": [[1, 1, 1, 1]],
            "api_stage_flag": [1, 0, 0],
            "api_kouku": {"ok": true},
            "api_opening_taisen_flag": 0,
            "api_opening_flag": 0,
            "api_hourai_flag": [1, 0, 0, 0],
            "api_hougeki1": [{"ok": true}]
        })
    }

    #[test]
    fn load_repo_battle_knowledge_assets_prefers_filesystem_contents() {
        let root = tempfile::tempdir().unwrap();
        let protocol_path = root.path().join("battle_protocol_fields.json");
        std::fs::write(
			&protocol_path,
			r#"{"scriptVersion":"x","summary":{"moduleCount":1,"protocolFieldCount":1},"fields":[]}"#,
		)
		.unwrap();

        let (source, raw) = load_text_asset_from(&protocol_path, "{}").unwrap();
        assert_eq!(source, RepoBattleKnowledgeSource::Filesystem(protocol_path));
        assert_eq!(
            raw,
            r#"{"scriptVersion":"x","summary":{"moduleCount":1,"protocolFieldCount":1},"fields":[]}"#
        );
    }

    #[test]
    fn load_text_asset_from_falls_back_to_embedded() {
        let (source, raw) =
            load_text_asset_from(Path::new("/definitely/missing.json"), "{\"ok\":true}").unwrap();
        assert_eq!(source, RepoBattleKnowledgeSource::Embedded);
        assert_eq!(raw, "{\"ok\":true}");
    }

    #[test]
    fn validate_day_battle_response_reports_unknown_enemy_ship_and_slotitem() {
        let assets = BattleKnowledgeAssets {
            sources: BattleKnowledgeAssetSources {
                protocol_fields: RepoBattleKnowledgeSource::Embedded,
                resource_rules: RepoBattleKnowledgeSource::Embedded,
                module_index: RepoBattleKnowledgeSource::Embedded,
                slot_resource_triggers: RepoBattleKnowledgeSource::Embedded,
            },
            protocol_fields: BattleProtocolFieldsAsset {
                script_version: "test".to_string(),
                summary: BattleProtocolFieldsSummary {
                    module_count: 0,
                    protocol_field_count: 0,
                },
                fields: vec![],
            },
            resource_rules: BattleResourceRulesAsset {
                script_version: "test".to_string(),
                summary: BattleResourceRulesSummary {
                    module_count: 0,
                    resource_rule_count: 0,
                    explicit_resource_path_count: 0,
                    ship_resource_rule_count: 0,
                    slotitem_resource_rule_count: 1,
                    texture_provider_rule_count: 0,
                },
                rules: vec![BattleResourceRule {
                    id: "slot".to_string(),
                    module_id: "1".to_string(),
                    readable_name: "CutinResourcesPreloadTask".to_string(),
                    resource_kind: "slotitem".to_string(),
                    action: "getSlotitem".to_string(),
                    target_type: Some("item_up".to_string()),
                    provider: None,
                    texture_ids: vec![],
                    ship_mst_id_source: None,
                    damaged_source: None,
                    slot_mst_id_sources: vec!["slot".to_string()],
                    explicit_paths: vec![],
                    trigger_hints: vec!["battle".to_string()],
                }],
            },
            module_index: BattleModuleIndexAsset {
                script_version: "test".to_string(),
                summary: BattleModuleIndexSummary {
                    module_count: 0,
                    protocol_field_count: 0,
                    resource_rule_count: 0,
                },
                modules: vec![],
            },
            slot_resource_triggers: BattleSlotResourceTriggersAsset {
                script_version: "test".to_string(),
                summary: BattleSlotResourceTriggersSummary {
                    module_count: 0,
                    slot_resource_trigger_count: 0,
                },
                triggers: vec![],
            },
        };

        let response = serde_json::json!({
            "api_ship_ke": [999999],
            "api_ship_lv": [1],
            "api_e_nowhps": [10],
            "api_e_maxhps": [10],
            "api_eSlot": [[888888, -1, -1, -1, -1]],
            "api_eParam": [[1, 1, 1, 1]],
            "api_f_nowhps": [20],
            "api_f_maxhps": [20],
            "api_fParam": [[1, 1, 1, 1]],
            "api_stage_flag": [0, 0, 0],
            "api_hourai_flag": [0, 0, 0, 0],
            "api_kouku": null,
            "api_opening_atack": null,
            "api_hougeki1": null,
            "api_hougeki2": null,
            "api_raigeki": null
        });
        let report =
            validate_day_battle_response(&ApiManifest::default(), &response, &assets).unwrap();

        assert!(report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == BattleValidationFindingKind::UnknownShipMstId)
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == BattleValidationFindingKind::UnknownSlotitemMstId)
        );
    }

    #[test]
    fn append_battle_rule_provider_assets_adds_known_versioned_paths() {
        let mut list = CacheList::new();
        let versions = BTreeMap::from([
            ("battle".to_string(), "1".to_string()),
            ("battle_result".to_string(), "2".to_string()),
            ("common".to_string(), "3".to_string()),
            ("map".to_string(), "4".to_string()),
        ]);

        append_battle_rule_provider_assets(&versions, &mut list).unwrap();

        assert!(list.items.iter().any(|item| item.path == "kcs2/img/battle/battle_main.json"));
        assert!(list.items.iter().any(|item| item.path == "kcs2/img/common/common_misc.png"));
    }

    #[test]
    fn append_battle_rule_provider_assets_skips_existing_entries() {
        let mut list = CacheList::new();
        let versions = BTreeMap::from([("battle".to_string(), "1".to_string())]);
        list.add("kcs2/img/battle/battle_main.json".to_string(), Some("1".to_string()));
        let before_len = list.items.len();

        append_battle_rule_provider_assets(&versions, &mut list).unwrap();

        let after_matches = list
            .items
            .iter()
            .filter(|item| item.path == "kcs2/img/battle/battle_main.json")
            .count();
        assert_eq!(after_matches, 1);
        assert!(list.items.len() >= before_len);
    }

    #[test]
    fn validate_day_battle_response_checks_opening_and_hourai_payload_flags() {
        let assets = build_day_battle_assets();
        let manifest = build_manifest_with_enemy();
        let mut response = build_valid_day_battle_response();
        response["api_opening_flag"] = serde_json::json!(1);
        response["api_hourai_flag"][2] = serde_json::json!(1);

        let report = validate_day_battle_response(&manifest, &response, &assets).unwrap();

        assert!(report.has_errors());
        assert!(report.findings.iter().any(|finding| {
            finding.kind == BattleValidationFindingKind::FlagPayloadMismatch
                && finding.field.as_deref() == Some("api_opening_atack")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.kind == BattleValidationFindingKind::FlagPayloadMismatch
                && finding.field.as_deref() == Some("api_hougeki3")
        }));
    }

    #[test]
    fn validate_day_battle_response_uses_main_js_damage_thresholds() {
        let assets = build_day_battle_assets();
        let manifest = build_manifest_with_enemy();
        let report =
            validate_day_battle_response(&manifest, &build_valid_day_battle_response(), &assets)
                .unwrap();

        assert!(!report.has_errors());
        assert!(report.expected_resources.iter().any(|resource| {
            resource.kind == "ship"
                && resource.entity_id == 1501
                && resource.target_type == "banner"
                && resource.path.contains("/banner/")
        }));
        assert!(!report.expected_resources.iter().any(|resource| {
            resource.kind == "ship"
                && resource.entity_id == 1501
                && resource.target_type == "banner_dmg"
        }));
    }

    #[test]
    fn validate_day_battle_response_separates_expected_and_candidate_slot_resources() {
        let assets = build_day_battle_assets();
        let manifest = build_manifest_with_enemy();
        let report =
            validate_day_battle_response(&manifest, &build_valid_day_battle_response(), &assets)
                .unwrap();

        assert!(report.expected_resources.iter().any(|resource| {
            resource.kind == "slotitem"
                && resource.entity_id == 42
                && resource.target_type == "item_up"
        }));
        assert!(report.candidate_resources.iter().any(|resource| {
            resource.kind == "slotitem"
                && resource.entity_id == 42
                && resource.target_type == "btxt_flat"
        }));
    }

    #[test]
    fn analyze_day_battle_incident_flags_missing_btxt_flat_as_protocol_suspicion() {
        let assets = build_day_battle_assets();
        let manifest = build_manifest_with_enemy();
        let payload = serde_json::json!({
            "api_atoll_cell": 0,
            "api_balloon_cell": 0,
            "api_deck_id": 1,
            "api_eParam": [[6,16,6,7],[6,16,6,7]],
            "api_eSlot": [[1502,513,-1,-1,-1],[1502,513,-1,-1,-1]],
            "api_e_effect_list": [[0],[0]],
            "api_e_maxhps": [24,24],
            "api_e_nowhps": [24,24],
            "api_fParam": [[15,27,16,6],[104,0,36,75],[0,5,33,29]],
            "api_f_maxhps": [15,80,71],
            "api_f_nowhps": [15,80,71],
            "api_formation": [1,1,2],
            "api_hougeki1": {
                "api_at_eflag": [0,0],
                "api_at_list": [0,1],
                "api_at_type": [0,0],
                "api_cl_list": [[1],[1]],
                "api_damage": [[11],[13]],
                "api_df_list": [[0],[0]],
                "api_si_list": [[2,147],[102,8]]
            },
            "api_hourai_flag": [1,0,0,0],
            "api_kouku": {
                "api_plane_from":[[3],[]],
                "api_stage1":{"api_disp_seiku":1,"api_e_count":0,"api_e_lostcount":0,"api_f_count":99,"api_f_lostcount":0,"api_touch_plane":[-1,-1]},
                "api_stage2":{"api_e_count":0,"api_e_lostcount":0,"api_f_count":99,"api_f_lostcount":2},
                "api_stage3":{"api_e_sp_list":[null,null],"api_ebak_flag":[0,1],"api_ecl_flag":[0,1],"api_edam":[0,24],"api_erai_flag":[0,1],"api_f_sp_list":[null,null,null],"api_fbak_flag":[0,0,0],"api_fcl_flag":[0,0,0],"api_fdam":[0,0,0],"api_frai_flag":[0,0,0]}
            },
            "api_midnight_flag": 0,
            "api_opening_flag": 0,
            "api_opening_taisen_flag": 0,
            "api_search": [1,1],
            "api_ship_ke": [1501,1501],
            "api_ship_lv": [7,7],
            "api_smoke_type": 0,
            "api_stage_flag": [1,1,1]
        });

        let report = analyze_day_battle_incident(
			&manifest,
			&payload,
			&assets,
			Some(
				"http://w18i.kancolle-server.com/kcs2/resources/slot/btxt_flat/0102_8293.png?version=3",
			),
		)
		.unwrap();

        assert!(report.protocol_suspicions.iter().any(|finding| {
            finding.kind == BattleValidationFindingKind::ProtocolSuspicion
                && finding.resource_path.as_deref()
                    == Some("kcs2/resources/slot/btxt_flat/0102_8293.png")
        }));
        assert!(report.trigger_matches.iter().any(|trigger| {
            trigger.protocol_source == "api_hougeki1.api_si_list[*][*]"
                && trigger.resource_target == "slot/btxt_flat"
        }));
    }
}
