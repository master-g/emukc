//! Which attack-type values each client dispatch stage accepts.
//!
//! The decoded `battle_attack_type_acceptance.json` asset plus the R2 check
//! that reads it. Split out of `battle_rules` because the acceptance sets are
//! sourced and checked on their own, independent of payload shape.

use super::*;

/// Where a consumer sends attack-type values its own dispatch does not name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAttackTypeFallback {
    pub readable_name: String,
    pub module_ids: Vec<String>,
    pub accepted_values: Vec<i64>,
    pub closed: bool,
}

/// One dispatch stage: the client module that consumes an attack-type field,
/// and every value it is willing to dispatch. Decoded from `main.js`; the
/// client is the only source for what it accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAttackTypeStage {
    pub id: String,
    pub protocol_field: String,
    pub protocol_sources: Vec<String>,
    pub consumer_readable_name: String,
    pub consumer_module_ids: Vec<String>,
    pub accepted_values: Vec<i64>,
    pub fallback: Option<BattleAttackTypeFallback>,
    pub effective_accepted_values: Vec<i64>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAttackTypeAcceptanceSummary {
    pub attack_type_stage_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAttackTypeAcceptanceAsset {
    pub script_version: String,
    pub summary: BattleAttackTypeAcceptanceSummary,
    pub stages: Vec<BattleAttackTypeStage>,
}

impl BattleAttackTypeAcceptanceAsset {
    /// The stage whose id matches, or `None` when this bundle has no such
    /// consumer.
    pub fn stage(&self, id: &str) -> Option<&BattleAttackTypeStage> {
        self.stages.iter().find(|stage| stage.id == id)
    }
}

/// The attack-type values carried by one shelling phase. The phase is an
/// object in every response the server emits; the array form is tolerated so a
/// malformed payload produces a shape finding rather than a panic here.
pub(super) fn collect_attack_type_values(
    phase: &serde_json::Value,
    value_field: &str,
) -> Vec<serde_json::Value> {
    let read = |object: &serde_json::Value| -> Vec<serde_json::Value> {
        object.get(value_field).and_then(serde_json::Value::as_array).cloned().unwrap_or_default()
    };
    match phase {
        serde_json::Value::Array(entries) => entries.iter().flat_map(read).collect(),
        serde_json::Value::Object(_) => read(phase),
        _ => Vec::new(),
    }
}

/// R2: every attack-type value must be one the stage's consumer module will
/// dispatch. The acceptance sets come from `main.js` (see
/// `battle_attack_type_acceptance.json`); a value outside them either throws in
/// the client or plays the wrong animation, so it is an error.
pub(super) fn check_attack_type_acceptance(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    assets: &BattleKnowledgeAssets,
    phase_field: &str,
    stage_id: &str,
    value_field: &str,
) {
    let Some(stage) = assets.attack_type_acceptance.stage(stage_id) else {
        // An asset that cannot answer the question must not read as "nothing to
        // check": that is the silent pass this whole check exists to remove.
        push_error(
            report,
            BattleValidationFindingKind::BootstrapGap,
            Some(phase_field),
            format!(
                "battle knowledge carries no `{stage_id}` dispatch stage, so `{value_field}` cannot be checked"
            ),
            None,
        );
        return;
    };
    let Some(phase) = object.get(phase_field).filter(|phase| !phase.is_null()) else {
        return;
    };

    for value in collect_attack_type_values(phase, value_field) {
        let Some(attack_type) = value.as_i64() else {
            continue;
        };
        if stage.effective_accepted_values.contains(&attack_type) {
            continue;
        }
        push_error(
            report,
            BattleValidationFindingKind::UnacceptedAttackType,
            Some(&format!("{phase_field}.{value_field}")),
            format!(
                "`{value_field}` value `{attack_type}` is not dispatched by `{}` ({stage_id}); it accepts {:?}",
                stage.consumer_readable_name, stage.effective_accepted_values
            ),
            None,
        );
    }
}
